//! Utilities for formatting diagnostic suggestions.
//!
//! This module provides functions to generate human-readable suggestions from [`DiagnosticKind`] values,
//! which can help users understand how to fix or improve their code based on compiler or analyzer feedback.
//!
//! Suggestions are returned as optional strings, since not all diagnostics include fix-it hints.
//! Internally, suggestions may use a [`SymbolInterner`] to produce readable symbol and typing names.
//!
//! # Public API
//! - [`format_suggestion`]: Formats a suggestion using a provided interner (for end users).
//! - [`format_suggestion_debug`]: Formats a suggestion without an interner (for debugging).
//!
//! # Internal
//! - `format_suggestion_internal`: Shared ops used by both public-facing functions.
//!
//! # Example
//! ```rust
//! if let Some(suggestion) = format_suggestion(&diag_kind, &interner) {
//!     println!("💡 {}", suggestion);
//! }
//! ```
//!
//! # Design Notes
//! This module complements the [`formatting`] module by offering structured, interner-aware
//! formatting for diagnostic suggestions, separate from primary error/warning messages.

use crate::aiplan4rust::diagnostic::kind::Kind;
use crate::aiplan4rust::diagnostic::renderer::formatting;
use crate::aiplan4rust::diagnostic::{renderer, DiagnosticKind};
use crate::aiplan4rust::interner::SymbolInterner;
use crate::aiplan4rust::lang::{Requirement, SymbolId, Type};
use crate::aiplan4rust::semantic::symbol::{Declaration, Symbol, SymbolKind, Usage};
use crate::aiplan4rust::syntax::ast::AstKind;
use crate::aiplan4rust::syntax::Span;

/// Formats a suggestion string using the provided `StringInterner`.
///
/// This function generates a user-facing suggestion based on the diagnostic kind,
/// resolving any interned identifiers using the provided interner.
///
/// # Arguments
///
/// * `kind` - The diagnostic kind from which to derive the suggestion.
/// * `interner` - Reference to a `StringInterner` used to resolve identifiers.
///
/// # Returns
///
/// An `Option<String>` containing the suggestion, if one exists.
pub fn format_suggestion(kind: &DiagnosticKind, interner: &SymbolInterner) -> Option<String> {
    format_suggestion_internal(kind, Some(interner))
}

/// Formats a suggestion string for debugging purposes, without resolving interned identifiers.
///
/// This function is intended for test or debugging environments where a readable output is needed
/// without depending on a `StringInterner`.
///
/// # Arguments
///
/// * `kind` - The diagnostic kind from which to derive the suggestion.
///
/// # Returns
///
/// An `Option<String>` containing the suggestion, if one exists.
pub fn format_suggestion_debug(kind: &DiagnosticKind) -> Option<String> {
    format_suggestion_internal(kind, None)
}

