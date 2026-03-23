//! This module provides semantic checks for declared symbols in the symbol table,
//! specifically targeting detection of duplicated declarations. It integrates with
//! the diagnostic infrastructure to report errors or warnings as needed during analysis.

use crate::aiplan4rust::diagnostic::{Diagnostic, DiagnosticManager, Provider};
use crate::aiplan4rust::semantic::checks::{CheckContext, SemanticCheckError};
use crate::aiplan4rust::semantic::symbol::Scope;
use crate::aiplan4rust::semantic::symbol::SymbolKind;
use crate::aiplan4rust::semantic::symbol::{Declaration, Symbol, SymbolEntry};
use crate::aiplan4rust::syntax::ast::AstKind;
use std::collections::{HashMap, HashSet};

/// Entry point for checking declared symbols in the symbol table for semantic issues
/// such as duplicate declarations.
///
/// # Parameters
/// - `context`: A reference to the semantic analysis context, which contains the symbol table and AST.
/// - `diagnostic_manager`: A mutable reference to the diagnostic manager used to record warnings and errors.
///
/// # Returns
/// - `Ok(true)` if no critical errors (e.g., conflicting symbol declarations) were found.
/// - `Ok(false)` if conflicting symbol declarations were found (errors were logged).
/// - `Err(ParserInternalError)` if an internal parser error occurred during the check.
///
/// # Example
/// ```
/// let result = check_declared_symbols(&context, &mut diagnostic_manager)?;
/// if !result {
///     // Handle semantic errors
/// }
/// ```
pub fn check_symbol_declarations(
    context: &CheckContext,
    diagnostic_manager: &mut DiagnosticManager,
) -> Result<bool, SemanticCheckError> {
    check_symbol_declarations_internal(context, diagnostic_manager, None)
}

/// Internal helper function that performs detailed checking for duplicate symbol declarations.
///
/// This function iterates through all symbols in the symbol table and ensures that
/// each symbol is declared only once within the same scope or nested scopes.
///
/// ### Scoping Logic
/// It uses a "scope path" approach: if a declaration A is made in a scope that is
/// a parent of (or identical to) the scope of declaration B, a conflict is detected.
///
/// ### Specific Cases
/// - `DomainName` and `ProblemName` are ignored as duplicates are allowed by design.
/// - Duplicate variables within skeletons (predicates/functions) are downgraded
///   to warnings.
///
/// # Parameters
/// - `context`: The semantic context holding the AST and symbol table.
/// - `diagnostic_manager`: The system used to report diagnostics (warnings/errors).
/// - `kinds_to_check`: An optional set of `SymbolKind`s to restrict the check.
///
/// # Returns
/// - `Ok(true)` if no critical errors were found.
/// - `Ok(false)` if conflicting symbol declarations were found (errors were logged).
/// - `Err(SemanticCheckError)` if an internal error occurred (e.g., missing AST node).
fn check_symbol_declarations_internal(
    context: &CheckContext,
    diagnostic_manager: &mut DiagnosticManager,
    kinds_to_check: Option<&HashSet<SymbolKind>>,
) -> Result<bool, SemanticCheckError> {
    let mut checked = true;
    let symbol_table = context.symbol_table();

    // Pre-allocate the map outside the loop to reuse its memory capacity across all symbols.
    // This avoids thousands of small heap allocations by using references (&Scope, &Declaration)
    // that point directly to data already owned by the symbol table.
    let mut seen_scopes: HashMap<&Scope, &Declaration> = HashMap::with_capacity(8);

    // Iterate through each unique symbol entry in the table
    for symbol in symbol_table.values() {
        // Clear the map for the new symbol while keeping the allocated memory bucket.
        // This is a zero-allocation operation that significantly speeds up analysis.
        seen_scopes.clear();

        for declaration in symbol.declarations() {
            // STEP 1: Filter by symbol category (if a restriction is provided)
            if let Some(kinds) = kinds_to_check {
                if !kinds.contains(&declaration.symbol_kind()) {
                    continue;
                }
            }

            // STEP 2: Skip global names like Domain or Problem names
            if skip_declaration(declaration)? {
                continue;
            }

            let current_scope = declaration.scope();

            // STEP 3: Search for a scope conflict
            // We check if the current scope starts with any previously recorded scope.
            // For instance, if current_scope is [Action, Parameters] and we already saw [Action],
            // 'starts_with' returns true -> A conflict is detected.
            let maybe_conflict = seen_scopes
                .iter()
                .find(|(s, _)| current_scope.starts_with(s));

            if let Some((&conflicting_scope, &previous_declaration)) = maybe_conflict {
                // STEP 4: Delegate conflict handling
                // This helper decides whether to emit a Warning (legacy/tolerant) or an Error (strict).
                if !handle_declaration_conflict(
                    context,
                    diagnostic_manager,
                    symbol,
                    declaration,
                    previous_declaration,
                    conflicting_scope,
                )? {
                    // If the helper returns false, it's a blocking semantic error
                    checked = false;
                }
            } else {
                // STEP 5: Register the new scope
                // No parent scope found, so we record this declaration for future nested checks.
                seen_scopes.insert(current_scope, declaration);
            }
        }
    }

    Ok(checked)
}

