use crate::aiplan4rust::error::error_manager::ErrorManager;
use crate::aiplan4rust::error::parsing_error::{ParserErrorKind, ParsingError};
use crate::aiplan4rust::frontend::ParserInternalError;
use crate::aiplan4rust::pddl_display::PDDLDisplay;
use crate::aiplan4rust::syntax::ast::{Ast, AstKind};
use crate::aiplan4rust::syntax::lexer::Lexer;
use crate::aiplan4rust::syntax::parser_result::ParserResult;
use crate::aiplan4rust::syntax::pddl::{HDDLParser, PDDLParser};
use crate::aiplan4rust::syntax::syntax_tree::SyntaxTree;
use crate::aiplan4rust::syntax::token::{LexicalError, Token, NUMBER_TYPE, OBJECT_TYPE};

use lalrpop_util::{ErrorRecovery, ParseError};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::mem;
use std::str::FromStr;
use std::time::SystemTime;

/// An enum representing different types of PDDL and HDDL expressions.
///
/// The `Language` enum is used to specify which type of planning language is being
/// used in the context of parsing. It currently supports two variants:
/// PDDL (Planning Domain Definition Language) and HDDL (Hierarchical Domain Definition Language).
/// These variants help determine the syntax and semantics of the expressions being parsed.
///
/// # Variants
/// - `PDDL`: Represents the Planning Domain Definition Language (PDDL), a widely used language for
///   defining planning problems and domains.
/// - `HDDL`: Represents the Hierarchical Domain Definition Language (HDDL), an extension of PDDL
///   that incorporates hierarchical structures for domain and problem representations.
///
/// # Example
/// ```rust
/// let language = Language::PDDL;
/// ```
///
/// # Notes
/// - This enum is designed to support multiple types of planning languages. As the `aiplan4rust`
///   library evolves, additional languages or variants may be added to accommodate new planning
///   languages or extensions.

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Language {
    PDDL,
    HDDL,
}

impl FromStr for Language {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "pddl" => Ok(Language::PDDL),
            "hddl" => Ok(Language::HDDL),
            _ => Err(format!("Invalid language: {}", s)),
        }
    }
}

#[derive(Debug)]
/// A structure for analyzing the syntax of PDDL expressions.
///
/// The `SyntaxAnalyzer` is responsible for parsing and validating PDDL expressions from
/// a given source string. It works with an optional path for file-based sources and an
/// error manager to handle any parsing errors.
///
/// # Fields
/// - `source`: A reference to the source string containing the PDDL expression to analyze.
/// - `path`: An optional `PathBuf` representing the file path from which the source is read.
/// - `pddl_fragment`: The `PDDLFragment` that represents the parsed expression.
/// - `error_manager`: A mutable reference to the `ErrorManager` for managing parsing errors.
///
/// # Example
/// ```rust
/// use aiplan4rust::aiplan4rust::syntax::syntax::PDDLFragment;
/// let source = "(define (problem test) ...)";
/// let mut error_manager = ErrorManager::new();
/// let pddl_expression = PDDLFragment::Domain; // Example expression
/// let syntax_analyzer = SyntaxAnalyzer {
///     source,
///     path: None,
///     pddl_fragment,
///     error_manager: &mut error_manager,
/// };
/// ```
pub struct Parser<'a> {
    filename: Option<&'a str>,
    source: Option<&'a str>,
    error_manager: ErrorManager,
}

impl<'a> Parser<'a> {
    /// Creates a new `Parser` instance.
    ///
    /// # Returns
    /// Returns a new instance of `Parser`.
    pub fn new() -> Self {
        Self {
            filename: None,
            source: None,
            error_manager: ErrorManager::new(),
        }
    }

    /// Returns a reference to the `ErrorManager` associated with this instance.
    ///
    /// The `ErrorManager` stores and manages errors encountered during processing.
    /// This function allows access to the error manager for querying or handling errors.
    ///
    /// # Returns
    /// A reference to the `ErrorManager`.
    pub fn error_manager(&self) -> &ErrorManager {
        &self.error_manager
    }