/// Formats a diagnostic suggestion string, optionally resolving identifiers using a `StringInterner`.
///
/// This internal function analyzes the given [`DiagnosticKind`] and attempts to produce a
/// context-sensitive suggestion to help the user resolve the issue. If an interner is provided,
/// it is used to resolve any interned identifiers to human-readable strings.
///
/// This function is intended for internal use only. For public-facing usage, prefer
/// [`format_suggestion`] or [`format_suggestion_debug`] which wrap this function.
///
/// # Arguments
///
/// * `kind` - The [`DiagnosticKind`] for which a suggestion is to be generated.
/// * `interner` - An optional reference to a [`StringInterner`] for resolving identifiers.
///
/// # Returns
///
/// An [`Option<String>`] containing the suggestion if one is applicable, or `None` if no suggestion is relevant.
///
/// # Example
///
/// ```
/// if let Some(suggestion) = format_suggestion_internal(&kind, Some(&interner)) {
///     println!("💡 Suggestion: {}", suggestion);
/// }
/// ```
///
/// # Note
///
/// This function does not emit any diagnostics; it only generates suggestion strings.
///
/// [`DiagnosticKind`]: crate::aiplan4rust::diagnostics::DiagnosticKind
/// [`StringInterner`]: crate::aiplan4rust::interner::SymbolInterner
/// [`format_suggestion`]: crate::aiplan4rust::renderer::formatting::format_suggestion
/// [`format_suggestion_debug`]: crate::aiplan4rust::renderer::formatting::format_suggestion_debug
fn format_suggestion_internal(
    kind: &DiagnosticKind,
    interner: Option<&SymbolInterner>,
) -> Option<String> {
    match kind {
        Kind::UnexpectedToken { expected, .. } | Kind::UnexpectedEof { expected } => {
            renderer::formatting::format_expected_message(expected)
        }
        Kind::ExtraToken { .. } => Some(format_extra_token_suggestion()),
        Kind::InvalidNumber { .. } => Some(format_invalid_number_suggestion()),
        Kind::DuplicateDefinitionBlock { block } => {
            Some(format_duplicated_definition_block_suggestion(*block))
        }
        Kind::InvalidDefinitionBlockOrder { block, order } => {
            Some(format_invalid_definition_block_order(*block, order))
        }

        Kind::InvalidToken => Some(format_invalid_token_suggestion()),
        Kind::InvalidSymbolSignature { declaration, .. } => Some(
            format_invalid_symbol_signature_suggestion(declaration, interner),
        ),
        Kind::TypeMismatchInExpression { ty1, ty2 } => Some(
            format_type_mismatch_in_expression_suggestion(ty1, ty2, interner),
        ),
        Kind::InvalidTypesInNumericExpression { ty1, ty2 } => Some(
            format_invalid_types_in_numeric_expression_suggestion(ty1, ty2, interner),
        ),
        Kind::RequirementViolation {
            node_kind,
            required,
        } => Some(format_requirement_violation_suggestion(node_kind, required)),
        Kind::DuplicatedSymbolDeclarationInScope {
            symbol,
            original_declaration,
            conflicting_declaration,
            scope,
        } => Some(format_duplicated_symbol_declaration_suggestion(
            symbol,
            original_declaration,
            conflicting_declaration,
            scope,
            interner,
        )),
        Kind::DuplicateVariableSkeletonDeclaration {
            symbol,
            original_declaration,
            conflicting_declaration,
            scope,
        } => Some(format_duplicate_variable_skeleton_declaration_suggestion(
            symbol,
            original_declaration,
            conflicting_declaration,
            scope,
            interner,
        )),
        Kind::CyclicTaskOrdering => Some(format_cyclic_task_ordering_suggestion()),
        Kind::UndeclaredSymbol { usage } => format_undeclared_symbol_suggestion(usage, interner),
        Kind::SymbolConflictsWithKeyword {
            declaration,
            expected_kind,
            requirements,
        } => format_symbol_conflicts_with_keyword_suggestion(
            declaration,
            expected_kind,
            requirements,
            interner,
        ),
        Kind::SymbolDeclaredAmbiguouslyAsKeyword {
            declaration,
            requirements,
            ..
        } => format_symbol_declared_ambiguously_as_keyword_suggestion(
            declaration,
            requirements,
            interner,
        ),
        Kind::UnusedSymbol { declaration } => {
            format_unused_symbol_suggestion(declaration, interner)
        }
        Kind::DomainProblemNameMismatch { domain_name, .. } => {
            format_domain_problem_name_mismatch_suggestion(domain_name, interner)
        }
        Kind::AmbiguousTypePredicateSymbol { ty, .. } => {
            format_ambiguous_type_predicate_symbol_suggestion(ty, interner)
        }
        Kind::TaskArgumentIsSupertypeOfDeclaration {
            argument,
            type_declared,
            type_used,
        } => {
            format_task_argument_supertype_suggestion(argument, type_declared, type_used, interner)
        }
        Kind::DuplicateEitherType { duplicate_types } => {
            format_duplicate_either_type_suggestion(duplicate_types, interner)
        }
        Kind::CyclicTypeDeclaration { cycle } => {
            format_cyclic_type_declaration_suggestion(cycle, interner)
        }
        Kind::CrossConflictSymbolDeclaration {
            problem_declaration,
            conflicting_domain_declarations,
        } => format_cross_conflict_symbol_declaration_suggestion(
            problem_declaration,
            conflicting_domain_declarations,
            interner,
        ),

        Kind::DuplicateRequirementWarning {
            duplicate_requirements,
        } => format_duplicate_requirement_warning(duplicate_requirements),
        Kind::CustomError { suggestion, .. } => suggestion.clone(),
        Kind::CustomWarning { suggestion, .. } => suggestion.clone(),
        Kind::DuplicatedDeclaration {
            ty,
            kind,
            duplicate_spans,
            ..
        } => format_duplicated_declaration_suggestion(*ty, *kind, duplicate_spans, interner),
        Kind::IncompatibleTypeDeclarations {
            symbol,
            kind,
            original_span,
            expected_types,
            found_types,
        } => format_incompatible_type_declarations_suggestion(
            *symbol,
            *kind,
            *original_span,
            expected_types,
            found_types,
            interner,
        ),
        Kind::DeprecatedFeature { node_kind, .. } => {
            format_deprecated_feature_suggestion(*node_kind)
        }
        Kind::MissingMandatoryBlock { kind, .. } => {
            format_missing_mandatory_block_suggestion(*kind)
        }
        Kind::UndeclaredType { type_id, .. } => {
            format_undeclared_type_suggestion(*type_id, interner)
        }
        Kind::RedundantTypeUnion {
            symbol_id,
            simplified_type,
            ..
        } => format_redundant_type_union_suggestion(*symbol_id, simplified_type, interner),
    }
}

