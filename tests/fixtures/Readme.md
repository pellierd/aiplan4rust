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

## IPC 2008 PDDL - Woodworking (Empty Typed Lists)

- **Status**: Manually Patched (Fixture Cleaning)
- **File**: `./tests/fixtures/pddl/ipc08/seq-sat/woodworking-strips/pb11.pddl`
- **Issue**: Syntax error due to the "Empty Typed List" generator bug.
- **Technical Breakdown**:
    - The problem file contains a type declaration `- board` without any preceding object identifiers.
    - **BNF Violation**: PDDL requires at least one identifier before the hyphen in a typed list:
      `<typed list (name)> ::= name+ - <type>`. (Note: many BNF versions use `+` to indicate at least one element is
      required before the separator).
- **Resolution Strategy**:
    - **Manual Stripping**: The orphan declaration `- board` has been removed from the fixture to align with strict PDDL
      3.1 syntax.
- **Project Policy**:
    - `aiplan4rust` maintains a strict LALRPOP grammar. We do not implement "parser laxity" to accommodate broken legacy
      generators. Benchmarks must be standard-compliant to be included.
    -

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
