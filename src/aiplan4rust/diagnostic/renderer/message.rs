//! Module responsible for formatting diagnostic messages.
//!
//! This module provides functions to generate human-readable messages
//! from diagnostic kinds (`DiagnosticKind`). It supports formatting
//! with or without a `StringInterner` to resolve symbol and typing
//! identifiers into strings, enabling detailed and user-friendly
//! output.
//!
//! The main entry points are:
//! - `format_message` for detailed formatting with identifier resolution,
//! - `format_message_debug` for quick debug-oriented formatting without resolution.
//!
//! Internally, the module dispatches to specialized formatting functions
//! tailored to different diagnostic kinds, ensuring precise and
//! context-aware messages for various error and warning scenarios.

use crate::aiplan4rust::diagnostic::kind::Kind;
use crate::aiplan4rust::diagnostic::renderer::formatting;
use crate::aiplan4rust::diagnostic::DiagnosticKind;
use crate::aiplan4rust::interner::SymbolInterner;
use crate::aiplan4rust::lang::{SymbolId, Type};
use crate::aiplan4rust::semantic::symbol::{Declaration, Symbol, SymbolKind, Usage};
use crate::aiplan4rust::syntax::ast::AstKind;
use crate::Language;

/// Formats a diagnostic message using a `StringInterner`.
///
/// This function produces a detailed and user-friendly string describing the diagnostic,
/// resolving identifiers through the provided interner for more readable output.
///
/// # Arguments
///
/// * `kind` - The diagnostic kind to format.
/// * `interner` - Reference to a `StringInterner` used to resolve identifiers.
///
/// # Returns
///
/// A formatted string representing the error or warning message.
///
/// # Example
///
/// ```
/// let interner = StringInterner::new();
/// let message = format_message(&diagnostic_kind, &interner);
/// println!("{}", message);
/// ```
pub fn format_message(kind: &DiagnosticKind, interner: &SymbolInterner) -> String {
    format_message_internal(kind, Some(interner))
}

/// Formats a diagnostic message for debug purposes.
///
/// This function produces a simple, human-readable string describing the diagnostic
/// without using a `StringInterner`. It is primarily intended for debugging scenarios
/// where a quick, straightforward output is sufficient.
///
/// # Arguments
///
/// * `kind` - The diagnostic kind to format.
///
/// # Returns
///
/// A formatted string representing the error or warning message.
///
/// # Example
///
/// ```
/// let message = format_message_debug(&diagnostic_kind);
/// println!("{}", message);
/// ```
pub fn format_message_debug(kind: &DiagnosticKind) -> String {
    format_message_internal(kind, None)
}

