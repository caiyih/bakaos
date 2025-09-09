use alloc::{borrow::Cow, vec::Vec};

use crate::{auxv::AuxVec, LoadError};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProcessContextLimit {
    pub argv: usize,
    pub envp: usize,
}

impl ProcessContextLimit {
    #[allow(non_upper_case_globals)]
    pub const Unlimited: ProcessContextLimit = ProcessContextLimit {
        argv: usize::MAX,
        envp: usize::MAX,
    };
}

pub struct ProcessContext<'a> {
    pub argv: Vec<Cow<'a, str>>,
    pub envp: Vec<Cow<'a, str>>,
    pub auxv: AuxVec,
    pub limit: ProcessContextLimit,
}

impl ProcessContext<'_> {
    /// Creates a new, empty ProcessContext with unlimited argv/envp limits.
    ///
    /// The returned context contains empty `argv` and `envp` vectors and an empty `auxv`.
    ///
    /// # Examples
    ///
    /// ```
    /// use linux_loader::ProcessContext;
    ///
    /// let ctx = ProcessContext::new();
    /// assert!(ctx.argv.is_empty());
    /// assert!(ctx.envp.is_empty());
    /// ```
    pub fn new() -> Self {
        Self {
            argv: Vec::new(),
            envp: Vec::new(),
            auxv: AuxVec::new(),
            limit: ProcessContextLimit::Unlimited,
        }
    }

    /// Creates an empty `ProcessContext` constrained by the given length limits.
    ///
    /// The returned context has empty `argv`, `envp`, and a new empty `auxv`.
    /// The provided `limit` controls the maximum number of entries allowed when
    /// extending `argv` and `envp` (used by `extend_argv` / `extend_envp`).
    ///
    /// # Examples
    ///
    /// ```
    /// use linux_loader::{ProcessContext, ProcessContextLimit};
    ///
    /// let limit = ProcessContextLimit { argv: 2, envp: 4 };
    /// let ctx = ProcessContext::new_limited(limit);
    /// assert!(ctx.argv.is_empty());
    /// assert!(ctx.envp.is_empty());
    /// assert_eq!(ctx.limit.argv, 2);
    /// assert_eq!(ctx.limit.envp, 4);
    /// ```
    pub fn new_limited(limit: ProcessContextLimit) -> Self {
        Self {
            argv: Vec::new(),
            envp: Vec::new(),
            auxv: AuxVec::new(),
            limit,
        }
    }
}

