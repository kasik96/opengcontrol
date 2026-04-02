use std::collections::HashMap;
use std::sync::RwLock;

use crate::protocol::feature::{FeatureCode, FeatureIndex};

/// Thread-safe lazy cache mapping FeatureCode → FeatureIndex.
/// Populated on first use of each feature via IRoot::get_feature().
pub struct FeatureIndexCache(RwLock<HashMap<FeatureCode, FeatureIndex>>);

impl FeatureIndexCache {
    pub fn new() -> Self {
        Self(RwLock::new(HashMap::new()))
    }

    pub fn get(&self, code: FeatureCode) -> Option<FeatureIndex> {
        self.0.read().unwrap().get(&code).copied()
    }

    pub fn insert(&self, code: FeatureCode, index: FeatureIndex) {
        self.0.write().unwrap().insert(code, index);
    }
}

impl Default for FeatureIndexCache {
    fn default() -> Self {
        Self::new()
    }
}
