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

use crate::{auxv::AuxVecKey, ILoadExecutable, LinuxLoader, LoadError, ProcessContext};

const SHEBANG_MAX_LEN: usize = 127;

impl<'a> LinuxLoader<'a> {
    /// Create a LinuxLoader by parsing a script's shebang (#!) and loading its interpreter as an ELF.
    ///
    /// This reads up to SHEBANG_MAX_LEN+1 bytes from `data`, parses the first line as a shebang to
    /// extract the interpreter path and its arguments, constructs a ProcessContext with an argv that
    /// places the interpreter, its shebang arguments, and the original `path` (the script) in the
    /// expected order, marks the image as non-ELF in auxv, and then opens and loads the interpreter
    /// via the filesystem as the next executable.
    ///
    /// Returns a configured LinuxLoader on success.
    ///
    /// Errors returned include (but are not limited to):
    /// - LoadError::FailedToLoad: if reading the header from `data` fails.
    /// - LoadError::NotShebang: if the file does not begin with a valid `#!` shebang line.
    /// - LoadError::CanNotFindInterpreter: if the interpreter path from the shebang cannot be opened.
    /// - Other LoadError variants propagated from ELF loading when the interpreter is opened.
    ///
    /// # Examples
    ///
    /// ```
    /// // Pseudocode example showing intended use:
    /// // let loader = LinuxLoader::from_shebang(&script_file, "/usr/bin/myscript", fs_tree, mmu, alloc)?;
    /// ```
    pub fn from_shebang(
        data: &impl ILoadExecutable,
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

    /// Returns true if the byte is a line terminator (newline, carriage return) or a null terminator.
    ///
    /// This is used to detect the end of a shebang line when parsing the first bytes of a file.
    ///
    /// # Examples
    ///
    /// ```
    /// assert!(is_end(&b'\n'));
    /// assert!(is_end(&b'\r'));
    /// assert!(is_end(&b'\0'));
    /// assert!(!is_end(&b'a'));
    /// ```
    fn is_end(&c: &u8) -> bool {
        c == b'\n' || c == b'\r' || c == b'\0'
    }

    /// Parse a shebang header and extract the interpreter path and its arguments.
    ///
    /// Returns the interpreter file path and a vector of argument tokens (each as `Cow<'a, str>`)
    /// on success. If the buffer does not begin with `#!` this returns `LoadError::NotShebang`.
    /// The end of the shebang line is determined by `is_end` (newline, carriage return, or NUL);
    /// if no terminator is found the whole buffer after `#!` is treated as the shebang content.
    /// Further validation and UTF-8 conversion errors are reported by `split_file_arg` as `LoadError`.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::borrow::Cow;
    /// let (file, args) = LinuxLoader::parse_header(b"#!/bin/sh -x\n").unwrap();
    /// assert_eq!(file, "/bin/sh");
    /// assert_eq!(args, vec![Cow::from("-x")]);
    /// ```
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

    /// Parse a shebang line fragment into the interpreter path and its argument tokens.
    ///
    /// Splits the provided byte slice at the first ASCII space into a file path and the remainder
    /// (arguments). Both parts are validated as UTF-8 and trimmed of surrounding whitespace. The
    /// argument string is then split on spaces into non-empty tokens and returned as a `Vec<Cow<'a, str>>`.
    ///
    /// Returns `LoadError::InvalidShebangString` if either the file or argument byte sequences are not valid UTF-8.
    ///
    /// # Examples
    ///
    /// ```
    /// let content = b"/usr/bin/env python -u";
    /// let (file, args) = split_file_arg(content).unwrap();
    /// assert_eq!(file, "/usr/bin/env");
    /// assert_eq!(args, vec![Cow::from("python"), Cow::from("-u")]);
    /// ```
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

    /// Insert interpreter and script entries into a process's argv and mark the image as non-ELF.
    ///
    /// This function prepends the interpreter `file`, followed by the provided `argv` elements,
    /// and finally `script` to the front of `ctx.argv`. It reserves capacity to avoid
    /// unnecessary reallocations and rotates the vector so the new entries appear at the beginning.
    /// After updating argv, it sets the auxiliary vector entry `AT_NOTELF = 1` to indicate the
    /// loaded file is not an ELF binary.
    ///
    /// Returns Ok(()) on success.
    ///
    /// # Examples
    ///
    /// ```
    /// // Construct a minimal ProcessContext (assumes Default is implemented).
    /// let mut ctx = ProcessContext::default();
    /// let interp = "/bin/sh";
    /// let shebang_args: Vec<std::borrow::Cow<'_, str>> = vec![std::borrow::Cow::from("-x")];
    /// let script_path = "/tmp/myscript";
    ///
    /// // Pre-populate ctx.argv to simulate original argv (e.g., script name).
    /// ctx.argv.push("/tmp/original".into());
    ///
    /// // Insert interpreter, its args, and the script into the context.
    /// push_ctx(&mut ctx, interp, &shebang_args, script_path).unwrap();
    ///
    /// // After insertion, argv now begins with the interpreter and its args,
    /// // and the auxiliary vector contains AT_NOTELF = 1.
    /// assert_eq!(ctx.argv[0], interp);
    /// assert_eq!(ctx.argv[1], "-x");
    /// assert_eq!(ctx.argv[2], script_path);
    /// assert_eq!(ctx.auxv.get(&AuxVecKey::AT_NOTELF), Some(&1));
    /// ```
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

    /// Loads the interpreter specified by a shebang line and delegates loading to `from_elf`.
    ///
    /// This attempts to open `file` from the provided filesystem `fs`. If the file is found,
    /// the interpreter is loaded as an ELF by calling `Self::from_elf` with the updated
    /// process `ctx`, `mmu`, and `alloc`.
    ///
    /// Errors:
    /// - Returns `LoadError::CanNotFindInterpreter` if the interpreter file cannot be opened.
    /// - Any error from `Self::from_elf` is propagated.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// // Given a parsed shebang with `file` and an updated `ctx`, load the interpreter:
    /// // let loader = LinuxLoader::load_shebang_script(ctx, "/bin/sh", fs, mmu, alloc)?;
    /// ```
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
        /// Helper used by unit tests to validate shebang parsing and context updates.
        ///
        /// Parses a shebang header, updates a fresh `ProcessContext` with the interpreter,
        /// its arguments, and the script path, then asserts that `ctx.argv` matches
        /// `expected` and that the `AT_NOTELF` auxv entry is set to `1`.
        ///
        /// Parameters:
        /// - `header`: raw bytes containing the shebang line to parse (may include trailing bytes).
        /// - `script_arg`: initial argv entries that should already be present in the context (e.g., existing script args).
        /// - `script`: path to the script being executed (appended as the final argv element).
        /// - `expected`: expected full argv after `push_ctx` has been applied.
        ///
        /// # Examples
        ///
        /// ```
        /// let header = b"#!/bin/sh -x\n";
        /// let script_args = &["existing"];
        /// let script = "/tmp/script.sh";
        /// let expected = &["/bin/sh", "-x", "existing", "/tmp/script.sh"];
        /// update_ctx_testcase(header, script_args, script, expected);
        /// ```
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

    fn create_given_len_str(str: &str, len: usize, char: u8) -> Vec<u8> {
        let mut s = vec![char; len];
        let str = str.as_bytes();

        s[..str.len()].copy_from_slice(str);
        s
    }
}
