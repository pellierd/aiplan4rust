//! # Atomic Symbols Module
//!
//! This module provides high-level constructors for leaf nodes in the Expression Tree.
//! These nodes represent the most basic building blocks of PDDL expressions:
//! objects, variables, and symbols (functions and predicates).
//!
//! All functions in this module are heavily inlined to ensure that the abstraction
//! layer between the LIR (Low-level Intermediate Representation) and the user-facing
//! API has zero runtime overhead.

use crate::aiplan4rust::lang::{FunctionSymbolId, ObjectId, PredicateSymbolId, VariableId};
use crate::aiplan4rust::lir::store::expr::ExprBuilder;
use crate::aiplan4rust::lir::store::expr::{ExprEntryKind, ExprId};

impl<'a> ExprBuilder<'a> {
    /// Creates a constant leaf node representing a PDDL Object.
    ///
    /// # Arguments
    /// * `id` - Any type that can be converted into an `ObjectId`.
    ///
    /// # Returns
    /// An `ExprId` pointing to the unique interned `Object` node.
    ///
    /// # Performance
    /// This function is marked `#[inline(always)]` to eliminate call overhead
    /// during intensive parsing or expression transformation.
    #[inline(always)]
    pub fn object<I: Into<ObjectId>>(&mut self, id: I) -> ExprId {
        self.intern(ExprEntryKind::Object(id.into()), &[])
    }

    /// Creates a leaf node representing a Variable.
    ///
    /// # Arguments
    /// * `id` - Any type that can be converted into a `VariableId`.
    ///
    /// # Returns
    /// An `ExprId` pointing to the unique interned `Variable` node.
    ///
    /// # Note
    /// Variables are typically used within the scope of quantifiers (`forall`, `exists`).
    #[inline(always)]
    pub fn variable<I: Into<VariableId>>(&mut self, id: I) -> ExprId {
        self.intern(ExprEntryKind::Variable(id.into()), &[])
    }

    /// Creates a leaf node representing a Function symbol (functor).
    ///
    /// # Arguments
    /// * `id` - Any type that can be converted into a `FunctionSymbolId`.
    ///
    /// # Returns
    /// An `ExprId` pointing to the interned `FunctionSymbol`.
    ///
    /// # Note
    /// This represents the symbol itself, not the application of a function to arguments.
    #[inline(always)]
    pub fn function_symbol<I: Into<FunctionSymbolId>>(&mut self, id: I) -> ExprId {
        self.intern(ExprEntryKind::FunctionSymbol(id.into()), &[])
    }

    /// Creates a leaf node representing a Predicate symbol.
    ///
    /// # Arguments
    /// * `id` - Any type that can be converted into a `PredicateSymbolId`.
    ///
    /// # Returns
    /// An `ExprId` pointing to the interned `PredicateSymbol`.
    #[inline(always)]
    pub fn predicate<I: Into<PredicateSymbolId>>(&mut self, id: I) -> ExprId {
        self.intern(ExprEntryKind::PredicateSymbol(id.into()), &[])
    }
}

#[cfg(test)]
mod tests {
    use crate::aiplan4rust::lir::store::expr::{ExprBuilder, ExprStore};

    /// These tests verify the fundamental integrity of the Atomic Symbol constructors.
    ///
    /// Even though the logic is trivial, these assertions ensure two critical properties:
    /// 1. **Deduplication (Hash-Consing)**: Multiple calls with the same ID must return the exact
    ///    same `ExprId`, which is the cornerstone of the LIR's memory efficiency.
    /// 2. **Type Segregation**: Ensures that different symbol kinds (e.g., an Object vs. a Variable)
    ///    with the same numerical ID are correctly interned as distinct nodes, preventing
    ///    catastrophic semantic collisions in the expression tree.
    #[test]
    fn test_atomic_symbols_deduplication() {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);

        // Test que le Hash-Consing fonctionne pour les feuilles
        let o1 = builder.object(1);
        let o2 = builder.object(1);
        assert_eq!(o1, o2, "Objects with same ID must share ExprId");

        let v1 = builder.variable(1);
        assert_ne!(o1, v1, "Object and Variable with same ID must be distinct");

        let p1 = builder.predicate(10);
        let f1 = builder.function_symbol(10);
        assert_ne!(
            p1, f1,
            "Predicate and Function with same ID must be distinct"
        );
    }
}
