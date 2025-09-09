use alloc::{
    borrow::{Cow, ToOwned},
    sync::Arc,
    vec,
    vec::Vec,
};
use allocation_abstractions::IFrameAllocator;
use filesystem_abstractions::DirectoryTreeNode;
use hermit_sync::SpinMutex;
use mmu_abstractions::IMMU;

use crate::{auxv::AuxVecKey, IExecSource, LinuxLoader, LoadError, ProcessContext};

const SHEBANG_MAX_LEN: usize = 127;

impl<'a> LinuxLoader<'a> {
    /// Load an executable that starts with a shebang (#!) by parsing the interpreter and delegating to the interpreter's ELF loader.
    ///
    /// Attempts to read up to SHEBANG_MAX_LEN+1 bytes from `data` at offset 0, parse a shebang line to extract the interpreter path and its arguments, update a new ProcessContext so the interpreter sees the script path and args as argv, mark the binary as non-ELF, and then open and load the interpreter as an ELF.
    ///
    /// On success returns a LinuxLoader for the interpreter executable. Returns a `LoadError` for failures such as:
    /// - `LoadError::NotShebang` when the data does not start with `#!`
    /// - `LoadError::FailedToLoad` when reading the header fails
    /// - `LoadError::InvalidShebangString` when the header contains invalid UTF-8
    /// - `LoadError::CanNotFindInterpreter` when the interpreter file cannot be opened
    ///
    /// Note: `fs`, `mmu`, and `alloc` are passed through to the ELF loader and are intentionally not documented here as common service/client parameters.
    ///
    /// # Examples
    ///
    /// ```
    /// // Pseudocode example showing intended use; actual construction of `data`, `fs`, `mmu`, and `alloc` depends on test harness.
    /// // let loader = LinuxLoader::from_shebang(&executable_data, "/tmp/script.sh", fs, &mmu, &alloc)?;
    /// ```
    pub fn from_shebang(
        data: &impl IExecSource,
        path: &str,
        fs: Arc<DirectoryTreeNode>,
        mmu: &Arc<SpinMutex<dyn IMMU>>,
        alloc: &Arc<SpinMutex<dyn IFrameAllocator>>,
    ) -> Result<Self, LoadError> {
        let mut ctx = ProcessContext::default();

        let mut header = [0u8; SHEBANG_MAX_LEN + 2];
        let len = data
            .read_at(0, &mut header)
            .map_err(|_| LoadError::FailedToLoad)?;

        let (file, arg) = Self::parse_header(&header[..len])?;

        Self::push_ctx(&mut ctx, file, &arg, path)?;

        Self::load_shebang_script(ctx, file, fs, mmu, alloc)
    }

    /// Returns true if the given byte represents end-of-line or string termination.
    ///
    /// Recognized terminators: newline (`\n`), carriage return (`\r`), and NUL (`\0`).
    fn is_end(&c: &u8) -> bool {
        c == b'\n' || c == b'\r' || c == b'\0'
    }

    /// Parse a shebang header and extract the interpreter path and its arguments.
    ///
    /// Returns the interpreter file path and a vector of arguments (as `Cow<str>`).
    /// Returns `LoadError::NotShebang` if the slice does not start with `#!`.
    /// Other `LoadError` variants may be returned if parsing/UTF-8 decoding fails.
    fn parse_header(header: &[u8]) -> Result<(&str, Vec<Cow<'a, str>>), LoadError> {
        if header.len() < 2 || &header[..2] != b"#!" {
            return Err(LoadError::NotShebang);
        }

        let end_idx = match header.iter().position(Self::is_end) {
            Some(idx) => idx,
            None => header.len(), // Assume the whole line is the shebang
        };

        Self::split_file_arg(&header[2..end_idx])
    }

    /// Splits a shebang line fragment into the interpreter path and its arguments.
    ///
    /// The input `content` is expected to contain the interpreter path optionally
    /// followed by a space-separated argument string (no leading "#!"). This
    /// function:
    /// - Splits at the first ASCII space to separate the file from the args.
    /// - Validates both parts as UTF-8 and trims surrounding whitespace.
    /// - Splits the args on spaces, ignores empty segments, and returns them as
    ///   `Vec<Cow<'a, str>>`.
    ///
    /// Returns `Err(LoadError::InvalidShebangString)` if either the file or args
    /// are not valid UTF-8.
    fn split_file_arg(content: &[u8]) -> Result<(&str, Vec<Cow<'a, str>>), LoadError> {
        let index = content.iter().position(|b| *b == b' ');
        let (file, args) = match index {
            Some(index) => (&content[..index], &content[index + 1..]),
            None => (content, [].as_slice()),
        };

        let file = core::str::from_utf8(file)
            .map_err(|_| LoadError::InvalidShebangString)?
            .trim();
        let args = core::str::from_utf8(args)
            .map_err(|_| LoadError::InvalidShebangString)?
            .trim();

        let args = args
            .split(' ')
            .skip_while(|s| s.is_empty())
            .map(|s| Cow::from(s.to_owned()))
            .collect::<Vec<_>>();

        Ok((file, args))
    }