    /// This function parses a PDDL or HDDL file and returns a `SyntaxTree` and handling errors.
    ///
    /// # Arguments
    ///
    /// * `filename`: The name of the source file being parsed, typically used for error reporting.
    /// * `source`: The source code of the PDDL or HDDL file as a string to be parsed.
    /// * `language`: Specifies which language to use for parsing the source code (either PDDL or
    ///   HDDL).
    ///
    /// # Returns
    ///
    /// * `Result<ParserResult, ParserInternalError>`:
    ///   - If parsing is successful, returns a `ParserResult` containing the generated `SyntaxTree`
    ///   and any errors encountered during parsing.
    ///   - If parsing fails due to lexical or parsing errors, returns an internal error with
    ///   details about the failure.
    ///
    /// # Error Handling
    ///
    /// The function handles both lexical errors (such as invalid tokens) and parsing errors (such
    /// as invalid grammar). It processes any errors and returns them as part of the `ParserResult`
    /// if parsing was unsuccessful.
    ///
    /// # Example
    ///
    /// ```rust
    /// let result = parser.parse("example.pddl", source_code, Language::PDDL);
    /// match result {
    ///     Ok(parser_result) => {
    ///         // Process the resulting syntax tree
    ///     },
    ///     Err(error) => {
    ///         // Handle internal parsing error
    ///     }
    /// }
    /// ```
    pub fn parse(
        &mut self,
        filename: &'a str,
        source: &'a str,
        language: &Language,
    ) -> Result<ParserResult, ParserInternalError> {
        // Store temporary references to the filename and source for later use
        self.filename = Some(filename);
        self.source = Some(source);

        // Initialize a vector to store LALRPOP errors that may occur during parsing
        let mut larlpop_errors = Vec::new();

        // Create a lexer from the provided source code
        let lexer = Lexer::new(source);

        // Attempt to parse the source code according to the language specified
        let parse_result = match language {
            Language::PDDL => PDDLParser::new().parse(&mut larlpop_errors, lexer),
            Language::HDDL => HDDLParser::new().parse(&mut larlpop_errors, lexer),
        };

        // Handle any syntax errors that were collected during parsing
        self.handle_syntax_errors(&larlpop_errors, source);

        if self
            .error_manager()
            .has_errors_of_kind(ParserErrorKind::LexicalError)
        {
            Ok(ParserResult::new(None, mem::take(&mut self.error_manager)))
        } else {
            match parse_result {
                Ok(mut ast) => {
                    self.process_ast(&mut ast, source)?;

                    println!("AST: {}", ast);

                    if self
                        .error_manager()
                        .has_errors_of_kind(ParserErrorKind::ParseError)
                    {
                        Ok(ParserResult::new(None, mem::take(&mut self.error_manager)))
                    } else {
                        let syntax_tree =
                            SyntaxTree::new(ast, Some(filename.to_string()), SystemTime::now());
                        Ok(ParserResult::new(
                            Some(syntax_tree),
                            mem::take(&mut self.error_manager),
                        ))
                    }
                }
                Err(_) => Ok(ParserResult::new(None, mem::take(&mut self.error_manager))),
            }
        }
    }

    /// Handles the syntax errors produced by the LALRPOP aiplan4rust.
    /// For each error, a `ParserError` is created and added to the error manager.
    ///
    /// # Arguments
    /// * `larlpop_errors`: A list of errors produced by the LALRPOP aiplan4rust.
    /// * `source`: The source code to reference when generating error messages.
    fn handle_syntax_errors(
        &mut self,
        larlpop_errors: &[ErrorRecovery<usize, Token, LexicalError>],
        source: &'a str,
    ) {
        for larlpop_error in larlpop_errors {
            // Convert each LALRPOP error into a ParserError and add it to the error manager
            let parser_error = self.to_parser_error(&larlpop_error.error, source, self.filename);
            self.error_manager.add_error(parser_error);
        }
    }

    /// Processes the abstract syntax tree (AST) based on the parsing result.
    /// This function normalizes the AST (keeps it in a Box) and initializes its position in the
    /// source code. If parsing fails or there are syntax errors, it returns `None`.
    ///
    /// # Arguments
    /// * `parse_result`: The result of the parsing attempt, containing the AST or an error.
    /// * `larlpop_errors`: A list of errors encountered during parsing.
    /// * `source`: The source code to initialize AST positions.
    fn process_ast(
        &mut self,
        ast: &mut Box<Ast>,
        source: &'a str,
    ) -> Result<(), ParserInternalError> {
        // Normalize the AST to ensure its structure is consistent
        self.normalize_ast(ast)?;
        // Initialize the position of the AST elements in the source code
        self.init_ast_position(ast, source);
        Ok(())
    }

