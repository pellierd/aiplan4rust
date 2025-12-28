use std::path::PathBuf;
use thiserror::Error;
use crate::aiplan4rust::AiplanError;
use crate::aiplan4rust::diagnostic::DiagnosticError;
use crate::aiplan4rust::artefact::error::ArtefactError;
use crate::aiplan4rust::serialization::SerializationError;


#[derive(Error, Debug)]
pub enum CliError {
    #[error(transparent)]
    Clap(#[from] clap::Error),

    #[error(transparent)]
    IO(#[from] ArtefactError),

    #[error(transparent)]
    Serialization(#[from] SerializationError),

    #[error(transparent)]
    Aiplan(#[from] AiplanError),

    #[error(transparent)]
    Diagnostic(#[from] DiagnosticError),

    /// Invalid command-line argument
    #[error("Invalid argument: {0}")]
    InvalidArgument(String),






    /// Failed to build an output path.
    #[error("Failed to build output path in directory '{dir}': {reason}")]
    InvalidOutputPath {
        dir: PathBuf,
        reason: String,
    },

    #[error("Domain and problem files must be both parsed or both raw.")]
    InconsistentFiles,

    /// Domain and problem files count mismatch
    #[error("Exactly two input files are required (domain and problem).")]
    InvalidFileCount,

    /// No input files were provided
    #[error("No input files provided.")]
    MissingInputFiles,
}


impl CliError {

    /// Creates an error indicating that a CLI argument is invalid.
    ///
    /// # Arguments
    /// * `msg` - A human-readable explanation of what is invalid.
    pub fn invalid_argument(msg: impl Into<String>) -> Self {
        CliError::InvalidArgument(msg.into())
    }


    /// Creates an error indicating that an output path could not be constructed.
    ///
    /// # Arguments
    /// * `dir` - The output directory in which the file was supposed to be created.
    /// * `reason` - A human-readable explanation of the failure.
    pub fn invalid_output_path(
        dir: impl Into<PathBuf>,
        reason: impl Into<String>,
    ) -> Self {
        CliError::InvalidOutputPath {
            dir: dir.into(),
            reason: reason.into(),
        }
    }

    /// Crée une nouvelle erreur `InconsistentFiles`.
    pub fn inconsistent_files() -> Self {
        CliError::InconsistentFiles
    }
    pub fn invalid_file_count() -> Self {
        CliError::InvalidFileCount
    }

    pub fn missing_input_files() -> Self {
        CliError::MissingInputFiles
    }
}
