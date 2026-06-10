//! # Atomic & Functional Terms Module
//!
//! This module provides the high-level API for constructing predicates, functions,
//! and timed literals within the LIR old.
//!
//! ## Design Goal: Zero-Allocation
//! To handle large-scale planning problems (with thousands of ground atoms), this
//! module leverages the `ExprBuilder` internal scratchpad buffers (`primary_buffer`).
//! This avoids the "heap churn" associated with allocating temporary vectors for
//! argument lists during the construction of complex terms.

use crate::aiplan4rust::compiler::lir::expr::ExprBuilder;
use crate::aiplan4rust::compiler::lir::expr::{ExprId, ExprKind};
use crate::aiplan4rust::support::lang::{
    AtomSkeletonId, FunctionSkeletonId, FunctionSymbolId, PredicateSymbolId,
};

impl<'a> ExprBuilder<'a> {
    /// Constructs an `AtomicFormula` (Predicate with arguments) using a zero-alloc strategy.
    ///
    /// This method represents a logical statement that is either true or false. It normalizes
    /// the predicate structure by prepending the predicate symbol to the argument list
    /// before interning.
    ///
    /// # Performance
    ///
    /// Utilizes the builder's `primary_buffer` to avoid heap allocations for the
    /// children list. This is highly efficient for high-frequency fact generation.
    ///
    /// # Arguments
    ///
    /// * `sym_id` - The identifier of the predicate symbol (e.g., `at`, `on`, `connected`).
    ///   Supports any type that implements `Into<PredicateSymbolId>`.
    /// * `args` - A slice of [`ExprId`] representing the terms/objects passed to the predicate.
    /// * `skel_id` - The skeleton identifier used for grounding and structural verification.
    ///
    /// # Returns
    ///
    /// * `ExprId` - The unique identifier of the interned atomic formula in the old.
    /// Constructs an atomic formula (predicate application).
    ///
    /// # Performance
    /// - **Zero-Allocation**: Reuses `primary_buffer` to old children.
    /// - **Inline Expansion**: Marked `#[inline]` to allow the compiler to optimize
    ///   the buffer swap logic directly into the caller.
    #[inline]
    pub fn atomic_formula<PID, SID>(&mut self, sym_id: PID, args: &[ExprId], skel_id: SID) -> ExprId
    where
        PID: Into<PredicateSymbolId>,
        SID: Into<AtomSkeletonId>,
    {
        // 1. Resolve Symbol (Inlined call)
        let sym_node = self.predicate(sym_id);
        let skel = skel_id.into();

        // 2. Buffer Management
        // We use a temporary swap to satisfy the borrow checker during `self.intern`.
        let mut buffer = std::mem::take(&mut self.primary_buffer);

        buffer.clear();
        // Pre-reserve to avoid incremental reallocations if the buffer was small
        if buffer.capacity() < args.len() + 1 {
            buffer.reserve(args.len() + 1);
        }

        buffer.push(sym_node);
        buffer.extend_from_slice(args);

        // 3. Interning
        let id = self.intern(ExprKind::AtomicFormula(skel), &buffer);

        // 4. Return the buffer to the pool
        self.primary_buffer = buffer;

        id
    }

    /// Constructs a `FunctionTerm` (Fluid or Function with arguments).
    ///
    /// Function terms represent numerical or symbolic fluents (e.g., `(fuel-level ship1)`).
    /// Like predicates, they are interned as a combination of a symbol and a list of arguments.
    ///
    /// # Performance
    ///
    /// Follows the same Zero-Alloc pattern as `atomic_formula`, reusing internal
    /// builder buffers to minimize memory pressure.
    ///
    /// # Arguments
    ///
    /// * `sym_id` - The identifier of the function symbol. Supports `Into<FunctionSymbolId>`.
    /// * `args` - A slice of [`ExprId`] representing the objects or sub-expressions.
    /// * `skel_id` - The function skeleton identifier for structural typing.
    ///
    /// # Returns
    ///
    /// * `ExprId` - The unique identifier of the interned functional term.
    /// Constructs a functional term (function application).
    ///
    /// # Arguments
    /// * `sym_id` - The symbol identifier for the function.
    /// * `args` - The arguments (terms) of the function.
    /// * `skel_id` - The structural skeleton for this function application.
    ///
    /// # Performance
    /// - **Zero-Allocation**: Reuses the `primary_buffer` to avoid `Vec` allocations.
    /// - **Cache Local**: The buffer swap technique keeps data on the stack during interning.
    #[inline]
    pub fn function_term<FID, SID>(&mut self, sym_id: FID, args: &[ExprId], skel_id: SID) -> ExprId
    where
        FID: Into<FunctionSymbolId>,
        SID: Into<FunctionSkeletonId>,
    {
        // 1. Resolve Symbol (Inlined call)
        let sym_node = self.function_symbol(sym_id);
        let skel = skel_id.into();

        // 2. Buffer Management (Swap pattern)
        // We take the buffer to gain owned access, satisfying the borrow checker.
        let mut buffer = std::mem::take(&mut self.primary_buffer);

        buffer.clear();
        // Defensive reserve: ensures a single potential reallocation for large arities.
        if buffer.capacity() < args.len() + 1 {
            buffer.reserve(args.len() + 1);
        }

        buffer.push(sym_node);
        buffer.extend_from_slice(args);

        // 3. Interning
        let id = self.intern(ExprKind::Function(skel), &buffer);

        // 4. Restoration
        // We return the buffer to the pool for the next call.
        self.primary_buffer = buffer;

        id
    }
}

