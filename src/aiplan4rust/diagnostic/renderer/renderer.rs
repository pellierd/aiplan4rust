//! Module responsible for rendering and displaying diagnostics to various outputs.
//!
//! This module provides functionality to format diagnostic messages, including
//! errors, warnings, and informational notes, with proper visual alignment,
//! coloring, and contextual source code snippets.
//!
//! It leverages the `DiagnosticManager` to retrieve diagnostics, the `StringInterner`
//! to resolve symbol names, and outputs formatted results through any implementor of
//! the `Write` trait (e.g., stdout, files, or buffers).
//!
//! Common formatting constants like tab width and visual markers (arrows, bars) are
//! defined here to ensure consistent display.
//!
//! # Components
//!
//! - `Renderer`: Core struct handling the formatting and output of diagnostics.
//! - Constants for tab expansion and visual decorations.
//!
//! # Usage
//!
//! Create a `Renderer` with references to a `DiagnosticManager` and `StringInterner`,
//! then invoke its methods to write formatted diagnostics to your desired output.

use crate::aiplan4rust::diagnostic::{Diagnostic, DiagnosticManager};
use crate::aiplan4rust::interner::StringInterner;
use crate::aiplan4rust::diagnostic::renderer::{formatting, message, suggestion};

use std::io::{self, Write};
use crate::Severity;

/// Renderer responsible for formatting and outputting diagnostics.
///
/// Holds references to the diagnostic manager and string interner to access
/// diagnostic data and symbol names. Writes formatted output to a generic
/// writer, allowing flexibility (e.g., stdout, file, buffer).
///
/// # Fields
///
/// - `diagnostic_manager`: Reference to the manager containing diagnostics.
/// - `interner`: Reference to the string interner for resolving symbol names.
/// - `output`: A boxed writer implementing `Write` where formatted diagnostics are sent.
pub struct Renderer<'a> {
    diagnostic_manager: &'a DiagnosticManager,
    interner: &'a StringInterner,
    output: Box<dyn Write>,
}

impl<'a> Renderer<'a> {
    /// Creates a new `Renderer` with references to the diagnostic manager and string interner.
    ///
    /// The default output is set to standard output (`stdout`).
    ///
    /// # Parameters
    ///
    /// - `diagnostic_manager`: Reference to the `DiagnosticManager` that manages diagnostics.
    /// - `interner`: Reference to the `StringInterner` used for string interning.
    ///
    /// # Returns
    ///
    /// A new instance of `Renderer`.
    pub fn new(diagnostic_manager: &'a DiagnosticManager, interner: &'a StringInterner) -> Self {
        Renderer {
            diagnostic_manager,
            interner,
            output: Box::new(io::stdout()),
        }
    }

    /// Returns a reference to the associated diagnostic manager.
    ///
    /// # Returns
    ///
    /// A reference to the `DiagnosticManager`.
    pub fn diagnostic_manager(&self) -> &DiagnosticManager {
        self.diagnostic_manager
    }

    /// Sets the output destination used by the renderer.
    ///
    /// This allows redirecting output to any writer implementing the `Write` trait.
    ///
    /// # Parameters
    ///
    /// - `output`: A boxed writer (`Box<dyn Write>`) to which the diagnostics will be written.
    pub fn set_output(&mut self, output: Box<dyn Write>) {
        self.output = output;
    }

    /// Writes the diagnostics to the configured output.
    ///
    /// This function attempts to write all diagnostics managed by the
    /// `DiagnosticManager` using the associated `StringInterner` for
    /// message formatting. The output is written to the current writer
    /// stored in the `Renderer`.
    ///
    /// # Errors
    ///
    /// Returns an `std::io::Error` if writing to the output fails.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut renderer = Renderer::new(&diagnostic_manager, &interner);
    /// renderer.display()?;
    /// ```
    pub fn display(&mut self) -> std::io::Result<()> {
        let writer = &mut self.output;
        Renderer::write_to(self.diagnostic_manager, self.interner, writer, true)
    }

