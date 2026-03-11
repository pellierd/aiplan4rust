use std::collections::HashSet;
use std::error::Error;
use crate::aiplan4rust::interner::SymbolInterner;
use crate::aiplan4rust::lang::{Type, TypedSymbol};
use crate::aiplan4rust::lir::expr::ExprBuilder;
use crate::aiplan4rust::lir::problem::LiftedProblem;
use crate::type_flattening::problem::flatten;

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    /// ### Objective
    /// Verify that the flattening process correctly resolves a "diamond" dependency
    /// where different complex types converge on the same set of atomic roots.
    ///
    /// ### Input
    /// - Atomic roots: `a`, `b`, `e`.
    /// - `c = {a, b}`
    /// - `d = {b, e}`
    /// - `f = {c, d}` (leads to {a, b, e})
    /// - `g = {a, d}` (leads to {a, b, e})
    ///
    /// ### Expected Output
    /// 1. Both `f` and `g` must be flattened into `Primitive` redirections.
    /// 2. `f` and `g` must point to the **same** Pivot ID because they share
    ///    the same set of unique leaf nodes: `{a, b, e}`.
    /// 3. The shared pivot must contain exactly 4 members: the Pivot ID (self-ref) + the 3 roots.
    fn test_flatten_diamond_dependency() -> Result<(), Box<dyn std::error::Error>> {
        let interner = SymbolInterner::new();
        let mut problem = LiftedProblem::new(interner, HashSet::new());

        // 1. Create atomic roots
        let a_name = problem.interner_mut().intern_symbol("a");
        let id_a = problem.add_type_symbol(a_name);
        let b_name = problem.interner_mut().intern_symbol("b");
        let id_b = problem.add_type_symbol(b_name);
        let e_name = problem.interner_mut().intern_symbol("e");
        let id_e = problem.add_type_symbol(e_name);

        problem.add_type_defs(TypedSymbol::new(id_a, Type::new()))?;
        problem.add_type_defs(TypedSymbol::new(id_b, Type::new()))?;
        problem.add_type_defs(TypedSymbol::new(id_e, Type::new()))?;

        // 2. Intermediate types
        let c_name = problem.interner_mut().intern_symbol("c");
        let id_c = problem.add_type_symbol(c_name);
        let d_name = problem.interner_mut().intern_symbol("d");
        let id_d = problem.add_type_symbol(d_name);

        // c = {a, b}, d = {b, e}
        problem.add_type_defs(TypedSymbol::new(id_c, Type::either(vec![id_a, id_b])))?;
        problem.add_type_defs(TypedSymbol::new(id_d, Type::either(vec![id_b, id_e])))?;

        // 3. Diamond Types: f and g
        let f_name = problem.interner_mut().intern_symbol("f");
        let id_f = problem.add_type_symbol(f_name);
        let g_name = problem.interner_mut().intern_symbol("g");
        let id_g = problem.add_type_symbol(g_name);

        // f = {c, d} -> resolves to {a, b, e}
        problem.add_type_defs(TypedSymbol::new(id_f, Type::either(vec![id_c, id_d])))?;
        // g = {a, d} -> resolves to {a, b, e}
        problem.add_type_defs(TypedSymbol::new(id_g, Type::either(vec![id_a, id_d])))?;

        // --- EXECUTION ---
        flatten(&mut problem)?;

        // --- VERIFICATIONS ---
        let type_f = problem.try_get_type(id_f)?.ty();
        let type_g = problem.try_get_type(id_g)?.ty();

        assert!(type_f.is_primitive(), "Type 'f' must be a primitive redirection");
        assert!(type_g.is_primitive(), "Type 'g' must be a primitive redirection");

        let pivot_id_f = type_f.members()[0];
        let pivot_id_g = type_g.members()[0];

        // Check if F and G merged into the same pivot
        assert_eq!(pivot_id_f, pivot_id_g, "F and G must point to the same PivotID (Canonicalization failure)");

        let pivot_def = problem.try_get_type(pivot_id_f)?.ty();
        assert!(pivot_def.is_either(), "The pivot must be an 'either' type holding roots");

        // Member count: [Self-ID, a, b, e] = 4
        assert_eq!(pivot_def.len(), 4, "Pivot should have 4 members (self-ref + 3 unique roots)");
        assert_eq!(pivot_def.members()[0], pivot_id_f, "Index 0 must be the mandatory self-reference");

        // Verify union completeness
        assert!(pivot_def.members().contains(&id_a), "Pivot missing root 'a'");
        assert!(pivot_def.members().contains(&id_b), "Pivot missing root 'b' (Deduplication error?)");
        assert!(pivot_def.members().contains(&id_e), "Pivot missing root 'e'");

        Ok(())
    }

    #[test]
    /// ### Objective
    /// Verify that the flattening process is permutation-stable. Two different `Either`
    /// types containing the same atomic roots but in a different order (e.g., `{a, b}` vs `{b, a}`)
    /// must map to the exact same `Pivot` ID and have a deterministic name.
    ///
    /// ### Input
    /// - Two atomic types: `a` and `b`.
    /// - Type `t1`: defined as `either(a, b)`.
    /// - Type `t2`: defined as `either(b, a)`.
    ///
    /// ### Expected Output
    /// 1. Both `t1` and `t2` are flattened into `Primitive` redirections.
    /// 2. `t1` and `t2` point to the **same** `Pivot` ID.
    /// 3. The pivot's generated name is deterministic (e.g., sorted alphabetically as `either_a_b`).
    fn test_flatten_permutation_stability() -> Result<(), Box<dyn std::error::Error>> {
        let interner = SymbolInterner::new();
        let mut problem = LiftedProblem::new(interner, HashSet::new());

        // 1. Define atomic roots
        let a_name = problem.interner_mut().intern_symbol("a");
        let id_a = problem.add_type_symbol(a_name);
        let b_name = problem.interner_mut().intern_symbol("b");
        let id_b = problem.add_type_symbol(b_name);

        problem.add_type_defs(TypedSymbol::new(id_a, Type::new()))?;
        problem.add_type_defs(TypedSymbol::new(id_b, Type::new()))?;

        // 2. Define two types with identical members but reversed order
        let t1_name = problem.interner_mut().intern_symbol("t1");
        let id_t1 = problem.add_type_symbol(t1_name);
        let t2_name = problem.interner_mut().intern_symbol("t2");
        let id_t2 = problem.add_type_symbol(t2_name);

        // t1 = {a, b}
        problem.add_type_defs(TypedSymbol::new(id_t1, Type::either(vec![id_a, id_b])))?;
        // t2 = {b, a}
        problem.add_type_defs(TypedSymbol::new(id_t2, Type::either(vec![id_b, id_a])))?;

        // --- EXECUTION ---
        flatten(&mut problem)?;

        // --- VERIFICATIONS ---
        let pivot_id_1 = problem.try_get_type(id_t1)?.ty().members()[0];
        let pivot_id_2 = problem.try_get_type(id_t2)?.ty().members()[0];

        // Check that both types share the same pivot (Deduplication)
        assert_eq!(pivot_id_1, pivot_id_2, "Permuted types must share the same pivot ID");

        // Verify that the pivot name is deterministic (e.g., sorted alphabetically)
        // This assumes your tracker sorts roots before generating the symbol name.
        let string_id = problem.type_symbols().try_get_ident(pivot_id_1)?;
        let name = problem.interner().try_resolve_symbol(*string_id)?;
        assert_eq!(
            name, "either_a_b",
            "The pivot name must be based on the alphabetical order of roots for determinism"
        );

        Ok(())
    }

    #[test]
    /// ### Objective
    /// Verify that the flattening process correctly handles redundant or overlapping
    /// unions, such as `either(robot, object)` where `object` is an atomic root.
    ///
    /// ### Input
    /// - An atomic root type: `object`.
    /// - A specific atomic type: `robot`.
    /// - A redundant union: `redundant_union` defined as `either(robot, object)`.
    ///
    /// ### Expected Output
    /// 1. The `redundant_union` type is flattened into a `Primitive` redirection.
    /// 2. A `Pivot` is created that successfully captures both atomic roots.
    /// 3. The system remains stable regardless of the semantic overlap between the types.
    fn test_flatten_root_absorption() -> Result<(), Box<dyn std::error::Error>> {

        let interner = SymbolInterner::new();
        let mut problem = LiftedProblem::new(interner, HashSet::new());

        // 1. Define the 'object' root type (empty/atomic)
        let obj_name = problem.interner_mut().intern_symbol("object");
        let id_obj = problem.add_type_symbol(obj_name);
        problem.add_type_defs(TypedSymbol::new(id_obj, Type::new()))?;

        // 2. Define a specific 'robot' type (atomic)
        let robot_name = problem.interner_mut().intern_symbol("robot");
        let id_robot = problem.add_type_symbol(robot_name);
        problem.add_type_defs(TypedSymbol::new(id_robot, Type::new()))?;

        // 3. Define a redundant union: either(robot, object)
        // Semantically, if 'object' is the super-type of everything, this union
        // is redundant, but the flattener must still produce a valid, stable pivot.
        let union_name = problem.interner_mut().intern_symbol("redundant_union");
        let id_union = problem.add_type_symbol(union_name);
        problem.add_type_defs(TypedSymbol::new(id_union, Type::either(vec![id_robot, id_obj])))?;

        // --- EXECUTION ---
        // Perform the flattening pass
        flatten(&mut problem)?;

        // --- VERIFICATIONS ---
        let type_union = problem.try_get_type(id_union)?.ty();

        // Ensure the union was converted to a primitive redirection
        assert!(type_union.is_primitive(), "The redundant union must be flattened");

        let pivot_id = type_union.members()[0];
        let pivot_def = problem.try_get_type(pivot_id)?.ty();

        // Verify the pivot contains the necessary roots
        // Even if redundant, the flattener tracks the union of atomic leaves.
        assert!(pivot_def.members().contains(&id_obj), "The pivot must include the 'object' root");
        assert!(pivot_def.members().contains(&id_robot), "The pivot must include the 'robot' root");
        assert!(pivot_def.members().contains(&pivot_id), "The pivot must be self-referenced");

        Ok(())
    }

    #[test]
    /// ### Objective
    /// Verify the basic functionality of the flattening pass by resolving a
    /// simple `Either` type into a `Primitive` type pointing to a generated `Pivot`.
    ///
    /// ### Input
    /// - Three atomic types: `a`, `b`, and `c`.
    /// - One complex type: `d` defined as `either(a, b, c)`.
    ///
    /// ### Expected Output
    /// 1. Type `d` is transformed into a `Primitive` (a redirection).
    /// 2. A new `Pivot` is created globally.
    /// 3. The pivot contains the pivot ID itself (self-reference) plus the three atomic roots.
    fn test_flatten_types_def_simple() -> Result<(), Box<dyn std::error::Error>> {

        let interner = SymbolInterner::new();
        let mut problem = LiftedProblem::new(interner, HashSet::new());

        // 1. Interning Names
        // We intern names separately to maintain a clear sequence of operations.
        let name_a = problem.interner_mut().intern_symbol("a");
        let name_b = problem.interner_mut().intern_symbol("b");
        let name_c = problem.interner_mut().intern_symbol("c");
        let name_d = problem.interner_mut().intern_symbol("d");

        // 2. Reserving TypeIds
        let id_a = problem.add_type_symbol(name_a);
        let id_b = problem.add_type_symbol(name_b);
        let id_c = problem.add_type_symbol(name_c);
        let id_d = problem.add_type_symbol(name_d);

        // 3. Structure Definitions
        // Define leaf/atomic types
        problem.add_type_defs(TypedSymbol::new(id_a, Type::new()))?;
        problem.add_type_defs(TypedSymbol::new(id_b, Type::new()))?;
        problem.add_type_defs(TypedSymbol::new(id_c, Type::new()))?;

        // Define the complex union: d = {a, b, c}
        problem.add_type_defs(TypedSymbol::new(id_d, Type::either(vec![id_a, id_b, id_c])))?;

        // 4. Execute Flattening Pass
        // This resolves the hierarchy and registers necessary pivots.
        flatten(&mut problem)?;

        // 5. Verification
        let sym_d = problem.try_get_type(id_d)?;
        assert!(sym_d.ty().is_primitive(), "D must be a primitive (pointer to pivot)");

        let pivot_id = sym_d.ty().members()[0];
        let pivot_sym = problem.try_get_type(pivot_id)?;
        let members = pivot_sym.ty().members();

        // The pivot must contain: [Pivot_ID, id_a, id_b, id_c]
        assert_eq!(members.len(), 4, "The pivot must contain exactly 4 members: [Pivot, a, b, c]");
        assert_eq!(members[0], pivot_id, "The first member (index 0) must be the self-reference");

        // Verify that all atomic roots are present
        assert!(members.contains(&id_a), "Pivot missing root 'a'");
        assert!(members.contains(&id_b), "Pivot missing root 'b'");
        assert!(members.contains(&id_c), "Pivot missing root 'c'");

        Ok(())
    }

    #[test]
    /// ### Objective
    /// Verify that the flattening process is idempotent. Running the pass multiple
    /// times must not change the state of the problem after the initial transformation.
    ///
    /// ### Input
    /// - Two atomic types: `a` and `b`.
    /// - One complex type: `t1` defined as `either(a, b)`.
    ///
    /// ### Expected Output
    /// 1. After the first pass, `t1` is converted to a `Primitive` pointing to a `Pivot`.
    /// 2. After the second pass, the total number of type definitions remains unchanged.
    /// 3. The `Pivot` ID associated with `t1` remains identical between passes.
    fn test_flatten_idempotence() -> Result<(), Box<dyn std::error::Error>> {

        let interner = SymbolInterner::new();
        let mut problem = LiftedProblem::new(interner, HashSet::new());

        // 1. Setup atomic types
        let a_name = problem.interner_mut().intern_symbol("a");
        let b_name = problem.interner_mut().intern_symbol("b");
        let id_a = problem.add_type_symbol(a_name);
        let id_b = problem.add_type_symbol(b_name);

        problem.add_type_defs(TypedSymbol::new(id_a, Type::new()))?;
        problem.add_type_defs(TypedSymbol::new(id_b, Type::new()))?;

        // 2. Setup complex type t1 = {a, b}
        let t1_name = problem.interner_mut().intern_symbol("t1");
        let id_t1 = problem.add_type_symbol(t1_name);
        problem.add_type_defs(TypedSymbol::new(id_t1, Type::either(vec![id_a, id_b])))?;

        // --- FIRST PASS ---
        // This pass should create the pivot and rewrite t1
        flatten(&mut problem)?;
        let count_after_first = problem.type_defs().len();
        let pivot_id_first = problem.try_get_type(id_t1)?.ty().members()[0];

        // --- SECOND PASS ---
        // This pass should detect that types are already flattened and do nothing
        flatten(&mut problem)?;
        let count_after_second = problem.type_defs().len();
        let pivot_id_second = problem.try_get_type(id_t1)?.ty().members()[0];

        // 3. Verifications
        assert_eq!(
            count_after_first, count_after_second,
            "The number of types must not increase on the second pass"
        );
        assert_eq!(
            pivot_id_first, pivot_id_second,
            "The pivot ID must remain stable"
        );

        Ok(())
    }

    #[test]
    /// ### Objective
    /// Verify that the flattening process correctly handles nested type hierarchies
    /// (Eithers of Eithers) and resolves them into a single flat set of atomic roots.
    ///
    /// ### Input
    /// - Three atomic types: `a`, `b`, and `c`.
    /// - Type `t1`: defined as `either(a, b)`.
    /// - Type `t2`: defined as `either(t1, c)`.
    ///
    /// ### Expected Output
    /// 1. Both `t1` and `t2` must be transformed into `Primitive` redirections.
    /// 2. The system must resolve the nested structure so that `t2` eventually
    ///    points to a `Pivot` representing the union of `{a, b, c}`.
    /// 3. The final pivot must be self-referenced and contain all 3 leaf nodes.
    fn test_flatten_mixed_hierarchy() -> Result<(), Box<dyn std::error::Error>> {

        let interner = SymbolInterner::new();
        let mut problem = LiftedProblem::new(interner, HashSet::new());

        // 1. Interning Symbols (Atomic and Hierarchy names)
        let name_a = problem.interner_mut().intern_symbol("a");
        let name_b = problem.interner_mut().intern_symbol("b");
        let name_c = problem.interner_mut().intern_symbol("c");
        let name_t1 = problem.interner_mut().intern_symbol("t1");
        let name_t2 = problem.interner_mut().intern_symbol("t2");

        // 2. Reserving Type IDs
        let id_a = problem.add_type_symbol(name_a);
        let id_b = problem.add_type_symbol(name_b);
        let id_c = problem.add_type_symbol(name_c);
        let id_t1 = problem.add_type_symbol(name_t1);
        let id_t2 = problem.add_type_symbol(name_t2);

        // 3. Definitions
        // Define base atomic types
        problem.add_type_defs(TypedSymbol::new(id_a, Type::new()))?;
        problem.add_type_defs(TypedSymbol::new(id_b, Type::new()))?;
        problem.add_type_defs(TypedSymbol::new(id_c, Type::new()))?;

        // t1 = {a, b}
        problem.add_type_defs(TypedSymbol::new(id_t1, Type::either(vec![id_a, id_b])))?;
        // t2 = {t1, c} -> Must resolve to {a, b, c} after flattening
        problem.add_type_defs(TypedSymbol::new(id_t2, Type::either(vec![id_t1, id_c])))?;

        // Execute the flattening pass
        flatten(&mut problem)?;

        // 4. Verification
        let sym_t2 = problem.try_get_type(id_t2)?;

        // Ensure t2 is now a primitive redirection
        assert!(sym_t2.ty().is_primitive(), "t2 should be a primitive redirection");

        let pivot_id = sym_t2.ty().members()[0];
        let pivot_type = problem.try_get_type(pivot_id)?.ty();
        let members = pivot_type.members();

        // The pivot must contain: [Pivot_ID, id_a, id_b, id_c]
        // Total length = 4
        assert_eq!(members.len(), 4, "The mixed hierarchy must be fully flattened to its atomic roots");

        // Verify all atomic roots are present in the final pivot
        assert!(members.contains(&id_a), "Pivot missing root 'a'");
        assert!(members.contains(&id_b), "Pivot missing root 'b'");
        assert!(members.contains(&id_c), "Pivot missing root 'c'");
        assert!(members.contains(&pivot_id), "Pivot must be self-referenced");

        Ok(())
    }

    #[test]
    /// ### Objective
    /// Ensure that "orphan" types (complex types defined in the hierarchy but not used
    /// in any expression) are correctly identified and flattened into pivots.
    ///
    /// ### Input
    /// - Two atomic types: `a` and `b`.
    /// - One complex type: `orphan`, defined as `either(a, b)`.
    /// - No constraints or actions are using the `orphan` type yet.
    ///
    /// ### Expected Output
    /// 1. The `orphan` type is converted to a `Primitive` redirection.
    /// 2. A unique `Pivot` is created for the combination `{a, b}`.
    /// 3. The pivot correctly references the atomic roots and itself for self-reference.
    fn test_flatten_orphan_types() -> Result<(), Box<dyn std::error::Error>> {

        let interner = SymbolInterner::new();
        let mut problem = LiftedProblem::new(interner, HashSet::new());

        // 1. Interning Symbols
        // Separate interning to keep the code clean and readable
        let name_a = problem.interner_mut().intern_symbol("a");
        let name_b = problem.interner_mut().intern_symbol("b");
        let name_orphan = problem.interner_mut().intern_symbol("orphan");

        // 2. Reserving IDs
        let id_a = problem.add_type_symbol(name_a);
        let id_b = problem.add_type_symbol(name_b);
        let id_orphan = problem.add_type_symbol(name_orphan);

        // 3. Definitions
        // Define base atomic types
        problem.add_type_defs(TypedSymbol::new(id_a, Type::new()))?;
        problem.add_type_defs(TypedSymbol::new(id_b, Type::new()))?;

        // Define the orphan complex type: orphan = {a, b}
        problem.add_type_defs(TypedSymbol::new(id_orphan, Type::either(vec![id_a, id_b])))?;

        // Execute the flattening pass
        flatten(&mut problem)?;

        // 4. Verification
        let ts_orphan = problem.try_get_type(id_orphan)?;

        // The original orphan type should now be a primitive (a redirection to a pivot)
        assert!(ts_orphan.ty().is_primitive(), "The orphan type must be flattened into a primitive");

        let pivot_id = ts_orphan.ty().members()[0];
        let pivot_def = problem.try_get_type(pivot_id)?.ty();

        // The pivot should contain: [Pivot_ID, id_a, id_b]
        assert_eq!(pivot_def.len(), 3, "Pivot should contain the pivot ID itself plus its two roots");
        assert!(pivot_def.members().contains(&id_a), "Pivot must contain root 'a'");
        assert!(pivot_def.members().contains(&id_b), "Pivot must contain root 'b'");
        assert!(pivot_def.members().contains(&pivot_id), "Pivot must be self-referenced");

        Ok(())
    }

    #[test]
    /// ### Objective
    /// Verify that the flattening process correctly identifies and transforms ad-hoc
    /// `Either` types located inside quantified expressions (Exists/Forall).
    ///
    /// ### Input
    /// - A problem with two atomic types: `a` and `b`.
    /// - A constraint containing an `Exists` expression where the variable `?X`
    ///   is defined with a complex type `either(a, b)`.
    ///
    /// ### Expected Output
    /// 1. The `flatten` pass must detect the complex type inside the expression.
    /// 2. A new `Pivot` type representing `{a, b}` must be created globally.
    /// 3. The expression node must be updated so the variable `?X` now points to this `Pivot`.
    fn test_flatten_quantified_expression_types() -> Result<(), Box<dyn Error>> {

        let interner = SymbolInterner::new();
        let mut problem = LiftedProblem::new(interner, HashSet::new());

        // 1. Define base types (Roots)
        // Symbols are interned separately for clarity
        let sym_a = problem.interner_mut().intern_symbol("a");
        let sym_b = problem.interner_mut().intern_symbol("b");

        let id_a = problem.add_type_symbol(sym_a);
        let id_b = problem.add_type_symbol(sym_b);

        problem.add_type_defs(TypedSymbol::new(id_a, Type::new()))?;
        problem.add_type_defs(TypedSymbol::new(id_b, Type::new()))?;

        // 2. Build an expression with an "Ad-hoc" Either type
        let mut builder = ExprBuilder::new();

        // Define the complex type members (either a b) using usize conversion
        let adhoc_members = vec![id_a.as_usize(), id_b.as_usize()];

        // Create variable ?X associated with this type
        let var_x = builder.typed_variable(1, &adhoc_members);
        let list_x = builder.typed_variable_list(vec![var_x]);

        // Body of the expression: P()
        let atomic_p = builder.atomic_formula(3, vec![]);
        let exists_x = builder.exists(list_x, atomic_p);

        builder.set_root(exists_x)?;
        let expr = builder.finish();

        // Inject the expression into the problem's constraints
        problem.set_problem_constraints(expr);

        // 3. Transformation: Flatten
        // The flattening must visit global types AND expression trees
        flatten(&mut problem)?;

        // 4. Verification
        let final_expr = problem.problem_constraints();

        // Access the quantifier variables from the root node
        let vars = final_expr.try_root_node()?.content().try_quantifier_vars()?;
        let var_type = vars[0].ty();

        // A flattened type must have exactly one member (the redirection to the pivot)
        assert_eq!(var_type.members().len(), 1, "The type in the quantifier must be a pivot (len=1)");

        let pivot_id = var_type.members()[0];
        let pivot_def = problem.try_get_type(pivot_id)?.ty();

        // Ensure the pivot correctly contains the original atomic roots and itself
        assert!(pivot_def.members().contains(&id_a), "Pivot must contain root 'a'");
        assert!(pivot_def.members().contains(&id_b), "Pivot must contain root 'b'");
        assert!(pivot_def.members().contains(&pivot_id), "Pivot must be self-referenced");

        Ok(())
    }
}
