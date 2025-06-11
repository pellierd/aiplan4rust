use crate::aiplan4rust::diagnostic::DiagnosticManager;
use crate::aiplan4rust::diagnostic::Severity;

use std::io::{self, Write};
use colored::*;

pub struct Renderer<'a> {
    diagnostic_manager: &'a DiagnosticManager,
    output: Box<dyn Write>,
}

impl<'a> Renderer<'a> {

    pub fn new(diagnostic_manager: &'a DiagnosticManager) -> Self {
        Renderer {
            diagnostic_manager,
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
        Renderer::write_to(self.diagnostic_manager, writer, true).expect("Failed to write diagnostics to output");
    }

    pub fn write_to<W: Write>(
        diagnostic_manager: &DiagnosticManager,
        writer: &mut W,
        color: bool,
    ) -> io::Result<()> {
        for diagnostic in diagnostic_manager.diagnostics() {
            let mut output = String::new();

            let filename = diagnostic.filename();
            let span = diagnostic.span();
            let kind = diagnostic.kind();

            // Format severity string conditionnellement coloré
            let severity_str = match kind.severity() {
                Severity::Error => {
                    if color {
                        format!("error[{}]", kind.code()).red().bold().to_string()
                    } else {
                        format!("error[{}]", kind.code())
                    }
                }
                Severity::Warning => {
                    if color {
                        format!("warning[{}]", kind.code()).yellow().to_string()
                    } else {
                        format!("warning[{}]", kind.code())
                    }
                }
                _ => format!("{}", kind.code()),
            };

            output.push_str(&format!("{}: {}\n", severity_str, kind.message()));

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

            if let Some(source) = diagnostic_manager.get_source(filename) {
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

            if let Some(suggestion) = kind.suggestion() {
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

// Constantes, fonctions auxiliaires inchangées

const TAB_WIDTH: usize = 4;
const RIGHT_ARROW: &str = "-->";
const VERTICAL_BAR: &str = "|";

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
