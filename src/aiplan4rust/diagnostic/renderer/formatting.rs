use crate::aiplan4rust::interner::{Ident, StringInterner};
use crate::aiplan4rust::lang::{Requirement, Type};
use crate::aiplan4rust::semantic::symbol::{Declaration, Symbol};
use crate::aiplan4rust::syntax::{Span, SyntaxDisplay};

/// Returns the name of the given identifier as a `String`, optionally resolving it
/// through a string interner.
///
/// # Parameters
/// - `ident`: The identifier to convert to a string.
/// - `interner`: Optional reference to a `StringInterner` used to resolve the identifier.
///
/// # Returns
/// A `String` representing the resolved name of the identifier.
/// - If the `interner` is provided and the identifier is found, returns the resolved string.
/// - If the `interner` is provided but the identifier is not found, returns `"unknown(<ident>)"`.
/// - If the `interner` is not provided, returns the raw identifier as a string.
pub(crate) fn ident_to_string(ident: Ident, interner: Option<&StringInterner>) -> String {
    if let Some(interner) = interner {
        interner
            .resolve_ident(ident)
            .map(|s| s.to_string())
            .unwrap_or_else(|| format!("unknown({})", ident))
    } else {
        ident.to_string()
    }
}

/// Returns the name of the given symbol as a `String`, optionally resolving it
/// through a string interner by using the `InternerDisplay` trait implementation.
///
/// # Parameters
/// - `symbol`: Reference to the `Symbol` whose name is to be retrieved.
/// - `interner`: Optional reference to a `StringInterner` used to resolve the symbol's identifier.
///
/// # Returns
/// A `String` representing the resolved name of the symbol.
/// - If the `interner` is provided and the identifier is found, returns the resolved string.
/// - If the `interner` is provided but the identifier is not found, returns `"unknown(<ident>)"`.
/// - If the `interner` is not provided, returns the raw identifier as a string.
pub(crate) fn symbol_to_string(symbol: &Symbol, interner: Option<&StringInterner>) -> String {
    ident_to_string(symbol.ident(), interner)
}

/// Converts a `Type` to a `String`, optionally resolving identifiers via a `StringInterner`.
///
/// # Parameters
/// - `ty`: The `Type` to convert to a string.
/// - `interner`: An optional reference to a `StringInterner` used to resolve identifiers.
///
/// # Returns
/// A `String` representation of the `Type`. If `interner` is provided, the identifiers
/// inside the `Type` are resolved using it; otherwise, the default string representation is used.
pub(crate) fn type_to_string(ty: &Type, interner: Option<&StringInterner>) -> String {
    if let Some(interner) = interner {
        ty.to_syntax_string(interner)
    } else {
        ty.to_string()
    }
}

/// Formats a list of `Ident` values into a comma-separated string, resolving each ident using
/// an optional `StringInterner`.
///
/// # Parameters
/// - `idents`: Slice of `Ident` to format.
/// - `interner`: Optional reference to a `StringInterner` used to resolve identifiers.
///
/// # Returns
/// A string of comma-separated identifiers, each converted to string via `ident_to_string`.
pub(crate) fn format_ident_list(idents: &[Ident], interner: Option<&StringInterner>) -> String {
    idents
        .iter()
        .map(|&ident| ident_to_string(ident, interner))
        .collect::<Vec<_>>()
        .join(", ")
}

/// Converts a `Span` into a user-friendly string representation of its line range.
///
/// This function is designed to simplify span information for end-user messages
/// by focusing only on line numbers. If the span covers a single line,
/// it returns `"line N"`. If it spans multiple lines, it returns `"lines N–M"`.
///
/// # Arguments
///
/// * `span` - A reference to the `Span` to be formatted.
///
/// # Returns
///
/// A `String` describing the line(s) the span covers.
///
/// # Examples
///
/// ```rust
/// let span = Span::new(0, 10, 12, 1, 12, 5); // example: same line
/// assert_eq!(span_to_string(&span), "line 12");
///
/// let span = Span::new(0, 20, 14, 1, 16, 10); // example: multi-line
/// assert_eq!(span_to_string(&span), "lines 14–16");
/// ```
pub(crate) fn span_to_string(span: &Span) -> String {
    if span.begin_line() == span.end_line() {
        format!("line {}", span.begin_line())
    } else {
        format!("lines {}–{}", span.begin_line(), span.end_line())
    }
}