    /// Initializes the position information of an abstract syntax tree (AST).
    /// This function computes the line and column numbers for each node in the AST
    /// using a `FastLineTable`, which maps byte offsets to positions efficiently.
    ///
    /// # Arguments
    /// - `ast`: A mutable reference to the root node of the AST.
    /// - `source`: The source code string from which the AST was parsed.
    fn init_ast_position(&self, ast: &mut Ast, source: &str) {
        // Create a `FastLineTable` with an interval of 100 lines for coarse indexing.
        // The interval value (100) can be adjusted depending on the size of the source text.
        let table = FastLineTable::new(source, 100);

        // Recursively set positions for all AST nodes
        self.init_ast_position_rec(ast, &table);
    }

    /// Recursively sets the start and end positions (line, column) for each AST node.
    ///
    /// # Arguments
    /// - `ast`: A mutable reference to an AST node.
    /// - `table`: A reference to the `FastLineTable` used to compute positions.
    fn init_ast_position_rec(&self, ast: &mut Ast, table: &FastLineTable) {
        // Compute and set the start position of the current AST node
        let (line, column) = table.get_position(ast.start_offset());
        ast.set_start_position(line, column);

        // Compute and set the end position of the current AST node
        let (line, column) = table.get_position(ast.end_offset());
        ast.set_end_position(line, column);

        // Recursively process all child nodes of the current AST node
        for child in ast.children_mut() {
            self.init_ast_position_rec(child, table);
        }
    }

    /// Finds the line and column of a character position in a string.
    ///
    /// This function computes the line and column numbers corresponding to a specific
    /// character position (`position`) within the provided `input` string. It assumes
    /// that lines are separated by newline characters (`\n`), with line and column
    /// numbering starting at 1.
    ///
    /// # Parameters
    /// - `offset`: The zero-based index of the character in the string whose line and column are to
    ///     be determined.
    /// - `source`: A reference to the input string where the character position is located.
    ///
    /// # Returns
    /// A tuple `(usize, usize)` where:
    /// - The first element is the line number (starting from 1).
    /// - The second element is the column number (starting from 1).
    ///
    /// # Example
    /// ```rust
    /// let input = "Hello\nRustaceans!";
    /// let offset = 8; // The character 'R' in "Rustaceans!"
    /// let (line, column) = get_position(offset, input);
    /// assert_eq!((line, column), (2, 1)); // 'R' is on line 2, column 1
    /// ```
    ///
    /// # Notes
    /// - If `offset` is greater than the length of the string, the function
    ///   will return the line and column corresponding to the end of the string.
    /// - The function handles multiline input and correctly resets the column
    ///   count after encountering a newline.
    ///
    /// # Panics
    /// This function does not explicitly panic but assumes that the `offset` is within
    /// the range of valid indices for the string. Out-of-range values may result in unexpected
    /// behavior.
    fn get_position(&self, offset: usize, source: &str) -> (usize, usize) {
        let mut line = 1;
        let mut column = 1;
        for (index, ch) in source.chars().enumerate() {
            if index == offset {
                break;
            }
            match ch {
                '\n' => {
                    line += 1;
                    column = 1;
                }
                _ => {
                    column += 1;
                }
            }
        }
        (line, column)
    }

    /// Recursively normalizes an Abstract Syntax Tree (AST) by processing its nodes and normalizing
    /// any `TypedList` nodes it encounters. This function traverses the entire AST, applying
    /// normalization to `TypedList` nodes and leaving other nodes unchanged.
    ///
    /// Specifically, this function:
    /// - Recursively visits each node in the AST.
    /// - When it encounters a `TypedList` node, it applies the `normalise_typed_list` function to
    ///     normalize it.
    /// - For non-`TypedList` nodes, it recursively processes their children.
    ///
    /// This function ensures that all `TypedList` nodes are normalized, making it easier to handle
    /// them in subsequent phases of processing, such as analysis or code generation.
    ///
    /// # Arguments
    ///
    /// * `ast` - A mutable reference to the `Ast` node that represents the current node in the AST.
    ///   This function will modify `ast` if it is a `TypedList` node by normalizing it.
    ///
    /// # Example
    /// ```rust
    /// let mut ast = ... // AST that may contain `TypedList` nodes
    /// checker.normalise_ast(&mut ast); // Normalizes all `TypedList` nodes in the AST
    /// ```
    fn normalize_ast(&mut self, ast: &mut Ast) -> Result<(), ParserInternalError> {
        // Match on the kind of the current AST node
        match ast.kind() {
            AstKind::RequireDef => {
                self.normalize_require_def(ast)?;
            }
            AstKind::TypedList => {
                self.normalize_typed_list(ast)?;
            }
            // For other nodes, recursively normalize their children
            _ => {
                for child in ast.children_mut() {
                    self.normalize_ast(child)?;
                }
            }
        }

        Ok(())
    }

