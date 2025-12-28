use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use logos::Logos;
use crate::aiplan4rust::artefact::error::ArtefactError;
use crate::aiplan4rust::artefact::ir::header::{Header, HEADER_PAYLOAD_SEPARATOR};
use crate::aiplan4rust::artefact::ir::content::IRContent;
use crate::aiplan4rust::artefact::IRKind;
use crate::aiplan4rust::artefact::language::Language;
use crate::aiplan4rust::artefact::raw::content::RawContent;
use crate::aiplan4rust::artefact::raw::kind::RawKind;
use crate::aiplan4rust::lir::problem::LiftedProblem;
use crate::aiplan4rust::semantic::SemanticContext;
use crate::aiplan4rust::serialization::{SerdeSerializable, SerializationError};
use crate::aiplan4rust::syntax::lexer::Token;

/// Représentation d'une source pour le pipeline
#[derive(Debug, Clone)]
pub enum Source {
    Raw {
        path: PathBuf,
        content: RawContent,
    },
    IR {
        path: PathBuf,
        content: IRContent,
    },
    Text {
        path: PathBuf,
        content: String,
    },
    Binary {
        path: PathBuf,
        content: Vec<u8>,
    },
}


impl Source {

    // Constructeur pour le variant Raw
    pub fn new_raw(path: impl Into<PathBuf>, content: RawContent) -> Self {
        Source::Raw {
            path: path.into(),
            content,
        }
    }

    // Constructeur pour le variant IR
    pub fn new_ir(
        path: impl Into<PathBuf>,
        content: IRContent,
    ) -> Self {
        Source::IR {
            path: path.into(),
            content,
        }
    }

    // Constructeur pour TextUnknown
    pub fn new_text(path: impl Into<PathBuf>, content: impl Into<String>) -> Self {
        Source::Text {
            path: path.into(),
            content: content.into(),
        }
    }

    // Constructeur pour BinaryUnknown
    pub fn new_binary(path: impl Into<PathBuf>, content: Vec<u8>) -> Self {
        Source::Binary {
            path: path.into(),
            content,
        }
    }

    // -------------------------------------------------------------------------
    // Public reading API
    // -------------------------------------------------------------------------

    /// Lit un fichier et retourne le bon `Input` (Raw / IR / Unknown)
    pub fn read_from_file(path: impl AsRef<Path>) -> Result<Self, ArtefactError> {
        let path_buf = path.as_ref().to_path_buf();
        let bytes = Self::read_file(&path_buf)?;
        Self::read_from_bytes(path_buf, bytes)
    }