#[cfg(test)]
mod tests {
    use crate::aiplan4rust::compiler::lir::expr::builder::ExprBuilder;
    use crate::aiplan4rust::compiler::lir::expr::{ExprKind, ExprStore};
    use crate::aiplan4rust::support::lang::{
        AtomSkeletonId, FunctionSkeletonId, FunctionSymbolId, PredicateSymbolId, VariableId,
    };

    /// Objective: Verify that identical atomic formulas are deduplicated via Hash-Consing.
    /// Input: Calling builder.atomic_formula twice with identical predicate, arguments, and skeleton.
    /// Output: Both calls must return the exact same ExprId.
    #[test]
    fn test_atomic_formula_deduplication() {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);

        let sym = PredicateSymbolId::from(1);
        let skel = AtomSkeletonId::from(10);
        let arg1 = builder.variable(VariableId::from(1));
        let arg2 = builder.variable(VariableId::from(2));

        let f1 = builder.atomic_formula(sym, &[arg1, arg2], skel);
        let f2 = builder.atomic_formula(sym, &[arg1, arg2], skel);

        assert_eq!(
            f1, f2,
            "Identical atomic formulas must share the same ExprId via Hash-Consing"
        );
    }

    /// Objective: Ensure function terms correctly old the symbol as the first child followed by arguments.
    /// Input: A function term with one numeric argument.
    /// Output: A Function node where children[0] is the FunctionSymbol and children[1] is the argument.
    #[test]
    fn test_function_term_structure() {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);

        let sym_id = FunctionSymbolId::from(5);
        let skel = FunctionSkeletonId::from(50);
        let arg = builder.number(10.0);

        let f_id = builder.function_term(sym_id, &[arg], skel);
        let node = builder.get(f_id).expect("Function node must exist");

        // Verify the node kind matches the skeleton
        assert!(matches!(node.kind(), ExprKind::Function(s) if s == &skel));

        // Verify the structure: [SymbolNode, ArgumentNode]
        let children = node.children();
        assert_eq!(
            children.len(),
            2,
            "Function term must have 2 children (symbol + 1 arg)"
        );

        // First child must be the FunctionSymbol metadata node
        let sym_node = builder.get(children[0]).expect("Symbol node must exist");
        assert!(matches!(sym_node.kind(), ExprKind::FunctionSymbol(s) if s == &sym_id));

        // Second child is the actual argument
        assert_eq!(
            children[1], arg,
            "Second child must be the numeric argument"
        );
    }

    /// Objective: Ensure the internal builder buffer is correctly cleared between unrelated calls.
    /// Input: Constructing two different atomic formulas sequentially.
    /// Output: Nodes must not leak data from one another; children[0] (symbols) must differ.
    #[test]
    fn test_buffer_integrity_across_calls() {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);

        let sym1 = PredicateSymbolId::from(1);
        let sym2 = PredicateSymbolId::from(2);

        let f1 = builder.atomic_formula(sym1, &[], AtomSkeletonId::from(1));
        let f2 = builder.atomic_formula(sym2, &[], AtomSkeletonId::from(2));

        let node1 = builder.get(f1).unwrap();
        let node2 = builder.get(f2).unwrap();

        assert_ne!(
            node1.children()[0],
            node2.children()[0],
            "Symbols must be different; internal buffer contamination detected if equal"
        );
    }

    /// Objective: Verify handling of mixed argument types (Variables and Numbers) in formulas.
    /// Input: Atomic formula with a VariableId and a literal Number.
    /// Output: A node with 3 children: [PredicateSymbol, Variable, Number].
    #[test]
    fn test_mixed_arguments_formula() {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);

        let var = builder.variable(VariableId::from(1));
        let val = builder.number(42.0);
        let sym = PredicateSymbolId::from(1);

        let formula = builder.atomic_formula(sym, &[var, val], AtomSkeletonId::from(1));

        let node = builder.get(formula).expect("Formula node must exist");
        let children = node.children();

        assert_eq!(
            children.len(),
            3,
            "Should have 3 children: Symbol + Var + Val"
        );

        // Detailed structure check
        let sym_node = builder.get(children[0]).unwrap();
        assert!(matches!(sym_node.kind(), ExprKind::PredicateSymbol(s) if s == &sym));
        assert_eq!(children[1], var);
        assert_eq!(children[2], val);
    }
}
