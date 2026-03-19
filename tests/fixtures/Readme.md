# Rejected Benchmarks Tracking

## IPC 2004 - Airport (All Temporal Variants)
- **Status**: Completely Removed from Integration Tests
- **Reason**: Systematic violation of PDDL 2.1 ISO Syntax.
- **Technical Breakdown**:
  1. **Illegal Nesting**: These files place `(when ...)` inside `(at <time> ...)`.
     *Official BNF Requirement*: `(when <gd> (at <time> <effect>))`.
  2. **Missing Requirements**: Use of numeric fluents without declaring `:number-fluents`.
- **Validation Consistency**:
  All variants produce `Error: Syntax error in timed effect` and `Error: Unreadable structure` in the `VAL` reference validator.
- **Project Policy**:
  `aiplan4rust` prioritizes strict PDDL 2.1 compliance. Legacy benchmarks that rely on non-standard parser laxity are excluded to ensure the integrity of the semantic analyzer and state propagator.
