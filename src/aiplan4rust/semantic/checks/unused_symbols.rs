use crate::aiplan4rust::diagnostic::Diagnostic;
use crate::aiplan4rust::diagnostic::DiagnosticManager;
use crate::aiplan4rust::diagnostic::Provider;
use crate::aiplan4rust::interner::SymbolInterner;
use crate::aiplan4rust::lang::Requirement::DurativeActions;
use crate::aiplan4rust::lang::Requirement::NumericFluents;
use crate::aiplan4rust::lang::Requirement::Typing;
use crate::aiplan4rust::lang::Requirement::{Adl, Fluents};
use crate::aiplan4rust::semantic::checks::{CheckContext, SemanticCheckError};
use crate::aiplan4rust::semantic::symbol::SymbolKind;
use crate::aiplan4rust::semantic::symbol::{Declaration, Scope, SymbolEntry};
use crate::aiplan4rust::syntax::ast::{AstKind, AstNode};
use crate::aiplan4rust::tree::Tree;

/// Checks for symbols that are declared but never used within their scope or any parent scope,
/// emitting warnings for such unused declarations.
///
/// This function iterates over all symbol declarations in the annotated syntax arena and verifies
/// whether each declaration has at least one usage within its scope or any parent scope. It skips
/// checking for symbols of kinds specified in `skip_symbols` or those determined to be skipped by
/// domain-specific rules.
///
/// Built-in PDDL symbols like `"object"` and `"number"` are always ignored as they are considered
/// inherently valid.
///
/// # Parameters
/// - `ast_old`: A reference to the `AnnotatedSyntaxTree` containing the symbol table,
///   declarations, and usages.
/// - `skip_symbols`: A slice of `SymbolKind` indicating symbol kinds to exclude from the
///   unused-symbol check.
/// - `source`: The `DiagnosticSource` from which the diagnostic originates.
/// - `diagnostic_manager`: A mutable reference to the `DiagnosticManager` where warning diagnostics
///   will be recorded.
///
/// # Returns
/// - `Ok(true)` if the check completes successfully; warnings for unused symbols are recorded
///   through `diagnostic_manager`.
/// - `Err(ParserInternalError)` if any internal error occurs during processing, such as missing
///   AST entries.
///
/// # Notes
/// - Symbols declared as built-in PDDL types or those matching skip rules are not checked.
/// - For each unused symbol declaration found, a warning diagnostic is emitted.
///
/// # Example
/// ```no_run
/// let result = check_unused_symbols_warning(
///     &ast_old,
///     &[SymbolKind::Requirement],
///     source,
///     &mut diagnostic_manager,
/// );
/// if let Err(e) = result {
///     eprintln!("Error during unused symbol check: {:?}", e);
/// }
/// ```
pub fn check_unused_symbols(
    context: &CheckContext,
    skip_symbols: &[SymbolKind],
    provider: Provider,
    diagnostic_manager: &mut DiagnosticManager,
) -> Result<bool, SemanticCheckError> {
    let symbol_table = context.symbol_table();

    for symbol_entry in symbol_table.values() {
        for declaration in symbol_entry.declarations() {
            let declaration_kind = declaration.symbol_kind();

            // 1. Filter out declarations that should be ignored (built-ins, specific kinds, etc.)
            if skip_unused_symbol_declaration(declaration, context)?
                || skip_symbols.iter().any(|kind| *kind == declaration_kind)
            {
                continue;
            }

            // 2. Validate against PDDL built-in keywords (e.g., 'object', 'number')
            check_pddl_builtin_symbol_declaration(
                declaration,
                context,
                provider,
                diagnostic_manager,
            );

            // 3. Verify if the declaration is actually used or logically bound
            // We pass the entry and current declaration to the helper for clarity.
            if !has_valid_usage(symbol_entry, declaration) {
                let warning = Diagnostic::warning_unused_symbol(
                    declaration.clone(),
                    provider,
                    context.source_id(),
                    declaration.span().clone(),
                );
                diagnostic_manager.add_diagnostic(warning);
            }
        }
    }

    Ok(true)
}

/// Determines if a specific symbol declaration has a valid usage or a logical binding.
///
/// A declaration is considered "used" if:
/// 1. It is explicitly referenced elsewhere in the PDDL (e.g., in an action precondition).
/// 2. It is a `Predicate` that is logically completed by a `DerivedPredicate` definition.
///
/// # Parameters
/// - `entry`: The symbol table entry containing all declarations and usages for this identifier.
/// - `declaration`: The specific declaration being checked for usage.
fn has_valid_usage(entry: &SymbolEntry, declaration: &Declaration) -> bool {
    let decl_kind = declaration.symbol_kind();
    let decl_scope = declaration.scope();

    // --- PART 1: EXPLICIT USAGE SEARCH ---
    // Check if the symbol is consumed in the domain (e.g., in actions or effects).
    for usage in entry.usages() {
        let usage_kind = usage.symbol_kind();

        // A usage matches if it occurs within a compatible (descendant) scope.
        let scope_match = usage.scope().starts_with(decl_scope);

        // And if the kind is identical or semantically compatible.
        let kind_match =
            usage_kind == decl_kind || decl_kind.can_share_name_space_with(&usage_kind);

        if scope_match && kind_match {
            return true;
        }
    }

    // --- PART 2: SYMBOLIC BINDING LOGIC ---
    // If no explicit usage was found, check if this is a Predicate "saved"
    // by the existence of a matching Derived Predicate definition.
    if decl_kind == SymbolKind::Predicate {
        for other_decl in entry.declarations() {
            if other_decl.symbol_kind() == SymbolKind::DerivedPredicate {
                // The predicate is bound to a derived definition, so it is valid.
                return true;
            }
        }
    }

    // If no explicit usage or binding is found, the declaration is unused.
    false
}

