use crate::aiplan4rust::error::ParserErrorKind;
use crate::aiplan4rust::error::ParsingError;

/// The `ErrorManager` is responsible for centralized management of errors and warnings
/// produced during the syntax analysis, normalization, and semantics analysis processes.
/// It allows storing, displaying, and retrieving errors and warnings, ensuring
/// that all issues are properly tracked and reported.
///
/// This struct provides methods for:
/// - Adding errors and warnings to the list.
/// - Checking for errors of specific types.
/// - Resetting the error collection.
/// - Displaying all errors and warnings in a sorted order (by line and column).
///
/// It helps keep track of the state of the aiplan4rust and provides an easy way to
/// retrieve or display issues encountered during the parsing process.
#[derive(Debug, Clone, Default)]
pub struct ErrorManager {
    errors: Vec<ParsingError>,
}
impl ErrorManager {
    /// Creates a new, empty `ErrorsManager`.
    ///
    /// # Returns
    /// A new, empty error and warning manager.
    pub fn new() -> Self {
        ErrorManager { errors: Vec::new() }
    }

    /// Adds a single `ParserError` to the error collection.
    ///
    /// This function appends the provided `ParserError` to the internal `errors`
    /// vector, allowing the error manager to keep track of all encountered
    /// parsing errors.
    ///
    /// # Parameters
    ///
    /// - `error`: The `ParsingError` to be added to the collection.
    ///
    /// # Note
    /// This function modifies the internal state of the `ErrorManager` by adding
    /// the given error to the `errors` vector.
    pub fn add_error(&mut self, error: ParsingError) {
        self.errors.push(error);
    }

    /// Adds a list of `ParserError`s to the current `errors` collection.
    ///
    /// This function takes a vector of `ParserError`s and appends them to the
    /// existing `errors` collection of the struct that contains this method.
    /// It allows for accumulating multiple parsing errors in a single structure
    /// for later handling or reporting.
    ///
    /// # Arguments
    ///
    /// * `errors` - A vector containing the `ParserError`s to be added.
    ///
    /// # Note
    /// This function directly modifies the internal state of the struct, so it
    /// does not return a value. The errors are appended in the order they appear
    /// in the input vector.
    pub fn add_errors(&mut self, errors: Vec<ParsingError>) {
        self.errors.extend(errors);
    }

    /// Returns an iterator over all errors stored in the manager.
    ///
    /// # Returns
    /// An iterator (`impl Iterator<Item = &ParserError>`) over all errors.
    pub fn errors(&self) -> impl Iterator<Item = &ParsingError> {
        self.errors.iter()
    }

    /// Returns an iterator over errors of a specific kind.
    ///
    /// # Parameters
    /// - `kind`: The type of error to filter, specified as a variant of the `Kind` enum.
    ///
    /// # Returns
    /// An iterator (`impl Iterator<Item = &ParserError>`) filtered by the specified kind.
    pub fn errors_of_kind(&self, kind: ParserErrorKind) -> impl Iterator<Item = &ParsingError> {
        self.errors.iter().filter(move |e| *e.kind() == kind)
    }

    /// Checks if there are any errors of a specific type in the list of errors.
    ///
    /// This function iterates through the list of errors in `self.errors` and checks if
    /// any of the errors match the type specified by the `kind` parameter. It returns `true`
    /// if at least one error of the given type is found, otherwise returns `false`.
    ///
    /// # Parameters
    /// - `kind`: The type of error to search for. It is a `Kind` value representing a
    ///   specific error type.
    ///
    /// # Returns
    /// Returns `true` if there is at least one error of the specified type, otherwise returns
    /// `false`.
    ///
    /// # Example
    /// ```rust
    /// use aiplan4rust::aiplan4rust::error::parsing_error::ParserErrorKind::LexicalError;
    ///
    /// let error_manager = ErrorManager::new();
    /// if error_manager.has_errors_of_kind(LexicalError) {
    ///     println!("There are syntax errors!");
    /// } else {
    ///     println!("No syntax errors.");
    /// }
    /// ```
    pub fn has_errors_of_kind(&self, kind: ParserErrorKind) -> bool {
        self.errors.iter().any(|e| *e.kind() == kind)
    }

    /// Resets the error manager by clearing all recorded errors.
    ///
    /// This function removes all errors from the `self.errors` list, effectively resetting
    /// the state of the error manager. After calling this function, the error manager
    /// will no longer have any recorded errors.
    ///
    /// # Example
    /// ```rust
    /// let mut error_manager = ErrorManager::new();
    /// error_manager.add_error(...);
    /// error_manager.reset();
    /// assert_eq!(error_manager.errors.len(), 0); // No errors after reset
    /// ```
    pub fn reset(&mut self) {
        self.errors.clear();
    }

    pub fn add_errors_from(&mut self, other: &ErrorManager) {
        self.errors.extend(other.errors.iter().cloned());
    }

    pub fn is_empty(&self) -> bool {
        self.errors.is_empty()
    }

    /// Displays all errors and warnings sorted by line and column.
    ///
    /// This method first sorts the errors and warnings by line number. In case of ties,
    /// it sorts them by column number. After sorting, it iterates through the errors
    /// and warnings, displaying each one.
    ///
    /// # Example
    /// ```rust
    /// let error_manager = ErrorManager::new();
    /// error_manager.add_error(...);
    /// error_manager.display_all(); // Displays all sorted errors
    /// ```
    pub fn display_all(&self) {
        let mut sorted_errors = self.errors.clone();

        // Sort errors by line number, then by column number if line numbers are equal
        sorted_errors.sort_by_key(|e| (e.line(), e.column()));

        for error in sorted_errors {
            println!("{}", error);
        }
    }
}