/// Formats a diagnostic message into a string, optionally using a `StringInterner` to resolve identifiers.
///
/// This internal function handles all variants of `DiagnosticKind` by delegating
/// formatting to specialized helper functions. If an interner is provided, it is
/// used to produce more readable output for symbols and types; otherwise, a simpler
/// debug-friendly message is generated.
///
/// # Arguments
///
/// * `kind` - The diagnostic kind to format.
/// * `interner` - Optional reference to a `StringInterner` used for identifier resolution.
///
/// # Returns
///
/// A formatted string describing the diagnostic message.
///
/// # Note
///
/// This function is intended for internal use. Public-facing APIs should use
/// `format_message` or `format_message_debug` depending on whether interning is required.
fn format_message_internal(kind: &DiagnosticKind, interner: Option<&SymbolInterner>) -> String {
    match kind {
        Kind::UnexpectedToken { token, .. } => format_unexpected_token(token),
        Kind::UnexpectedEof { .. } => format_unexpected_eof(),
        Kind::InvalidToken => format_invalid_token(),
        Kind::ExtraToken { token } => format_extra_token(token),
        Kind::InvalidNumber { number, .. } => format_invalid_number(number),
        Kind::DuplicateDefinitionBlock { block } => format_duplicated_definition_block(*block),
        Kind::InvalidDefinitionBlockOrder { block, .. } => {
            format_invalid_definition_block_order(*block)
        }
        Kind::InvalidSymbolSignature { declaration, .. } => {
            format_invalid_symbol_signature(declaration, interner)
        }
        Kind::TypeMismatchInExpression { ty1, ty2 } => format_type_mismatch(ty1, ty2, interner),
        Kind::InvalidTypesInNumericExpression { ty1, ty2 } => {
            format_invalid_types_in_numeric_expression(ty1, ty2, interner)
        }
        Kind::RequirementViolation { node_kind, .. } => format_requirement_violation(node_kind),
        Kind::DuplicatedSymbolDeclarationInScope { symbol, .. } => {
            format_duplicated_symbol_declaration(symbol, interner)
        }
        Kind::DuplicateVariableSkeletonDeclaration { symbol, .. } => {
            format_duplicate_variable_skeleton_declaration(symbol, interner)
        }
        Kind::CyclicTaskOrdering => format_cyclic_task_ordering(),
        Kind::UndeclaredSymbol { usage } => format_undeclared_symbol(usage, interner),
        Kind::SymbolConflictsWithKeyword { declaration, .. } => {
            format_symbol_conflicts_with_keyword(declaration, interner)
        }
        Kind::SymbolDeclaredAmbiguouslyAsKeyword { declaration, .. } => {
            format_symbol_declared_ambiguously_as_keyword(declaration, interner)
        }
        Kind::UnusedSymbol { declaration } => format_unused_symbol(declaration, interner),
        Kind::DomainProblemNameMismatch {
            domain_name,
            problem_name,
        } => format_domain_problem_name_mismatch(domain_name, problem_name, interner),
        Kind::AmbiguousTypePredicateSymbol { ty, .. } => {
            format_ambiguous_type_predicate_symbol(ty, interner)
        }
        Kind::TaskArgumentIsSupertypeOfDeclaration { argument, .. } => {
            format_task_argument_is_supertype(argument, interner)
        }
        Kind::DuplicateEitherType { duplicate_types } => {
            format_duplicate_either_type(duplicate_types, interner)
        }
        Kind::CyclicTypeDeclaration { .. } => format_cyclic_type_declaration(),
        Kind::CrossConflictSymbolDeclaration {
            problem_declaration,
            ..
        } => format_cross_conflict_symbol_declaration(problem_declaration, interner),

        Kind::DuplicateRequirementWarning { .. } => format_duplicate_requirement_warning(),
        Kind::CustomError { message, .. } => message.to_string(),
        Kind::CustomWarning { message, .. } => message.to_string(),
        Kind::DuplicatedDeclaration { ty, kind, .. } => {
            format_duplicated_declaration(*ty, *kind, interner)
        }
        Kind::IncompatibleTypeDeclarations { symbol, kind, .. } => {
            format_incompatible_type_declarations(*symbol, *kind, interner)
        }
        Kind::DeprecatedFeature { node_kind, .. } => format_deprecated_feature(*node_kind),
        Kind::MissingMandatoryBlock { language, kind } => {
            format_missing_mandatory_block(*language, *kind)
        }
        Kind::UndeclaredType { type_id, .. } => format_undeclared_type(*type_id, interner),
        Kind::RedundantTypeUnion {
            symbol_id,
            original_type,
            simplified_type,
        } => format_redundant_type_union(*symbol_id, original_type, simplified_type, interner),
    }
}

fn format_redundant_type_union(
    symbol_id: SymbolId,
    original_type: &Type<SymbolId>,
    simplified_type: &Type<SymbolId>,
    interner: Option<&SymbolInterner>,
) -> String {
    let symbol_name = formatting::ident_to_string(symbol_id, interner);

    format!(
        "The type union for symbol `{}` is redundant. Original type `{}` was simplified to `{}`.",
        symbol_name, original_type, simplified_type
    )
}

fn format_undeclared_type(type_id: SymbolId, interner: Option<&SymbolInterner>) -> String {
    let type_name = formatting::ident_to_string(type_id, interner);

    format!("The type `{}` is not declared.", type_name)
}

