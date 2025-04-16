use crate::aiplan4rust::error::{Diagnostic, DiagnosticManager, DiagnosticSeverity};
use std::fs::File;
use std::io::{self, Write};
use colored::*;  // Pour coloration terminale

pub struct DiagnosticRenderer<'a> {
    diagnostic_manager: &'a DiagnosticManager<'a>,
    output: Box<dyn Write>,
}

impl<'a> DiagnosticRenderer<'a> {
    pub fn new(diagnostic_manager: &'a DiagnosticManager) -> Self {
        DiagnosticRenderer {
            diagnostic_manager,
            output: Box::new(io::stdout()),
        }
    }

    pub fn set_output(&mut self, output: Box<dyn Write>) {
        self.output = output;
    }

    pub fn display_with_suggestions(&mut self) {
        for diagnostic in self.diagnostic_manager.diagnostics() {
            let mut output = String::new();

            let filename = diagnostic.filename();
            let span = diagnostic.span();
            let kind = diagnostic.kind();

            // Couleur du message selon la sévérité
            let severity_str = match kind.severity() {
                DiagnosticSeverity::Error => format!("error[{}]", kind.code()).red().bold(),
                DiagnosticSeverity::Warning => format!("warning[{}]", kind.code()).yellow(),
                _ => format!("{}", kind.code()).normal(),
            };

            output.push_str(&format!("{}: {}\n", severity_str, kind.message()));

            output.push_str(&format!(
                "{} {}:{}:{}\n",
                RIGHT_ARROW.bright_blue(),
                filename,
                span.begin_line().to_string(),
                span.begin_column().to_string()
            ));

            let line_num_str = span.begin_line().to_string();
            let gutter_width = line_num_str.len();

            if let Some(source) = self.diagnostic_manager.get_source(filename) {
                if let Some(line) = source.lines().nth(span.begin_line() - 1) {
                    // Ligne vide avec barre verticale alignée
                    output.push_str(&format!(
                        "{:>width$} {}\n",
                        "", VERTICAL_BAR.bright_blue(),
                        width = gutter_width
                    ));
                    // Ligne de code avec numéro de ligne en violet
                    output.push_str(&format!(
                        "{} {} {}\n",
                        format!("{:>width$}", span.begin_line(), width = gutter_width).bright_blue(),
                        VERTICAL_BAR.bright_blue(),
                        expand_tabs(line, TAB_WIDTH)
                    ));

                    // Soulignement
                    let underline_start = compute_visual_offset(line, span.begin_column());
                    let underline_len = (span.end_column().saturating_sub(span.begin_column())).max(1);

                    let underline = match kind.severity() {
                        DiagnosticSeverity::Error => "^".repeat(underline_len).red(),
                        DiagnosticSeverity::Warning => "^".repeat(underline_len).yellow(),
                        _ => "^".repeat(underline_len).normal(),
                    };

                    output.push_str(&format!(
                        "{:>width$} {} {}{}\n",
                        "",
                        VERTICAL_BAR.bright_blue(),
                        " ".repeat(underline_start),
                        underline,
                        width = gutter_width
                    ));

                    // Ligne verticale vide
                    output.push_str(&format!("{:>width$} {}\n", "", VERTICAL_BAR.bright_blue(), width = gutter_width));
                }
            }

            // Suggestion (note)
            if let Some(suggestion) = kind.suggestion() {
                output.push_str(&format!(
                    "{} {}\n",
                    "  = note:".bright_cyan().bold(),
                    suggestion
                ));
            }

            write!(self.output, "{}", output).unwrap();
        }
    }
}

const TAB_WIDTH: usize = 4;
const RIGHT_ARROW: &str = "-->";
const VERTICAL_BAR: &str = "|";

fn compute_visual_offset(line: &str, column: usize) -> usize {
    let mut offset = 0;
    let mut chars = line.chars();
    for i in 0..(column - 1) {
        if let Some(c) = chars.next() {
            offset += match c {
                '\t' => TAB_WIDTH - (offset % TAB_WIDTH),
                _ => 1,
            };
        }
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
