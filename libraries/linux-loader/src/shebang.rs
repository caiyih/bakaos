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
    pub fn from_shebang(
        data: &impl ILoadExecutable,
        path: &str,
        fs: Arc<DirectoryTreeNode>,
        mmu: &Arc<SpinMutex<dyn IMMU>>,
        alloc: &Arc<SpinMutex<dyn IFrameAllocator>>,
    ) -> Result<Self, LoadError<'a>> {
        let mut ctx = ProcessContext::default();

        let mut header = [0u8; SHEBANG_MAX_LEN + 2];
        let len = data.read_at(0, &mut header).map_err(LoadError::new)?;

        let (file, arg) = Self::parse_header(&header[..len]).map_err(LoadError::new)?;

        Self::push_ctx(&mut ctx, file, &arg, path).map_err(LoadError::new)?;

        Self::load_shebang_script(ctx, file, fs, mmu, alloc)
    }

    fn is_end(&c: &u8) -> bool {
        c == b'\n' || c == b'\r' || c == b'\0'
    }

    fn parse_header(header: &[u8]) -> Result<(&str, Vec<Cow<'a, str>>), &'static str> {
        if header.len() < 2 || &header[..2] != b"#!" {
            return Err("Not a shebang");
        }

        let end_idx = match header.iter().position(Self::is_end) {
            Some(idx) => idx,
            None => header.len(), // Assume the whole line is the shebang
        };

        Self::split_file_arg(&header[2..end_idx])
    }

    fn split_file_arg(content: &[u8]) -> Result<(&str, Vec<Cow<'a, str>>), &'static str> {
        let index = content.iter().position(|b| *b == b' ');
        let (file, args) = match index {
            Some(index) => (&content[..index], &content[index + 1..]),
            None => (content, [].as_slice()),
        };

        let file = core::str::from_utf8(file)
            .map_err(|_| "Invalid shebang string")?
            .trim();
        let args = core::str::from_utf8(args)
            .map_err(|_| "Invalid shebang string")?
            .trim();

        let args = args
            .split(' ')
            .skip_while(|s| s.is_empty())
            .map(|s| Cow::from(s.to_owned()))
            .collect::<Vec<_>>();

        Ok((file, args))
    }

    fn push_ctx(
        ctx: &mut ProcessContext<'a>,
        file: &str,
        argv: &[Cow<'a, str>],
        script: &str,
    ) -> Result<(), &'static str> {
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

    fn load_shebang_script(
        ctx: ProcessContext<'a>,
        file: &str,
        fs: Arc<DirectoryTreeNode>,
        mmu: &Arc<SpinMutex<dyn IMMU>>,
        alloc: &Arc<SpinMutex<dyn IFrameAllocator>>,
    ) -> Result<Self, LoadError<'a>> {
        let interpreter = fs
            .open(file, None)
            .map_err(|_| LoadError::new("No such file"))?;

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

    fn create_given_len_str(str: &str, len: usize, char: u8) -> Vec<u8> {
        let mut s = vec![char; len];
        let str = str.as_bytes();

        s[..str.len()].copy_from_slice(str);
        s
    }
}