/// Handles a naming conflict between two declarations of the same symbol within overlapping scopes.
///
/// This function implements the decision logic for PDDL/HDDL namespace overlapping:
///
/// ### 1. Silence (No Diagnostic)
/// Some overlaps are perfectly valid and expected. For instance, a `Constant` can share
/// its name with a `PrimitiveType` (a common PDDL pattern). In such cases, we do nothing.
///
/// ### 2. Warning: Ambiguous Type/Predicate
/// While PDDL allows a `PrimitiveType` and a `Predicate` to share a name, it creates
/// syntactic ambiguity in some expressions. We emit a warning to alert the user,
/// though it is technically valid.
///
/// ### 3. Warning: IPC-2000 Skeleton Exception
/// Legacy domains sometimes repeat variable names in predicate signatures (skeletons).
/// We downgrade this to a warning to maintain backward compatibility with historical domains.
///
/// ### 4. Error: Duplicate Declaration
/// If the kinds cannot share a namespace (e.g., an `Action` and a `Method` with the same name,
/// or two `Variables` in the same scope), a blocking semantic error is reported.
///
/// # Parameters
/// - `context`: The current semantic check context.
/// - `diagnostic_manager`: Manager to collect diagnostics.
/// - `symbol`: The entry in the symbol table for the conflicting identifier.
/// - `declaration`: The current (new) declaration being processed.
/// - `previous_declaration`: The existing declaration found in an ancestor scope.
/// - `conflicting_scope`: The specific scope where the collision occurred.
///
/// # Returns
/// - `Ok(true)` if the conflict is allowed (silently or with a warning).
/// - `Ok(false)` if the conflict is a fatal semantic error.
/// - `Err(SemanticCheckError)` if a tree or scope inconsistency is encountered.
fn handle_declaration_conflict(
    context: &CheckContext,
    diagnostic_manager: &mut DiagnosticManager,
    symbol: &SymbolEntry,
    declaration: &Declaration,
    previous_declaration: &Declaration,
    conflicting_scope: &Scope,
) -> Result<bool, SemanticCheckError> {
    let mut is_valid = true;
    let current_kind = declaration.symbol_kind();
    let previous_kind = previous_declaration.symbol_kind();
    let ast_entry = context.syntax_tree().try_node(declaration.node_id())?;

    // --- CASE 1: Shared Namespaces (Silent or Warning) ---
    // We check if the language rules (via can_share_name_space_with) allow this overlap.
    if current_kind.can_share_name_space_with(&previous_kind) {
        // Check if the naming conflict involves Derived Predicates.
        // If it does, we allow the overlap silently (return Ok(true)) as
        // PDDL allows multiple axioms to define the same predicate.
        if is_derived_predicate_sharing(current_kind, previous_kind) {
            return Ok(true);
        }

        // Specific check: Even if sharing is allowed, Type/Predicate is risky.
        // Why only this one? Because Constants and Types are easily distinguished by
        // position, whereas Predicates and Types can appear in similar parenthetical
        // structures, potentially confusing the user.
        if is_ambiguous_type_predicate(current_kind, previous_kind) {
            let (pred_decl, type_decl) = match (current_kind, previous_kind) {
                (SymbolKind::Predicate, _) => (declaration, previous_declaration),
                (_, SymbolKind::Predicate) => (previous_declaration, declaration),
                _ => unreachable!(
                    "is_ambiguous_type_predicate returned true but no predicate was found"
                ),
            };

            diagnostic_manager.add_diagnostic(Diagnostic::warning_ambiguous_type_predicate_symbol(
                type_decl.clone(),
                pred_decl.clone(),
                Provider::Analyzer,
                context.source_id(),
                ast_entry.span().clone(),
            ));
        }

        // For other cases (Action/Task or Constant/Type), we stay silent.
        return Ok(true);
    } else {
        // --- CASE 2: Real Conflicts or Historical Exceptions ---

        // Resolve the scope node to identify where the conflict happens.
        let scope_index = conflicting_scope
            .iter()
            .last()
            .ok_or_else(SemanticCheckError::empty_scope)?;

        let scope_node = context
            .syntax_tree()
            .get_node(*scope_index)
            .ok_or_else(|| SemanticCheckError::missing_scope_node(*scope_index))?;

        // Legacy Exception: IPC-2000 allows duplicate variables in "skeletons".
        if is_skeleton_exception(current_kind, scope_node.kind()) {
            diagnostic_manager.add_diagnostic(
                Diagnostic::warning_duplicate_variable_skeleton_declaration(
                    Symbol::new(symbol.ident(), current_kind),
                    previous_declaration.clone(),
                    declaration.clone(),
                    scope_node.kind(),
                    Provider::Analyzer,
                    context.source_id(),
                    ast_entry.span().clone(),
                ),
            );
        } else {
            // Standard Case: This is a hard duplicate error.
            is_valid = false;
            diagnostic_manager.add_diagnostic(
                Diagnostic::error_duplicated_symbol_declaration_in_scope(
                    Symbol::new(symbol.ident(), current_kind),
                    previous_declaration.clone(),
                    declaration.clone(),
                    scope_node.kind(),
                    Provider::Analyzer,
                    context.source_id(),
                    ast_entry.span().clone(),
                ),
            );
        }
    }

    Ok(is_valid)
}

