//! Utilities for formatting internal language elements into user-friendly string representations.
//!
//! This module provides functions to convert identifiers, symbols, types, declarations, and
//! requirements into human-readable strings, optionally using a `StringInterner` to resolve
//! interned identifiers. It also includes utilities for formatting source code spans and expected
//! token messages, intended for improving diagnostics and error reporting.
//!
//! # Features
//! - Identifier and symbol string conversion (`ident_to_string`, `symbol_to_string`)
//! - Type pretty-printing with optional interner support (`type_to_string`)
//! - Span formatting into readable line ranges (`span_to_string`)
//! - Comma-separated formatting of identifier, declaration, and requirement lists
//! - Friendly formatting for expected parser tokens (`format_expected_message`)

use crate::aiplan4rust::semantic::symbol::{Declaration, Symbol, SymbolKind};
use crate::aiplan4rust::support::interner::SymbolInterner;
use crate::aiplan4rust::support::lang::{Requirement, SymbolId, Type};
use crate::aiplan4rust::syntax::{Span, SyntaxInternerDisplay};
use crate::Severity;
use colored::Colorize;

/// Number of spaces to which a tab character (`\t`) expands.
///
/// Used for calculating visual offsets and expanding tabs in source code lines.
const TAB_WIDTH: usize = 4;

/// String used to indicate the current position or focus in diagnostic output.
///
/// Typically displayed as an arrow pointing to a specific column.
const RIGHT_ARROW: &str = "-->";