fn format_redundant_type_union_suggestion(
    symbol_id: SymbolId,
    simplified_type: &Type<SymbolId>,
    interner: Option<&SymbolInterner>,
) -> Option<String> {
    let symbol_name = formatting::ident_to_string(symbol_id, interner);

    Some(format!(
        "Consider updating the declaration of `{}` to use the simplified type `{}` to improve readability.",
        symbol_name, simplified_type
    ))
}

fn format_undeclared_type_suggestion(
    type_id: SymbolId,
    interner: Option<&SymbolInterner>,
) -> Option<String> {
    let type_name = formatting::ident_to_string(type_id, interner);

    Some(format!(
        "Ensure that the type `{}` is defined in the '(:types ...)' block of your domain, or check for typos.",
        type_name
    ))
}

fn format_missing_mandatory_block_suggestion(kind: AstKind) -> Option<String> {
    match kind {
        AstKind::Init => Some(
            "Add an ':init' block to define the starting state, e.g., '(:init (at robot loc1))'.".to_string()
        ),
        AstKind::Goal => Some(
            "Add a ':goal' block to define the objective, e.g., '(:goal (at robot loc2))'.".to_string()
        ),
        AstKind::InitialTaskNetwork => Some(
            "Add an ':htn' block to define the task network, including ':parameters' and ':subtasks'.".to_string()
        ),
        // On retourne None pour les cas où on n'a pas de suggestion spécifique
        // ou si on veut laisser le message d'erreur principal se suffire à lui-même.
        _ => None,
    }
}

/// Returns a suggestion message for a definition block that is declared in an invalid order.
///
/// # Arguments
///
/// * `block` - The `AstKind` of the block that is out of order.
/// * `expected_order` - A slice of `AstKind` listing the blocks that should appear before this one.
///
/// # Returns
///
/// A `String` suggesting the correct order for the block.
pub fn format_invalid_definition_block_order(block: AstKind, expected_order: &[AstKind]) -> String {
    let expected_names: Vec<String> = expected_order
        .iter()
        .map(|k| format!("'{}'", k.to_syntax_string())) // entoure chaque block de quotes
        .collect();
    format!(
        "Definition block '{}' is defined out of order. Expected to appear before: {}.",
        block.to_syntax_string(),
        expected_names.join(", ")
    )
}

/// Returns a user suggestion message for a duplicated definition block.
///
/// # Arguments
///
/// * `block` - The `AstKind` variant that was duplicated.
///
/// # Returns
///
/// A `String` suggesting that the user remove or relocate the earlier occurrence of the duplicated block.
pub fn format_duplicated_definition_block_suggestion(block: AstKind) -> String {
    format!(
        "Remove or relocate the earlier occurrence of the definition block '{}.'",
        block.to_syntax_string()
    )
}

/// Returns a formatted suggestion message for an invalid number.
///
/// Advises the user to use only digits and an optional decimal point.
fn format_invalid_number_suggestion() -> String {
    "Ensure you only use digits and an optional decimal point.".to_string()
}

/// Returns the formatted suggestion message for the `ExtraToken` diagnostic kind.
///
/// The message advises the user to check for unnecessary or misplaced tokens.
fn format_extra_token_suggestion() -> String {
    "Extra token detected. Check for unnecessary symbols or misplaced characters.".to_string()
}

/// Returns the suggestion message for the `InvalidToken` diagnostic kind.
///
/// This message advises the user to check for typos or invalid characters.
fn format_invalid_token_suggestion() -> String {
    "Make sure there are no typos or invalid characters.".to_string()
}

/// Returns a suggestion message for the `InvalidSymbolSignature` diagnostic kind.
///
/// This function formats a message indicating that a symbol's signature is invalid,
/// advising to ensure the symbol is declared properly in the corresponding block.
///
/// # Arguments
///
/// * `declaration` - The declaration containing the symbol.
/// * `interner` - Optional reference to a string interner used to convert symbols to strings.
///
/// # Returns
///
/// A formatted suggestion message as a `String`.
fn format_invalid_symbol_signature_suggestion(
    declaration: &Declaration,
    interner: Option<&SymbolInterner>,
) -> String {
    let symbol = declaration.symbol();
    let name = renderer::formatting::symbol_to_string(symbol, interner);
    match symbol.kind() {
        SymbolKind::Function => {
            format!("Ensure a function named '{}' with the correct signature is declared in the ':functions' block.", name)
        }
        SymbolKind::Predicate => {
            format!("Ensure a predicate named '{}' with the correct signature is declared in the ':predicates' block.", name)
        }
        SymbolKind::Task => {
            format!("Ensure a compound task '{}' with the correct signature is declared in the ':tasks' block.", name)
        }
        SymbolKind::Action => {
            format!("Ensure an action '{}' with the correct signature is declared in the ':action' block.", name)
        }
        _ => {
            format!(
                "Ensure '{}' with the correct signature is declared properly.",
                name
            )
        }
    }
}