/// Determines whether a declaration should be skipped during unused symbol checking.
///
/// This function returns `true` if the declaration represents a symbol kind or context
/// that should not be considered when checking for unused symbols. This includes:
/// - Declarations of built-in domain constructs like `Requirement`, `Action`, `DASymbol`, or
///   `Method`.
/// - Declarations of symbols like `DomainName`, `ProblemName`, or those associated with
///   specific PDDL requirements (e.g., `Typing`, `NumericFluents`, `DurativeActions`).
/// - Variables that appear within the scope of atomic formula skeletons, atomic function skeletons,
///   or task definitions—these are typically considered local and not subject to unused symbol
///   diagnostics.
///
/// # Parameters
/// - `declaration`: A reference to the `Declaration` to evaluate.
/// - `annotated_syntax_tree`: A reference to the `AnnotatedSyntaxTree`, used for contextual
///   analysis.
///
/// # Returns
/// - `Ok(true)` if the declaration should be skipped during unused symbol checking.
/// - `Ok(false)` if it should be checked.
/// - `Err(ParserInternalError)` if the required AST context cannot be retrieved.
///
/// # Note
/// Skipping these declarations avoids false positives when analyzing domain-level constructs or
/// locally scoped variables that are not meant to be globally referenced.
fn skip_unused_symbol_declaration(
    declaration: &Declaration,
    context: &CheckContext,
) -> Result<bool, SemanticCheckError> {
    // Skip if the declaration is of a built-in kind: Requirement, Action, DASymbol, or Method
    if matches!(
        declaration.symbol_kind(),
        SymbolKind::DomainName
            | SymbolKind::ProblemName
            | SymbolKind::Requirement
            | SymbolKind::Action
            | SymbolKind::DASymbol
            | SymbolKind::Method
            | SymbolKind::TaskID
    ) {
        return Ok(true);
    }

    let requirements = context.declared_requirements();
    match declaration.symbol_ident() {
        SymbolInterner::OBJECT_SYMBOL_ID
            if requirements.contains(&Typing) || requirements.contains(&Adl) =>
        {
            return Ok(true)
        }
        SymbolInterner::NUMBER_SYMBOL_ID | SymbolInterner::TOTAL_TIME_SYMBOL_ID
            if requirements.contains(&NumericFluents) =>
        {
            return Ok(true)
        }
        SymbolInterner::DURATION_VARIABLE_SYMBOL_ID if requirements.contains(&DurativeActions) => {
            return Ok(true)
        }
        _ => {}
    }

    // Skip if the declaration is a variable and its scope contains an atomic skeleton syntax.
    // Variables within such scopes are typically local and do not need to be checked for duplicates.
    let scope = declaration.scope();

    // Check if scope contains specific AST kinds
    let contains_formula =
        scope_contains_node_of_kind(scope, AstKind::AtomicFormulaSkeleton, context.syntax_tree())?;
    let contains_function = scope_contains_node_of_kind(
        scope,
        AstKind::AtomicFunctionSkeleton,
        context.syntax_tree(),
    )?;
    let contains_task =
        scope_contains_node_of_kind(scope, AstKind::TaskDef, context.syntax_tree())?;

    // Check if symbol is a variable
    let is_variable = matches!(declaration.symbol_kind(), SymbolKind::Variable);

    // Return true if variable and any AST kind is present
    if is_variable && (contains_formula || contains_function || contains_task) {
        return Ok(true);
    }

    Ok(false)
}