    /// Normalizes the requirements within an abstract syntax tree (AST) by removing duplicate
    /// `Requirement` nodes. It ensures that only unique requirements are retained in the tree.
    ///
    /// This function modifies the provided `Ast` by removing duplicate requirements. If a duplicate
    /// is found, a warning is printed, and the duplicate is removed. If any non-`Requirement` nodes
    /// are encountered in the AST, a `ParserInternalError` is returned.
    ///
    /// # Arguments
    /// * `ast` - A mutable reference to the `Ast` (Abstract Syntax Tree) to be modified. The AST
    ///   should contain `Requirement` nodes, and this function will filter out any duplicate ones.
    ///
    /// # Errors
    /// This function returns a `ParserInternalError` if it encounters a non-`Requirement` node in the
    /// `Ast`'s children.
    ///
    /// # Example
    /// ```rust
    /// let mut ast = Ast::new(); // Assume Ast is properly initialized with children
    /// normalize_require_def(&mut ast)?;
    /// ```
    /// The function will modify `ast` by retaining only unique `Requirement` nodes and printing a
    /// warning for any duplicates that are removed.
    ///
    /// # Notes
    /// - The function uses a `HashSet` to track seen requirements and ensures that only the first
    ///   occurrence of each requirement is kept.
    /// - The `retain` method is used to filter out duplicate `Requirement` nodes in-place.
    /// - If a non-`Requirement` node is encountered, a `ParserInternalError` is returned.
    fn normalize_require_def(&mut self, ast: &mut Ast) -> Result<(), ParserInternalError> {
        let mut seen_requirements = HashSet::new();
        let children = ast.children_mut();

        // Variable to handle errors
        let mut encountered_error = None;

        // Filter duplicates using `retain`
        children.retain(|child| {
            match child.kind() {
                AstKind::Requirement(requirement) => {
                    if !seen_requirements.insert(requirement.clone()) {
                        // Ensure `source` is not None before using `unwrap`
                        if let Some(source) = &self.source {
                            let (line, column) = self.get_position(child.start_offset(), source);
                            let content = format!(
                                "Duplicate declaration of requirement '{}' detected. Please ensure requirements are not repeated.",
                                requirement.to_pddl_string()
                            );
                            let warning = ParsingError::new(
                                ParserErrorKind::ParseWarning,
                                self.filename.as_deref().map(|s| s.to_string()), // Using `as_deref()` to avoid unwrap
                                line,
                                column,
                                content,
                            );
                            self.error_manager.add_error(warning);
                            false // Ignore this duplicate requirement
                        } else {
                            // If `source` is None, return an error
                            encountered_error = Some(ParserInternalError::new(
                                "Source string is missing".to_string(),
                            ));
                            false
                        }
                    } else {
                        true // Keep this requirement as it's not a duplicate
                    }
                }
                _ => {
                    let error_message = format!(
                        "Unexpected child type found: Expected a requirement, found {:?}",
                        child.kind()
                    );
                    encountered_error = Some(ParserInternalError::new(error_message));
                    false
                }
            }
        });

        // Return the encountered error if it exists
        if let Some(error) = encountered_error {
            return Err(error);
        }

        Ok(())
    }