/// Returns a suggestion message for the `TypeMismatchInExpression` diagnostic kind.
///
/// This function formats a message indicating that two types are incompatible in an expression,
/// advising to ensure compatibility according to the typing hierarchy.
///
/// # Arguments
///
/// * `ty1` - The first typing involved in the mismatch.
/// * `ty2` - The second typing involved in the mismatch.
/// * `interner` - Optional reference to a string interner used to convert types to strings.
///
/// # Returns
///
/// A formatted suggestion message as a `String`.
fn format_type_mismatch_in_expression_suggestion(
    ty1: &Type<SymbolId>,
    ty2: &Type<SymbolId>,
    interner: Option<&SymbolInterner>,
) -> String {
    let ty1_str = renderer::formatting::type_to_string(ty1, interner);
    let ty2_str = renderer::formatting::type_to_string(ty2, interner);

    format!(
        "The typing '{}' cannot be used with '{}' — make sure the types are compatible according to the typing hierarchy.",
        ty1_str,
        ty2_str
    )
}

/// Returns a suggestion message for the `InvalidTypesInNumericExpression` diagnostic kind.
///
/// This function formats a message indicating that operands in a numeric expression
/// are not of typing 'number', advising to ensure both operands are numeric types.
///
/// # Arguments
///
/// * `ty1` - The first operand's typing.
/// * `ty2` - The second operand's typing.
/// * `interner` - Optional reference to a string interner used to convert types to strings.
///
/// # Returns
///
/// A formatted suggestion message as a `String`.
fn format_invalid_types_in_numeric_expression_suggestion(
    ty1: &Type<SymbolId>,
    ty2: &Type<SymbolId>,
    interner: Option<&SymbolInterner>,
) -> String {
    let ty1_str = renderer::formatting::type_to_string(ty1, interner);
    let ty2_str = renderer::formatting::type_to_string(ty2, interner);

    format!(
        "Numeric logic require operands of typing 'number', but found '{}' and '{}'. \
        Ensure both operands are numeric types.",
        ty1_str, ty2_str
    )
}

/// Returns a suggestion message for the `RequirementViolation` diagnostic kind.
///
/// This function formats a message indicating that an expression typing requires certain
/// requirements, listing them clearly.
///
/// # Arguments
///
/// * `node_kind` - The kind of expression node requiring the constraints.
/// * `required` - A list of requirements that must be met.
///
/// # Returns
///
/// A formatted suggestion message as a `String`.
fn format_requirement_violation_suggestion(
    node_kind: &AstKind,
    required: &[Requirement],
) -> String {
    format!(
        "Expression typing '{}' requires one of these requirements: {}.",
        node_kind, // or format!("{:?}", node_kind) if needed
        renderer::formatting::format_requirement_list(required)
    )
}

/// Returns a suggestion message for duplicated symbol declarations within the same scope.
///
/// This function informs that a symbol is declared multiple times in a given scope,
/// specifying the conflicting declarations and advising to rename or fix usage.
///
/// # Arguments
///
/// * `symbol` - The symbol that is duplicated.
/// * `declaration1` - The original declaration of the symbol.
/// * `declaration2` - The conflicting declaration of the symbol.
/// * `scope` - The scope in which the duplication occurs.
///
/// # Returns
///
/// A formatted suggestion message as a `String`.
fn format_duplicated_symbol_declaration_suggestion(
    symbol: &Symbol,
    declaration1: &Declaration,
    declaration2: &Declaration,
    scope: &AstKind,
    interner: Option<&SymbolInterner>,
) -> String {
    let symbol_name = renderer::formatting::symbol_to_string(symbol, interner);
    format!(
        "The symbol '{}' is declared twice in the '{}' scope: once as a '{}' and again as a '{}'. \
         Consider renaming one of the declarations or ensuring consistent usage.",
        symbol_name,
        scope,
        declaration1.symbol_kind(),
        declaration2.symbol_kind()
    )
}

/// Returns a suggestion message for duplicated variable declarations within a skeleton scope.
///
/// # Arguments
///
/// * `symbol` - The variable symbol that is duplicated.
/// * `declaration1` - The original declaration.
/// * `declaration2` - The conflicting declaration.
/// * `scope` - The skeleton scope (e.g., AtomicFormulaSkeleton).
fn format_duplicate_variable_skeleton_declaration_suggestion(
    symbol: &Symbol,
    _declaration1: &Declaration,
    _declaration2: &Declaration,
    scope: &AstKind,
    interner: Option<&SymbolInterner>,
) -> String {
    let symbol_name = renderer::formatting::symbol_to_string(symbol, interner);
    format!(
        "The variable '{}' appears multiple times in the parameters of '{}'. \
         Consider using unique names (e.g., '{}_1', '{}_2') \
         to improve clarity and avoid ambiguity.",
        symbol_name, scope, symbol_name, symbol_name
    )
}

