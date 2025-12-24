use std::fs;
use crate::aiplan4rust::cli::error::CliError;
use std::path::{Path, PathBuf};
use crate::aiplan4rust::io::{Output, IRContent, IRKind, Extension};
use crate::aiplan4rust::serialization::SerdeFormat;
use crate::aiplan4rust::serialization::SerdeSerializable;
use colored::Colorize;
use crate::aiplan4rust::io::error::IOError;
use crate::aiplan4rust::semantic::SemanticContext;
use crate::aiplan4rust::syntax::ast::AstKind;





/// Ensure that the parent directory of the given path exists.
/// If it does not exist, attempt to create it. On failure, prints an error and exits.
///
/// # Arguments
///
/// * `path` - The path whose parent directory should be ensured.
pub fn ensure_parent_dir_exists<P: AsRef<Path>>(path: P) {
    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        if !parent.exists() {
            if let Err(e) = fs::create_dir_all(parent) {
                eprintln!("Error creating output directory {}: {}", parent.display(), e);
                std::process::exit(1);
            }
        }
    }
}

/// Ensures that the given directory exists.
/// If it does not exist, attempts to create it. Exits the process on error.
///
/// # Arguments
///
/// * `dir` - The directory path to ensure existence for.
pub fn ensure_dir_exists<P: AsRef<Path>>(dir: P) {
    let dir = dir.as_ref();
    if !dir.exists() {
        if let Err(e) = fs::create_dir_all(dir) {
            eprintln!("Error creating directory {}: {}", dir.display(), e);
            std::process::exit(1);
        }
    }
}