    /// Normalizes a `TypedList` node in the Abstract Syntax Tree (AST).
    /// This function simplifies and standardizes the `TypedList` structure by applying the
    /// appropriate normalization to its elements, types, and next `TypedList` node, ensuring
    /// consistency in the AST structure.
    ///
    /// The function performs the following transformations:
    /// - Splits the `TypedList` node into individual components: elements, types, and the next
    ///     `TypedList`.
    /// - Applies normalization to the `TypedList` by ensuring that all elements are consistently
    ///     typed, and that types are either provided or defaulted (to `OBJECT` or `NUMBER`).
    /// - Ensures the consistency of the list structure, making it easier to process later.
    ///
    /// This transformation is crucial in ensuring that the `TypedList` is in a form that can be
    /// easily manipulated by later stages of processing, such as code generation, validation, and
    /// other transformations.
    ///
    /// # Arguments
    /// * `ast`: A mutable reference to the `Ast` node representing the `TypedList` to be normalized.
    ///
    /// # Returns
    /// Returns a `Result<(), ParserInternalError>`. On success, it returns `Ok(())`, and on failure,
    /// it returns a `ParserInternalError` if an error occurs during normalization.
    ///
    /// # Example
    /// ```rust
    /// let mut ast = ... // A `TypedList` node in the AST
    /// typed_list.normalize_typed_list(&mut ast); // Normalizes the `TypedList` node
    /// ```
    ///
    /// # Complexity
    /// This function performs a split and normalization on the components of the `TypedList`,
    /// which involves a traversal of the child nodes of the AST. The complexity is linear relative
    /// to the number of child nodes.
    fn normalize_typed_list(&mut self, ast: &mut Ast) -> Result<(), ParserInternalError> {
        // Split the `TypedList` into its components: elements, types, and the next `TypedList`
        let (elements, types, next_typed_list) = self.split_typed_list(ast)?;

        // Apply normalization on the split components (elements, types, and next list)
        self.apply_normalisation(ast, elements, types, next_typed_list)?;
        Ok(())
    }

    /// Splits an AST of type `TypedList` into three specific components: elements, type, and the
    /// next typed list.
    ///
    /// This function processes a mutable reference to an AST and extracts three components:
    /// - The `Ast` elements (such as primitive types, constants, etc.)
    /// - The type associated with the elements (if present)
    /// - The next `TypedList` (if it exists)
    ///
    /// # Parameters
    /// - `ast`: A mutable reference to the AST to be analyzed. The AST is traversed to extract the
    ///   elements, their associated type, and the next `TypedList` if available.
    ///
    /// # Returns
    /// Returns a `Result` containing a tuple `(Vec<Box<Ast>>, Option<Box<Ast>>, Option<Box<Ast>>)` on success, where:
    /// - `Vec<Box<Ast>>`: A vector of `Box<Ast>` containing the elements found (e.g., primitive
    ///     types, constants, etc.).
    /// - `Option<Box<Ast>>`: An `Option` containing a `Box<Ast>` representing the type found, or
    ///     `None` if no type is found.
    /// - `Option<Box<Ast>>`: An `Option` containing a `Box<Ast>` representing the next `TypedList`
    ///     found, or `None` if no next typed list is found.
    ///
    /// On failure, the function returns a `ParserInternalError` with a descriptive error message if:
    /// - The AST is not of type `TypedList`.
    /// - An unexpected AST node type is encountered during traversal.
    ///
    /// # Example
    /// ```rust
    /// let ast = ...; // An AST that has been previously constructed
    /// let (elements, types, next_typed_list) = split_typed_list(ast);
    /// ```
    ///
    /// # Complexity
    /// This function traverses all the children of the AST exactly once, which gives it a linear
    /// complexity relative to the number of child nodes in the AST.
    ///
    /// # Notes
    /// - This function assumes that the `ast` is of type `TypedList`, as validated at the beginning
    ///     of the method. In case of an invalid AST type or unexpected node, an error is returned.
    fn split_typed_list(
        &self,
        ast: &mut Ast,
    ) -> Result<(Vec<Box<Ast>>, Option<Box<Ast>>, Option<Box<Ast>>), ParserInternalError> {
        let mut elements: Vec<Box<Ast>> = Vec::new();
        let mut types: Option<Box<Ast>> = None;
        let mut next_typed_list: Option<Box<Ast>> = None;

        if *ast.kind() != AstKind::TypedList {
            return Err(ParserInternalError::new(format!(
                "Expected AST of type 'TypedList', found: '{:?}'",
                ast.kind()
            )));
        }

        for child in ast.children() {
            match child.kind() {
                AstKind::PrimitiveType(_)
                | AstKind::Constant(_)
                | AstKind::Variable(_)
                | AstKind::AtomicFunctionSkeleton => {
                    elements.push(child.clone());
                }
                AstKind::Type => {
                    types = Some(child.clone());
                }
                AstKind::TypedList => {
                    next_typed_list = Some(child.clone());
                }
                _ => {
                    return Err(ParserInternalError::new(format!(
                        "Unexpected AST Node: '{:?}'",
                        child.kind()
                    )));
                }
            }
        }

        Ok((elements, types, next_typed_list))
    }