/// Returns a suggestion message for cyclic task ordering errors.
///
/// This function advises the user to check for loops in task dependencies or ordering constraints,
/// which cause cyclic ordering problems.
///
/// # Returns
///
/// A static suggestion message as a `String`.
fn format_cyclic_task_ordering_suggestion() -> String {
    "Check for loops in your task dependencies or ordering constraints.".to_string()
}

/// Returns a suggestion message for undeclared symbols based on their kind.
///
/// This function generates a helpful message to guide users to properly declare the symbol
/// according to its kind (function, predicate, action, variable, etc.).
///
/// # Arguments
///
/// * `usage` - The symbol usage information.
/// * `interner` - Optional string interner to convert symbols to strings.
///
/// # Returns
///
/// An optional suggestion string explaining where and how to declare the missing symbol.
fn format_undeclared_symbol_suggestion(
    usage: &Usage,
    interner: Option<&SymbolInterner>,
) -> Option<String> {
    let name = renderer::formatting::symbol_to_string(&usage.symbol(), interner);
    match usage.symbol_kind() {
        SymbolKind::Function => Some(format!(
            "Function '{}' is not declared. Declare it in the ':functions' section.",
            name
        )),
        SymbolKind::Predicate => Some(format!(
            "Predicate '{}' is not declared. Declare it in the ':predicates' section.",
            name
        )),
        SymbolKind::Action => Some(format!(
            "Action '{}' is not declared. Define it using the ':action' keyword.",
            name
        )),
        SymbolKind::DASymbol => Some(format!(
            "Durative action '{}' is not declared. Define it using the ':durative-action' keyword.",
            name
        )),
        SymbolKind::Method => Some(format!(
            "Method '{}' is not declared. Define it in the ':methods' section.",
            name
        )),
        SymbolKind::Task => Some(format!(
            "Task '{}' is not declared. Define it using the ':task' keyword.",
            name
        )),
        SymbolKind::TaskID => Some(format!(
            "Task identifier '{}' is not declared. Check the task network for missing definitions.",
            name
        )),
        SymbolKind::Constant => Some(format!(
            "Constant '{}' is not declared. Declare it in the ':constants' section (domain) or ':objects' section (problem).",
            name
        )),
        SymbolKind::DomainName => Some(format!(
            "Domain '{}' is not recognized. Make sure it matches the ':domain' declaration.",
            name
        )),
        SymbolKind::PrimitiveType => Some(format!(
            "Type '{}' is not declared. Declare it in the ':types' section.",
            name
        )),
        SymbolKind::ProblemName => Some(format!(
            "Problem '{}' is not recognized. Ensure the problem name is correctly defined.",
            name
        )),
        SymbolKind::Requirement => Some(format!(
            "Requirement '{}' is not recognized. Check for typos or unsupported features.",
            name
        )),
        SymbolKind::Variable => Some(format!(
            "Variable '{}' is not declared. You likely need to add it to the ':parameters' list of the enclosing definition (e.g., '?x - typing').",
            name
        )),
    }
}

/// Returns a suggestion message when a symbol conflicts with a reserved keyword.
///
/// This function generates a message explaining the conflict between a symbol and a reserved keyword,
/// including the required kind and related requirements.
///
/// # Arguments
///
/// * `declaration` - The symbol declaration causing the conflict.
/// * `expected_kind` - The expected kind the symbol should be declared as.
/// * `requirements` - The list of requirements related to this conflict.
/// * `interner` - Optional string interner to convert symbols to strings.
///
/// # Returns
///
/// An optional suggestion string describing the keyword conflict and guidance to fix it.
fn format_symbol_conflicts_with_keyword_suggestion(
    declaration: &Declaration,
    expected_kind: &SymbolKind,
    requirements: &[Requirement],
    interner: Option<&SymbolInterner>,
) -> Option<String> {
    let name = renderer::formatting::symbol_to_string(declaration.symbol(), interner);
    let reqs = renderer::formatting::format_requirement_list(requirements);
    Some(format!(
        "Symbol '{}' conflicts with a reserved keyword under requirements: {}. \
         It must be declared as a {:?} (e.g., typing, function, variable).",
        name, reqs, expected_kind
    ))
}

/// Returns a suggestion message when a symbol is ambiguously declared as a language keyword.
///
/// This function generates a message explaining that a symbol is ambiguous due to being used as
/// a language keyword with certain requirements, and suggests renaming or changing the symbol.
///
/// # Arguments
///
/// * `declaration` - The symbol declaration that is ambiguous.
/// * `requirements` - The list of requirements related to the ambiguity.
/// * `interner` - Optional string interner to convert symbols to strings.
///
/// # Returns
///
/// An optional suggestion string describing the ambiguity and guidance to fix it.
fn format_symbol_declared_ambiguously_as_keyword_suggestion(
    declaration: &Declaration,
    requirements: &[Requirement],
    interner: Option<&SymbolInterner>,
) -> Option<String> {
    let name = renderer::formatting::symbol_to_string(declaration.symbol(), interner);
    let reqs = renderer::formatting::format_requirement_list(requirements);
    Some(format!(
        "Symbol '{}' is ambiguous because it is used as a language keyword with \
        requirements: {}. Consider renaming or using a different symbol.",
        name, reqs
    ))
}

