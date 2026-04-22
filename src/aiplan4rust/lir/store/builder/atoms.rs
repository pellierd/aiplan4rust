//! # Atomic & Functional Terms Module
//!
//! This module provides the high-level API for constructing predicates, functions,
//! and timed literals within the LIR store.
//!
//! ## Design Goal: Zero-Allocation
//! To handle large-scale planning problems (with thousands of ground atoms), this
//! module leverages the `ExprBuilder` internal scratchpad buffers (`primary_buffer`).
//! This avoids the "heap churn" associated with allocating temporary vectors for
//! argument lists during the construction of complex terms.

use crate::aiplan4rust::lang::{
    AtomSkeletonId, FunctionSkeletonId, FunctionSymbolId, PredicateSymbolId,
};
use crate::aiplan4rust::lir::store::builder::builder::ExprBuilder;
use crate::aiplan4rust::lir::store::{ExprEntryKind, ExprId};

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
    /// * `ExprId` - The unique identifier of the interned atomic formula in the store.
    pub fn atomic_formula<PID, SID>(&mut self, sym_id: PID, args: &[ExprId], skel_id: SID) -> ExprId
    where
        PID: Into<PredicateSymbolId>,
        SID: Into<AtomSkeletonId>,
    {
        // 1. Resolve metadata (Symbol node and Skeleton ID)
        let sym_node = self.predicate(sym_id.into());
        let skel = skel_id.into();

        // 2. Prepare the buffer (Zero-Alloc path)
        self.primary_buffer.clear();
        self.primary_buffer.push(sym_node);
        self.primary_buffer.extend_from_slice(args);

        // 3. Intern the formula
        // We temporarily "take" the buffer to avoid borrow-checker conflicts between
        // the buffer slice and the mutable call to `self.intern`.
        let mut data = std::mem::take(&mut self.primary_buffer);
        let id = self.intern(ExprEntryKind::AtomicFormula(skel), &data);

        // 4. Cleanup and restore the buffer to the builder for future use
        data.clear();
        self.primary_buffer = data;

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
    pub fn function_term<FID, SID>(&mut self, sym_id: FID, args: &[ExprId], skel_id: SID) -> ExprId
    where
        FID: Into<FunctionSymbolId>,
        SID: Into<FunctionSkeletonId>,
    {
        // 1. Resolve symbol node and skeleton
        let sym_node = self.function_symbol(sym_id.into());
        let skel = skel_id.into();

        // 2. Populate the scratchpad buffer
        self.primary_buffer.clear();
        self.primary_buffer.push(sym_node);
        self.primary_buffer.extend_from_slice(args);

        // 3. Perform interning with deduplication
        // Swap technique to satisfy the borrow checker while maintaining the zero-alloc profile.
        let mut data = std::mem::take(&mut self.primary_buffer);
        let id = self.intern(ExprEntryKind::Function(skel), &data);

        // 4. Cleanup and restore buffer
        data.clear();
        self.primary_buffer = data;

        id
    }

    /// Constructs a `TimedInitialLiteral` (TIL).
    ///
    /// A Timed Initial Literal is a specialized PDDL construct representing an
    /// expression (usually an assignment or a predicate) that is added to the
    /// initial state at a specific point in time.
    ///
    /// # Arguments
    ///
    /// * `time` - The timestamp (f64) when the expression becomes effective.
    ///   Note: Negative times are normalized to 0.0 or treated as immediate.
    /// * `expr` - The [`ExprId`] of the formula or assignment to trigger at `time`.
    ///
    /// # Returns
    ///
    /// * `ExprId` - The unique identifier of the timed literal node.
    pub fn timed_initial_literal(&mut self, time: f64, expr: ExprId) -> ExprId {
        // Create a normalized numeric node for the timestamp
        let time_node = self.number(time);

        // Intern the binary relation [Time, Expression]
        // This ensures the (Time, Expr) pair is unique in the store.
        self.intern(ExprEntryKind::TimedInitialLiteral, &[time_node, expr])
    }
}

#[cfg(test)]
mod tests {
    use crate::aiplan4rust::lang::{
        AtomSkeletonId, FunctionSkeletonId, FunctionSymbolId, PredicateSymbolId, VariableId,
    };
    use crate::aiplan4rust::lir::store::builder::ExprBuilder;
    use crate::aiplan4rust::lir::store::{ExprEntryKind, ExprStore};

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

    /// Objective: Ensure function terms correctly store the symbol as the first child followed by arguments.
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
        assert!(matches!(node.kind(), ExprEntryKind::Function(s) if s == &skel));

        // Verify the structure: [SymbolNode, ArgumentNode]
        let children = node.children();
        assert_eq!(
            children.len(),
            2,
            "Function term must have 2 children (symbol + 1 arg)"
        );

        // First child must be the FunctionSymbol metadata node
        let sym_node = builder.get(children[0]).expect("Symbol node must exist");
        assert!(matches!(sym_node.kind(), ExprEntryKind::FunctionSymbol(s) if s == &sym_id));

        // Second child is the actual argument
        assert_eq!(
            children[1], arg,
            "Second child must be the numeric argument"
        );
    }

    /// Objective: Verify that Timed Initial Literals (TIL) normalize time values.
    /// Input: Creating TILs with 0.0 and -0.0.
    /// Output: Identical ExprIds because number interning normalizes floating-point zero signs.
    #[test]
    fn test_timed_literal_normalization() {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);
        let atom = builder.variable(VariableId::from(1));

        let til1 = builder.timed_initial_literal(0.0, atom);
        let til2 = builder.timed_initial_literal(-0.0, atom);

        assert_eq!(
            til1, til2,
            "TIL with 0.0 and -0.0 must be identical due to internal number normalization"
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
        assert!(matches!(sym_node.kind(), ExprEntryKind::PredicateSymbol(s) if s == &sym));
        assert_eq!(children[1], var);
        assert_eq!(children[2], val);
    }
}