fn format_missing_mandatory_block(language: Language, kind: AstKind) -> String {
    match kind {
        AstKind::Init => {
            format!(
                "The ':init' block is mandatory in a {} problem definition.",
                language.to_string()
            )
        }
        AstKind::Goal => "The ':goal' block is mandatory in a PDDL problem definition.".to_string(),
        AstKind::InitialTaskNetwork => {
            "The ':htn' block (Initial Task Network) is mandatory in a HDDL problem definition."
                .to_string()
        }
        _ => {
            format!(
                "The '{}' block is mandatory in this {} definition.",
                kind,
                language.to_string()
            )
        }
    }
}

/// Returns a suggestion message for a definition block that is invalidly placed.
///
/// # Arguments
///
/// * `block` - The `AstKind` of the block that is defined out of order.
///
/// # Returns
///
/// A `String` indicating that the block is out of order.
pub fn format_invalid_definition_block_order(block: AstKind) -> String {
    format!(
        "Definition block '{}' is defined out of order.",
        block.to_syntax_string()
    )
}

/// Returns a suggestion message for a duplicated definition block.
///
/// # Arguments
///
/// * `block` - The `AstKind` variant that was duplicated.
///
/// # Returns
///
/// A `String` suggesting that the user remove or relocate the earlier occurrence of the duplicated block.
pub fn format_duplicated_definition_block(block: AstKind) -> String {
    format!(
        "Duplicate definition block found '{}'.",
        block.to_syntax_string()
    )
}

/// Formats a message for an invalid number
///
/// # Arguments
///
/// * `number` - The invalid number string token.
///
/// # Returns
///
/// A formatted string indicating an invalid number.
fn format_invalid_number(number: &str) -> String {
    format!("Invalid number format '{}'.", number)
}

/// Formats a message for an unexpected token.
///
/// # Arguments
///
/// * `token` - The unexpected token string.
///
/// # Returns
///
/// A formatted string indicating an unexpected token.
fn format_unexpected_token(token: &str) -> String {
    format!("Unexpected token '{}'.", token)
}

/// Formats a message for an unexpected end of input (EOF).
///
/// # Returns
///
/// A formatted string indicating an unexpected EOF.
fn format_unexpected_eof() -> String {
    "Unexpected end of input (EOF).".to_string()
}

/// Formats a message for an invalid or malformed token.
///
/// # Returns
///
/// A formatted string indicating an invalid token.
fn format_invalid_token() -> String {
    "Unrecognized or malformed token.".to_string()
}

/// Formats a message for an unexpected extra token.
///
/// # Arguments
///
/// * `token` - The extra token string.
///
/// # Returns
///
/// A formatted string indicating an unexpected extra token.
fn format_extra_token(token: &str) -> String {
    format!("Unexpected extra token '{}'.", token)
}

/// Formats a message indicating an invalid symbol signature.
///
/// # Arguments
///
/// * `declaration` - The symbol declaration.
/// * `interner` - Optional interner for symbol name resolution.
///
/// # Returns
///
/// A formatted string describing the invalid symbol signature, with specific
/// formatting depending on the symbol kind.
fn format_invalid_symbol_signature(
    declaration: &Declaration,
    interner: Option<&SymbolInterner>,
) -> String {
    let symbol = declaration.symbol();
    let name = formatting::symbol_to_string(symbol, interner);
    match symbol.kind() {
        SymbolKind::Function => {
            format!("Function '{}' does not match any declared signature.", name)
        }
        SymbolKind::Predicate => format!(
            "Predicate '{}' does not match any declared signature.",
            name
        ),
        SymbolKind::Task => format!(
            "Compound task '{}' does not match any declared signature.",
            name
        ),
        SymbolKind::Action => format!(
            "Primitive task '{}' does not match any declared signature.",
            name
        ),
        _ => format!(
            "Symbol '{}' of kind {:?} does not match any declared signature.",
            name,
            symbol.kind()
        ),
    }
}