/// Validates whether a symbol is correctly declared as a built-in symbol according to PDDL
/// specifications.
///
/// This function checks if the given declaration corresponds to a reserved built-in symbol
/// (such as `object`, `number`, `total-time`, or `?duration`) and verifies whether its
/// kind matches the expected `SymbolKind` based on the domain's declared PDDL requirements.
///
/// It emits an error diagnostic if the declaration uses an incorrect kind for a reserved
/// built-in symbol, and emits a warning diagnostic if the symbol is correctly declared but
/// its usage may cause ambiguity or confusion due to keyword overlap.
///
/// # Parameters
/// - `declaration`: Reference to the `Declaration` to validate.
/// - `ast_old`: Reference to the `AnnotatedSyntaxTree` providing requirements and
///   structural context needed for validation.
/// - `checker`: The `Checker` context associated with this validation, used as diagnostic source.
/// - `diagnostic_manager`: Mutable reference to the `DiagnosticManager` where diagnostics
///   (errors or warnings) will be recorded.
///
/// # Returns
/// - `true` if the declaration either matches a known built-in symbol with the correct kind,
///   or if it does not correspond to any recognized built-in symbol (no validation needed).
/// - `false` if the declaration matches a known built-in symbol but is declared with
///   an incorrect kind, in which case an error diagnostic is emitted.
///
/// # Diagnostics
/// - Emits an error if a reserved built-in symbol is declared with a wrong kind.
/// - Emits a warning if the symbol is correctly declared but might cause ambiguity due to
///   keyword overlap.
///
/// # Example
/// ```no_run
/// let result = check_pddl_builtin_symbol_declaration(
///     &declaration,
///     &ast_old,
///     checker,
///     &mut diagnostic_manager,
/// );
/// if !result {
///     eprintln!("Symbol declared with incorrect kind.");
/// }
/// ```
fn check_pddl_builtin_symbol_declaration(
    declaration: &Declaration,
    context: &CheckContext,
    provider: Provider,
    diagnostic_manager: &mut DiagnosticManager,
) -> bool {
    let requirements = context.declared_requirements();
    let current_kind = declaration.symbol_kind();

    // 1. On identifie si le nom est un mot-clé réservé selon les requirements
    let (expected_kind, reqs) = match declaration.symbol_ident() {
        SymbolInterner::OBJECT_SYMBOL_ID
            if requirements.contains(&Typing) || requirements.contains(&Adl) =>
        {
            (SymbolKind::PrimitiveType, vec![Typing, Adl])
        }
        SymbolInterner::NUMBER_SYMBOL_ID if requirements.contains(&NumericFluents) => {
            (SymbolKind::PrimitiveType, vec![NumericFluents, Fluents])
        }
        SymbolInterner::TOTAL_TIME_SYMBOL_ID if requirements.contains(&NumericFluents) => {
            (SymbolKind::Function, vec![NumericFluents, Fluents])
        }
        SymbolInterner::DURATION_VARIABLE_SYMBOL_ID if requirements.contains(&DurativeActions) => {
            (SymbolKind::Variable, vec![DurativeActions])
        }
        _ => return true, // Pas un mot-clé, on valide la déclaration
    };

    // 2. CAS A : Collision Directe (Genre identique)
    // L'utilisateur essaie de déclarer "object" comme un "PrimitiveType".
    // C'est ta stratégie : ERREUR car ça entre en conflit avec ta racine interne.
    if current_kind == expected_kind {
        let error = Diagnostic::error_symbol_conflicts_with_keyword(
            declaration.clone(),
            expected_kind,
            reqs,
            provider,
            context.source_id(),
            declaration.span().clone(),
        );
        diagnostic_manager.add_diagnostic(error);
        return true; // On marque comme trouvé mais invalide
    }

    // 3. CAS B : Usage Ambigu (Genre différent)
    // L'utilisateur déclare "object" comme "Constant".
    // On vérifie si notre nouvelle stratégie autorise ce partage.
    if current_kind.can_share_name_space_with(&expected_kind) {
        // C'est autorisé (ex: Constant vs Type), mais c'est risqué.
        // -> WARNING (ton ancienne stratégie d'ambiguïté)
        let warning = Diagnostic::warning_symbol_declared_ambiguously_as_keyword(
            declaration.clone(),
            expected_kind,
            reqs,
            provider,
            context.source_id(),
            declaration.span().clone(),
        );
        diagnostic_manager.add_diagnostic(warning);
        false
    } else {
        // CAS C : Conflit Radical (ex: Variable nommée "object")
        // Ce n'est pas autorisé par can_share_name_space_with.
        // On pourrait ici mettre une erreur plus grave ou rester sur le warning.
        false
    }
}

/// Checks if the given `scope` contains at least one AST node of the specified `kind`.
///
/// This function iterates over all `NodeId`s in the scope's stack and attempts to
/// retrieve the corresponding AST nodes from the provided AST arena. If any node
/// matches the given `kind`, it returns `Ok(true)`. If no nodes match, it returns
/// `Ok(false)`. If any node ID is invalid or cannot be found, it returns an error.
///
/// # Parameters
/// - `scope`: Reference to the `Scope` to search within.
/// - `kind`: The `AstKind` to look for in the scope.
/// - `ast`: Reference to the AST arena containing all AST nodes.
///
/// # Returns
/// - `Ok(true)` if at least one node of the specified kind is found in the scope.
/// - `Ok(false)` if no matching nodes are found.
/// - `Err(ParserInternalError)` if a node ID in the scope does not correspond to any AST node.
///
/// # Example
/// ```
/// let found = scope_contains_node_of_kind(&scope, AstKind::Function, &ast)?;
/// if found {
///     println!("Scope contains a function node");
/// }
/// ```
fn scope_contains_node_of_kind(
    scope: &Scope,
    kind: AstKind,
    ast: &Tree<AstNode>,
) -> Result<bool, SemanticCheckError> {
    for &id in scope.iter() {
        let node = ast.try_node(id)?;
        if node.kind() == kind {
            return Ok(true);
        }
    }
    Ok(false)
}