/// Returns a suggestion message for an unused symbol declaration.
///
/// This function generates a message indicating that a symbol is declared but not used,
/// and suggests removing it to clean up the code.
///
/// # Arguments
///
/// * `declaration` - The symbol declaration that is unused.
/// * `interner` - Optional string interner to convert symbols to strings.
///
/// # Returns
///
/// An optional suggestion string indicating the symbol is unused.
fn format_unused_symbol_suggestion(
    declaration: &Declaration,
    interner: Option<&SymbolInterner>,
) -> Option<String> {
    let name = renderer::formatting::symbol_to_string(declaration.symbol(), interner);
    Some(format!(
        "{} symbol '{}' is declared but not used. \
        Consider removing it to clean up your code.",
        declaration.symbol_kind(),
        name
    ))
}

/// Returns a suggestion message when the problem's domain name does not match the domain definition.
///
/// This function generates a message advising to check that the problem's domain name matches
/// the expected domain name.
///
/// # Arguments
///
/// * `domain_name` - The domain name symbol involved in the mismatch.
/// * `interner` - Optional string interner to convert symbols to strings.
///
/// # Returns
///
/// An optional suggestion string indicating the domain name mismatch.
fn format_domain_problem_name_mismatch_suggestion(
    domain_name: &Declaration,
    interner: Option<&SymbolInterner>,
) -> Option<String> {
    let domain_str = renderer::formatting::symbol_to_string(domain_name.symbol(), interner);
    Some(format!(
        "Check that the problem's domain name matches the domain definition: \
         expected '{}'.",
        domain_str
    ))
}

/// Returns a suggestion message when a symbol is declared both as a typing and a predicate.
///
/// This function generates a message advising to rename one of the conflicting declarations
/// to avoid ambiguity.
///
/// # Arguments
///
/// * `types` - The typing symbol involved in the ambiguity.
/// * `interner` - Optional string interner to convert symbols to strings.
///
/// # Returns
///
/// An optional suggestion string indicating the ambiguity.
fn format_ambiguous_type_predicate_symbol_suggestion(
    ty: &Declaration,
    interner: Option<&SymbolInterner>,
) -> Option<String> {
    let symbol = renderer::formatting::symbol_to_string(ty.symbol(), interner);
    Some(format!(
        "The symbol '{}' is declared both as a typing and a predicate. \
         Consider renaming one of them to avoid ambiguity.",
        symbol
    ))
}

/// Returns a suggestion message when a task argument's typing is a supertype of the declared typing.
///
/// This function advises that argument types should match exactly and suggests defining
/// a new method with matching types.
///
/// # Arguments
///
/// * `argument` - The argument symbol causing the issue.
/// * `type_declared` - The declared typing of the argument.
/// * `type_used` - The actual typing used which is a supertype.
/// * `interner` - Optional string interner to convert symbols and types to strings.
///
/// # Returns
///
/// An optional suggestion string explaining the supertype mismatch.
fn format_task_argument_supertype_suggestion(
    argument: &Declaration,
    type_declared: &Type<SymbolId>,
    type_used: &Type<SymbolId>,
    interner: Option<&SymbolInterner>,
) -> Option<String> {
    Some(format!(
        "The argument '{}' uses typing '{}' which is a supertype of the declared typing '{}'. \
         Argument types should match exactly. \
         Prefer defining a new method with matching types instead.",
        renderer::formatting::symbol_to_string(argument.symbol(), interner),
        renderer::formatting::type_to_string(type_used, interner),
        renderer::formatting::type_to_string(type_declared, interner)
    ))
}

/// Returns a suggestion message for duplicate types found in an 'either' typing declaration.
///
/// This function notifies that duplicate types are ignored but suggests removing them
/// to clean up the code.
///
/// # Arguments
///
/// * `duplicate_types` - A slice of duplicated typing identifiers.
/// * `interner` - Optional string interner to convert symbols and types to strings.
///
/// # Returns
///
/// An optional suggestion string describing the duplicate types found.
fn format_duplicate_either_type_suggestion(
    duplicate_types: &[SymbolId],
    interner: Option<&SymbolInterner>,
) -> Option<String> {
    let listed_types = if duplicate_types.len() == 1 {
        format!(
            "typing '{}'",
            renderer::formatting::format_ident_list(duplicate_types, interner)
        )
    } else {
        format!(
            "types '{}'",
            renderer::formatting::format_ident_list(duplicate_types, interner)
        )
    };
    Some(format!(
        "Duplicate {} found in an 'either' typing declaration; \
         these duplicates are ignored but consider removing them to clean up your code.",
        listed_types,
    ))
}