    // -------------------------------------------------------------------------
    // Internal helpers
    // -------------------------------------------------------------------------
    /// Reads an `Input` from a byte vector, attempting IR deserialization first, then falling back to raw or unknown content.
    ///
    /// This function performs the following steps:
    /// 1. Attempts to parse a JSON header and payload using [`Self::try_to_read_ir`].
    ///    - If a valid header is found, the payload is deserialized according to the IRKind and format.
    /// 2. If no header is found, attempts to interpret the bytes as UTF-8:
    ///    - If the text corresponds to a recognized raw kind, returns `Input::Raw`.
    ///    - Otherwise, returns `Input::TextUnknown`.
    /// 3. If UTF-8 conversion fails, returns `Input::BinaryUnknown`.
    ///
    /// # Arguments
    ///
    /// * `path` - Path of the source file, used for tracking and errors.
    /// * `bytes` - Raw bytes read from the source file.
    ///
    /// # Returns
    ///
    /// Returns a fully constructed [`Source`] variant based on the content and detected format.
    ///
    /// # Errors
    ///
    /// Propagates any deserialization errors from:
    /// - UTF-8 conversion (`std::str::Utf8Error`)
    /// - IR payload deserialization (`SerializationError`)
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use std::path::PathBuf;
    /// # use aiplan4rust::ir::{Input, IRKind, SemanticContext, LiftedProblem};
    /// let bytes: Vec<u8> = std::fs::read("example.ir").unwrap();
    /// let input = Input::read_from_bytes(PathBuf::from("example.ir"), bytes).unwrap();
    /// ```
    pub fn read_from_bytes(path: PathBuf, bytes: Vec<u8>) -> Result<Self, ArtefactError> {
        // Attempt to read IR header and payload
        match Self::try_to_read_ir(&bytes)? {
            Some((header, payload)) => {
                match header.ir_kind() {
                    IRKind::ParsedDomain => {
                        let sc = SemanticContext::deserialize_from_bytes(payload, header.format())?;
                        let content = IRContent::ParsedDomain(sc, header.format());
                        Ok(Source::new_ir(path, content))
                    }
                    IRKind::ParsedProblem => {
                        let sc = SemanticContext::deserialize_from_bytes(payload, header.format())?;
                        let content = IRContent::ParsedProblem(sc, header.format());
                        Ok(Source::new_ir(path, content))
                    }
                    IRKind::LiftedProblem => {
                        let pb = LiftedProblem::deserialize_from_bytes(payload, header.format())?;
                        let content = IRContent::LiftedProblem(pb, header.format());
                        Ok(Source::new_ir(path, content))
                    }
                }
           }
            None => {
                // No IR header found → fallback to UTF-8
                match String::from_utf8(bytes) {
                    Ok(content) => {
                        if let Some(kind) = infer_raw_kind(&content) {
                            let language = detect_raw_language(&content);
                            let content = RawContent::new(kind, language, content);
                            return Ok(Source::new_raw(path, content))
                        }

                        Ok(Source::Text { path, content })
                    }
                    Err(e) => {
                        // Binary unknown
                        Ok(Source::Binary { path, content: e.into_bytes() })
                    }
                }
            }
        }
    }

