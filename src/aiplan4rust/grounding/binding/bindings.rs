use std::collections::HashMap;
use crate::aiplan4rust::lang::{VariableId, ObjectId};

#[derive(Debug, Clone, Default)]
pub struct Bindings {
    mapping: HashMap<VariableId, ObjectId>,
}

impl Bindings {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn with_capacity(capacity: usize) -> Self {
        Self { mapping: HashMap::with_capacity(capacity) }
    }

    pub fn insert(&mut self, var: VariableId, obj: ObjectId) {
        self.mapping.insert(var, obj);
    }

    pub fn get(&self, var: &VariableId) -> Option<ObjectId> {
        self.mapping.get(var).copied()
    }

    pub fn len(&self) -> usize {
        self.mapping.len()
    }

    pub fn is_empty(&self) -> bool {
        self.mapping.is_empty()
    }

    pub fn clear(&mut self) {
        self.mapping.clear();
    }

    pub fn is_bound(&self, var: &VariableId) -> bool {
        self.mapping.contains_key(var)
    }
}