    /// Writes formatted diagnostics to the provided writer, including error/warning messages,
    /// source location information, annotated source code snippets, and optional suggestions.
    ///
    /// This function iterates through all diagnostics in the given `DiagnosticManager`, formats them
    /// into a user-friendly display (similar to compiler messages), and writes the result to the given
    /// `writer`. It supports optional colored output using the `color` flag.
    ///
    /// # Type Parameters
    ///
    /// * `W` - A type implementing the `Write` trait where the formatted diagnostics will be written.
    ///
    /// # Parameters
    ///
    /// * `diagnostic_manager` - The `DiagnosticManager` containing all the diagnostics to display.
    /// * `interner` - The `StringInterner` used to resolve interned literals (like source file names).
    /// * `writer` - A mutable writer (e.g., `stdout`, `stderr`, or a file) to write the output to.
    /// * `color` - If `true`, enables colored output for severity labels, arrows, and underlines.
    ///
    /// # Returns
    ///
    /// * `Ok(())` on success.
    /// * `Err(io::Error)` if an I/O error occurs while writing to the writer.
    ///
    /// # Example
    ///
    /// ```rust
    /// use my_crate::diagnostics::write_to;
    ///
    /// let diagnostics = DiagnosticManager::new();
    /// let interner = StringInterner::default();
    ///
    /// write_to(&diagnostics, &interner, &mut std::io::stdout(), true).unwrap();
    /// ```
    ///
    /// # Output Format
    ///
    /// ```text
    /// error[E001]: Unexpected token
    /// → domain.pddl:3:15
    ///    │
    ///  3 │   (define (problem)
    ///    │               ^^^^^
    ///    │
    /// = help: expected `:domain` section here
    /// ```
    ///
    pub fn write_to<W: Write>(
        diagnostic_manager: &DiagnosticManager,
        interner: &StringInterner,
        writer: &mut W,
        color: bool,
    ) -> io::Result<()> {
        // Iterate over all diagnostics to format and print them
        for diagnostic in diagnostic_manager.diagnostics() {
            let mut output = String::new();

            // Add the header: severity + error/warning message with optional color
            output.push_str(&format_header_line(diagnostic, interner, color));

            // Add the source location (filename, line, column) with arrow
            output.push_str(&format_location(diagnostic, interner, color));

            // Add the annotated snippet from the source file, if available
            if let Some(snippet) = format_source_snippet(diagnostic_manager, diagnostic, color) {
                output.push_str(&snippet);
            }

            // Add a suggestion, if one is available for the diagnostic
            if let Some(suggestion) = format_suggestion(diagnostic, interner, color) {
                output.push_str(&suggestion);
            }

            // Finally, write the complete formatted diagnostic to the output writer
            write!(writer, "{}\n", output)?;
        }

        Ok(())
    }
}

/// Formats the header line of a diagnostic message, including severity label and diagnostic message.
///
/// The severity label is derived from the diagnostic code's first character:
/// - Codes starting with 'E' produce an error label.
/// - Codes starting with 'W' produce a warning label.
/// - Other codes are shown as-is.
///
/// The diagnostic message is formatted using the provided interner.
///
/// # Parameters
/// - `diagnostic`: Reference to the `Diagnostic` containing code and kind information.
/// - `interner`: Reference to the `StringInterner` used to format the diagnostic message.
/// - `color`: Whether to apply ANSI color codes to severity labels.
///
/// # Returns
/// A formatted `String` in the form:
/// ```text
/// error[E1023]: detailed message
/// ```
/// or
/// ```text
/// warning[W4056]: detailed message
/// ```
/// depending on severity.
///
/// # Example
/// ```
/// let header = format_header_line(&diagnostic, &interner, true);
/// println!("{}", header);
/// ```
fn format_header_line(
    diagnostic: &Diagnostic,
    interner: &StringInterner,
    color: bool,
) -> String {
    let kind = diagnostic.kind();
    let code = diagnostic.code();

    let severity_str = match kind.severity() {
        Severity::Error => formatting::error_label(&code, color),
        Severity::Warning => formatting::warning_label(&code, color),
        _ => code.clone(),
    };

    format!("{}: {}\n", severity_str, message::format_message(kind, interner))
}

