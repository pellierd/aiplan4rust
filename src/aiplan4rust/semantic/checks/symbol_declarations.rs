//! This module provides semantic checks for declared symbols in the symbol table,
//! specifically targeting detection of duplicated declarations. It integrates with
//! the diagnostic infrastructure to report errors or warnings as needed during analysis.

use crate::aiplan4rust::semantic::checks::{CheckContext, SemanticCheckError};
use crate::aiplan4rust::semantic::rules::{
    can_kind_share_namespace, can_share_namespace, is_structural,
};
use crate::aiplan4rust::semantic::symbol::Scope;
use crate::aiplan4rust::semantic::symbol::SymbolKind;
use crate::aiplan4rust::semantic::symbol::{Declaration, Symbol, SymbolEntry};
use crate::aiplan4rust::support::diagnostic::{Diagnostic, DiagnosticManager};
use crate::aiplan4rust::support::interner::SymbolInterner;
use crate::aiplan4rust::support::lang::Requirement::ActionCosts;
use crate::aiplan4rust::support::lang::{Requirement, SymbolId};
use crate::aiplan4rust::syntax::ast::AstKind;
use crate::SymbolTable;
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
    symbol_table: &SymbolTable,
    diagnostic_manager: &mut DiagnosticManager,
) -> Result<bool, SemanticCheckError> {
    check_symbol_declarations_internal(context, symbol_table, diagnostic_manager, None)
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
    symbol_table: &SymbolTable,
    diagnostic_manager: &mut DiagnosticManager,
    kinds_to_check: Option<&HashSet<SymbolKind>>,
) -> Result<bool, SemanticCheckError> {
    let mut checked = true;

    // Pre-allocate the map outside the loop to reuse its memory capacity across all symbols.
    // This avoids thousands of small heap allocations by using references (&Scope, &Declaration)
    // that point directly to data already owned by the symbol table.
    let mut seen_scopes: HashMap<&Scope, &Declaration> = HashMap::with_capacity(8);

    // Iterate through each unique symbol entry in the table
    for symbol in symbol_table {
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
            if is_structural(declaration.symbol().kind()) {
                continue;
            }

            // --- NOUVELLE ÉTAPE : Protection des mots-clés PDDL ---
            // On vérifie si ce symbole entre en collision avec un mot-clé réservé.
            // Si la fonction retourne false (erreur bloquante), on marque l'analyse comme invalide.
            if !check_pddl_builtin_symbol_declaration(declaration, context, diagnostic_manager) {
                checked = false;
                // Optionnel : 'continue' si on veut quand même voir les autres erreurs du symbole,
                // ou on laisse couler vers la détection de doublons.
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
    let ast_entry = context.syntax_tree().try_node(declaration.source())?;

    // --- CASE 1: Shared Namespaces (Silent or Warning) ---
    // We check if the language rules (via can_share_name_space_with) allow this overlap.
    if can_share_namespace(declaration, previous_declaration) {
        // Check if the naming conflict involves Derived Predicates.
        // If it does, we allow the overlap silently (return Ok(true)) as
        // PDDL allows multiple axioms to define the same predicate.
        if current_kind == SymbolKind::Predicate
            && (declaration.is_derived() || previous_declaration.is_derived())
        {
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

            diagnostic_manager.report(Diagnostic::warning_ambiguous_type_predicate_symbol(
                type_decl.clone(),
                pred_decl.clone(),
                context.provider(),
                context.source(),
                ast_entry.span(),
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
            diagnostic_manager.report(Diagnostic::warning_duplicate_variable_skeleton_declaration(
                Symbol::new(symbol.id(), current_kind),
                previous_declaration.clone(),
                declaration.clone(),
                scope_node.kind(),
                context.provider(),
                context.source(),
                ast_entry.span(),
            ));
        } else {
            // Standard Case: This is a hard duplicate error.
            is_valid = false;
            diagnostic_manager.report(Diagnostic::error_duplicated_symbol_declaration_in_scope(
                Symbol::new(symbol.id(), current_kind),
                previous_declaration.clone(),
                declaration.clone(),
                scope_node.kind(),
                context.provider(),
                context.source(),
                ast_entry.span(),
            ));
        }
    }

    Ok(is_valid)
}

/// Validates whether a symbol declaration conflicts with PDDL reserved built-in symbols.
///
/// This function identifies if a declaration uses a reserved identifier (such as `object`,
/// `number`, or `total-time`) based on the domain's active requirements. It then
/// determines the severity of the overlap:
///
/// 1. **Direct Collision**: The declaration uses the same name and the same [`SymbolKind`]
///    as the built-in (e.g., declaring `object` as a `PrimitiveType`). This is treated as
///    an **Error**.
/// 2. **Ambiguous Usage**: The declaration uses a reserved name but with a different,
///    yet compatible [`SymbolKind`] (e.g., declaring `object` as a `Constant`).
///    This is treated as a **Warning**.
/// 3. **Namespace Conflict**: The declaration uses a reserved name with an incompatible
///    kind. This is treated as a validation failure.
///
/// # Parameters
///
/// - `declaration`: A reference to the [`Declaration`] being validated.
/// - `context`: The [`CheckContext`] providing access to PDDL requirements,
///   the symbol interner, and diagnostic metadata.
/// - `diagnostic_manager`: A mutable reference to the [`DiagnosticManager`] for
///   recording errors and warnings.
///
/// # Returns
///
/// - `true`: If a conflict was identified and handled (even if it resulted in an error/warning).
///   This signal typically stops further standard validation for this specific symbol.
/// - `false`: If no recognized built-in keyword was matched, or if the conflict is
///   not considered a "keyword collision" requiring special diagnostics.
///
/// # Diagnostics
///
/// - **Error**: Emitted when a reserved keyword is re-declared with its native kind.
/// - **Warning**: Emitted when a reserved keyword is used for a different but compatible
///   namespace (as defined by `can_share_name_space_with`).
///
/// [`CheckContext`]: crate::semantics::CheckContext
/// [`Declaration`]: crate::semantics::Declaration
/// [`SymbolKind`]: crate::semantics::SymbolKind
fn check_pddl_builtin_symbol_declaration(
    declaration: &Declaration,
    context: &CheckContext,
    diagnostic_manager: &mut DiagnosticManager,
) -> bool {
    // 1. Built-in Identification:
    // Check if the symbol ID corresponds to a reserved PDDL keyword
    // based on the domain's active requirements.
    let (expected_kind, reqs) =
        match get_builtin_definition(declaration.symbol().id(), context.declared_requirements()) {
            Some(def) => def,
            None => return true, // Not a reserved keyword, proceed with standard validation.
        };

    // 2. Kind and Signature Validation:
    // If the symbol is a built-in, verify that the declaration uses the correct
    // SymbolKind and follows the expected PDDL signature (arguments/types).
    if declaration.symbol().kind() == expected_kind {
        // Use the signature helper to validate compliance (e.g., arity 0 for total-cost).
        if is_compliant_builtin_signature(declaration) {
            return true; // Valid built-in redeclaration, accepted silently.
        }

        // 3. Enforcement:
        // Report an error if the built-in is redeclared with an invalid signature.
        diagnostic_manager.report(Diagnostic::error_symbol_conflicts_with_keyword(
            declaration.clone(),
            expected_kind,
            reqs,
            context.provider(),
            context.source(),
            declaration.span().clone(),
        ));
        return false;
    }

    // 4. Namespace Conflict Handling:
    // If the kind differs, check if the language rules allow cross-namespace sharing.
    // Issues a warning for ambiguous usage (e.g., total-cost used as a Predicate).
    if can_kind_share_namespace(
        declaration.symbol().kind(),
        declaration.is_derived(),
        expected_kind,
    ) {
        diagnostic_manager.report(Diagnostic::warning_symbol_declared_ambiguously_as_keyword(
            declaration.clone(),
            expected_kind,
            reqs,
            context.provider(),
            context.source(),
            declaration.span(),
        ));
    }

    false
}

/// Retrieves the expected [`SymbolKind`] and the associated [`Requirement`] list for a PDDL built-in symbol.
///
/// This function acts as a registry for reserved keywords that become active only when
/// specific requirements are declared in the domain (e.g., `total-cost` requires `:numeric-fluents`
/// or `:action-costs`).
///
/// # Arguments
///
/// * `symbol_id` - The internal identifier of the symbol to check.
/// * `requirements` - The set of requirements currently active in the PDDL context.
///
/// # Returns
///
/// * `Some((SymbolKind, Vec<Requirement>))` - If the symbol is a reserved built-in under the current requirements.
///   The vector contains all requirements that trigger this reservation.
/// * `None` - If the symbol is not reserved or if the necessary requirements are missing.
fn get_builtin_definition(
    symbol_id: SymbolId,
    requirements: &HashSet<Requirement>,
) -> Option<(SymbolKind, Vec<Requirement>)> {
    match symbol_id {
        // "number" is reserved as a PrimitiveType if numeric fluents are enabled.
        SymbolInterner::NUMBER_SYMBOL_ID if requirements.contains(&Requirement::NumericFluents) => {
            Some((SymbolKind::PrimitiveType, vec![Requirement::NumericFluents]))
        }

        // "total-cost" is a special built-in function activated by either :numeric-fluents or :action-costs.
        SymbolInterner::TOTAL_COST_SYMBOL_ID => {
            let mut found = Vec::new();
            if requirements.contains(&Requirement::NumericFluents) {
                found.push(Requirement::NumericFluents);
            }
            if requirements.contains(&ActionCosts) {
                found.push(ActionCosts);
            }

            if !found.is_empty() {
                Some((SymbolKind::Function, found))
            } else {
                None
            }
        }

        // "total-time" is a built-in function for temporal or metric planning.
        SymbolInterner::TOTAL_TIME_SYMBOL_ID
            if requirements.contains(&Requirement::NumericFluents) =>
        {
            Some((SymbolKind::Function, vec![Requirement::NumericFluents]))
        }

        // "?duration" is a reserved variable name within durative action specifications.
        SymbolInterner::DURATION_VARIABLE_SYMBOL_ID
            if requirements.contains(&Requirement::DurativeActions) =>
        {
            Some((SymbolKind::Variable, vec![Requirement::DurativeActions]))
        }

        _ => None,
    }
}

/// Validates whether a declaration's signature (arguments and return type)
/// complies with PDDL architectural standards for built-in symbols.
///
/// This helper ensures that reserved symbols are not only used with the correct
/// [`SymbolKind`], but also with the correct structure. For example, in PDDL 3.1,
/// `total-cost` must be a nullary function (0 arguments) returning a numeric value.
///
/// # Arguments
///
/// * `declaration` - The symbol declaration to validate.
///
/// # Returns
///
/// * `true` - If the signature matches the standard requirements for the built-in,
///   or if the symbol has no specific signature constraints (e.g., `number`).
/// * `false` - If the signature violates PDDL constraints (e.g., `total-cost` declared with parameters).
fn is_compliant_builtin_signature(declaration: &Declaration) -> bool {
    match declaration.symbol().id() {
        // total-cost and total-time must be functions of arity 0 (no arguments)
        // and must return a numeric type (either explicitly via 'number' or implicitly).
        SymbolInterner::TOTAL_COST_SYMBOL_ID | SymbolInterner::TOTAL_TIME_SYMBOL_ID => {
            let has_no_args = declaration
                .arguments()
                .as_ref()
                .map_or(true, |a| a.is_empty());

            // PDDL 3.1: If no type is specified, 'number' is assumed for fluents.
            let is_number = declaration
                .ty()
                .as_ref()
                .map_or(true, |t| t.is_number() || t.is_empty());

            has_no_args && is_number
        }

        // For other built-ins like 'number', having the correct SymbolKind
        // (PrimitiveType) is sufficient, so the signature is always considered compliant.
        _ => true,
    }
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
