//! Module responsible for formatting diagnostic messages.
//!
//! This module provides functions to generate human-readable messages
//! from diagnostic kinds (`DiagnosticKind`). It supports formatting
//! with or without a `StringInterner` to resolve symbol and type
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


use crate::aiplan4rust::diagnostic::DiagnosticKind;
use crate::aiplan4rust::diagnostic::kind::Kind;
use crate::aiplan4rust::diagnostic::renderer::formatting;
use crate::aiplan4rust::interner::{Ident, StringInterner};
use crate::aiplan4rust::lang::Type;
use crate::aiplan4rust::semantic::symbol::{Declaration, Symbol, SymbolKind, Usage};
use crate::aiplan4rust::syntax::ast::AstKind;

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
pub fn format_message(kind: &DiagnosticKind, interner: &StringInterner) -> String {
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
fn format_message_internal(kind: &DiagnosticKind, interner: Option<&StringInterner>) -> String {
    match kind {
        Kind::UnexpectedToken { token, .. } => format_unexpected_token(token),
        Kind::UnexpectedEof { .. } => format_unexpected_eof(),
        Kind::InvalidToken => format_invalid_token(),
        Kind::ExtraToken { token } => format_extra_token(token),
        Kind::User { message } => format_user_message(message),
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
        Kind::CyclicTaskOrdering => format_cyclic_task_ordering(),
        Kind::UndeclaredSymbol { usage } => format_undeclared_symbol(usage, interner),
        Kind::SymbolConflictsWithKeyword { declaration, .. } => {
            format_symbol_conflicts_with_keyword(declaration, interner)
        }
        Kind::SymbolDeclaredAmbiguouslyAsKeyword { declaration, .. } => {
            format_symbol_declared_ambiguously_as_keyword(declaration, interner)
        }
        Kind::UnusedSymbol { declaration } => format_unused_symbol(declaration, interner),
        Kind::DomainProblemNameMismatch { domain_name, problem_name } => {
            format_domain_problem_name_mismatch(domain_name, problem_name, interner)
        }
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
        Kind::CrossConflictSymbolDeclaration { problem_declaration, .. } => {
            format_cross_conflict_symbol_declaration(problem_declaration, interner)
        }
        Kind::ImplicitEitherTypeDeclaration { ty, .. } => {
            format_implicit_either_type_declaration(*ty, interner)
        }
        Kind::DuplicateRequirementWarning { .. } => format_duplicate_requirement_warning(),
        Kind::CustomError { message, .. } => message.to_string(),
        Kind::CustomWarning { message, .. } => message.to_string(),
    }
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

/// Formats a user-provided message.
///
/// # Arguments
///
/// * `message` - The user message string.
///
/// # Returns
///
/// The same message string as provided.
fn format_user_message(message: &str) -> String {
    message.to_string()
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
fn format_invalid_symbol_signature(declaration: &Declaration, interner: Option<&StringInterner>) -> String {
    let symbol = declaration.symbol();
    let name = formatting::symbol_to_string(symbol, interner);
    match symbol.kind() {
        SymbolKind::Function => format!("Function '{}' does not match any declared signature.", name),
        SymbolKind::Predicate => format!("Predicate '{}' does not match any declared signature.", name),
        SymbolKind::Task => format!("Compound task '{}' does not match any declared signature.", name),
        SymbolKind::Action => format!("Primitive task '{}' does not match any declared signature.", name),
        _ => format!("Symbol '{}' of kind {:?} does not match any declared signature.", name, symbol.kind()),
    }
}

/// Formats a message indicating a type mismatch between two types.
///
/// # Arguments
///
/// * `ty1` - The first type involved in the mismatch.
/// * `ty2` - The second type involved in the mismatch.
/// * `interner` - Optional interner for resolving type names.
///
/// # Returns
///
/// A formatted string describing the type mismatch.
fn format_type_mismatch(ty1: &Type, ty2: &Type, interner: Option<&StringInterner>) -> String {
    let ty1_str = formatting::type_to_string(ty1, interner);
    let ty2_str = formatting::type_to_string(ty2, interner);
    format!("Type mismatch between '{}' and '{}'.", ty1_str, ty2_str)
}

/// Formats a message indicating invalid operand types in a numeric expression.
///
/// # Arguments
///
/// * `ty1` - The first operand type.
/// * `ty2` - The second operand type.
/// * `interner` - Optional interner for resolving type names.
///
/// # Returns
///
/// A formatted string describing the invalid operand types for a numeric expression.
fn format_invalid_types_in_numeric_expression(ty1: &Type, ty2: &Type, interner: Option<&StringInterner>) -> String {
    let ty1_str = formatting::type_to_string(ty1, interner);
    let ty2_str = formatting::type_to_string(ty2, interner);
    format!("Invalid operand types for numeric expression: '{}' and '{}'. Operands must be numeric types.", ty1_str, ty2_str)
}

/// Formats a message indicating a requirement violation for a given expression type.
///
/// # Arguments
///
/// * `node_kind` - The kind of AST node representing the expression type.
///
/// # Returns
///
/// A formatted string stating that the expression type is disallowed by current requirements.
fn format_requirement_violation(node_kind: &AstKind) -> String {
    format!("Expression type '{}' disallowed by current requirements.", node_kind)
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
fn format_duplicated_symbol_declaration(symbol: &Symbol, interner: Option<&StringInterner>) -> String {
    let name = formatting::symbol_to_string(symbol, interner);
    format!("Symbol '{}' is declared multiple times in the same scope.", name)
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
fn format_undeclared_symbol(usage: &Usage, interner: Option<&StringInterner>) -> String {
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
fn format_symbol_conflicts_with_keyword(declaration: &Declaration, interner: Option<&StringInterner>) -> String {
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
fn format_symbol_declared_ambiguously_as_keyword(declaration: &Declaration, interner: Option<&StringInterner>) -> String {
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
fn format_unused_symbol(declaration: &Declaration, interner: Option<&StringInterner>) -> String {
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
fn format_domain_problem_name_mismatch(domain_name: &Declaration, problem_name: &Declaration, interner: Option<&StringInterner>) -> String {
    let domain_str = formatting::symbol_to_string(domain_name.symbol(), interner);
    let problem_str = formatting::symbol_to_string(problem_name.symbol(), interner);
    format!("Domain '{}' and problem '{}' names do not match.", domain_str, problem_str)
}

/// Formats a message indicating an ambiguous symbol declared both as a type and a predicate.
///
/// # Arguments
///
/// * `ty` - The declaration of the ambiguous symbol.
/// * `interner` - Optional interner for resolving the symbol's name.
///
/// # Returns
///
/// A formatted string stating the ambiguity of the symbol.
fn format_ambiguous_type_predicate_symbol(ty: &Declaration, interner: Option<&StringInterner>) -> String {
    let ty_name = formatting::symbol_to_string(ty.symbol(), interner);
    format!("Ambiguous symbol '{}': declared both as a type and a predicate.", ty_name)
}

/// Formats a message indicating that a task argument uses a broader (super) type than declared.
///
/// # Arguments
///
/// * `argument` - The declaration of the argument.
/// * `interner` - Optional interner for resolving the argument's name.
///
/// # Returns
///
/// A formatted string warning about the upcasting issue.
fn format_task_argument_is_supertype(argument: &Declaration, interner: Option<&StringInterner>) -> String {
    format!(
        "Type mismatch: argument '{}' uses a broader type than declared (upcasting is discouraged).",
        formatting::symbol_to_string(argument.symbol(), interner)
    )
}

/// Formats a message listing duplicate primitive types in an 'either' type.
///
/// # Arguments
///
/// * `duplicate_types` - Slice of identifiers that are duplicated.
/// * `interner` - Optional interner for resolving the identifiers.
///
/// # Returns
///
/// A formatted string listing the duplicate types.
fn format_duplicate_either_type(duplicate_types: &[Ident], interner: Option<&StringInterner>) -> String {
    let names = formatting::format_ident_list(duplicate_types, interner);
    format!("Duplicate primitive types in 'either' type: {}.", names)
}

/// Formats a message indicating that type declarations form a cyclic hierarchy, which is invalid.
///
/// # Returns
///
/// A formatted string describing the invalid cyclic type declarations.
fn format_cyclic_type_declaration() -> String {
    "Type declarations form a cycle; this creates an invalid type hierarchy.".to_string()
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
fn format_cross_conflict_symbol_declaration(problem_declaration: &Declaration, interner: Option<&StringInterner>) -> String {
    format!(
        "Conflicting declaration for symbol '{}' found between problem and domain.",
        formatting::symbol_to_string(problem_declaration.symbol(), interner)
    )
}

/// Formats a message indicating that a type was declared multiple times and
/// was implicitly interpreted as an `(either ...)` type.
///
/// # Arguments
///
/// * `ty` - The identifier of the type declared multiple times.
/// * `interner` - Optional interner for resolving the identifier.
///
/// # Returns
///
/// A formatted string explaining the implicit interpretation as an either-type.
fn format_implicit_either_type_declaration(ty: Ident, interner: Option<&StringInterner>) -> String {
    format!(
        "Type `{}` was declared multiple times and was implicitly interpreted as an `(either ...)` type.",
        formatting::ident_to_string(ty, interner),
    )
}

/// Formats a warning message indicating duplicated requirement declarations.
///
/// # Returns
///
/// A formatted string warning that duplicated requirement declarations were ignored.
fn format_duplicate_requirement_warning() -> String {
    "Requirements definition contains duplicated declarations which have been ignored.".to_string()
}
