//! Auxiliary Vector related types

use core::ops::{Deref, DerefMut};

use alloc::{collections::btree_map::BTreeMap, vec::Vec};

/// Represents a key value pair collection of auxiliary vector entries.
/// It provides methods for inserting and retrieving auxiliary vector entries.
///
/// # Examples
///
/// ```
/// use linux_loader::auxv::{AuxVec, AuxVecKey};
///
/// let mut aux = AuxVec::new();
/// aux.insert(AuxVecKey::AT_ENTRY, 0x1000);
/// aux.insert(AuxVecKey::AT_NULL, 0);
/// assert_eq!(aux.get(&AuxVecKey::AT_ENTRY), Some(&0x1000));
/// assert_eq!(aux.get(&AuxVecKey::AT_NULL), Some(&0));
/// ```
#[derive(Debug, Default, Clone)]
pub struct AuxVec {
    map: BTreeMap<AuxVecKey, usize>,
}

impl AuxVec {
    /// Creates a new, empty AuxVec.
    ///
    /// This constructor is `const` and returns an AuxVec with an empty underlying map.
    ///
    /// # Examples
    ///
    /// ```
    /// use linux_loader::auxv::AuxVec;
    ///
    /// let aux = AuxVec::new();
    /// assert!(aux.is_empty());
    /// ```
    pub const fn new() -> Self {
        Self {
            map: BTreeMap::new(),
        }
    }

    /// Collects the auxiliary vector entries into a Vec in reverse map order.
    ///
    /// The returned Vec contains `AuxVecEntry` items produced from the internal map;
    /// iterating in reverse ensures `AT_NULL`, if present, appears as the last element.
    ///
    /// # Examples
    ///
    /// ```
    /// use linux_loader::auxv::{AuxVec, AuxVecKey};
    ///
    /// let mut aux = AuxVec::new();
    /// aux.insert(AuxVecKey::AT_ENTRY, 0x1000);
    /// aux.insert(AuxVecKey::AT_NULL, 0);
    /// let entries = aux.collect();
    /// assert_eq!(entries.last().unwrap().key, AuxVecKey::AT_NULL);
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
    /// Returns a shared reference to the underlying `BTreeMap`, allowing `AuxVec` to be used like a map.
    ///
    /// This enables method calls that expect `&BTreeMap<AuxVecKey, usize>` (for example `get`, `contains_key`, iteration).
    ///
    /// # Examples
    ///
    /// ```
    /// use linux_loader::auxv::{AuxVec, AuxVecKey};
    ///
    /// let mut aux = AuxVec::new();
    /// aux.insert(AuxVecKey::AT_ENTRY, 0x1000);
    /// // `deref` is called implicitly so we can call `get` as if `aux` were a BTreeMap
    /// assert_eq!(aux.get(&AuxVecKey::AT_ENTRY), Some(&0x1000));
    /// ```
    fn deref(&self) -> &Self::Target {
        &self.map
    }
}

impl DerefMut for AuxVec {
    /// Returns a mutable reference to the underlying `BTreeMap<AuxVecKey, usize>`,
    /// allowing the `AuxVec` to be used like a map (e.g., insert, remove, clear).
    ///
    /// # Examples
    ///
    /// ```
    /// use linux_loader::auxv::{AuxVec, AuxVecKey};
    ///
    /// let mut aux = AuxVec::new();
    /// aux.insert(AuxVecKey::AT_ENTRY, 0x1000);
    /// assert_eq!(aux.get(&AuxVecKey::AT_ENTRY), Some(&0x1000));
    /// ```
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.map
    }
}

/// Represents an auxiliary vector entry key.
///
/// The `AuxVecKey` enum defines the keys used in the auxiliary vector,
/// which is a collection of key-value pairs passed to a new process by the kernel.
///
/// # Examples
///
/// ```
/// use linux_loader::auxv::AuxVecKey;
///
/// let key = AuxVecKey::AT_ENTRY;
/// assert_eq!(key as usize, 9);
/// ```
///
/// # See Also
///
/// - [getauxval](https://man7.org/linux/man-pages/man3/getauxval.3.html)
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
    /// Creates a new AuxVecEntry from the given auxiliary-vector key and value.
    ///
    /// # Examples
    ///
    /// ```
    /// use linux_loader::auxv::{AuxVecEntry, AuxVecKey};
    ///
    /// let entry = AuxVecEntry::new(AuxVecKey::AT_ENTRY, 0x1000);
    /// assert_eq!(entry.key, AuxVecKey::AT_ENTRY);
    /// assert_eq!(entry.value, 0x1000);
    /// ```
    pub const fn new(key: AuxVecKey, val: usize) -> Self {
        AuxVecEntry { key, value: val }
    }
}

/// Some common auxiliary vector values set by the kernel.
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
