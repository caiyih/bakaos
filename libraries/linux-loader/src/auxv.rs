use core::ops::{Deref, DerefMut};

use alloc::{collections::btree_map::BTreeMap, vec::Vec};

#[derive(Debug, Default, Clone)]
pub struct AuxVec {
    map: BTreeMap<AuxVecKey, usize>,
}

impl AuxVec {
    /// Creates a new, empty AuxVec.
    ///
    /// # Examples
    ///
    /// ```
    /// let aux = AuxVec::new();
    /// assert!(aux.is_empty());
    /// ```
    pub const fn new() -> Self {
        Self {
            map: BTreeMap::new(),
        }
    }

    /// Collect the auxiliary vector entries in descending key order.
    ///
    /// Returns a Vec of AuxVecEntry built from the internal map by iterating
    /// the entries in reverse (largest key first). If `AT_NULL` is present it
    /// will appear as the last element in the returned vector.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut auxv = AuxVec::new();
    /// auxv.insert(AuxVecKey::AT_ENTRY, 0x1000);
    /// auxv.insert(AuxVecKey::AT_NULL, 0);
    /// let entries = auxv.collect();
    /// assert_eq!(entries.iter().last().unwrap().key, AuxVecKey::AT_NULL);
    /// ```
    pub fn collect(&self) -> Vec<AuxVecEntry> {
        self.map
            .iter()
            .rev()
            .map(|(k, v)| AuxVecEntry { key: *k, value: *v })
            .collect()
    }
}

impl Deref for AuxVec {
    type Target = BTreeMap<AuxVecKey, usize>;
    /// Returns a shared reference to the underlying BTreeMap, allowing read-only map-like access to the auxiliary vector.
    ///
    /// # Examples
    ///
    /// ```
    /// let auxv = AuxVec::new();
    /// // Access the map through deref to read entries:
    /// let map_ref: &alloc::collections::btree_map::BTreeMap<AuxVecKey, usize> = auxv.deref();
    /// assert!(map_ref.is_empty());
    /// ```
    fn deref(&self) -> &Self::Target {
        &self.map
    }
}

impl DerefMut for AuxVec {
    /// Returns a mutable reference to the underlying `BTreeMap`, allowing direct mutation of the auxiliary-vector entries.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut auxv = AuxVec::new();
    /// auxv.deref_mut().insert(AuxVecKey::AT_ENTRY, 0x1000);
    /// assert_eq!(auxv.get(&AuxVecKey::AT_ENTRY), Some(&0x1000));
    /// ```
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.map
    }
}

#[repr(usize)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[allow(non_camel_case_types)]
pub enum AuxVecKey {
    /// End of vector
    AT_NULL = 0,
    /// Entry should be ignored
    AT_IGNORE = 1,
    /// File descriptor of program
    AT_EXECFD = 2,
    /// Program headers for program
    AT_PHDR = 3,
    /// Size of program header entry
    AT_PHENT = 4,
    /// Number of program headers
    AT_PHNUM = 5,
    /// System page size
    AT_PAGESZ = 6,
    /// Base address of interpreter
    AT_BASE = 7,
    /// Flags
    AT_FLAGS = 8,
    /// Entry point of program
    AT_ENTRY = 9,
    /// Program is not ELF
    AT_NOTELF = 10,
    /// Real uid
    AT_UID = 11,
    /// Effective uid
    AT_EUID = 12,
    /// Real gid
    AT_GID = 13,
    /// Effective gid
    AT_EGID = 14,
    /// String identifying CPU for optimizations
    AT_PLATFORM = 15,
    /// Arch dependent hints at CPU capabilities
    AT_HWCAP = 16,
    /// Frequency at which times() increments
    AT_CLKTCK = 17,
    /// Secure mode boolean
    AT_SECURE = 23,
    /// String identifying real platform, may differ from AT_PLATFORM.
    AT_BASE_PLATFORM = 24,
    /// Address of 16 random bytes
    AT_RANDOM = 25,
    /// Extension of AT_HWCAP
    AT_HWCAP2 = 26,
    /// Filename of program
    AT_EXECFN = 31,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct AuxVecEntry {
    pub key: AuxVecKey,
    pub value: usize,
}

impl AuxVecEntry {
    /// Create a new `AuxVecEntry` with the given key and value.
    ///
    /// # Examples
    ///
    /// ```
    /// let entry = AuxVecEntry::new(AuxVecKey::AT_ENTRY, 0x1000);
    /// assert_eq!(entry.key, AuxVecKey::AT_ENTRY);
    /// assert_eq!(entry.value, 0x1000);
    /// ```
    pub const fn new(key: AuxVecKey, val: usize) -> Self {
        AuxVecEntry { key, value: val }
    }
}

#[derive(Debug, Default, Clone)]
pub struct AuxVecValues<'a> {
    pub random: Option<[u8; 16]>,
    pub platform: Option<&'a str>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[allow(non_snake_case)]
    fn test_auxv_AT_NULL_is_last() {
        let mut auxv = AuxVec::new();

        auxv.insert(AuxVecKey::AT_ENTRY, 0x1000);
        auxv.insert(AuxVecKey::AT_NULL, 0);
        auxv.insert(AuxVecKey::AT_ENTRY, 0x1000);

        let auxv_entries: Vec<AuxVecEntry> = auxv.collect();

        assert_eq!(auxv_entries.iter().last().unwrap().key, AuxVecKey::AT_NULL);
    }
}
