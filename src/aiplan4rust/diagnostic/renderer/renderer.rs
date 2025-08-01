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

use crate::aiplan4rust::diagnostic::DiagnosticManager;
use crate::aiplan4rust::diagnostic::Severity;
use crate::aiplan4rust::interner::StringInterner;
use crate::aiplan4rust::diagnostic::renderer::{message, suggestion};

use std::io::{self, Write};
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

    pub fn new(diagnostic_manager: &'a DiagnosticManager, interner: &'a StringInterner) -> Self {
        Renderer {
            diagnostic_manager,
            interner,
            output: Box::new(io::stdout()),
        }
    }

    pub fn diagnostic_manager(&self) -> &DiagnosticManager {
        self.diagnostic_manager
    }

    pub fn set_output(&mut self, output: Box<dyn Write>) {
        self.output = output;
    }

    pub fn display(&mut self) {
        // On prend une référence mutable au writer en dehors de l'appel
        let writer = &mut self.output;
        Renderer::write_to(self.diagnostic_manager, self.interner, writer, true).expect("Failed to write diagnostics to output");
    }

    pub fn write_to<W: Write>(
        diagnostic_manager: &DiagnosticManager,
        interner: &StringInterner,
        writer: &mut W,
        color: bool,
    ) -> io::Result<()> {
        for diagnostic in diagnostic_manager.diagnostics() {
            let mut output = String::new();

            let filename = interner.try_resolve_literal(diagnostic.source()).unwrap();
            let span = diagnostic.span();
            let kind = diagnostic.kind();

            // Récupère le code complet depuis Diagnostic (ex: "E001")
            let code = diagnostic.code();

            // Format severity string conditionnellement coloré en fonction de la première lettre du code
            let severity_str = match code.chars().next() {
                Some('E') => {
                    if color {
                        format!("error[{}]", code).red().bold().to_string()
                    } else {
                        format!("error[{}]", code)
                    }
                }
                Some('W') => {
                    if color {
                        format!("warning[{}]", code).yellow().to_string()
                    } else {
                        format!("warning[{}]", code)
                    }
                }
                _ => code.clone(),
            };

            output.push_str(&format!("{}: {}\n", severity_str, message::format_message(kind, interner)));

            // Flèche droite --> en bleu clair ou sans couleur
            let arrow = if color {
                RIGHT_ARROW.bright_blue().to_string()
            } else {
                RIGHT_ARROW.to_string()
            };
            output.push_str(&format!(
                "{} {}:{}:{}\n",
                arrow,
                filename,
                span.begin_line(),
                span.begin_column()
            ));

            // Largeur de la colonne du numéro de ligne
            let line_num_str = span.begin_line().to_string();
            let gutter_width = line_num_str.len();

            if let Some(source) = diagnostic_manager.get_source(diagnostic.source()) {
                if let Some(line) = source.lines().nth(span.begin_line() - 1) {
                    // Bar vertical en bleu clair ou sans couleur
                    let vertical_bar = if color {
                        VERTICAL_BAR.bright_blue().to_string()
                    } else {
                        VERTICAL_BAR.to_string()
                    };

                    output.push_str(&format!(
                        "{:>width$} {}\n",
                        "",
                        vertical_bar,
                        width = gutter_width
                    ));

                    output.push_str(&format!(
                        "{} {} {}\n",
                        format!("{:>width$}", span.begin_line(), width = gutter_width),
                        vertical_bar,
                        expand_tabs(line, TAB_WIDTH)
                    ));

                    let underline_start = compute_visual_offset(line, span.begin_column());
                    let underline_len = (span.end_column().saturating_sub(span.begin_column())).max(1);

                    // Caret underline en couleur ou non selon la sévérité
                    let underline = match kind.severity() {
                        Severity::Error => {
                            if color {
                                "^".repeat(underline_len).red().to_string()
                            } else {
                                "^".repeat(underline_len)
                            }
                        }
                        Severity::Warning => {
                            if color {
                                "^".repeat(underline_len).yellow().to_string()
                            } else {
                                "^".repeat(underline_len)
                            }
                        }
                        _ => "^".repeat(underline_len),
                    };

                    output.push_str(&format!(
                        "{:>width$} {} {}{}\n",
                        "",
                        vertical_bar,
                        " ".repeat(underline_start),
                        underline,
                        width = gutter_width
                    ));

                    output.push_str(&format!(
                        "{:>width$} {}\n",
                        "",
                        vertical_bar,
                        width = gutter_width
                    ));
                }
            }

            if let Some(suggestion) = suggestion::format_suggestion(kind, interner)   {
                if color {
                    output.push_str(&format!(
                        "{} {}\n",
                        "= help:".bright_cyan().bold(),
                        suggestion
                    ));
                } else {
                    output.push_str(&format!("= help: {}\n", suggestion));
                }
            }

            write!(writer, "{}", output)?;
        }

        Ok(())
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
fn compute_visual_offset(line: &str, column: usize) -> usize {
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
/// let expanded = expand_tabs(line, 4);
/// // `expanded` will have spaces replacing the tabs
/// ```
fn expand_tabs(line: &str, tab_width: usize) -> String {
    let mut expanded = String::new();
    let mut col = 0;
    for c in line.chars() {
        match c {
            '\t' => {
                let spaces = tab_width - (col % tab_width);
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