    /// Applies normalization to the given `TypedList` node by splitting elements into individual
    /// typed elements and adding implicit types commonly used in PDDL, such as `primitive`,
    /// `object`, and `number`.
    ///
    /// This function normalizes a `TypedList` AST node by processing each element in the provided
    /// `elements` vector, splitting them into individual nodes, and assigning a type to each
    /// element. If no explicit type is provided, a default type is assigned. The function also
    /// processes an optional `next_typed_list`, and if provided, it normalizes and attaches it as
    /// the last child of the normalized `TypedList`.
    ///
    /// The normalization also handles the addition of PDDL-specific types (e.g., `primitive`,
    /// `object`, and `number`), ensuring that the structure of the AST conforms to the expectations
    /// for PDDL.
    ///
    /// # Arguments
    ///
    /// * `ast` - A mutable reference to the AST that will be updated with the normalized
    ///     `TypedList`.
    /// * `elements` - A vector of `Ast` elements that will be used to build the normalized
    ///     `TypedList`. These elements are split into individual elements, each with an associated
    ///      type.
    /// * `types` - An optional `Ast` node representing the type to be used for the elements. If
    ///     `None`, a default type is used. For PDDL compatibility, common types like `primitive`,
    ///     `object`, and `number` are used implicitly.
    /// * `next_typed_list` - An optional `Ast` node representing the next `TypedList` to be linked
    ///     as a child of the current `TypedList`. If `None`, no further list is linked.
    ///
    /// # Example
    /// ```rust
    /// let mut ast = Ast::new(...);  // Example AST initialization
    /// let elements = vec![Box::new(Ast::new(...)), Box::new(Ast::new(...))];
    /// let types = Some(Box::new(Ast::new(...)));
    /// let next_typed_list = Some(Box::new(Ast::new(...)));
    /// checker.apply_normalisation(&mut ast, elements, types, next_typed_list);
    /// ```
    fn apply_normalisation(
        &mut self,
        ast: &mut Ast,
        elements: Vec<Box<Ast>>,
        mut types: Option<Box<Ast>>,
        next_typed_list: Option<Box<Ast>>,
    ) -> Result<(), ParserInternalError> {
        // Filter the types in place if they are provided
        if let Some(ref mut ty) = types {
            self.filter_duplicate_types(ty)?; // Modify the types directly
        }

        let mut normalised_typed_list = Box::new(Ast::new(
            AstKind::TypedList,
            Vec::new(),
            ast.start_offset(),
            ast.end_offset(),
        ));
        let mut current_typed_list = &mut normalised_typed_list;

        for element in elements {
            let start = element.start_offset();
            let end = element.end_offset();
            let mut children = Vec::new();
            let mut cloned_element = element.clone();
            if matches!(cloned_element.kind(), AstKind::AtomicFunctionSkeleton) {
                self.normalize_ast(&mut cloned_element)?
            }
            children.push(cloned_element);
            if let Some(ref ty) = types {
                children.push(ty.clone());
            } else {
                children.push(self.create_default_type(&element, start, end));
            }
            let next_typed_list_node =
                Box::new(Ast::new(AstKind::TypedList, Vec::new(), start, end));
            children.push(next_typed_list_node);
            current_typed_list.set_children(children);
            current_typed_list = current_typed_list.children_mut().last_mut().unwrap();
        }
        if let Some(mut list) = next_typed_list {
            self.normalize_typed_list(&mut list)?;
            *current_typed_list = Box::new(*list);
        }

        *ast = *normalised_typed_list;
        Ok(())
    }