/// Formats a list of `Declaration`s into a comma-separated string by extracting their symbol identifiers
/// and resolving them using an optional `StringInterner`.
///
/// # Parameters
/// - `declarations`: Slice of `Declaration` to format.
/// - `interner`: Optional reference to a `StringInterner` used to resolve identifiers.
///
/// # Returns
/// A string of comma-separated symbols (idents) of the declarations, resolved via `format_ident_list`.
pub(crate) fn format_declaration_list(
    declarations: &[Declaration],
    interner: Option<&StringInterner>
) -> String {
    let idents: Vec<Ident> = declarations.iter().map(|decl| decl.symbol_ident()).collect();
    format_ident_list(&idents, interner)
}

/// Formats a slice of `Requirement`s into a comma-separated string,
/// each requirement enclosed in single quotes.
///
/// # Parameters
/// - `requirements`: Slice of `Requirement` items to format.
///
/// # Returns
/// A `String` listing all requirements, each wrapped in single quotes
/// and separated by commas.
///
/// # Example
/// ```
/// let reqs = vec![Requirement::A, Requirement::B];
/// let formatted = format_requirements_list(&reqs);
/// assert_eq!(formatted, "'A', 'B'");
/// ```
pub(crate) fn format_requirement_list(requirements: &[Requirement]) -> String {
    requirements
        .iter()
        .map(|r| format!("'{}'", r))  // Assumes Requirement implements Display
        .collect::<Vec<_>>()
        .join(", ")
}

/// Formats a user-friendly error message based on a list of expected tokens.
///
/// This function takes a slice of expected token strings and returns an optional
/// formatted message describing what tokens were expected at a certain point in parsing.
///
/// # Arguments
///
/// * `expected` - A slice of strings representing the tokens expected by the parser.
///
/// # Returns
///
/// An `Option<String>` containing a descriptive message:
/// - If no expected tokens are provided (`expected` is empty), returns a generic unexpected input message.
/// - If exactly one token is expected, returns a message specifying that token.
/// - If multiple tokens are expected, returns a message listing all possible expected tokens.
///
/// # Examples
///
/// ```
/// let expected = vec!["identifier".to_string()];
/// assert_eq!(
///     format_expected_message(&expected),
///     Some("Expected token: `identifier`.".to_string())
/// );
///
/// let multiple = vec![";".to_string(), "}".to_string()];
/// assert_eq!(
///     format_expected_message(&multiple),
///     Some("Expected one of the following tokens: ';', '}'.".to_string())
/// );
/// ```
pub(crate) fn format_expected_message(expected: &[String]) -> Option<String> {
    match expected.len() {
        0 => Some("Unexpected input. Please verify the syntax near this token.".to_string()),
        1 => Some(format!("Expected token: `{}`.", expected[0])),
        _ => Some(format!(
            "Expected one of the following tokens: {}.",
            join_expected_tokens(expected)
        )),
    }
}

/// Helper function that joins a slice of expected tokens into a formatted string list.
///
/// Each token is wrapped in single quotes and separated by commas.
///
/// # Arguments
///
/// * `expected` - A slice of token strings.
///
/// # Returns
///
/// A single string listing all tokens, e.g. `'token1', 'token2', 'token3'`.
pub(crate) fn join_expected_tokens(expected: &[String]) -> String {
    expected
        .iter()
        .map(|t| format!("'{}'", t))
        .collect::<Vec<_>>()
        .join(", ")
}