    /// Reads the entire content of a file into a `Vec<u8>`.
    ///
    /// This function attempts to open the file at the given path and read all its bytes
    /// into a vector. It returns an error if the file cannot be opened or if reading
    /// fails for any reason (e.g., I/O error, permissions issue).
    ///
    /// # Arguments
    ///
    /// * `path` - A reference to a `Path` representing the file to read.
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<u8>)` containing all bytes of the file if successful.
    /// * `Err(IOError)` if opening or reading the file fails.
    ///
    /// # Errors
    ///
    /// This function can return the following errors wrapped in `IOError`:
    /// - `IOError::open_failed(path, e)` if the file cannot be opened.
    /// - `IOError::read_failed(path, e)` if reading the file fails.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use std::path::Path;
    /// use aiplan4rust::io::error::IOError;
    /// use aiplan4rust::source::Input;
    ///
    /// let path = Path::new("example.pddl");
    /// match Input::read_file(path) {
    ///     Ok(bytes) => println!("Read {} bytes from the file.", bytes.len()),
    ///     Err(err) => eprintln!("Failed to read file: {:?}", err),
    /// }
    /// ```
    ///
    /// # Notes
    ///
    /// - This function reads the entire file into memory. For very large files, this may
    ///   cause high memory usage. Consider using buffered reading if needed.
    /// - Unlike `read_to_string`, this does not attempt to interpret the bytes as UTF-8.
    fn read_file(path: &Path) -> Result<Vec<u8>, ArtefactError> {
        let mut file = File::open(path)?;
        let mut content = Vec::new();
        file.read_to_end(&mut content)?;
        Ok(content)
    }

    /// Attempts to read an intermediate representation (IR) from a byte slice by parsing a header and payload.
    ///
    /// This function looks for a `HEADER_PAYLOAD_SEPARATOR` in the byte slice to split the header from the payload.
    /// The header is expected to be in JSON format and contains metadata including a magic number and format information.
    ///
    /// # Behavior
    /// - If the separator is **not found**, this function returns `Ok(None)`, indicating that no IR header was present.
    /// - If the separator is found, the header is parsed as UTF-8 and then deserialized as a `Header`.
    /// - If the magic number in the header is invalid, a `SerializationError::InvalidMagic` is returned.
    /// - On success, returns `Ok(Some((header, payload)))`, where `payload` is a slice of the original bytes after the separator.
    ///
    /// # Arguments
    ///
    /// * `bytes` - A byte slice containing the potentially serialized IR data (header + payload).
    ///
    /// # Returns
    ///
    /// - `Ok(Some((Header, &[u8])))` if a valid header is found and parsed successfully.
    /// - `Ok(None)` if no header separator is found in the input bytes.
    /// - `Err(SerializationError)` if there is a UTF-8 conversion error, JSON deserialization error, or the header's magic number is invalid.
    ///
    /// # Errors
    ///
    /// This function may return the following errors wrapped in `SerializationError`:
    /// - [`SerializationError::Utf8`] if the header bytes cannot be interpreted as UTF-8.
    /// - [`SerializationError::SerdeJson`] if the header JSON is malformed.
    /// - [`SerializationError::InvalidMagic`] if the header's magic number is incorrect.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use aiplan4rust::serialization::{SerializationError, try_to_read_ir};
    /// # use aiplan4rust::io::header::HEADER_PAYLOAD_SEPARATOR;
    /// # use aiplan4rust::io::header::Header;
    /// let mut bytes = b"{\"magic\":\"AIPL\",\"version\":1}".to_vec();
    /// bytes.extend_from_slice(HEADER_PAYLOAD_SEPARATOR.as_bytes());
    /// bytes.extend_from_slice(b"{\"some\":\"payload\"}");
    ///
    /// let result = try_to_read_ir(&bytes).unwrap();
    /// assert!(result.is_some());
    /// let (header, payload) = result.unwrap();
    /// assert_eq!(payload, b"{\"some\":\"payload\"}");
    /// ```
    fn try_to_read_ir(bytes: &[u8]) -> Result<Option<(Header, &[u8])>, SerializationError> {
        let sep_index = match bytes
            .windows(HEADER_PAYLOAD_SEPARATOR.len())
            .position(|window| window == HEADER_PAYLOAD_SEPARATOR.as_bytes())
        {
            Some(i) => i,
            None => return Ok(None),
        };

        let header_bytes = &bytes[..sep_index];
        let payload = &bytes[sep_index + HEADER_PAYLOAD_SEPARATOR.len()..];

        let header_str = match std::str::from_utf8(header_bytes) {
            Ok(s) => s,
            Err(_) => return Ok(None), // invalid UTF-8 → treat as not IR
        };

        let header: Header = match serde_json::from_str(header_str) {
            Ok(h) => h,
            Err(_) => return Ok(None), // invalid JSON → treat as not IR
        };

        if !header.validate_magic() {
            return Ok(None); // invalid magic → treat as not IR
        }

        Ok(Some((header, payload)))
    }

    // -------------------------------------------------------------------------
    // Accessors
    // -------------------------------------------------------------------------

    pub fn path(&self) -> &Path {
        match self {
            Source::Raw { path, .. } => path,
            Source::IR { path, .. } => path,
            Source::Text { path, .. } => path,
            Source::Binary { path, .. } => path,
        }
    }

    // --- RawContent ---
    pub fn raw_content(&self) -> Option<&RawContent> {
        match self {
            Source::Raw { content, .. } => Some(content),
            _ => None,
        }
    }

    pub fn try_raw_content(&self) -> Result<&RawContent, ArtefactError> {
        match self {
            Source::Raw { content, .. } => Ok(content),
            _ => Err(ArtefactError::missing_raw_content()),
        }
    }

    // --- IRContent ---
    pub fn ir_content(&self) -> Option<&IRContent> {
        match self {
            Source::IR { content, .. } => Some(content),
            _ => None,
        }
    }

    pub fn try_ir_content(&self) -> Result<&IRContent, ArtefactError> {
        match self {
            Source::IR { content, .. } => Ok(content),
            _ => Err(ArtefactError::missing_ir_content()),
        }
    }

    // --- Unknown binary content ---
    pub fn binary_content(&self) -> Option<&[u8]> {
        match self {
            Source::Binary { content, .. } => Some(content),
            _ => None,
        }
    }

    pub fn try_binary_content(&self) -> Result<&[u8], ArtefactError> {
        match self {
            Source::Binary { content, .. } => Ok(content),
            _ => Err(ArtefactError::missing_binary_content()),
        }
    }

    pub fn try_parsed_content(&self) -> Result<&SemanticContext, ArtefactError> {
        match self {
            Source::IR { content, .. } => match content {
                IRContent::ParsedDomain(domain, _) => Ok(domain),
                IRContent::ParsedProblem(problem, _) => Ok(problem),
                _ => Err(ArtefactError::missing_ir_content()),
            },
            _ => Err(ArtefactError::missing_ir_content()),
        }
    }

    /// Retourne le contenu Parsed (Context) par valeur, sans consommer l'Input.
    pub fn parsed_content_owned(&self) -> Result<SemanticContext, ArtefactError> {
        match self {
            Source::IR { content, .. } => match content {
                IRContent::ParsedDomain(domain, _) => Ok(domain.clone()),  // clone seulement le contenu
                IRContent::ParsedProblem(problem, _) => Ok(problem.clone()),
                _ => Err(ArtefactError::MissingIRContent),
            },
            _ => Err(ArtefactError::MissingIRContent),
        }
    }

    // --- Unknown text content ---
    pub fn text_content(&self) -> Option<&String> {
        match self {
            Source::Text { content, .. } => Some(content),
            _ => None,
        }
    }

    pub fn try_text_content(&self) -> Result<&String, ArtefactError> {
        match self {
            Source::Text { content, .. } => Ok(content),
            _ => Err(ArtefactError::missing_text_content()),
        }
    }

    /// Returns true if this Input is a raw source (PDDL or HDDL)
    pub fn is_raw(&self) -> bool {
        matches!(self, Source::Raw { .. })
    }

    /// Returns true if this Input is a raw domain (PDDL or HDDL)
    pub fn is_raw_domain(&self) -> bool {
        matches!(self, Source::Raw { content, .. } if content.kind() == RawKind::Domain)
    }

    /// Returns true if this Input is a raw problem (PDDL or HDDL)
    pub fn is_raw_problem(&self) -> bool {
        matches!(self, Source::Raw { content, .. } if content.kind() == RawKind::Problem)
    }

    /// Returns true if this Input is a raw PDDL source
    pub fn is_raw_pddl(&self) -> bool {
        matches!(self, Source::Raw { content, .. } if content.language() == Language::PDDL)
    }

    /// Returns true if this Input is a raw HDDL source
    pub fn is_raw_hddl(&self) -> bool {
        matches!(self, Source::Raw { content, .. } if content.language() == Language::HDDL)
    }

    /// Returns true if this Input is an intermediate representation (IR).
    pub fn is_ir(&self) -> bool {
        matches!(self, Source::IR { .. })
    }

    /// Returns true if this Input is a parsed IR domain (ParsedDomain)
    pub fn is_parsed_domain(&self) -> bool {
        matches!(self.ir_kind(), Some(IRKind::ParsedDomain))
    }

    /// Returns true if this Input is a parsed IR problem (ParsedProblem)
    pub fn is_parsed_problem(&self) -> bool {
        matches!(self.ir_kind(), Some(IRKind::ParsedProblem))
    }

    /// Returns true if this Input is a lifted IR problem (LiftedProblem)
    pub fn is_lifted_problem(&self) -> bool {
        matches!(self.ir_kind(), Some(IRKind::LiftedProblem))
    }

    /// Returns true if this Input is an unknown text source.
    pub fn is_text(&self) -> bool {
        matches!(self, Source::Text { .. })
    }

    /// Returns true if this Input is an unknown binary source.
    pub fn is_binary(&self) -> bool {
        matches!(self, Source::Binary { .. })
    }

    /// Retourne Some(RawKind) si c'est Raw, None sinon
    pub fn raw_kind(&self) -> Option<RawKind> {
        match self {
            Source::Raw { content, .. } => Some(content.kind()),
            _ => None,
        }
    }

    /// Retourne Some(IRKind) si c'est IR, None sinon
    pub fn ir_kind(&self) -> Option<IRKind> {
        match self {
            Source::IR { content, .. } => Some(content.kind()),
            _ => None,
        }
    }

    // --- Domain / Problem helpers ---
    pub fn is_domain(&self) -> bool {
        matches!(self.raw_kind(), Some(RawKind::Domain))
            || matches!(self.ir_kind(), Some(IRKind::ParsedDomain))
    }

    pub fn is_problem(&self) -> bool {
        matches!(self.raw_kind(), Some(RawKind::Problem))
            || matches!(self.ir_kind(), Some(IRKind::ParsedProblem))
            || matches!(self.ir_kind(), Some(IRKind::LiftedProblem))
    }
}