    /// Filters duplicate `PrimitiveType` children from the `Ast`.
    ///
    /// This function iterates over the children of the given `Ast` and removes any duplicate
    /// `PrimitiveType` declarations. It adds a warning to the `error_manager` if a duplicate is found.
    /// If a child is not a `PrimitiveType`, an error is returned.
    ///
    /// # Arguments
    /// * `ty` - The mutable reference to the `Ast` whose children are to be filtered.
    ///
    /// # Returns
    /// * `Ok(())` if the operation succeeds without errors.
    /// * `Err(ParserInternalError)` if an unexpected child type is encountered.
    ///
    /// # Example
    /// ```
    /// let result = analyzer.filter_duplicate_types(&mut ast);
    /// match result {
    ///     Ok(()) => println!("Types filtered successfully."),
    ///     Err(e) => println!("Error: {}", e),
    /// }
    /// ```
    fn filter_duplicate_types(&mut self, ty: &mut Ast) -> Result<(), ParserInternalError> {
        let mut seen_types = HashSet::new(); // A set to track unique type names
        let mut unique_children = Vec::new(); // A vector for unique children

        for child in ty.children_mut() {
            if let AstKind::PrimitiveType(name) = &child.kind() {
                if seen_types.insert(name.clone()) {
                    unique_children.push(child.clone()); // Keep unique type declarations
                } else {
                    // Ensure `source` is not None before using `unwrap`
                    if let Some(source) = &self.source {
                        let (line, column) = self.get_position(child.start_offset(), source);
                        let content = format!(
                            "Duplicate type declaration detected and removed: {}. Duplicate declarations can lead to ambiguous behavior or parsing issues. Please ensure type declarations are unique.",
                            name
                        );
                        let warning = ParsingError::new(
                            ParserErrorKind::ParseWarning,
                            self.filename.as_deref().map(|s| s.to_string()), // Using `as_deref()` to avoid unwrap
                            line,
                            column,
                            content,
                        );
                        self.error_manager.add_error(warning);
                    } else {
                        // If `source` is None, return an error
                        return Err(ParserInternalError::new(
                            "Source string is missing".to_string(),
                        ));
                    }
                }
            } else {
                let error_message = format!(
                    "Expected an Ast of kind PrimitiveType, but found {}.",
                    child.kind()
                );
                return Err(ParserInternalError::new(error_message)); // Return error for unexpected kind
            }
        }

        ty.set_children(unique_children); // Replace original children with unique ones
        Ok(())
    }

    /// Creates a default type for a given AST element. This function assigns a type to an element
    /// based on its kind. Specifically, it assigns the type `primitive` to `AtomicFunctionSkeleton`
    /// elements with the value `number`, and the type `primitive` with the value `object` to all
    /// other elements.
    ///
    /// The function generates an `Ast` node of type `Type`, which contains a child `PrimitiveType`
    /// node. The `PrimitiveType` is either set to `number` or `object` based on the element's kind.
    /// The generated type is then wrapped in an `Ast` node and returned as a `Box<Ast>`.
    ///
    /// # Arguments
    ///
    /// * `element` - A reference to the `Ast` node representing the element for which the type is
    ///   being created. The type assigned depends on the kind of this node.
    /// * `start` - The start position of the element in the source text, used for accurate position
    ///   tracking.
    /// * `end` - The end position of the element in the source text, used for accurate position
    ///   tracking.
    ///
    /// # Returns
    ///
    /// A `Box<Ast>` representing the created `Type` node, which contains the `PrimitiveType` node
    /// as its child. The `PrimitiveType` is assigned a value based on the element's kind:
    /// - For `AtomicFunctionSkeleton`, the type is set to `number`.
    /// - For other types, the type is set to `object`.
    ///
    /// # Example
    /// ```rust
    /// let element = Ast::new(AstKind::AtomicFunctionSkeleton, Vec::new(), 0, 5);
    /// let start = 0;
    /// let end = 5;
    /// let default_type = checker.create_default_type(&element, start, end);
    /// ```
    fn create_default_type(&self, element: &Ast, start: usize, end: usize) -> Box<Ast> {
        let primitive_type_kind = match element.kind() {
            AstKind::AtomicFunctionSkeleton => AstKind::PrimitiveType(NUMBER_TYPE.to_string()),
            _ => AstKind::PrimitiveType(OBJECT_TYPE.to_string()),
        };
        let primitive_type = Box::new(Ast::new(primitive_type_kind, Vec::new(), start, end));
        let mut ty = Box::new(Ast::new(AstKind::Type, Vec::new(), start, end));
        ty.children_mut().push(primitive_type);
        ty
    }