/// Formats a message indicating a typing mismatch between two types.
///
/// # Arguments
///
/// * `ty1` - The first typing involved in the mismatch.
/// * `ty2` - The second typing involved in the mismatch.
/// * `interner` - Optional interner for resolving typing names.
///
/// # Returns
///
/// A formatted string describing the typing mismatch.
fn format_type_mismatch(
    ty1: &Type<SymbolId>,
    ty2: &Type<SymbolId>,
    interner: Option<&SymbolInterner>,
) -> String {
    let ty1_str = formatting::type_to_string(ty1, interner);
    let ty2_str = formatting::type_to_string(ty2, interner);
    format!("Type mismatch between '{}' and '{}'.", ty1_str, ty2_str)
}

/// Formats a message indicating invalid operand types in a numeric expression.
///
/// # Arguments
///
/// * `ty1` - The first operand typing.
/// * `ty2` - The second operand typing.
/// * `interner` - Optional interner for resolving typing names.
///
/// # Returns
///
/// A formatted string describing the invalid operand types for a numeric expression.
fn format_invalid_types_in_numeric_expression(
    ty1: &Type<SymbolId>,
    ty2: &Type<SymbolId>,
    interner: Option<&SymbolInterner>,
) -> String {
    let ty1_str = formatting::type_to_string(ty1, interner);
    let ty2_str = formatting::type_to_string(ty2, interner);
    format!("Invalid operand types for numeric expression: '{}' and '{}'. Operands must be numeric types.", ty1_str, ty2_str)
}

/// Formats a message indicating a requirement violation for a given expression typing.
///
/// # Arguments
///
/// * `node_kind` - The kind of AST node representing the expression typing.
///
/// # Returns
///
/// A formatted string stating that the expression typing is disallowed by current requirements.
fn format_requirement_violation(node_kind: &AstKind) -> String {
    format!(
        "Expression typing '{}' disallowed by current requirements.",
        node_kind
    )
}

/// Formats a message indicating a duplicated symbol declaration within the same scope.
///
/// # Arguments
///
/// * `symbol` - The duplicated symbol.
/// * `interner` - Optional interner for resolving the symbol's name.
///
/// # Returns
///
/// A formatted string warning about the symbol being declared multiple times in the same scope.
fn format_duplicated_symbol_declaration(
    symbol: &Symbol,
    interner: Option<&SymbolInterner>,
) -> String {
    let name = formatting::symbol_to_string(symbol, interner);
    format!(
        "Symbol '{}' is declared multiple times in the same scope.",
        name
    )
}

/// Formats a message indicating a duplicated variable declaration within a skeleton scope.
///
/// # Arguments
///
/// * `symbol` - The duplicated variable symbol.
/// * `interner` - Optional interner for resolving the symbol's name.
///
/// # Returns
///
/// A formatted string warning about the variable being declared multiple times in a skeleton.
fn format_duplicate_variable_skeleton_declaration(
    symbol: &Symbol,
    interner: Option<&SymbolInterner>,
) -> String {
    let name = formatting::symbol_to_string(symbol, interner);
    format!(
        "Variable '{}' is declared multiple times in the same skeleton definition.",
        name
    )
}

/// Formats a message indicating detection of a cyclic task-ordering constraint.
///
/// # Returns
///
/// A formatted string describing the cyclic task-ordering constraint.
fn format_cyclic_task_ordering() -> String {
    "Cyclic task-ordering constraint detected.".to_string()
}

/// Formats a message indicating an undeclared symbol usage.
///
/// # Arguments
///
/// * `usage` - The usage of the symbol.
/// * `interner` - Optional interner for resolving the symbol's name.
///
/// # Returns
///
/// A formatted string stating that the symbol is undeclared.
fn format_undeclared_symbol(usage: &Usage, interner: Option<&SymbolInterner>) -> String {
    let name = formatting::symbol_to_string(usage.symbol(), interner);
    format!("{} symbol '{}' is undeclared.", usage.symbol_kind(), name)
}

/// Formats a message indicating that a symbol conflicts with a language keyword.
///
/// # Arguments
///
/// * `declaration` - The declaration of the symbol conflicting with a keyword.
/// * `interner` - Optional interner for resolving the symbol's name.
///
/// # Returns
///
/// A formatted string stating that the symbol is used as a language keyword.
fn format_symbol_conflicts_with_keyword(
    declaration: &Declaration,
    interner: Option<&SymbolInterner>,
) -> String {
    let name = formatting::symbol_to_string(declaration.symbol(), interner);
    format!("Symbol '{}' is used as a language keyword", name)
}

