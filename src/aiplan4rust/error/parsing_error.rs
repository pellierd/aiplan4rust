use std::fmt;

/// `ParsingError` represents an error or warning encountered during the parsing process
/// of source code or data. It can be used to capture issues that occur during both lexical
/// analysis (tokenization) and syntax parsing.
///
/// This struct holds information about the error's type, its location in the source,
/// and a description of the error or warning. It is used throughout the parsing process
/// to report errors back to the user or for further processing.
///
/// ## Fields
///
/// - `kind`: An instance of the `Kind` enum that describes the type of error or warning:
///   - `Kind::LexerError`: Indicates an error during lexical analysis, typically when tokens
///     cannot be recognized or processed correctly.
///   - `Kind::ParseError`: Denotes a syntax error that occurred during the parsing phase.
///   - `Kind::ParseWarning`: A warning that indicates a potential issue that may not block
///     parsing but should be reviewed by the user.
/// - `file_path`: An optional `PathBuf` that holds the path of the file where the error
///   occurred, allowing better traceability in multi-file projects.
/// - `line`: A `usize` representing the line number in the source code where the error or warning
///   was detected.
/// - `column`: A `usize` indicating the column number in the line where the error occurred.
/// - `content`: A `String` that provides a detailed message about the error or warning, describing
///   what went wrong or what to pay attention to.
///
/// ## Example
///
/// ```rust
/// let error = ParsingError {
///     kind: Kind::ParseError,
///     file_path: Some(PathBuf::from("example.pddl")),
///     line: 5,
///     column: 12,
///     content: "Unexpected token in expression".to_string(),
/// };
/// ```
///
/// ## Notes
/// The `ParseError` struct is designed to provide detailed and structured information
/// about errors and warnings during parsing, allowing developers to better understand
/// and address issues in their source code.
#[derive(Clone, Debug, PartialEq)]
pub struct ParsingError {
    kind: ParserErrorKind,
    file_path: Option<String>,
    line: usize,
    column: usize,
    content: String,
}

/// `Kind` enum defines the different types of errors or warnings that can be represented by a
/// `ParseError` in the context of parsing operations.
///
/// ## Example
///
/// ```rust
/// let error = ParsingError {
///     kind: Kind::LexerError("Unrecognized token".to_string()),
///     file_path: Some(PathBuf::from("source.pddl")),
///     line: 3,
///     column: 14,
///     content: "Unrecognized token".to_string(),
/// };
/// ```
#[derive(Clone, Debug, PartialEq)]
pub enum ParserErrorKind {
    /// Represents an error that occurred during lexical analysis, such as failing to tokenize input
    /// correctly. The associated `String` contains the error message.
    LexicalError,
    /// Indicates a syntax error. The associated `String` holds the error message explaining what
    /// went wrong.
    ParseError,
    /// Represents a non-critical issue. The associated `String` contains a warning message
    /// describing the minor problem.
    ParseWarning,
}

/// Implements the `fmt::Display` trait for the `Kind` enum.
///
/// This trait implementation allows the `Kind` enum, which represents different types of
/// parsing-related errors or warnings, to be formatted into a human-readable string. This
/// enables displaying the specific kind of error or warning when used with the `format!` or
/// `println!` macros, aiding in debugging or logging.
///
/// The implementation works as follows:
/// - If the `Kind` is `LexerError`, it will display as `"Lexical Error"`.
/// - If the `Kind` is `ParseError`, it will display as `"Parse Error"`.
/// - If the `Kind` is `ParseWarning`, it will display as `"Parse Warning"`.
impl fmt::Display for ParserErrorKind {
    /// Formats the `Kind` enum into a user-readable string.
    ///
    /// # Arguments
    ///
    /// * `f` - The formatter that will format the error kind.
    ///
    /// # Returns
    ///
    /// The result of writing the formatted string to `f`.
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ParserErrorKind::LexicalError => write!(f, "Lexical Error"),
            ParserErrorKind::ParseError => write!(f, "Parse Error"),
            ParserErrorKind::ParseWarning => write!(f, "Parse Warning"),
        }
    }
}

impl ParsingError {
    /// Creates a new `ParsingError`.
    ///
    /// # Parameters
    /// - `kind`: The type of the error or warning, which is one of the variants from the `Kind`
    ///     enum (e.g., `Kind::LexerError`, `Kind::ParseError`, `Kind::ParseWarning`).
    /// - `line`: The line number in the source document where the error occurred.
    /// - `column`: The column number within the line where the error or warning was found.
    /// - `content`: A description or message explaining the error or warning.
    ///
    /// # Returns
    /// Returns a new instance of `ParsingError`.
    pub fn new(
        kind: ParserErrorKind,
        file_path: Option<String>,
        line: usize,
        column: usize,
        content: String,
    ) -> ParsingError {
        ParsingError {
            kind,
            file_path,
            line,
            column,
            content,
        }
    }

    /// Retrieves the kind of error or warning associated with this `ParsingError`.
    ///
    /// # Returns
    /// Returns a reference to the `Kind` of this error (either `Kind::LexerError`,
    /// `Kind::ParseError`, or `Kind::ParseWarning`).

    pub fn kind(&self) -> &ParserErrorKind {
        &self.kind
    }
    /// Retrieves the file path where the error occurred, if available.
    ///
    /// # Returns
    /// An `Option<&str>` containing the file path if provided, or `None` if no file path is available.
    pub fn file_path(&self) -> Option<&str> {
        self.file_path.as_deref()
    }

    /// Retrieves the line number where the error occurred.
    ///
    /// # Returns
    /// A `usize` representing the line number in the source document.
    pub fn line(&self) -> usize {
        self.line
    }

    /// Retrieves the column number where the error occurred.
    ///
    /// # Returns
    /// A `usize` representing the column number within the line.
    pub fn column(&self) -> usize {
        self.column
    }

    /// Retrieves the error message or description.
    ///
    /// # Returns
    /// A reference to a `String` containing the error message.
    pub fn content(&self) -> &str {
        &self.content
    }
}

/// Implements the `fmt::Display` trait for the `Parsingrror` struct.
///
/// This trait implementation formats the `ParsingError` into a human-readable string, which can
/// be used for displaying error messages in a clear and structured way. This is particularly useful
/// for debugging, logging, and providing detailed error output to users.
///
/// The format of the error message varies based on the presence of the `file_path` field:
/// - If `file_path` is `Some`, the error message will include the file name, line number, column
///   number, error kind, and the content of the error description. If the `file_name` is not available
///   or cannot be converted to a `str`, the full file path will be displayed.
/// - If `file_path` is `None`, only the line number, column number, error kind, and content will be shown.
///
/// The format is as follows:
/// - With file path: `"file_name:line:column: error_kind: error_content"`
/// - Without file path: `"line:column: error_kind: error_content"`
impl fmt::Display for ParsingError {
    /// Formats the `ParsingError` into a user-readable string.
    ///
    /// # Arguments
    ///
    /// * `f` - The formatter that will format the `ParsingError`.
    ///
    /// # Returns
    ///
    /// The result of writing the formatted string to `f`.
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match &self.file_path {
            Some(path) => {
                write!(
                    f,
                    "{} [{}] {}:{} {}",
                    self.kind, path, self.line, self.column, self.content
                )
            }
            None => {
                write!(
                    f,
                    "{} {}:{} {}",
                    self.kind, self.line, self.column, self.content
                )
            }
        }
    }
}