/// Returns a suggestion message for cycles detected in the typing hierarchy.
///
/// This function informs about the types involved in a cyclic inheritance
/// and suggests removing the cycle to resolve the issue.
///
/// # Arguments
///
/// * `cycle` - A slice of typing declarations forming the cycle.
/// * `interner` - Optional string interner to convert symbols and types to strings.
///
/// # Returns
///
/// An optional suggestion string describing the detected cycle.
fn format_cyclic_type_declaration_suggestion(
    cycle: &[Declaration], // Replace `TypeDeclaration` with the actual typing used in your code
    interner: Option<&SymbolInterner>,
) -> Option<String> {
    Some(format!(
        "Cycle detected in typing hierarchy involving types: {}. \
         Remove the cyclic inheritance to resolve the issue.",
        renderer::formatting::format_declaration_list(cycle, interner)
    ))
}

/// Returns a suggestion message for symbol declaration conflicts between problem and domain.
///
/// This function lists the locations of conflicting domain declarations and advises resolving
/// the conflict by consistent declarations or renaming.
///
/// # Arguments
///
/// * `problem_declaration` - The declaration of the symbol in the problem.
/// * `conflicting_domain_declarations` - A slice of conflicting domain declarations.
/// * `interner` - Optional string interner to convert symbols to strings.
///
/// # Returns
///
/// An optional suggestion string describing the conflict and its locations.
fn format_cross_conflict_symbol_declaration_suggestion(
    problem_declaration: &Declaration,
    conflicting_domain_declarations: &[Declaration],
    interner: Option<&SymbolInterner>,
) -> Option<String> {
    let domain_spans: Vec<String> = conflicting_domain_declarations
        .iter()
        .map(|decl| renderer::formatting::span_to_string(&decl.span()))
        .collect();

    let formatted_lines = match domain_spans.len() {
        0 => String::from("an unknown location"),
        1 => domain_spans[0].clone(),
        2 => format!("{} and {}", domain_spans[0], domain_spans[1]),
        _ => {
            let (all_but_last, last) = domain_spans.split_at(domain_spans.len() - 1);
            format!("{} and {}", all_but_last.join(", "), last[0])
        }
    };

    Some(format!(
        "Symbol `{}` declared as `{}` in the problem conflicts with domain declarations at lines: {}. \
         Please resolve these conflicts by ensuring consistent declarations or consider renaming the symbol in the problem.",
        renderer::formatting::symbol_to_string(problem_declaration.symbol(), interner),
        problem_declaration.symbol().kind(),
        formatted_lines,
    ))
}

/// Generates a contextual suggestion for resolving duplicated declarations.
///
/// This function identifies the locations of redundant declarations and provides
/// specific guidance based on the [`AstKind`]. It encourages the user to either
/// consolidate declarations or use the explicit PDDL `(either ...)` syntax to
/// resolve the implicit merge performed by the normalizer.
///
/// # Arguments
///
/// * `ty` - The [`SymbolId`] of the entity (object, type, constant, or function) that is duplicated.
/// * `kind` - The [`AstKind`] of the declaration block to tailor the suggestion text.
/// * `duplicate_spans` - A slice of [`Span`] marking the redundant declaration locations.
/// * `interner` - An optional [`SymbolInterner`] to resolve the identifier's string name.
///
/// # Returns
///
/// Returns an [`Option<String>`] containing the formatted suggestion. Returns `None` if
/// no suggestion can be generated (though in practice, this follows a duplicated declaration).
fn format_duplicated_declaration_suggestion(
    ty: SymbolId,
    kind: AstKind,
    duplicate_spans: &[Span],
    interner: Option<&SymbolInterner>,
) -> Option<String> {
    let identifier = renderer::formatting::ident_to_string(ty, interner);

    // 1. Formatage des localisations (ta logique existante est très bien)
    let duplicate_locations: Vec<String> = duplicate_spans
        .iter()
        .map(|span| renderer::formatting::span_to_string(span))
        .collect();

    let locations_text = match duplicate_locations.len() {
        0 => String::from("unknown locations"),
        1 => duplicate_locations[0].clone(),
        _ => {
            let (all_but_last, last) = duplicate_locations.split_at(duplicate_locations.len() - 1);
            format!("{} and {}", all_but_last.join(", "), last[0])
        }
    };

    // 2. Adaptation du message selon le Kind
    let (entity_type, suggestion) = match kind {
        AstKind::ObjectsDef => (
            "object",
            format!("consider grouping `{}` under a single declaration with an explicit `(either ...)` type", identifier)
        ),
        AstKind::TypesDef => (
            "type",
            format!("consider defining the hierarchy of `{}` once using `(either ...)` if it inherits from multiple supertypes", identifier)
        ),
        AstKind::ConstantsDef => (
            "constant",
            format!("ensure `{}` is defined only once to avoid conflicting constant definitions", identifier)
        ),
        AstKind::FunctionsDef => (
            "function",
            format!("check if the multiple declarations of `{}` have consistent parameter types or use a single signature", identifier)
        ),
        _ => (
            "entity",
            format!("consider explicitly using `(either ...)` to resolve the multiple declarations of `{}`", identifier)
        ),
    };

    Some(format!(
        "The {} `{}` was also declared at {}. {}.",
        entity_type, identifier, locations_text, suggestion
    ))
}