/// Formats a message indicating that a symbol is ambiguously declared as a language keyword.
///
/// # Arguments
///
/// * `declaration` - The declaration of the ambiguous symbol.
/// * `interner` - Optional interner for resolving the symbol's name.
///
/// # Returns
///
/// A formatted string stating that the symbol is ambiguous as a language keyword.
fn format_symbol_declared_ambiguously_as_keyword(
    declaration: &Declaration,
    interner: Option<&SymbolInterner>,
) -> String {
    let name = formatting::symbol_to_string(declaration.symbol(), interner);
    format!("Symbol '{}' is ambiguous as a language keyword", name)
}

/// Formats a message indicating that a symbol is declared but unused.
///
/// # Arguments
///
/// * `declaration` - The declaration of the unused symbol.
/// * `interner` - Optional interner for resolving the symbol's name.
///
/// # Returns
///
/// A formatted string stating that the symbol is unused.
fn format_unused_symbol(declaration: &Declaration, interner: Option<&SymbolInterner>) -> String {
    let name = formatting::symbol_to_string(&declaration.symbol(), interner);
    format!("{} symbol '{}' is unused", declaration.symbol_kind(), name)
}

/// Formats a message indicating that domain and problem names do not match.
///
/// # Arguments
///
/// * `domain_name` - The declaration of the domain name.
/// * `problem_name` - The declaration of the problem name.
/// * `interner` - Optional interner for resolving the names.
///
/// # Returns
///
/// A formatted string stating that the domain and problem names do not match.
fn format_domain_problem_name_mismatch(
    domain_name: &Declaration,
    problem_name: &Declaration,
    interner: Option<&SymbolInterner>,
) -> String {
    let domain_str = formatting::symbol_to_string(domain_name.symbol(), interner);
    let problem_str = formatting::symbol_to_string(problem_name.symbol(), interner);
    format!(
        "Domain '{}' and problem '{}' names do not match.",
        domain_str, problem_str
    )
}

/// Formats a message indicating an ambiguous symbol declared both as a typing and a predicate.
///
/// # Arguments
///
/// * `types` - The declaration of the ambiguous symbol.
/// * `interner` - Optional interner for resolving the symbol's name.
///
/// # Returns
///
/// A formatted string stating the ambiguity of the symbol.
fn format_ambiguous_type_predicate_symbol(
    ty: &Declaration,
    interner: Option<&SymbolInterner>,
) -> String {
    let ty_name = formatting::symbol_to_string(ty.symbol(), interner);
    format!(
        "Ambiguous symbol '{}': declared both as a typing and a predicate.",
        ty_name
    )
}

/// Formats a message indicating that a task argument uses a broader (super) typing than declared.
///
/// # Arguments
///
/// * `argument` - The declaration of the argument.
/// * `interner` - Optional interner for resolving the argument's name.
///
/// # Returns
///
/// A formatted string warning about the upcasting issue.
fn format_task_argument_is_supertype(
    argument: &Declaration,
    interner: Option<&SymbolInterner>,
) -> String {
    format!(
        "Type mismatch: argument '{}' uses a broader typing than declared (upcasting is discouraged).",
        formatting::symbol_to_string(argument.symbol(), interner)
    )
}

/// Formats a message listing duplicate primitive types in an 'either' typing.
///
/// # Arguments
///
/// * `duplicate_types` - Slice of identifiers that are duplicated.
/// * `interner` - Optional interner for resolving the identifiers.
///
/// # Returns
///
/// A formatted string listing the duplicate types.
fn format_duplicate_either_type(
    duplicate_types: &[SymbolId],
    interner: Option<&SymbolInterner>,
) -> String {
    let names = formatting::format_ident_list(duplicate_types, interner);
    format!("Duplicate primitive types in 'either' typing: {}.", names)
}