/// Infers the type of a raw source based on its textual content.
///
/// This function attempts to determine whether the raw source represents
/// a PDDL/HDDL domain or problem by scanning the initial tokens for the
/// typical `(define (domain ...)` or `(define (problem ...)` patterns.
///
/// # Arguments
///
/// * `source` - A string slice containing the raw source text.
///
/// # Returns
///
/// * `Some(RawKind::Domain)` if the source appears to define a domain.
/// * `Some(RawKind::Problem)` if the source appears to define a problem.
/// * `None` if the type cannot be inferred.
///
/// # Notes
///
/// * The function uses lexical analysis to detect the structure and ignores
///   errors or unexpected tokens during scanning.
/// * Only the first matching `(define ...)` construct is considered.
fn infer_raw_kind(source: &str) -> Option<RawKind> {
    let mut lexer = Token::lexer(source);
    while let Some(token_res) = lexer.next() {
        let token = match token_res {
            Ok(tok) => tok,
            Err(_) => continue,
        };

        // Detect the form (define (domain ...) or (define (problem ...))
        if token == Token::LParen {
            if let (Some(Ok(Token::Define)), Some(Ok(Token::LParen)), Some(Ok(next))) =
                (lexer.next(), lexer.next(), lexer.next())
            {
                match next {
                    Token::Domain => return Some(RawKind::Domain),
                    Token::Problem => return Some(RawKind::Problem),
                    _ => {}
                }
            }
        }
    }
    None
}


/// Detects the language of a raw source text, distinguishing between PDDL and HDDL.
///
/// This function analyzes the source text and attempts to identify whether it is
/// written in PDDL or HDDL. Detection is based on the presence of HDDL-specific
/// tokens such as `Task`, `Method`, `Hierarchy`, etc. If none of these tokens
/// are found, the function assumes the language is PDDL.
///
/// # Arguments
///
/// * `source` - A string slice representing the raw source text to analyze.
///
/// # Returns
///
/// * `Language::HDDL` if an HDDL-specific token is found.
/// * `Language::PDDL` otherwise.
///
/// # Notes
///
/// * Detection is based solely on token presence.
/// * Invalid or unrecognized tokens are ignored.
fn detect_raw_language(source: &str) -> Language {
    let mut lexer = Token::lexer(source);

    while let Some(token_res) = lexer.next() {
        let token = match token_res {
            Ok(tok) => tok,
            Err(_) => continue,
        };

        // HDDL-specific tokens
        match token {
            Token::Task
            | Token::Method
            | Token::MethodPreconditions
            | Token::Hierarchy
            | Token::Htn => return Language::HDDL,
            _ => {}
        }
    }

    Language::PDDL
}