    /// Converts a `ParseError` into a `ParsingError`.
    ///
    /// This function takes a `ParseError` and converts it into a `ParsingError`, which can be used
    /// for logging or error reporting. It formats the error message based on the type of `ParseError`
    /// encountered (e.g., unrecognized token, invalid token, etc.) and includes the source location and
    /// file path (if available).
    ///
    /// # Arguments
    /// * `error` - The `ParseError` to be converted, containing the error details.
    /// * `source` - The source code as a string, used to get the position of the error.
    /// * `file_path` - An optional `PathBuf` representing the file path of the source.
    ///
    /// # Returns
    /// A `ParsingError` with the formatted error message, type, and location.
    ///
    /// # Example
    /// ```
    /// let parser_error = self.to_parser_error(&parse_error, &source_code, Some(file_path));
    /// ```
    fn to_parser_error(
        &self,
        error: &ParseError<usize, Token, LexicalError>,
        source: &str,
        file_path: Option<&str>,
    ) -> ParsingError {
        let file_path = file_path.map(|s| s.to_string());
        match error {
            ParseError::UnrecognizedToken {
                token: (start, t, _end),
                expected,
            } => {
                let (line, column) = self.get_position(*start, source);
                let token = t.symbol().escape_debug().to_string();
                let content = format!("Unexpected token \"{}\". Expected: {:?}", token, expected);
                ParsingError::new(
                    ParserErrorKind::LexicalError,
                    file_path,
                    line,
                    column,
                    content,
                )
            }
            ParseError::InvalidToken { location } => {
                let (line, column) = self.get_position(*location, source);
                let content = "Invalid token".to_string();
                ParsingError::new(
                    ParserErrorKind::LexicalError,
                    file_path,
                    line,
                    column,
                    content,
                )
            }
            ParseError::User { error } => {
                let content = error.to_string();
                ParsingError::new(ParserErrorKind::LexicalError, file_path, 0, 0, content)
            }
            ParseError::UnrecognizedEof { location, expected } => {
                let (line, column) = self.get_position(*location, source);
                let content = format!("Unrecognized EOF. Expected: {:?}", expected);
                ParsingError::new(
                    ParserErrorKind::LexicalError,
                    file_path,
                    line,
                    column,
                    content,
                )
            }
            ParseError::ExtraToken {
                token: (start, t, _end),
            } => {
                let (line, column) = self.get_position(*start, source);
                let token = t.symbol().escape_debug().to_string();
                let content = format!("Extra token \"{}\" encountered.", token);
                ParsingError::new(
                    ParserErrorKind::LexicalError,
                    file_path,
                    line,
                    column,
                    content,
                )
            }
        }
    }
}

/// A fast line table for efficiently mapping byte offsets to line and column numbers.
/// It uses a coarse index to accelerate lookups.
///
/// # Fields
/// - `line_starts`: A vector storing the starting byte offset of each line.
/// - `coarse_index`: A vector storing precomputed offsets and their corresponding line numbers
///   at intervals of `k` lines to speed up lookups.
/// - `k`: The interval for the pre-index (determines how frequently the coarse index stores values).
struct FastLineTable {
    line_starts: Vec<usize>,
    coarse_index: Vec<(usize, usize)>, // (Offset, Line number) every K lines
}

impl FastLineTable {
    /// Constructs a new `FastLineTable` from a given source string.
    ///
    /// # Arguments
    /// - `source`: The input string whose line positions will be indexed.
    /// - `k`: The interval at which the coarse index stores line offsets.
    ///
    /// # Returns
    /// A new instance of `FastLineTable`.
    fn new(source: &str, k: usize) -> Self {
        let mut line_starts = vec![0]; // The first line always starts at offset 0
        let mut coarse_index = vec![];

        // Iterate through each byte in the source string
        for (i, b) in source.bytes().enumerate() {
            if b == b'\n' {
                let line_number = line_starts.len() + 1; // Compute the next line number
                line_starts.push(i + 1); // Store the offset of the next line

                // Store coarse index entry every `k` lines
                if line_number % k == 0 {
                    coarse_index.push((i + 1, line_number));
                }
            }
        }

        Self {
            line_starts,
            coarse_index,
        }
    }

    /// Retrieves the line and column number corresponding to a given byte offset.
    ///
    /// # Arguments
    /// - `offset`: The byte offset in the source string.
    ///
    /// # Returns
    /// A tuple `(line_number, column_number)`, where:
    /// - `line_number` is the 1-based index of the line.
    /// - `column_number` is the 1-based index of the column within the line.
    fn get_position(&self, offset: usize) -> (usize, usize) {
        // Fast lookup using the coarse index (binary search)
        let mut approx_line = match self
            .coarse_index
            .binary_search_by_key(&offset, |&(pos, _)| pos)
        {
            Ok(idx) => self.coarse_index[idx].1, // Exact match found
            Err(idx) => {
                if idx == 0 {
                    1 // If the offset is before the first indexed entry, start from line 1
                } else {
                    self.coarse_index[idx - 1].1 // Start from the nearest coarse index entry
                }
            }
        };

        // Fine-tune the search with a linear scan from the approximate starting point
        while approx_line < self.line_starts.len() && self.line_starts[approx_line] <= offset {
            approx_line += 1;
        }

        // Compute the column by subtracting the line start offset from the given offset
        let line_start = self.line_starts[approx_line - 1];
        (approx_line, offset - line_start + 1)
    }
}