    /// Prepend the interpreter and its arguments to the process context's argv and mark the binary as non-ELF.
    ///
    /// The resulting argv will have the interpreter `file` followed by `argv` elements, then `script`,
    /// then the original entries that were previously in `ctx.argv`.
    /// Also inserts AT_NOTELF=1 into `ctx.auxv` to indicate the image being executed is not an ELF binary.
    fn push_ctx(
        ctx: &mut ProcessContext<'a>,
        file: &str,
        argv: &[Cow<'a, str>],
        script: &str,
    ) -> Result<(), LoadError> {
        let mut to_insert = vec![Cow::from(file.to_owned())];
        to_insert.extend_from_slice(argv);
        to_insert.push(Cow::from(script.to_owned()));

        // to_insert's length is limited, since the whole shebang's length is limited
        // but the given argv may be vary long. Here we try to avoid reallocate the given argv
        // by reserving enough space in advance and using rotate to avoid reallocation
        ctx.argv.reserve(to_insert.len());
        ctx.argv.extend_from_slice(&to_insert);
        ctx.argv.rotate_right(to_insert.len());

        // This executable is not an ELF executable
        ctx.auxv.insert(AuxVecKey::AT_NOTELF, 1);

        Ok(())
    }

    /// Loads the interpreter specified by a shebang and delegates to `from_elf` to load it as an ELF executable.
    ///
    /// Attempts to open `file` from the provided filesystem `fs`; if opening fails, returns
    /// `LoadError::CanNotFindInterpreter`. On success, forwards the opened interpreter, the
    /// interpreter path, the updated `ProcessContext`, and the provided `mmu`/`alloc` handles to
    /// `Self::from_elf` and returns its result.
    ///
    /// # Errors
    ///
    /// Returns `LoadError::CanNotFindInterpreter` when the interpreter file cannot be opened by `fs`.
    fn load_shebang_script(
        ctx: ProcessContext<'a>,
        file: &str,
        fs: Arc<DirectoryTreeNode>,
        mmu: &Arc<SpinMutex<dyn IMMU>>,
        alloc: &Arc<SpinMutex<dyn IFrameAllocator>>,
    ) -> Result<Self, LoadError> {
        let interpreter = fs
            .open(file, None)
            .map_err(|_| LoadError::CanNotFindInterpreter)?;

        Self::from_elf(&interpreter, file, ctx, mmu, alloc)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shebang_detection() {
        fn shebang_detection_testcase(s: &[u8], valid: bool) {
            assert_eq!(
                LinuxLoader::parse_header(s).is_ok(),
                valid,
                "{:?}, {:?}",
                s,
                core::str::from_utf8(s)
            );
        }

        shebang_detection_testcase(b"#!/bin/sh", true);
        shebang_detection_testcase(b"#!/bin/sh ", true);
        shebang_detection_testcase(b"#!/bin/sh \n", true);
        shebang_detection_testcase(b"#!/bin/sh \r\n", true);
        shebang_detection_testcase(b"#!/bin/busybox sh", true);
        shebang_detection_testcase(b"#!/bin/busybox sh \n", true);
        shebang_detection_testcase(b"#!/bin/busybox sh \r\nsome content", true);

        shebang_detection_testcase(&[0x45, 0x7f, 0x46, 0x4c], false); // elf magic number
        shebang_detection_testcase(b"", false);
        shebang_detection_testcase(b"#", false);
        shebang_detection_testcase(b"!#/bin/sh ", false);
        shebang_detection_testcase(
            // `#!/bin/sh`` with some invalid suffix
            &[
                0x21, 0x23, 0x62, 0x2f, 0x6e, 0x69, 0x73, 0x2f, 0x0a, 0x68, 0x00, 0x00, 0x01, 0x10,
            ],
            false,
        );
    }

    #[test]
    fn test_context_update() {
        fn update_ctx_testcase(
            header: &[u8],
            script_arg: &[&str],
            script: &str,
            expected: &[&str],
        ) {
            let mut ctx = ProcessContext::new();

            ctx.extend_argv(
                &script_arg
                    .iter()
                    .map(|s| Cow::from(s.to_owned()))
                    .collect::<Vec<_>>(),
            )
            .unwrap();

            let (file, argv) = LinuxLoader::parse_header(header).unwrap();

            LinuxLoader::push_ctx(&mut ctx, file, &argv, script).unwrap();

            assert_eq!(
                ctx.argv,
                expected.iter().map(|s| Cow::from(*s)).collect::<Vec<_>>()
            );

            assert_eq!(ctx.auxv.get(&AuxVecKey::AT_NOTELF).unwrap(), &1);
        }

        update_ctx_testcase(
            b"#!/bin/sh",
            &[],
            "/home/script.sh",
            &["/bin/sh", "/home/script.sh"],
        );
        update_ctx_testcase(
            b"#!/bin/busybox sh",
            &[],
            "/home/script.sh",
            &["/bin/busybox", "sh", "/home/script.sh"],
        );

        update_ctx_testcase(
            b"#!/bin/sh -x    \r\nSome content",
            &[],
            "/home/script.sh",
            &["/bin/sh", "-x", "/home/script.sh"],
        );

        update_ctx_testcase(
            b"#!/bin/sh -x    \r\nSome content",
            &["arg0", "arg1", "arg2"],
            "/tmp/script.sh",
            &["/bin/sh", "-x", "/tmp/script.sh", "arg0", "arg1", "arg2"],
        );

        for ending in [b' ', b'\n', b'\r', 0] {
            update_ctx_testcase(
                &create_given_len_str("#!/bin/sh sh", 1024, ending),
                &[],
                "/home/script.sh",
                &["/bin/sh", "sh", "/home/script.sh"],
            );
        }
    }

    /// Creates a fixed-length byte vector of size `len`, fills it with `char`, and copies `str` bytes into the beginning.
    ///
    /// The returned `Vec<u8>` contains `str.as_bytes()` starting at index 0; any remaining bytes are set to `char`.
    /// Panics if `str.len() > len`.
    ///
    /// # Examples
    ///
    /// ```
    /// let v = create_given_len_str("hi", 5, b'.');
    /// assert_eq!(v, b"hi...");
    /// ```
    fn create_given_len_str(str: &str, len: usize, char: u8) -> Vec<u8> {
        let mut s = vec![char; len];
        let str = str.as_bytes();

        s[..str.len()].copy_from_slice(str);
        s
    }
}