/// Formats a message indicating that typing declarations form a cyclic hierarchy, which is invalid.
///
/// # Returns
///
/// A formatted string describing the invalid cyclic typing declarations.
fn format_cyclic_type_declaration() -> String {
    "Type declarations form a cycle; this creates an invalid typing hierarchy.".to_string()
}

/// Formats a message indicating a conflicting declaration of a symbol
/// found between the problem and domain declarations.
///
/// # Arguments
///
/// * `problem_declaration` - The declaration of the conflicting symbol in the problem.
/// * `interner` - Optional interner for resolving the symbol's name.
///
/// # Returns
///
/// A formatted string describing the conflict between problem and domain symbol declarations.
fn format_cross_conflict_symbol_declaration(
    problem_declaration: &Declaration,
    interner: Option<&SymbolInterner>,
) -> String {
    format!(
        "Conflicting declaration for symbol '{}' found between problem and domain.",
        formatting::symbol_to_string(problem_declaration.symbol(), interner)
    )
}

/// Formats a diagnostic message for symbols declared multiple times with compatible types.
///
/// This message informs the user that the symbol has been merged. In PDDL normalization,
/// multiple declarations of the same symbol result in a union of their parent types
/// using an `(either ...)` construct.
///
/// # Arguments
///
/// * `ty` - The [`SymbolId`] of the duplicated symbol.
/// * `kind` - The [`AstKind`] of the declaration to determine the entity type.
/// * `interner` - The interner used to resolve the symbol's string representation.
fn format_duplicated_declaration(
    ty: SymbolId,
    kind: AstKind,
    interner: Option<&SymbolInterner>,
) -> String {
    let identifier = formatting::ident_to_string(ty, interner);
    let entity_name = ast_kind_to_entity_name(kind);

    format!(
        "{} `{}` was declared multiple times and its parent types were implicitly merged into an `(either ...)` type.",
        entity_name,
        identifier
    )
}

/// Formats the primary error header for incompatible type declarations.
///
/// This header is used when a symbol cannot be merged due to type constraints,
/// such as a name being used for both a numeric fluent and an object.
///
/// # Arguments
///
/// * `symbol` - The [`SymbolId`] of the conflicting symbol.
/// * `kind` - The [`AstKind`] used to specify the entity in the message.
/// * `interner` - The interner used to resolve the symbol's string representation.
///
/// # Example
///
/// Output: `incompatible type declarations for function distance`
fn format_incompatible_type_declarations(
    symbol: SymbolId,
    kind: AstKind,
    interner: Option<&SymbolInterner>,
) -> String {
    let identifier = formatting::ident_to_string(symbol, interner);
    let entity_name = ast_kind_to_entity_name(kind);

    format!(
        "incompatible type declarations for {} `{}`",
        entity_name.to_lowercase(),
        identifier
    )
}

/// Maps an [`AstKind`] to its human-readable singular entity name.
///
/// This helper is used to normalize the terminology used in diagnostic messages
/// across different PDDL definitions (objects, types, constants, etc.).
///
/// # Arguments
///
/// * `kind` - The AST node kind representing the definition block.
///
/// # Returns
///
/// A static string slice containing the capitalized entity name (e.g., "Function").
fn ast_kind_to_entity_name(kind: AstKind) -> &'static str {
    match kind {
        AstKind::ObjectsDef => "Object",
        AstKind::TypesDef => "Type",
        AstKind::ConstantsDef => "Constant",
        AstKind::FunctionsDef => "Function",
        _ => "Entity",
    }
}

/// Formats a warning message indicating duplicated requirement declarations.
///
/// # Returns
///
/// A formatted string warning that duplicated requirement declarations were ignored.
fn format_duplicate_requirement_warning() -> String {
    "Requirements definition contains duplicated declarations which have been ignored.".to_string()
}

/// Formate un message d'avertissement pour les fonctionnalités obsolètes.
fn format_deprecated_feature(kind: AstKind) -> String {
    match kind {
        AstKind::Length => {
            "The ':length' section is deprecated since PDDL 2.1. \
             It was originally used for parallelization hints but is now ignored by modern planners."
                .to_string()
        }
        _ => format!("The feature '{:?}' is deprecated in the current PDDL/HDDL standard.", kind),
    }
}
