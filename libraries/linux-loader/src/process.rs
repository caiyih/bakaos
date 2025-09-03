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

pub enum ProcessContextError {
    ArgumentCountExceeded,
    EnvironmentCountExceeded,
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
    pub fn extend_argv(&mut self, argv: Vec<Cow<'a, str>>) -> Result<(), ProcessContextError> {
        if argv.len() + self.argv.len() > self.limit.argv {
            return Err(ProcessContextError::ArgumentCountExceeded);
        }

        self.argv.extend(argv);

        Ok(())
    }

    /// Extends the envp with the given envp.
    pub fn extend_envp(&mut self, envp: Vec<Cow<'a, str>>) -> Result<(), ProcessContextError> {
        if envp.len() + self.envp.len() > self.limit.envp {
            return Err(ProcessContextError::EnvironmentCountExceeded);
        }

        self.envp.extend(envp);

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
}