/// Determines if a name sharing conflict involves Derived Predicates.
///
/// In PDDL, it is valid for a symbol to have multiple definitions if they are
/// part of a derived predicate's logic. This includes:
/// - Multiple `:derived` axioms defining the same predicate (logical OR).
/// - A `:derived` definition for a predicate already declared in the `(:predicates)` block.
///
/// # Parameters
/// - `k1`: The `SymbolKind` of the first declaration.
/// - `k2`: The `SymbolKind` of the second declaration.
///
/// # Returns
/// `true` if the combination of kinds represents a valid derived predicate overlap.
fn is_derived_predicate_sharing(k1: SymbolKind, k2: SymbolKind) -> bool {
    matches!(
        (k1, k2),
        (SymbolKind::DerivedPredicate, SymbolKind::DerivedPredicate)
            | (SymbolKind::Predicate, SymbolKind::DerivedPredicate)
            | (SymbolKind::DerivedPredicate, SymbolKind::Predicate)
    )
}

/// Determines if the name conflict involves a `PrimitiveType` and a `Predicate`.
///
/// In PDDL, types and predicates often reside in different namespaces or are allowed
/// to share names depending on the specific requirement (e.g., `:typing`). This check
/// identifies such overlaps to allow for specific diagnostic handling (like warnings
/// instead of hard errors).
///
/// # Parameters
/// - `k1`: The `SymbolKind` of the first declaration.
/// - `k2`: The `SymbolKind` of the second declaration.
///
/// # Returns
/// `true` if one kind is a `PrimitiveType` and the other is a `Predicate`, `false` otherwise.
fn is_ambiguous_type_predicate(k1: SymbolKind, k2: SymbolKind) -> bool {
    let has_type = k1 == SymbolKind::PrimitiveType || k2 == SymbolKind::PrimitiveType;
    let has_pred = k1 == SymbolKind::Predicate || k2 == SymbolKind::Predicate;
    has_type && has_pred
}

/// Identifies the legacy IPC-2000 exception for duplicate variables.
///
/// This exception allows multiple declarations of the same variable name specifically
/// within "skeletons" (predicate or function definitions). This is often found in
/// older PDDL domains (like the Logistics domain from IPC-2000) where variables
/// like `?v` might be repeated in a single signature.
///
/// # Parameters
/// - `kind`: The `SymbolKind` of the symbol being checked (expected to be `Variable`).
/// - `scope_kind`: The `AstKind` of the scope where the symbol is declared.
///
/// # Returns
/// `true` if the symbol is a `Variable` and is declared within an `AtomicFormulaSkeleton`
/// or an `AtomicFunctionSkeleton`.
fn is_skeleton_exception(kind: SymbolKind, scope_kind: AstKind) -> bool {
    kind == SymbolKind::Variable
        && matches!(
            scope_kind,
            AstKind::AtomicFormulaSkeleton | AstKind::AtomicFunctionSkeleton
        )
}

/// Determines whether a given symbol declaration should be skipped from duplicate checking.
///
/// Currently skips declarations of kind:
/// - `DomainName`
/// - `ProblemName`
///
/// These kinds are allowed to appear multiple times in a program without being considered
/// as semantic errors.
///
/// # Parameters
/// - `declaration`: The declaration to evaluate.
///
/// # Returns
/// - `Ok(true)` if the declaration should be skipped.
/// - `Ok(false)` otherwise.
/// - `Err(ParserInternalError)` if the operation fails unexpectedly.
fn skip_declaration(declaration: &Declaration) -> Result<bool, SemanticCheckError> {
    if matches!(
        declaration.symbol_kind(),
        SymbolKind::DomainName | SymbolKind::ProblemName
    ) {
        return Ok(true);
    }

    Ok(false)
}
