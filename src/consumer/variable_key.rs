use std::fmt::Display;
use std::fmt::Formatter;
use std::hash::Hash;
use std::hash::Hasher;

use rustc_hash::FxHasher;

/// The hash value of the variable key string
pub type VariableKeyHash = u64;

/// Represents a variable key on the hub.
///
/// When constructed from a key string, this calculates a hash of the variable key and stores it inside,
/// which is used for fast lookups of Variable ID from the key.
///
/// It is recommended construct a variable key only once per key string and reuse it for all operations.
#[derive(Copy, Clone, Debug)]
pub struct VariableKey<'a> {
    pub(super) key_hash: VariableKeyHash,
    /// String reference to the variable key string, used to print the unhashed key in case of errors
    key_str: &'a str,
}

impl<'a> From<&'a str> for VariableKey<'a> {
    fn from(key_str: &'a str) -> Self {
        let mut hasher = FxHasher::default();
        key_str.hash(&mut hasher);
        let key_hash = hasher.finish();

        Self { key_hash, key_str }
    }
}

impl<'a> From<&'a String> for VariableKey<'a> {
    #[inline(always)]
    fn from(key_str: &'a String) -> Self {
        Self::from(key_str.as_str())
    }
}

impl Display for VariableKey<'_> {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(f, "{}", self.key_str)
    }
}