/// Formats the location of a diagnostic in the form of a file path and line/column numbers,
/// optionally prefixed by a colored arrow symbol.
///
/// The filename is resolved using the provided string interner. If the filename cannot be
/// resolved, it defaults to `"<unknown>"`.
///
/// # Parameters
/// - `diagnostic`: Reference to the `Diagnostic` containing the source and span information.
/// - `interner`: Reference to the `StringInterner` used to resolve the source file path.
/// - `color`: Whether to apply ANSI color codes to the arrow symbol.
///
/// # Returns
/// A `String` representing the location formatted as:
/// ```text
/// → filename:line:column
/// ```
/// with the arrow optionally colored.
///
/// # Example
/// ```
/// let location = format_location(&diagnostic, &interner, true);
/// println!("{}", location);
/// ```
fn format_location(
    diagnostic: &Diagnostic,
    interner: &StringInterner,
    color: bool,
) -> String {
    let filename = interner
        .try_resolve_literal(diagnostic.source())
        .unwrap_or("<unknown>");
    let span = diagnostic.span();

    let arrow = formatting::arrow(color);

    format!(
        "{} {}:{}:{}\n",
        arrow,
        filename,
        span.begin_line(),
        span.begin_column()
    )
}

/// Formats a source code snippet around the diagnostic's span, highlighting the relevant line and
/// underlining the specific span range.
///
/// This function retrieves the source content from the given `DiagnosticManager`
/// using the diagnostic's source ID. It extracts the line corresponding to the diagnostic's span,
/// expands tabs into spaces, and creates a visual snippet with a gutter showing line numbers.
/// The span range is underlined with color corresponding to the severity (error or warning).
///
/// # Parameters
/// - `manager`: Reference to the `DiagnosticManager` to access source content.
/// - `diagnostic`: Reference to the `Diagnostic` containing span and severity information.
/// - `color`: Whether to apply ANSI color codes to the snippet.
///
/// # Returns
/// An `Option<String>` containing the formatted snippet if the source and line are available,
/// otherwise `None`.
///
/// # Example
/// ```
/// if let Some(snippet) = format_source_snippet(&manager, &diagnostic, true) {
///     println!("{}", snippet);
/// }
/// ```
fn format_source_snippet(
    manager: &DiagnosticManager,
    diagnostic: &Diagnostic,
    color: bool,
) -> Option<String> {
    let span = diagnostic.span();
    let kind = diagnostic.kind();
    let source = manager.get_source_content(diagnostic.source())?;
    let line = source.lines().nth(span.begin_line() - 1)?;

    let gutter_width = span.begin_line().to_string().len();
    let vertical_bar = formatting::vertical_bar(color);

    let mut snippet = String::new();
    snippet.push_str(&format!("{:>width$} {}\n", "", vertical_bar, width = gutter_width));
    snippet.push_str(&format!(
        "{} {} {}\n",
        format!("{:>width$}", span.begin_line(), width = gutter_width),
        vertical_bar,
        formatting::expand_tabs(line)
    ));

    let underline_start = formatting::compute_visual_offset(line, span.begin_column());
    let underline_len = (span.end_column().saturating_sub(span.begin_column())).max(1);

    let colored_underline = formatting::underline(kind.severity(), underline_len, color);

    snippet.push_str(&format!(
        "{:>width$} {} {}{}\n",
        "",
        vertical_bar,
        " ".repeat(underline_start),
        colored_underline,
        width = gutter_width
    ));

    snippet.push_str(&format!("{:>width$} {}\n", "", vertical_bar, width = gutter_width));
    Some(snippet)
}

/// Formats a suggestion message for a given diagnostic.
///
/// The suggestion is obtained from the `suggestion` module based on the diagnostic kind.
/// If `color` is `true`, the help label is colored using ANSI escape codes.
///
/// # Parameters
/// - `diagnostic`: A reference to the `Diagnostic` containing error/warning information.
/// - `interner`: A reference to a `StringInterner` used to resolve string identifiers.
/// - `color`: Whether to apply color formatting to the help label.
///
/// # Returns
/// An `Option<String>` containing the formatted suggestion message if a suggestion exists,
/// otherwise `None`.
///
/// # Example
/// ```
/// if let Some(suggestion) = format_suggestion(&diagnostic, &interner, true) {
///     println!("{}", suggestion);
/// }
/// ```
fn format_suggestion(
    diagnostic: &Diagnostic,
    interner: &StringInterner,
    color: bool,
) -> Option<String> {
    suggestion::format_suggestion(diagnostic.kind(), interner).map(|suggestion| {
        if color {
            format!("{} {}\n", formatting::help_label(true), suggestion)
        } else {
            format!("= help: {}\n", suggestion)
        }
    })
}