/// String used as a vertical bar in diagnostic output formatting.
///
/// Often used to visually separate line numbers or highlight spans.
const VERTICAL_BAR: &str = "|";

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
pub(crate) fn ident_to_string(ident: SymbolId, interner: Option<&SymbolInterner>) -> String {
    if let Some(interner) = interner {
        interner
            .resolve_symbol(ident)
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
pub(crate) fn symbol_to_string(symbol: &Symbol, interner: Option<&SymbolInterner>) -> String {
    ident_to_string(symbol.id(), interner)
}

/// Converts a `Type` to a `String`, optionally resolving identifiers via a `StringInterner`.
///
/// # Parameters
/// - `types`: The `Type` to convert to a string.
/// - `interner`: An optional reference to a `StringInterner` used to resolve identifiers.
///
/// # Returns
/// A `String` representation of the `Type`. If `interner` is provided, the identifiers
/// inside the `Type` are resolved using it; otherwise, the debug string representation is used.
pub(crate) fn type_to_string(ty: &Type<SymbolId>, interner: Option<&SymbolInterner>) -> String {
    if let Some(interner) = interner {
        ty.to_syntax_string_with_interner(interner)
    } else {
        ty.to_string()
    }
}

/// Formats a list of identifiers into a PDDL-style type string.
///
/// If the list contains multiple identifiers, they are wrapped in an `(either ...)` block.
/// If only one identifier is present, it is returned as a plain string.
///
/// # Parameters
/// - `idents`: Slice of `SymbolId` representing the types.
/// - `interner`: Optional reference to a `SymbolInterner` to resolve names.
///
/// # Returns
/// A formatted string: `"type_a"` or `"(either type_a type_b)"`.
pub(crate) fn idents_to_string_list(
    idents: &[SymbolId],
    interner: Option<&SymbolInterner>,
) -> String {
    match idents.len() {
        0 => "unknown".to_string(),
        1 => ident_to_string(idents[0], interner),
        _ => {
            let list = idents
                .iter()
                .map(|&id| ident_to_string(id, interner))
                .collect::<Vec<_>>()
                .join(" ");
            format!("(either {})", list)
        }
    }
}

/// Formats a list of symbols into a PDDL-style type string.
///
/// Helper wrapper around [`idents_to_string_list`] for slices of [`Symbol`].
#[allow(dead_code)]
pub(crate) fn format_symbol_list(symbols: &[Symbol], interner: Option<&SymbolInterner>) -> String {
    let idents: Vec<SymbolId> = symbols.iter().map(|s| s.id()).collect();
    idents_to_string_list(&idents, interner)
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
pub(crate) fn format_ident_list(idents: &[SymbolId], interner: Option<&SymbolInterner>) -> String {
    idents
        .iter()
        .map(|&ident| ident_to_string(ident, interner))
        .collect::<Vec<_>>()
        .join(", ")
}

/// Converts a `Span` into a user-friendly string representation of its line range.
///
/// This function is designed to simplification span information for end-user messages
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
    interner: Option<&SymbolInterner>,
) -> String {
    let idents: Vec<SymbolId> = declarations
        .iter()
        .map(|decl| decl.symbol_ident())
        .collect();
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
        .map(|r| format!("'{}'", r)) // Assumes Requirement implements Display
        .collect::<Vec<_>>()
        .join(", ")
}

/// Formats an error message based on the number of expected tokens.
///
/// - If no tokens are expected, returns a generic message.
/// - If one token is expected, it's shown directly.
/// - If multiple tokens are expected, they're joined and listed.
///
/// # Arguments
///
/// * `expected` - A slice of expected token strings.
///
/// # Returns
///
/// A formatted human-readable error message.
pub(crate) fn format_expected_message(expected: &[String]) -> Option<String> {
    match expected.len() {
        0 => Some("Unexpected input. Please verify the syntax near this token.".to_string()),
        1 => Some(format!(
            "Expected token: `{}`.",
            expected[0].trim_matches('"')
        )),
        _ => Some(format!(
            "Expected one of the following tokens: {}.",
            join_expected_tokens(expected)
        )),
    }
}

/// Maps a [`SymbolKind`] to its human-readable singular entity name.
///
/// This helper is used to normalize the terminology used in diagnostic messages
/// when referring to semantic symbols (e.g., "Object", "Predicate").
///
/// # Arguments
///
/// * `kind` - The symbol kind to convert.
///
/// # Returns
///
/// A static string slice containing the capitalized entity name.
#[allow(dead_code)]
pub(crate) fn symbol_kind_to_string(kind: SymbolKind) -> &'static str {
    match kind {
        SymbolKind::Constant => "Constant",
        SymbolKind::Variable => "Variable",
        SymbolKind::PrimitiveType => "Type",
        SymbolKind::Function => "Function",
        SymbolKind::Predicate => "Predicate",
        SymbolKind::Task => "Task",
        SymbolKind::Action => "Action",
        _ => "Entity",
    }
}

/// Joins and formats a list of expected token strings for display.
///
/// Each token is cleaned (double quotes removed), wrapped in single quotes,
/// and separated by commas.
///
/// # Example
/// Input: `["\"and\"", "\"not\""]`
/// Output: `'and', 'not'`
pub(crate) fn join_expected_tokens(expected: &[String]) -> String {
    expected
        .iter()
        .map(|t| format!("'{}'", t.trim_matches('"')))
        .collect::<Vec<_>>()
        .join(", ")
}

/// Returns a formatted error label string with the given error `code`.
///
/// If `color` is `true`, the label is colored red and bolded using ANSI escape codes.
/// Otherwise, a plain string is returned.
///
/// # Parameters
/// - `code`: The error code to include in the label (e.g., `"E123"`).
/// - `color`: Whether to apply color formatting.
///
/// # Returns
/// A formatted `String` representing the error label.
///
/// # Example
/// ```
/// let label = error_label("E001", true);
/// // label will be a red, bold "error[E001]" string.
/// ```
pub fn error_label(code: &str, color: bool) -> String {
    if color {
        format!("error[{}]", code).red().bold().to_string()
    } else {
        format!("error[{}]", code)
    }
}

/// Returns a formatted warning label string with the given warning `code`.
///
/// If `color` is `true`, the label is colored yellow using ANSI escape codes.
/// Otherwise, a plain string is returned.
///
/// # Parameters
/// - `code`: The warning code to include in the label (e.g., `"W001"`).
/// - `color`: Whether to apply color formatting.
///
/// # Returns
/// A formatted `String` representing the warning label.
///
/// # Example
/// ```
/// let label = warning_label("W001", false);
/// // label will be "warning[W001]" without color.
/// ```
pub fn warning_label(code: &str, color: bool) -> String {
    if color {
        format!("warning[{}]", code).yellow().to_string()
    } else {
        format!("warning[{}]", code)
    }
}

/// Returns a right arrow string symbol (`→`).
///
/// If `color` is `true`, the arrow is colored bright blue using ANSI escape codes.
/// Otherwise, a plain string is returned.
///
/// # Parameters
/// - `color`: Whether to apply color formatting.
///
/// # Returns
/// A formatted `String` containing the arrow symbol.
///
/// # Example
/// ```
/// let arrow = arrow(true);
/// // arrow will be a bright blue "→".
/// ```
pub fn arrow(color: bool) -> String {
    if color {
        RIGHT_ARROW.bright_blue().to_string()
    } else {
        RIGHT_ARROW.to_string()
    }
}

/// Returns a vertical bar string symbol (`|`).
///
/// If `color` is `true`, the bar is colored bright blue using ANSI escape codes.
/// Otherwise, a plain string is returned.
///
/// # Parameters
/// - `color`: Whether to apply color formatting.
///
/// # Returns
/// A formatted `String` containing the vertical bar.
///
/// # Example
/// ```
/// let bar = vertical_bar(false);
/// // bar will be "|".
/// ```
pub fn vertical_bar(color: bool) -> String {
    if color {
        VERTICAL_BAR.bright_blue().to_string()
    } else {
        VERTICAL_BAR.to_string()
    }
}

/// Returns an underline string consisting of `len` caret (`^`) characters,
/// colored according to the `severity` if `color` is enabled.
///
/// # Parameters
/// - `severity`: The severity level (e.g., Error, Warning) used to determine underline color.
/// - `len`: The length of the underline (number of carets).
/// - `color`: Whether to apply color formatting.
///
/// # Returns
/// A `String` containing the underline with optional color.
///
/// # Example
/// ```
/// let underline = underline(Severity::Error, 5, true);
/// // underline will be "^^^^^" colored red.
/// ```
pub fn underline(severity: Severity, len: usize, color: bool) -> String {
    let underline = "^".repeat(len);
    match severity {
        Severity::Error => {
            if color {
                underline.red().to_string()
            } else {
                underline
            }
        }
        Severity::Warning => {
            if color {
                underline.yellow().to_string()
            } else {
                underline
            }
        }
        _ => underline,
    }
}

/// Returns a help label string `= help:`
///
/// If `color` is `true`, the label is colored bright cyan and bolded using ANSI escape codes.
/// Otherwise, a plain string is returned.
///
/// # Parameters
/// - `color`: Whether to apply color formatting.
///
/// # Returns
/// A formatted `String` representing the help label.
///
/// # Example
/// ```
/// let help = help_label(true);
/// // help will be a bright cyan, bold "= help:".
/// ```
pub fn help_label(color: bool) -> String {
    if color {
        "= help:".bright_cyan().bold().to_string()
    } else {
        "= help:".to_string()
    }
}

/// Computes the visual offset of a given column in a line of text,
/// accounting for tab characters which have variable width.
///
/// Tabs are expanded to a fixed width (`TAB_WIDTH`) and the function
/// returns the visual column index corresponding to the input `column`.
///
/// # Parameters
///
/// - `line`: The input text line as a string slice.
/// - `column`: The 1-based column number in the line.
///
/// # Returns
///
/// The visual offset as a zero-based index, where tabs count as multiple spaces.
///
/// # Example
///
/// ```
/// let line = "\tfoo\tbar";
/// let offset = compute_visual_offset(line, 5);
/// // `offset` accounts for tab expansion before column 5
/// ```
pub fn compute_visual_offset(line: &str, column: usize) -> usize {
    const TAB_WIDTH: usize = 4; // ou récupère la constante TAB_WIDTH définie dans formatting
    let mut offset = 0;
    for c in line.chars().take(column.saturating_sub(1)) {
        offset += match c {
            '\t' => TAB_WIDTH - (offset % TAB_WIDTH),
            _ => 1,
        };
    }
    offset
}

/// Expands all tab characters in a given line into spaces,
/// based on the specified tab width.
///
/// Tabs are replaced by the number of spaces needed to reach the next tab stop.
///
/// # Parameters
///
/// - `line`: The input text line as a string slice.
/// - `tab_width`: The number of spaces per tab stop.
///
/// # Returns
///
/// A new `String` with tabs replaced by the appropriate number of spaces.
///
/// # Example
///
/// ```
/// let line = "\tfoo\tbar";
/// let expanded = expand_tabs(line);
/// // `expanded` will have spaces replacing the tabs
/// ```
pub fn expand_tabs(line: &str) -> String {
    let mut expanded = String::new();
    let mut col = 0;
    for c in line.chars() {
        match c {
            '\t' => {
                let spaces = TAB_WIDTH - (col % TAB_WIDTH);
                expanded.push_str(&" ".repeat(spaces));
                col += spaces;
            }
            _ => {
                expanded.push(c);
                col += 1;
            }
        }
    }
    expanded
}
