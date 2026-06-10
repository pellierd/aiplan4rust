//! Build script for the `aiplan4rust` compiler frontend.
//!
//! This script automatically processes the LALRPOP grammar file (`grammar.lalrpop`)
//! during the Cargo build pipeline and generates the corresponding parser module
//! along with a detailed conflict report.

use lalrpop::Configuration;
use std::path::PathBuf;

/// Execution entry point for Cargo's build automation.
///
/// This function locates the syntax module directory dynamically, configures the
/// LALRPOP generator behavior, generates a `.report` file for grammar analysis,
/// and outputs the finalized Rust parser source file.
///
/// # Panics
///
/// * Panics if the `CARGO_MANIFEST_DIR` environment variable is missing (meaning
///   the script was not executed via Cargo).
/// * Panics if the syntax directory cannot be created on the host filesystem.
/// * Panics if LALRPOP encounters a fatal parsing or generation error.
fn main() {
    // Dynamically retrieve the absolute path to the workspace crate root.
    // This prevents relative path resolution issues depending on where Cargo is invoked.
    let root = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());

    // Traverse down to the newly refactored compiler syntax submodule.
    let syntax_dir = root
        .join("src")
        .join("aiplan4rust")
        .join("compiler")
        .join("syntax");

    // Ensure the destination directory exists before running LALRPOP code generation.
    std::fs::create_dir_all(&syntax_dir).expect("Failed to create output directory");

    // Initialize and fine-tune LALRPOP compiler options.
    let mut config = Configuration::new();

    // Direct LALRPOP to look for `.lalrpop` source files inside the syntax folder.
    config.set_in_dir(&syntax_dir);

    // Force LALRPOP to output the generated `grammar.rs` directly into the
    // source tree under the syntax folder (in-tree generation).
    config.set_out_dir(&syntax_dir);

    // Enable colorized terminal feedback for syntax or conflict errors.
    config.always_use_colors();

    // Generate a detailed `grammar.report` file describing states, actions,
    // and shift/reduce conflicts for advanced debugging.
    config.emit_report(true);

    // Instruct Cargo to re-run this build script only if the `.lalrpop` file changes,
    // avoiding unnecessary full recompilations.
    config.emit_rerun_directives(true);

    // Enable maximum verbosity and debugging output in Cargo's build logs.
    config.log_verbose();
    config.log_debug();

    // Execute the parser generator pipeline.
    config.process().unwrap();
}
