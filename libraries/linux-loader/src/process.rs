use alloc::{borrow::Cow, vec::Vec};

use crate::auxv::AuxVec;

#[derive(Debug, Clone, Copy)]
pub struct ProcessContextLengthLimit {
    pub argv: usize,
    pub envp: usize,
}

impl ProcessContextLengthLimit {
    #[allow(non_upper_case_globals)]
    pub const Unlimited: ProcessContextLengthLimit = ProcessContextLengthLimit {
        argv: usize::MAX,
        envp: usize::MAX,
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessContextError {
    ArgumentCountExceeded,
    EnvironmentCountExceeded,
}

#[allow(clippy::from_over_into)]
impl Into<&'static str> for ProcessContextError {
    fn into(self) -> &'static str {
        match self {
            ProcessContextError::ArgumentCountExceeded => "Argument count exceeded",
            ProcessContextError::EnvironmentCountExceeded => "Environment count exceeded",
        }
    }
}

pub struct ProcessContext<'a> {
    pub argv: Vec<Cow<'a, str>>,
    pub envp: Vec<Cow<'a, str>>,
    pub auxv: AuxVec,
    pub limit: ProcessContextLengthLimit,
}

impl ProcessContext<'_> {
    pub fn new() -> Self {
        Self {
            argv: Vec::new(),
            envp: Vec::new(),
            auxv: AuxVec::new(),
            limit: ProcessContextLengthLimit::Unlimited,
        }
    }

    pub fn new_limited(limit: ProcessContextLengthLimit) -> Self {
        Self {
            argv: Vec::new(),
            envp: Vec::new(),
            auxv: AuxVec::new(),
            limit,
        }
    }
}

impl<'a> ProcessContext<'a> {
    /// Extends the argv with the given argv.
    pub fn extend_argv(&mut self, argv: &[Cow<'a, str>]) -> Result<(), ProcessContextError> {
        if argv.len() + self.argv.len() > self.limit.argv {
            return Err(ProcessContextError::ArgumentCountExceeded);
        }

        self.argv.extend_from_slice(argv);

        Ok(())
    }

    /// Extends the envp with the given envp.
    pub fn extend_envp(&mut self, envp: &[Cow<'a, str>]) -> Result<(), ProcessContextError> {
        if envp.len() + self.envp.len() > self.limit.envp {
            return Err(ProcessContextError::EnvironmentCountExceeded);
        }

        self.envp.extend_from_slice(envp);

        Ok(())
    }

    /// Extends the auxv with the given auxv.
    ///
    /// If `overwrite` is `true`, the existing auxv entries will be overwritten.
    pub fn extend_auxv(&mut self, auxv: AuxVec, overwrite: bool) {
        for (key, value) in auxv.iter() {
            if !overwrite && self.auxv.contains_key(key) {
                continue;
            }

            self.auxv.insert(*key, *value);
        }
    }

    pub fn merge(
        &mut self,
        other: ProcessContext<'a>,
        override_auxv: bool,
    ) -> Result<(), ProcessContextError> {
        self.extend_argv(&other.argv)?;
        self.extend_envp(&other.envp)?;
        self.extend_auxv(other.auxv, override_auxv);

        Ok(())
    }
}

impl Default for ProcessContext<'_> {
    fn default() -> Self {
        ProcessContext {
            argv: Vec::new(),
            envp: Vec::new(),
            auxv: AuxVec::default(),
            limit: ProcessContextLengthLimit::Unlimited,
        }
    }
}

#[cfg(test)]
mod tests {
    use alloc::vec;

    use crate::auxv::AuxVecKey;

    use super::*;

    #[test]
    fn test_length_limit() {
        let mut ctx = ProcessContext::new_limited(ProcessContextLengthLimit { argv: 5, envp: 5 });

        assert_eq!(
            ctx.extend_argv(&vec![Cow::from(""); 10]),
            Err(ProcessContextError::ArgumentCountExceeded)
        );

        assert_eq!(
            ctx.extend_envp(&vec![Cow::from(""); 10]),
            Err(ProcessContextError::EnvironmentCountExceeded)
        );
    }

    #[test]
    fn test_merge_context() {
        let mut given = ProcessContext::new();
        given
            .extend_argv(&[Cow::from("arg1"), Cow::from("arg2")])
            .unwrap();
        given
            .extend_envp(&[Cow::from("env1"), Cow::from("env2")])
            .unwrap();
        given.auxv.insert(AuxVecKey::AT_ENTRY, 0x1000);

        let mut ctx = ProcessContext::new();
        ctx.extend_argv(&[Cow::from("arg3"), Cow::from("arg4")])
            .unwrap();
        ctx.extend_envp(&[Cow::from("env3"), Cow::from("env4")])
            .unwrap();
        ctx.auxv.insert(AuxVecKey::AT_ENTRY, 0x2000);

        given.merge(ctx, true).unwrap();

        assert_eq!(
            given.argv,
            vec![
                Cow::from("arg1"),
                Cow::from("arg2"),
                Cow::from("arg3"),
                Cow::from("arg4")
            ]
        );
        assert_eq!(
            given.envp,
            vec![
                Cow::from("env1"),
                Cow::from("env2"),
                Cow::from("env3"),
                Cow::from("env4")
            ]
        );
        assert_eq!(given.auxv.get(&AuxVecKey::AT_ENTRY), Some(&0x2000));
    }

    #[test]
    fn test_auxv_entry_overwrite() {
        let mut given = ProcessContext::new();

        given.auxv.insert(AuxVecKey::AT_NULL, 0);
        given.auxv.insert(AuxVecKey::AT_ENTRY, 0x1000);
        given.auxv.insert(AuxVecKey::AT_NOTELF, 1);

        let mut ctx = ProcessContext::new();
        ctx.auxv.insert(AuxVecKey::AT_ENTRY, 0x2000);
        ctx.auxv.insert(AuxVecKey::AT_NULL, 0x3000);
        ctx.auxv.insert(AuxVecKey::AT_NOTELF, 0);

        given.merge(ctx, false).unwrap();

        assert_eq!(given.auxv.get(&AuxVecKey::AT_ENTRY), Some(&0x1000));
        assert_eq!(given.auxv.get(&AuxVecKey::AT_NULL), Some(&0));
        assert_eq!(given.auxv.get(&AuxVecKey::AT_NOTELF), Some(&1));
    }
}
