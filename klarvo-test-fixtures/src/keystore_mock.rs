use std::collections::HashMap;

use secrecy::SecretString;

/// Minimal in-memory key store for tests that need pre-canned API keys.
///
/// This is 1B.4's intra-crate-style fixture. Story 1C.1 delivers the canonical
/// `InMemoryKeyStore` with the full `KeyStore`-trait impl. This struct stays
/// unchanged through 1C.1 and is optionally superseded in 2.3.
pub struct MockKeyStore {
    canned_keys: HashMap<String, SecretString>,
}

impl MockKeyStore {
    pub fn new() -> Self {
        Self { canned_keys: HashMap::new() }
    }

    pub fn insert(&mut self, key: &str, value: SecretString) {
        self.canned_keys.insert(key.to_string(), value);
    }

    /// Returns a clone of the stored secret for `key`, or `None` if absent.
    pub fn get(&self, key: &str) -> Option<SecretString> {
        self.canned_keys.get(key).cloned()
    }
}

impl Default for MockKeyStore {
    fn default() -> Self {
        Self::new()
    }
}
