use std::fs;
use std::path::{Path, PathBuf};
use std::fmt;
use crate::aiplan4rust::artefact::error::ArtefactError;
use crate::aiplan4rust::artefact::Extension;
use crate::aiplan4rust::artefact::ir::header::{Header, HEADER_PAYLOAD_SEPARATOR};
use crate::aiplan4rust::artefact::ir::content::IRContent;
use crate::aiplan4rust::artefact::raw::content::RawContent;

#[derive(Debug)]
pub enum Output {
    Raw {
        path: PathBuf,
        content: RawContent,
    },
    IR {
        path: PathBuf,
        content: IRContent,
    },
}

impl Output {
    pub fn new_raw(path: PathBuf, content: RawContent) -> Self {
        Output::Raw { path, content }
    }

    pub fn new_ir(path: PathBuf, content: IRContent ) -> Self {
        Output::IR { path, content }
    }

    pub fn path(&self) -> &Path {
        match self {
            Output::Raw { path, .. } => path,
            Output::IR { path, .. } => path,
        }
    }

    // --- RawContent ---
    pub fn raw_content(&self) -> Option<&RawContent> {
        match self {
            Output::Raw { content, .. } => Some(content),
            _ => None,
        }
    }

    pub fn try_raw_content(&self) -> Result<&RawContent, ArtefactError> {
        match self {
            Output::Raw { content, .. } => Ok(content),
            _ => Err(ArtefactError::missing_raw_content()),
        }
    }

    // --- IRContent ---
    pub fn ir_content(&self) -> Option<&IRContent> {
        match self {
            Output::IR { content, .. } => Some(content),
            _ => None,
        }
    }

    pub fn try_ir_content(&self) -> Result<&IRContent, ArtefactError> {
        match self {
            Output::IR { content, .. } => Ok(content),
            _ => Err(ArtefactError::missing_ir_content()),
        }
    }


    pub fn write(&self) -> Result<(), ArtefactError> {
        match self {
            Output::Raw { content, .. } => fs::write(self.path(), content.inner())?,
            Output::IR { content, .. } => {
                let bytes = Self::serialize_with_header(content)?;
                fs::write(self.path(), bytes)?
            }
        }
        Ok(())
    }

    fn serialize_with_header(
        content: &IRContent,
    ) -> Result<Vec<u8>, ArtefactError> {
        // Crée le header
        let format = content.format();
        let header = Header::new(format, 1, content.kind());

        // Sérialise le header en JSON
        let header_str = serde_json::to_string_pretty(&header)?;

        // Sérialise le payload en bytes selon le format
        let payload = content.serialize_to_bytes()?; // Retourne Vec<u8>

        // Concatène header + séparateur + payload en bytes
        let mut result = Vec::with_capacity(header_str.len() + HEADER_PAYLOAD_SEPARATOR.len() + payload.len());
        result.extend_from_slice(header_str.as_bytes());
        result.extend_from_slice(HEADER_PAYLOAD_SEPARATOR.as_bytes());
        result.extend_from_slice(&payload);

        Ok(result)
    }

    /// Generates a default output path from a domain file and optionally a problem file,
    /// using the specified output `Extension`, and optionally placing the result in an
    /// output directory.
    ///
    /// # Behavior
    ///
    /// The generated filename follows these rules:
    ///
    /// - If `problem_file` is `Some`:
    ///   `{domain_stem}-{problem_stem}.{extension}`
    /// - If `problem_file` is `None`:
    ///   `{domain_stem}.{extension}`
    ///
    /// If `out_dir` is provided, the generated filename is joined to that directory.
    /// Otherwise, the path is relative to the current working directory.
    ///
    /// # Arguments
    ///
    /// * `domain_path` - Path to the domain file (e.g. `domain.pddl`).
    /// * `problem_path` - Optional path to the problem file (e.g. `problem.pddl`).
    /// * `extension` - The desired output file extension.
    /// * `out_dir` - Optional output directory.
    ///
    /// # Errors
    ///
    /// Returns `CliError::InvalidFileName` if:
    /// - `domain_file` has no valid file stem,
    /// - `problem_file` is provided but has no valid file stem,
    /// - or either filename is not valid UTF-8.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use std::path::{Path, PathBuf};
    /// use your_crate::{default_output_path, Extension};
    ///
    /// let domain = Path::new("domain.pddl");
    /// let problem = Path::new("problem.pddl");
    ///
    /// let output = default_output_path(
    ///     domain,
    ///     Some(problem),
    ///     Extension::Parsed,
    ///     None,
    /// ).unwrap();
    ///
    /// assert_eq!(output, PathBuf::from("domain-problem.prs"));
    /// ```
    ///
    /// ```rust
    /// use std::path::{Path, PathBuf};
    /// use your_crate::{default_output_path, Extension};
    ///
    /// let domain = Path::new("domain.pddl");
    /// let out_dir = Path::new("out_dir");
    ///
    /// let output = default_output_path(
    ///     domain,
    ///     None,
    ///     Extension::Parsed,
    ///     Some(out_dir),
    /// ).unwrap();
    ///
    /// assert_eq!(output, PathBuf::from("out_dir/domain.prs"));
    /// ```
    pub fn default_output_path(
        domain_path: &Path,
        problem_path: Option<&Path>,
        extension: Extension,
        out_dir: Option<&Path>,
    ) -> Result<PathBuf, ArtefactError> {
        // Extract domain stem using the helper
        let domain_stem = Self::file_stem_or_error(domain_path)?;

        // Build filename depending on optional problem file
        let filename = if let Some(problem) = problem_path {
            let problem_stem = Self::file_stem_or_error(problem)?;
            format!("{}-{}.{}", domain_stem, problem_stem, extension)
        } else {
            format!("{}.{}", domain_stem, extension)
        };

        // Join with output directory if provided
        Ok(match out_dir {
            Some(dir) => dir.join(filename),
            None => PathBuf::from(filename),
        })
    }

    /// Extracts the file stem (file name without extension) from a given `Path`.
    ///
    /// # Arguments
    ///
    /// * `path` - A reference to a `Path` from which to extract the file stem.
    ///
    /// # Returns
    ///
    /// * `Ok(&str)` - The file stem as a string slice if it exists and is valid UTF-8.
    /// * `Err(IOError)` - If the path has no file stem or contains invalid UTF-8.
    ///
    /// # Example
    ///
    /// ```rust
    /// use std::path::Path;
    /// # use crate::io::error::IOError; // Remplacer selon ton module
    ///
    /// let path = Path::new("domain.pddl");
    /// let stem = file_stem_or_error(path).unwrap();
    /// assert_eq!(stem, "domain");
    /// ```
    fn file_stem_or_error(path: &Path) -> Result<&str, ArtefactError> {
        path.file_stem()
            .and_then(|s| s.to_str())
            .ok_or_else(|| ArtefactError::invalid_file_name(path.display().to_string()))
    }

}

impl fmt::Display for Output {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Output::Raw { path, content } => {
                write!(
                    f,
                    "Raw Output {{ kind: {}, path: {}, language: {}, length: {} }}",
                    content.kind(),
                    path.display(),
                    content.language(),
                    content.inner().len()
                )
            }
            Output::IR { path, content } => {
                write!(
                    f,
                    "IR Output {{ kind: {}, path: {}, format: {} }}",
                    content.kind(),
                    path.display(),
                    content.format()
                )
            }
        }
    }
}
