use crate::aiplan4rust::support::lang::{ObjectId, VariableId};
use std::collections::HashMap;
use std::fmt;

/// ### Bindings Context
///
/// A lightweight environment container managing the active substitution mappings
/// from planning variables ([`VariableId`]) to concrete domain objects ([`ObjectId`]).
///
/// This structure encapsulates a standard hash map and serves as the primary evaluation
/// context queried by the non-allocating expression binding engine during the PDDL
/// grounding and instantiation phases.
#[derive(Debug, Clone, Default)]
pub struct Bindings {
    /// Internal map tracking the binding state of individual variables.
    mapping: HashMap<VariableId, ObjectId>,
}

impl Bindings {
    /// Creates a clean, empty `Bindings` context with default initial capacity.
    ///
    /// # Returns
    /// A default initialized `Bindings` instance.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates an empty `Bindings` context pre-allocated to hold the specified number of mappings.
    ///
    /// This is highly recommended when the number of parameters/variables is known beforehand
    /// to avoid dynamic rehashing overhead on the hot path.
    ///
    /// # Arguments
    /// * `capacity` - The number of variable bindings to allocate space for up front.
    ///
    /// # Returns
    /// A pre-allocated `Bindings` instance.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            mapping: HashMap::with_capacity(capacity),
        }
    }

    /// Inserts a new variable-to-object substitution mapping into the context.
    ///
    /// If the variable was already bound within this context, its previous object assignment
    /// is overwritten.
    ///
    /// # Arguments
    /// * `var` - The target variable identifier ([`VariableId`]) to bind.
    /// * `obj` - The concrete domain object identifier ([`ObjectId`]) to map it to.
    pub fn insert(&mut self, var: VariableId, obj: ObjectId) {
        self.mapping.insert(var, obj);
    }

    /// Retrieves the concrete object bound to the given variable, if it exists.
    ///
    /// # Arguments
    /// * `var` - A reference to the variable identifier being queried.
    ///
    /// # Returns
    /// * `Some(ObjectId)` - The copied object identifier if a mapping exists.
    /// * `None` - If the variable is currently unbound within this context.
    pub fn get(&self, var: &VariableId) -> Option<ObjectId> {
        self.mapping.get(var).copied()
    }

    /// Returns the total number of active variable bindings currently tracked.
    ///
    /// # Returns
    /// The size of the underlying mapping table.
    pub fn len(&self) -> usize {
        self.mapping.len()
    }

    /// Returns `true` if the context contains zero variable mappings.
    ///
    /// # Returns
    /// A boolean flag signaling whether the internal map is empty.
    pub fn is_empty(&self) -> bool {
        self.mapping.is_empty()
    }

    /// Clears all bindings from the context, resetting it for immediate reuse.
    ///
    /// This method strips the mapping entries in-place while retaining the internal
    /// heap capacity allocations to minimize subsequent allocation pressure.
    pub fn clear(&mut self) {
        self.mapping.clear();
    }

    /// Returns `true` if the specified variable has an active mapping in this context.
    ///
    /// # Arguments
    /// * `var` - A reference to the target variable identifier to check.
    ///
    /// # Returns
    /// A boolean flag indicating whether the key exists in the internal lookup table.
    pub fn is_bound(&self, var: &VariableId) -> bool {
        self.mapping.contains_key(var)
    }
}

// --- Formatting Implementations ---

impl fmt::Display for Bindings {
    /// Formats the active bindings context into a clean, human-readable string representation.
    ///
    /// Output format example:
    /// ```text
    /// { ?v1 -> o99, ?v2 -> o100 }
    /// ```
    ///
    /// # Errors
    /// Returns a [`fmt::Error`] if the underlying formatter stream fails to write.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_empty() {
            return write!(f, "{{}}");
        }

        write!(f, "{{ ")?;
        let mut first = true;
        for (var, obj) in &self.mapping {
            if !first {
                write!(f, ", ")?;
            }
            // Uses structural prefix tokens (? and o) standard for automated planning debugging
            write!(f, "?{:?} -> o{:?}", var, obj)?;
            first = false;
        }
        write!(f, " }}")
    }
}