/// Formats a diagnostic suggestion message for incompatible type declarations.
///
/// This message is generated when a symbol is redeclared with a type signature
/// that violates PDDL constraints (e.g., mixing `number` with object types).
/// Unlike simple duplicates, these conflicts represent semantic errors that
/// must be resolved by the user.
///
/// # Arguments
///
/// * `symbol` - The identifier of the symbol with the type conflict.
/// * `kind` - The [`AstKind`] of the declaration, used to provide a specific
///   entity name (e.g., "function") in the suggestion.
/// * `original_span` - The source code location of the first valid declaration
///   found, used as the reference point for the conflict.
/// * `interner` - The symbol interner used to resolve the human-readable name
///   of the identifier.
///
/// # Returns
///
/// Returns an [`Option<String>`] containing a detailed explanation of why the
/// types are incompatible and a hint on how to fix the PDDL code.
fn format_incompatible_type_declarations_suggestion(
    symbol: SymbolId,
    kind: AstKind,
    original_span: Span,
    expected_types: &[SymbolId],
    found_types: &[SymbolId],
    interner: Option<&SymbolInterner>,
) -> Option<String> {
    let identifier = renderer::formatting::ident_to_string(symbol, interner);
    let original_location = renderer::formatting::span_to_string(&original_span);

    // Helper to format [A, B] as "(either A B)" or [A] as "A"
    let format_type_list = |types: &[SymbolId]| -> String {
        match types.len() {
            0 => "any".to_string(),
            1 => renderer::formatting::ident_to_string(types[0], interner),
            _ => {
                let names: Vec<String> = types
                    .iter()
                    .map(|&id| renderer::formatting::ident_to_string(id, interner))
                    .collect();
                format!("(either {})", names.join(" "))
            }
        }
    };

    let expected_str = format_type_list(expected_types);
    let found_str = format_type_list(found_types);

    // Context-aware messages based on the nature of the entity.
    let (entity_type, rule_hint) = match kind {
        AstKind::FunctionsDef => (
            "function",
            "Numeric fluents (number) and object types are strictly separated in PDDL",
        ),
        _ => (
            "symbol",
            "A symbol cannot be both a numeric fluent and a typed object",
        ),
    };

    Some(format!(
        "The {} `{}` was first declared as `{}`, but this declaration uses `{}`. {}. \
        The original declaration was found at {}.",
        entity_type, identifier, expected_str, found_str, rule_hint, original_location
    ))
}

/// Formats a warning message about duplicated domain requirements.
///
/// Lists the duplicated requirements and advises removing them to avoid redundancy.
///
/// # Arguments
///
/// * `duplicate_requirements` - A slice of requirement identifiers.
///
/// # Returns
///
/// An optional formatted warning message.
fn format_duplicate_requirement_warning(duplicate_requirements: &[Requirement]) -> Option<String> {
    Some(format!(
        "The following requirement(s) are declared multiple times in the domain and have been ignored. \
         Consider removing them to prevent redundancy: {}.",
        renderer::formatting::format_requirement_list(duplicate_requirements),
    ))
}

/// Returns a suggested replacement or action for a deprecated language feature.
///
/// This function provides actionable advice to the user when they encounter
/// a `DeprecatedFeature` diagnostic. It aims to guide the user toward
/// modern PDDL/HDDL equivalents.
///
/// # Arguments
///
/// * `kind` - The [`AstKind`] of the feature that has been flagged as deprecated.
///
/// # Returns
///
/// * `Some(String)` - A specific instruction on how to modernize the code.
/// * `None` - If no specific suggestion is available for this feature.
///
/// # Examples
///
/// ```
/// let suggestion = format_deprecated_feature_suggestion(AstKind::LengthSpec);
/// assert!(suggestion.unwrap().contains("use ':constraints'"));
/// ```
fn format_deprecated_feature_suggestion(kind: AstKind) -> Option<String> {
    match kind {
        AstKind::Length => Some(
            "Remove this section. For plan quality or constraints, \
                 use ':constraints' or ':metric' instead."
                .to_string(),
        ),
        // Fallback suggestion for any other deprecated feature without a specific path.
        _ => Some(
            "Consult the PDDL 3.1 or HDDL standard specifications for modern alternatives."
                .to_string(),
        ),
    }
}