impl<'a> ProcessContext<'a> {
    /// Appends the provided argument strings to this context's argv, enforcing the configured argv length limit.
    ///
    /// If appending would make the total number of arguments exceed `self.limit.argv`, the method returns
    /// `Err(LoadError::ArgumentCountExceeded)` and does not modify `self.argv`.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::borrow::Cow;
    /// use linux_loader::ProcessContext;
    ///
    /// let mut ctx = ProcessContext::new();
    /// let args = [Cow::Borrowed("arg1"), Cow::Borrowed("arg2")];
    /// ctx.extend_argv(&args).unwrap();
    /// assert_eq!(ctx.argv.len(), 2);
    /// ```
    pub fn extend_argv(&mut self, argv: &[Cow<'a, str>]) -> Result<(), LoadError> {
        if argv.len() + self.argv.len() > self.limit.argv {
            return Err(LoadError::ArgumentCountExceeded);
        }

        self.argv.extend_from_slice(argv);

        Ok(())
    }

    /// Appends the given environment entries to this context, enforcing the configured envp limit.
    ///
    /// Extends `self.envp` with the provided slice. If the total number of environment
    /// entries would exceed `self.limit.envp`, the method returns
    /// `Err(LoadError::EnvironmentCountExceeded)` and does not modify the context.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::borrow::Cow;
    /// use linux_loader::{ProcessContext, LoadError};
    ///
    /// let mut ctx = ProcessContext::new();
    /// let envs: &[Cow<str>] = &[Cow::Borrowed("KEY=VAL")];
    /// ctx.extend_envp(envs).unwrap();
    /// assert_eq!(ctx.envp.len(), 1);
    /// ```
    pub fn extend_envp(&mut self, envp: &[Cow<'a, str>]) -> Result<(), LoadError> {
        if envp.len() + self.envp.len() > self.limit.envp {
            return Err(LoadError::EnvironmentCountExceeded);
        }

        self.envp.extend_from_slice(envp);

        Ok(())
    }

    /// Merge entries from another `AuxVec` into this context's auxiliary vector.
    ///
    /// Entries from `auxv` are inserted into `self.auxv`. If `overwrite` is `false`,
    /// existing keys in `self.auxv` are preserved and corresponding entries from
    /// `auxv` are skipped; if `overwrite` is `true`, incoming values replace existing ones.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::vec::Vec;
    /// use linux_loader::auxv::{AuxVec, AuxVecKey};
    /// use linux_loader::ProcessContext;
    ///
    /// let mut ctx = ProcessContext::new();
    /// let mut other = AuxVec::default();
    /// other.insert(AuxVecKey::AT_ENTRY, 0x1000);
    ///
    /// // Insert entries when overwrite is allowed
    /// ctx.extend_auxv(&other, true);
    /// assert_eq!(ctx.auxv.get(&AuxVecKey::AT_ENTRY), Some(&0x1000));
    /// ```
    pub fn extend_auxv(&mut self, auxv: &AuxVec, overwrite: bool) {
        for (key, value) in auxv.iter() {
            if !overwrite && self.auxv.contains_key(key) {
                continue;
            }

            self.auxv.insert(*key, *value);
        }
    }

    /// Merge another `ProcessContext` into `self`.
    ///
    /// Appends `other`'s `argv` and `envp`, enforcing `self.limit` (returns a
    /// `LoadError` if either would exceed its configured limit). Merges `other`'s
    /// `auxv` entries; when `override_auxv` is `true` entries from `other` replace
    /// existing keys, otherwise existing entries are preserved.
    ///
    /// # Errors
    ///
    /// Returns `Err(LoadError::ArgumentCountExceeded)` if the combined `argv` would
    /// exceed `self.limit.argv`, or `Err(LoadError::EnvironmentCountExceeded)` if
    /// the combined `envp` would exceed `self.limit.envp`.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::borrow::Cow;
    /// use linux_loader::ProcessContext;
    ///
    /// let mut base = ProcessContext::new();
    /// let mut other = ProcessContext::new();
    ///
    /// base.extend_argv(&[Cow::Borrowed("arg1")]).unwrap();
    /// other.extend_argv(&[Cow::Borrowed("arg2")]).unwrap();
    ///
    /// base.merge(&other, true).unwrap();
    /// assert_eq!(base.argv.len(), 2);
    /// ```
    pub fn merge(
        &mut self,
        other: &ProcessContext<'a>,
        override_auxv: bool,
    ) -> Result<(), LoadError> {
        self.extend_argv(&other.argv)?;
        self.extend_envp(&other.envp)?;
        self.extend_auxv(&other.auxv, override_auxv);

        Ok(())
    }
}

impl Default for ProcessContext<'_> {
    /// Returns an empty ProcessContext with no argv/envp entries and unlimited length limits.
    ///
    /// The created context has empty `argv` and `envp`, a default `AuxVec`, and `ProcessContextLimit::Unlimited`.
    ///
    /// # Examples
    ///
    /// ```
    /// use linux_loader::{ProcessContext, ProcessContextLimit};
    ///
    /// let ctx = ProcessContext::default();
    /// assert!(ctx.argv.is_empty());
    /// assert!(ctx.envp.is_empty());
    /// assert_eq!(ctx.limit, ProcessContextLimit::Unlimited);
    /// ```
    fn default() -> Self {
        ProcessContext {
            argv: Vec::new(),
            envp: Vec::new(),
            auxv: AuxVec::default(),
            limit: ProcessContextLimit::Unlimited,
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
        let mut ctx = ProcessContext::new_limited(ProcessContextLimit { argv: 5, envp: 5 });

        assert_eq!(
            ctx.extend_argv(&vec![Cow::from(""); 10]),
            Err(LoadError::ArgumentCountExceeded)
        );

        assert_eq!(
            ctx.extend_envp(&vec![Cow::from(""); 10]),
            Err(LoadError::EnvironmentCountExceeded)
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

        given.merge(&ctx, true).unwrap();

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

        given.merge(&ctx, false).unwrap();

        assert_eq!(given.auxv.get(&AuxVecKey::AT_ENTRY), Some(&0x1000));
        assert_eq!(given.auxv.get(&AuxVecKey::AT_NULL), Some(&0));
        assert_eq!(given.auxv.get(&AuxVecKey::AT_NOTELF), Some(&1));
    }
}
