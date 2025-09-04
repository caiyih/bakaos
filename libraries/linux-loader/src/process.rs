use alloc::{borrow::Cow, vec::Vec};

use crate::{auxv::AuxVec, LoadError};

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

pub struct ProcessContext<'a> {
    pub argv: Vec<Cow<'a, str>>,
    pub envp: Vec<Cow<'a, str>>,
    pub auxv: AuxVec,
    pub limit: ProcessContextLengthLimit,
}

impl ProcessContext<'_> {
    /// Creates a new, empty ProcessContext with unlimited argument and environment limits.
    ///
    /// The returned context has empty `argv` and `envp`, a default `AuxVec`, and
    /// `ProcessContextLengthLimit::Unlimited`.
    ///
    /// # Examples
    ///
    /// ```
    /// let ctx = ProcessContext::new();
    /// assert!(ctx.argv.is_empty());
    /// assert!(ctx.envp.is_empty());
    /// ```
    pub fn new() -> Self {
        Self {
            argv: Vec::new(),
            envp: Vec::new(),
            auxv: AuxVec::new(),
            limit: ProcessContextLengthLimit::Unlimited,
        }
    }

    /// Creates a new `ProcessContext` with empty `argv`, `envp`, and `auxv`, using the provided length `limit`.
    ///
    /// The returned context has no arguments, no environment variables, and an empty auxiliary vector.
    /// Use `ProcessContextLengthLimit::Unlimited` to create an unrestricted context.
    ///
    /// # Examples
    ///
    /// ```
    /// let limit = ProcessContextLengthLimit { argv: 4, envp: 8 };
    /// let ctx = ProcessContext::new_limited(limit);
    /// assert!(ctx.argv.is_empty());
    /// assert!(ctx.envp.is_empty());
    /// assert!(ctx.auxv.is_empty());
    /// assert_eq!(ctx.limit.argv, 4);
    /// assert_eq!(ctx.limit.envp, 8);
    /// ```
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
    /// Appends the provided `argv` entries to this context's argument list, enforcing the configured limit.
    ///
    /// Returns `Err(LoadError::ArgumentCountExceeded)` if the combined number of arguments would exceed
    /// `self.limit.argv`.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut ctx = ProcessContext::new();
    /// let args = [std::borrow::Cow::from("one"), std::borrow::Cow::from("two")];
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

    /// Append environment entries to the context, enforcing the configured environment limit.
    ///
    /// Returns `Ok(())` if the entries were appended. Returns `Err(LoadError::EnvironmentCountExceeded)`
    /// if adding the provided entries would exceed `self.limit.envp`.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::borrow::Cow;
    /// # use crate::process::{ProcessContext, ProcessContextLengthLimit};
    /// # use crate::load::LoadError;
    /// let mut ctx: ProcessContext<'static> = ProcessContext::new();
    /// ctx.extend_envp(&[Cow::Borrowed("KEY=value")]).unwrap();
    /// assert_eq!(ctx.envp.len(), 1);
    /// ```
    pub fn extend_envp(&mut self, envp: &[Cow<'a, str>]) -> Result<(), LoadError> {
        if envp.len() + self.envp.len() > self.limit.envp {
            return Err(LoadError::EnvironmentCountExceeded);
        }

        self.envp.extend_from_slice(envp);

        Ok(())
    }

    /// Merge entries from another `AuxVec` into this context's `auxv`.
    ///
    /// Existing keys are preserved unless `overwrite` is `true`, in which case
    /// values from `auxv` replace current values for matching keys. This method
    /// mutates `self.auxv` in place.
    ///
    /// # Examples
    ///
    /// ```
    /// use crate::auxv::{AuxVec, AuxKey};
    /// let mut ctx = crate::process::ProcessContext::new();
    /// let mut other = AuxVec::new();
    /// other.insert(AuxKey::AT_ENTRY, 0x1000);
    /// ctx.extend_auxv(&other, false);
    /// assert_eq!(ctx.auxv.get(&AuxKey::AT_ENTRY), Some(&0x1000));
    /// ```
    pub fn extend_auxv(&mut self, auxv: &AuxVec, overwrite: bool) {
        for (key, value) in auxv.iter() {
            if !overwrite && self.auxv.contains_key(key) {
                continue;
            }

            self.auxv.insert(*key, *value);
        }
    }

    /// Merges another ProcessContext into this one.
    ///
    /// Appends `other`'s argv and envp to `self`, enforcing this context's length limits.
    /// Merges `other.auxv` into `self.auxv`; if `override_auxv` is true, existing auxv entries in
    /// `self` will be replaced by entries from `other`, otherwise existing entries are preserved.
    ///
    /// # Parameters
    ///
    /// * `override_auxv` — when true, keys from `other.auxv` overwrite matching keys in `self.auxv`.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` on success. Returns `Err(LoadError::ArgumentCountExceeded)` if extending
    /// argv would exceed this context's argv limit, or `Err(LoadError::EnvironmentCountExceeded)`
    /// if extending envp would exceed this context's envp limit.
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::borrow::Cow;
    /// # use crate::process::{ProcessContext, ProcessContextLengthLimit};
    /// # use crate::auxv::AuxVec;
    /// // create two contexts
    /// let mut a = ProcessContext::new();
    /// let mut b = ProcessContext::new();
    /// a.extend_argv(&[Cow::Borrowed("a1")]).unwrap();
    /// a.extend_envp(&[Cow::Borrowed("A=1")]).unwrap();
    /// b.extend_argv(&[Cow::Borrowed("b1")]).unwrap();
    /// b.extend_envp(&[Cow::Borrowed("B=1")]).unwrap();
    /// // merge b into a without overwriting auxv entries
    /// a.merge(&b, false).unwrap();
    /// assert_eq!(a.argv.iter().map(|s| s.as_ref()).collect::<Vec<_>>(), vec!["a1", "b1"]);
    /// assert_eq!(a.envp.iter().map(|s| s.as_ref()).collect::<Vec<_>>(), vec!["A=1", "B=1"]);
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
    /// Returns an empty ProcessContext with unlimited length limits.
    ///
    /// The resulting context has no argv or envp entries, a default AuxVec, and
    /// `ProcessContextLengthLimit::Unlimited` as its limit.
    ///
    /// # Examples
    ///
    /// ```
    /// let ctx = ProcessContext::default();
    /// assert!(ctx.argv.is_empty() && ctx.envp.is_empty());
    /// ```
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
