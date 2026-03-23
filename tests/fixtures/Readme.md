# Rejected Benchmarks Tracking

## IPC 2004 PDDL - Airport (All Temporal Variants)

- **Status**: Completely Removed from Integration Tests
- **Reason**: Systematic violation of PDDL 2.1 ISO Syntax.
- **Technical Breakdown**:
    1. **Illegal Nesting**: These files place `(when ...)` inside `(at <time> ...)`.
       *Official BNF Requirement*: `(when <gd> (at <time> <effect>))`.
    2. **Missing Requirements**: Use of numeric fluents without declaring `:number-fluents`.
- **Validation Consistency**:
  All variants produce `Error: Syntax error in timed effect` and `Error: Unreadable structure` in the `VAL` reference
  validator.
- **Project Policy**:
  `aiplan4rust` prioritizes strict PDDL 2.1 compliance. Legacy benchmarks that rely on non-standard parser laxity are
  excluded to ensure the integrity of the semantic analyzer and state propagator.

## IPC 2020 HDDL - UMT-Translog (Typing Inconsistency)

- **Status**: Patched (Type Alignment)
- **Issue**: Semantic conflict between Domain constants and Problem object declarations.
- **Specific Case**: Domain defines `:constants Pferd - Regular_Truck`, but Problem 20 declares `Pferd - Tanker_Truck`.
- **Technical Problem**:
  In PDDL/HDDL, the Domain is the **source of truth**. `aiplan4rust` considers a constant's declaration in the domain as
  the base integrity contract. A problem file is only allowed to **specialize** an existing constant (i.e., declaring it
  as a valid **subtype** of the domain-defined type).
  If the problem provides a type that is not a descendant (like `Tanker_Truck` vs `Regular_Truck`), it creates a
  semantic rupture, making the object incompatible with the domain's operators.
- **Resolution Strategy**:
  **Forced "Domain-First" alignment.** We verify if the problem-type is a valid subtype of the domain-type. If it is not
  a specialization, we revert the constant to its original Domain definition (`Regular_Truck`) to ensure the object
  remains actionable and the hierarchy stays consistent.
- **Project Policy**:
  The Domain acts as the definitive contract. We resolve ambiguities by prioritizing the Domain's hierarchy, ensuring
  legacy benchmarks remain compatible with modern, strict semantic analysis.
