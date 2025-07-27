
use crate::aiplan4rust::core::arena::ArenaNode;
use crate::aiplan4rust::semantic::symbol::SymbolOrigin;
use crate::aiplan4rust::semantic::symbol::{Declaration, Scope, SymbolEntry,Usage};
use crate::aiplan4rust::semantic::symbol_table::{SymbolTableError, SymbolTableOrigin};
use crate::aiplan4rust::semantic::SymbolTable;
use crate::aiplan4rust::syntax::ast::{AstNode, Ast, AstKind};
use crate::aiplan4rust::lang::Type;
use crate::aiplan4rust::lang::TypedSymbol;
use crate::aiplan4rust::lang::TypedList;
use crate::aiplan4rust::syntax::tree::NodeRef;

/// `Comparator` is an enum that represents the different types of comparisons
/// that can be made between values, specifically for validating the number of children
/// in an Abstract Syntax Tree (AST) syntax.
///
/// This enum provides a variety of comparison operations that allow you to check:
/// - equality
/// - inequality
/// - relative magnitude (less than, greater than)
/// - inclusive comparisons (less than or equal, greater than or equal)
///
/// It is primarily used to validate the number of children a specific AST syntax should have
/// based on the expected length, ensuring that the AST structure follows the desired
/// constraints.
///
/// # Variants
/// - `Equal`: Checks if the value is equal to the expected value.
/// - `NotEqual`: Checks if the value is not equal to the expected value.
/// - `Less`: Checks if the value is less than the expected value.
/// - `Greater`: Checks if the value is greater than the expected value.
/// - `LessEq`: Checks if the value is less than or equal to the expected value.
/// - `GreaterEq`: Checks if the value is greater than or equal to the expected value.
///
/// `Comparator` makes it easy to express core comparison operations in a type_checker-safe manner.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
#[allow(dead_code)]
enum Comparator {
    Equal,
    NotEqual,
    Less,
    Greater,
    LessEq,    // Less than or equal
    GreaterEq, // Greater than or equal
}

/// A builder for constructing a [`Table`] from an abstract syntax arena (AST).
///
/// `SymbolTableBuilder` encapsulates the logic for traversing an [`ArenaAst`]
/// and populating a `SymbolTable` with symbols extracted from the arena. It
/// identifies the root syntax type_checker (e.g., Domain or Problem), determines the source
/// of the symbol table, and initializes it accordingly.
///
/// This pattern separates the concerns of AST traversal and symbol table
/// construction, allowing better testability and error handling.
///
/// # Example
///
/// ```rust
/// let mut builder = SymbolTableBuilder::new();
/// let symbol_table = builder.build(&ast)?;
/// ```
pub(crate) struct SymbolTableBuilder {
    table: SymbolTable,
}

impl SymbolTableBuilder {
    /// Creates a new `SymbolTableBuilder` with an empty symbol table.
    ///
    /// The internal table is initialized with [`SymbolSource::Unknown`] as the default source.
    /// The actual source (Domain or Problem) will be determined during the build process.
    pub fn new() -> Self {
        SymbolTableBuilder {
            table: SymbolTable::new(),
        }
    }

    /// Returns a shared reference to the internal symbol table.
    ///
    /// This is mainly used internally to inspect the current state during construction.
    fn table(&self) -> &SymbolTable {
        &self.table
    }

    /// Returns a mutable reference to the internal symbol table.
    ///
    /// This allows the table to be modified while inserting symbols during the build process.
    fn table_mut(&mut self) -> &mut SymbolTable {
        &mut self.table
    }

    /// Builds a complete [`SymbolTable`] from the given abstract syntax tree.
    ///
    /// This method:
    /// - Retrieves the root node of the AST.
    /// - Ensures that the root is either a `Domain` or a `Problem`.
    /// - Sets the symbol table's origin and root ID accordingly.
    /// - Initializes the symbol table by traversing the AST.
    ///
    /// # Arguments
    ///
    /// * `ast` - A reference to the [`Ast`] representing the parsed abstract syntax tree.
    ///
    /// # Errors
    ///
    /// Returns a [`SymbolTableError`] if:
    /// - The AST has no root node (`MissingRootNode`).
    /// - The root node is not a valid entry point (`UnexpectedAstKind`, expected `Domain` or `Problem`).
    /// - An error occurs during symbol initialization from the AST (`SymbolInitializationError`, etc.).
    ///
    /// # Returns
    ///
    /// A fully initialized [`SymbolTable`] on success.

    pub fn build(&mut self, ast: &Ast) -> Result<SymbolTable, SymbolTableError> {
        // Retrieve the root of the AST and handle the case where it is missing
        let root_ref = ast.arena().try_root_node_ref()?;

        let root_node = root_ref.node();

        // Determine the root kind and set the source of the symbol table
        match root_node.kind() {
            AstKind::Domain => {
                self.table_mut().set_origin(SymbolTableOrigin::Domain);
                self.table_mut().set_root_id(root_ref.id());
            }
            AstKind::Problem => {
                self.table_mut().set_origin(SymbolTableOrigin::Problem);
                self.table_mut().set_root_id(root_ref.id());
            }
            found => {
                return Err(SymbolTableError::unexpected_ast_kind(
                    root_ref.id(),
                    vec![AstKind::Domain, AstKind::Problem],
                    found,
                ));
            }
        }

        // Traverse the AST and initialize the symbol table
        self.initialize_from_ast(&root_ref, ast)?;

        // Return the constructed symbol table
        Ok(std::mem::take(&mut self.table))
    }


    /// Initializes the symbol table by processing nodes from the Abstract Syntax Tree (AST).
    ///
    /// This function recursively traverses the AST starting from the given `node_ref`,
    /// handling different kinds of AST nodes to populate the symbol table appropriately.
    /// Depending on the syntax kind, it may add symbol declarations, register symbol usages,
    /// or perform special handling for constructs such as actions and atomic formulas.
    ///
    /// # Arguments
    ///
    /// * `node_ref` - A reference to the AST syntax from which initialization begins.
    ///                This syntax acts as the root for the current scope of symbol processing.
    /// * `ast` - A reference to the entire AST (`ArenaAst`), used for syntax lookups and context.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` on success, or a [`AiplanError`] if an error occurs during
    /// symbol initialization, such as invalid syntax types or semantic errors.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// let mut builder = SymbolTableBuilder::new();
    /// let root_node_ref = ast.root_node_ref().unwrap();
    /// builder.initialize_from_ast(root_node_ref, &ast)?;
    /// ```
    fn initialize_from_ast(
        &mut self,
        node_ref: &NodeRef<AstNode>,
        ast: &Ast,
    ) -> Result<(), SymbolTableError> {
        let scope = Scope::new(node_ref.id(), None);
        self.init_from(node_ref, ast, scope)?;
        Ok(())
    }

    /// Recursively initializes the symbol table by processing AST nodes.
    ///
    /// This function examines the kind of the AST syntax referenced by `node_ref`
    /// and performs symbol table operations accordingly. It handles declarations,
    /// symbol usages, and special AST constructs such as actions, formulas, and HTN tasks.
    /// For unhandled syntax kinds, it recursively processes their child nodes.
    ///
    /// # Arguments
    ///
    /// * `node_ref` - The reference to the AST syntax currently being processed.
    /// * `ast` - The full abstract syntax arena (`ArenaAst`) that contains the syntax.
    /// * `scope` - The current scope, which manages symbol visibility and hierarchical
    ///   structure within the AST.
    ///
    /// # Behavior by AST syntax kind
    ///
    /// - `DomainName`, `ProblemName`: Add declaration symbols without further recursion.
    /// - `TypedList`: Initialize symbol entries with specialized typed list logic.
    /// - `PrimitiveType`, `Constant`, `Variable`: Register symbols as usages.
    /// - `ActionDef`, `DurativeActionDef`: Initialize action-related symbols.
    /// - `AtomicFormulaSkeleton`: Special symbol table handling for formula skeletons.
    /// - `AtomicFormula`, `FunctionTerm`: Recursively initialize atomic formulas and function terms.
    /// - `Forall`, `Exists`: Handle quantified expr and logical scopes.
    /// - `MethodDef`, `TaskDef`: Initialize HTN method and task definitions.
    /// - `Task`, `TaggedTask`: Handle HTN individual tasks and tagged tasks.
    /// - `TaskOrderingConstraint`: Initialize constraints between HTN tasks.
    /// - Other nodes: Recursively process all child nodes.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if all nodes and symbols are successfully processed, or
    /// a `ParserInternalError` if an error occurs at any step.
    ///
    /// # Errors
    ///
    /// This method returns an error if symbol registration or syntax processing fails.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// let mut builder = SymbolTableBuilder::new();
    /// builder.init_from(root_node_ref, &ast, initial_scope)?;
    /// ```
    fn init_from(
        &mut self,
        node_ref: &NodeRef<AstNode>,
        ast: &Ast,
        scope: Scope,
    ) -> Result<(), SymbolTableError> {
        // Determine the type_checker of the AST syntax and apply appropriate processing
        match node_ref.node().kind() {
            // Handle declarations: These simply register symbols without additional processing
            AstKind::DomainName
            | AstKind::ProblemName
            => {
                self.add_declaration_symbol(node_ref, ast, scope.clone(), None, None)?;
            }

            // Handle typed lists: Requires specialized initialization logic
            AstKind::TypedList => {
                self.init_from_typed_list(node_ref, ast, scope.clone())?;
            }

            // Handle primitive types, constants, and variables: Register them as symbol usages
            AstKind::PrimitiveType
            | AstKind::Constant
            | AstKind::Variable => {
                self.add_symbol_usage(node_ref, ast, scope.clone())?;
            }

            // Handle action definitions
            AstKind::ActionDef => {
                self.init_from_action_def(node_ref, ast, scope.clone())?;
            }

            // Handle durative actions, which include timing constraints
            AstKind::DurativeActionDef => {
                self.init_from_durative_action_def(node_ref, ast, scope.clone())?;
            }

            // Handle atomic formula skeletons: Requires custom symbol table handling
            AstKind::AtomicFormulaSkeleton => {
                self.init_from_atomic_formula_skeleton(node_ref, ast, scope.clone())?;
            }

            // Handle atomic formulas and function terms: These require recursive processing
            AstKind::AtomicFormula | AstKind::FunctionTerm => {
                self.init_from_atomic_formula(node_ref, ast, scope.clone())?;
            }

            // Handle quantified expr (`Forall` and `Exists`): Need special treatment for
            // logical scopes
            AstKind::Forall | AstKind::Exists => {
                self.init_from_quantified_expression(node_ref, ast, scope.clone())?;
            }

            // Handle hierarchical task network (HTN) method definitions
            AstKind::MethodDef => {
                self.init_from_method_def(node_ref, ast, scope.clone())?;
            }

            // Handle task definitions in HTN planning
            AstKind::TaskDef => {
                self.init_from_task_def(node_ref, ast, scope.clone())?;
            }

            // Handle individual task references in HTN planning
            AstKind::Task => {
                self.init_from_atomic_formula(node_ref, ast, scope.clone())?;
            }

            // Handle tagged tasks, which include additional metadata in HTN planning
            AstKind::TaggedTask => {
                self.init_from_tagged_task(node_ref, ast, scope.clone())?;
            }

            // Handle task ordering constraints in HTN planning
            AstKind::TaskOrderingConstraint => {
                self.init_from_task_ordering_constraint(node_ref, ast, scope.clone())?;
            }

            // Default case: If the AST syntax is not explicitly handled, process its children
            // recursively
            _ => {
                for child in node_ref.node().children() {
                    self.init_from(&ast.arena().try_node_ref(*child)?, ast, scope.clone())?;
                }
            }
        }

        // Successfully completed processing the AST syntax
        Ok(())
    }

    /// Adds a new symbol declaration to the symbol table.
    ///
    /// This function extracts the symbol's name and kind from the given AST syntax by
    /// using `try_symbol_ref`. It first asserts that the AST syntax's kind is valid for a
    /// declaration using `assert_ast_kind`. Then, it either updates an existing symbol
    /// in the symbol table by adding a new declaration or creates a new symbol with
    /// the declaration and inserts it into the table.
    ///
    /// # Arguments
    ///
    /// * `node_ref` - A reference to the AST syntax representing the symbol declaration.
    /// * `ast` - The abstract syntax arena (`ArenaAst`) containing the syntax.
    /// * `scope` - The current scope in which the symbol is declared, affecting visibility.
    /// * `types` - Optional vector of identifier types associated with the symbol declaration.
    /// * `arguments` - Optional vector of typed symbols representing the symbol's arguments.
    ///
    /// # Returns
    ///
    /// * `Ok(())` - When the symbol declaration is successfully added to the symbol table.
    /// * `Err(ParserInternalError)` - If the AST syntax kind is invalid, symbol extraction fails,
    ///   or insertion into the table encounters an error.
    ///
    /// # Panics
    ///
    /// This function will return an error if the AST syntax kind does not match expected
    /// declaration kinds.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// let mut builder = SymbolTableBuilder::new();
    /// let node_ref = ast.try_node_ref(node_id)?;
    /// builder.add_declaration_symbol(node_ref, &ast, current_scope, None, None)?;
    /// ```
    fn add_declaration_symbol(
        &mut self,
        node_ref: &NodeRef<AstNode>,
        ast: &Ast,
        scope: Scope,
        types: Option<Type>,
        arguments: Option<TypedList>,
    ) -> Result<(), SymbolTableError> {

        // Extract the symbol information from the AST
        let symbol_ref = ast.arena().try_symbol_ref(node_ref.id())?;
        let ident = symbol_ref.ident();

        // Check if the symbol is already in the symbol table and add a declaration
        let origin = SymbolOrigin::from(self.table().origin());
        if let Some(symbol) = self.table_mut().get_symbol_mut(ident) {
            let declaration =
                Declaration::new(symbol_ref, scope, origin, types, arguments, node_ref.node().span().clone(), node_ref.id(), None);
            symbol.add_declaration(declaration);
        } else {
            // Create a new symbol and add the declaration to it
            let mut symbol = SymbolEntry::new(ident);
            let declaration =
                Declaration::new(symbol_ref, scope, origin, types, arguments, node_ref.node().span().clone(), node_ref.id(), None);
            symbol.add_declaration(declaration);
            self.table_mut().insert_symbol(ident, symbol); // Insert the new symbol into the table
        }

        Ok(())
    }

    /// Adds the usage of a symbol found in the given AST syntax to the symbol table.
    ///
    /// This function processes AST nodes representing symbol usages, such as domain names,
    /// problem names, constants, variables, atomic formulas, function terms, and tasks.
    /// It first validates the AST syntax kind to ensure it is appropriate for symbol usage.
    /// For certain syntax kinds that represent complex expr (e.g., atomic formulas),
    /// it also verifies the presence of at least one child syntax.
    ///
    /// The symbol's identifier and kind are extracted, and the function then updates
    /// the symbol table: if the symbol already exists, the usage information is appended;
    /// otherwise, a new symbol entry is created with the usage data.
    ///
    /// # Arguments
    ///
    /// * `node_ref` - A reference to the AST syntax representing the symbol usage.
    /// * `ast` - The full abstract syntax arena (`ArenaAst`) containing the syntax.
    /// * `scope` - The scope context in which the symbol is used, which controls visibility.
    ///
    /// # Returns
    ///
    /// * `Ok(())` - Indicates the symbol usage was successfully recorded.
    /// * `Err(ParserInternalError)` - Returned if the AST syntax kind is invalid, the syntax’s
    ///   structure is incorrect (e.g., missing children), or if symbol extraction fails.
    ///
    /// # Panics
    ///
    /// The function will return an error if the AST syntax kind does not match any of the
    /// expected kinds or if required children are missing.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// let mut builder = SymbolTableBuilder::new();
    /// let node_ref = ast.try_node_ref(node_id)?;
    /// builder.add_symbol_usage(node_ref, &ast, current_scope)?;
    /// ```
    fn add_symbol_usage(
        &mut self,
        node_ref: &NodeRef<AstNode>,
        ast: &Ast,
        scope: Scope,
    ) -> Result<(), SymbolTableError> {
        let symbol_ref = if matches!(
            node_ref.node().kind(),
            AstKind::AtomicFormula | AstKind::FunctionTerm | AstKind::Task
        ) {
            let first_node_ref = ast.arena().try_node_ref(node_ref.node().children()[0])?;
            ast.arena().try_symbol_ref(first_node_ref.id())?
        } else {
            ast.arena().try_symbol_ref(node_ref.id())?
        };

        // Extract symbol name and type_checker based on the AST syntax's kind.
        //let symbol_ref= ast.try_symbol_ref(node_ref.id())?;
        let ident = symbol_ref.ident();

        // If the symbol exists, add the usage; otherwise, create a new symbol.
        let origin = SymbolOrigin::from(self.table().origin());
        if let Some(symbol) = self.table_mut().get_symbol_mut(ident) {
            let usage = Usage::new(symbol_ref, scope, origin, node_ref.node().span().clone(), node_ref.id());
            symbol.add_usage(usage);
        } else {
            let mut symbol = SymbolEntry::new(ident);
            let usage = Usage::new(symbol_ref, scope, origin, node_ref.node().span().clone(), node_ref.id());
            symbol.add_usage(usage);
            self.table_mut().insert_symbol(ident, symbol);
        }
        Ok(())
    }

    /// Initializes the symbol table from a `TypedList` AST syntax.
    ///
    /// This function processes a `TypedList` syntax in the AST by iterating over its children,
    /// which represent typed elements such as types, constants, variables, or other symbols.
    /// It handles the syntax structure by:
    /// - Returning early if there are no children (nothing to process).
    /// - Recursively processing each child syntax as a typed item.
    ///
    /// # Arguments
    ///
    /// * `node_ref` - A reference to the `TypedList` AST syntax to process.
    /// * `ast` - The full abstract syntax arena (`ArenaAst`) containing the syntax.
    /// * `scope` - The current scope within which the symbols are being initialized.
    ///
    /// # Returns
    ///
    /// * `Ok(())` if all children are successfully processed and the symbol table is updated.
    /// * `Err(ParserInternalError)` if the syntax is not of the expected kind or if any child syntax
    ///   fails to process correctly.
    ///
    /// # Errors
    ///
    /// Returns an error if the AST syntax kind is not `TypedList` or if any of the child nodes cannot
    /// be processed as a typed item.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// let typed_list_node = ast.try_node_ref(node_id)?;
    /// builder.init_from_typed_list(typed_list_node, &ast, current_scope)?;
    /// ```
    fn init_from_typed_list(
        &mut self,
        node_ref: &NodeRef<AstNode>,
        ast: &Ast,
        scope: Scope,
    ) -> Result<(), SymbolTableError> {

        let children = node_ref.node().children();

        if children.is_empty() {
            return Ok(());
        }

        for typed_item in children.iter() {
            self.init_from_typed_item(&ast.arena().try_node_ref(*typed_item)?, ast, scope.clone())?;
        }

        Ok(())
    }

    /// Initializes symbol declarations from a `TypedItem` syntax syntax.
    ///
    /// A `TypedItem` syntax typically represents a declaration where a list of symbols (e.g.,
    /// constants or variables) is associated with a type_checker (e.g., `?x ?y - location`). This function
    /// parses both the symbol list and the type_checker annotation and registers the symbols in the
    /// internal symbol table.
    ///
    /// # Parameters
    ///
    /// - `node_ref`: A reference to the `TypedItem` AST syntax to process.
    /// - `ast`: The full abstract syntax arena (`ArenaAst`) containing the syntax.
    /// - `scope`: The current scope in which the symbols are declared. This is cloned as needed
    ///   to maintain correct scoping during recursive calls.
    ///
    /// # Behavior
    ///
    /// - Verifies that the syntax kind is `TypedItem`.
    /// - Extracts the children of the syntax:
    ///   - If there is only one child, treats it as an untyped declaration (empty type_checker list).
    ///   - If there are exactly two children, parses the second child to extract the associated
    ///     types.
    ///   - Returns an error if the number of children is anything other than 1 or 2.
    /// - Processes the first child as the list of symbols to be declared, associating them with the
    ///   extracted types.
    /// - Adds each symbol to the symbol table with the given scope and types.
    ///
    /// # Errors
    ///
    /// Returns `ParserInternalError` if:
    /// - The syntax is not of kind `TypedItem`.
    /// - The number of children is invalid (not 1 or 2).
    /// - Parsing the type_checker annotation fails.
    /// - Adding the declarations to the symbol table fails.
    ///
    /// # Example
    ///
    /// ```text
    /// (?x ?y - location)  // symbols: ?x, ?y; type_checker: location
    /// (?z)                // symbol: ?z; no associated type_checker
    /// ```
    fn init_from_typed_item(
        &mut self,
        node_ref: &NodeRef<AstNode>,
        ast: &Ast,
        scope: Scope,
    ) -> Result<(), SymbolTableError> {

        let syntax_tree = ast.arena();
        let node = node_ref.node();

        // Match on the number of children to extract the types or fallback to an empty vector
        let types = match node.children().len() {
            1 => Type::new(),
            2 => {
                let ty_id = node.try_child(1)?;
                self.init_from_type(&syntax_tree.try_node_ref(ty_id)?, ast, scope.clone())?
            },
            n => {
                return Err(SymbolTableError::invalid_typed_item_arity(
                    node_ref.id(),
                    n,
                ));
            }
        };
        // Process the first element of the pair
        let elt_id = node.try_child(0)?;
        self.init_from_typed_item_elements(&syntax_tree.try_node_ref(elt_id)?, ast, scope, types)
    }

    /// Helper function to process an individual element of the `TypedList`.
    ///
    /// This function adds the element to the symbol table based on its type_checker, such as
    /// `PrimitiveType`, `Constant`, `Variable`, or handles nested elements like
    /// `AtomicFunctionSkeleton`.
    ///
    /// # Parameters
    ///
    /// - `node_ref`: The AST syntax representing the element to process. This syntax is expected
    ///   to be one of the accepted kinds (e.g., `PrimitiveType`, `Constant`, `Variable`, or `AtomicFunctionSkeleton`).
    /// - `ast`: The full abstract syntax arena (`ArenaAst`) that contains the syntax.
    /// - `scope`: The current scope in which the symbol is being declared or used.
    /// - `types`: A vector of type_checker identifiers associated with the element.
    ///
    /// # Behavior
    ///
    /// - Validates that the AST syntax kind is one of the expected types.
    /// - If the element is a constant, variable, or primitive type_checker, it adds a declaration symbol
    ///   to the symbol table with the provided types.
    /// - If the element is an atomic function skeleton, it recursively initializes it accordingly.
    /// - The function assumes all other syntax kinds are invalid and will panic if encountered.
    ///
    /// # Returns
    ///
    /// - Returns `Ok(())` on successful processing.
    /// - Returns an error if the AST syntax kind is invalid or any symbol table operation fails.
    fn init_from_typed_item_elements(
        &mut self,
        node_ref: &NodeRef<AstNode>,
        ast: &Ast,
        scope: Scope,
        types: Type,
    ) -> Result<(), SymbolTableError> {
        // Ensure the node_ref is of a valid kind for typed item elements
        match node_ref.node().kind() {
            AstKind::PrimitiveType | AstKind::Constant | AstKind::Variable => {
                // For constants and variables, add a declaration symbol with the provided types
                self.add_declaration_symbol(node_ref, ast, scope.clone(), Some(types.clone()), None)?;
            }

            AstKind::AtomicFunctionSkeleton => {
                // Recursively initialize atomic function skeleton elements
                self.init_from_atomic_function_skeleton(node_ref, ast, scope.clone(), types.clone())?;
            }

            found => {
                // Return a structured error instead of unreachable panic
                return Err(SymbolTableError::unexpected_ast_kind(
                    node_ref.id(),
                    vec![
                        AstKind::PrimitiveType,
                        AstKind::Constant,
                        AstKind::Variable,
                        AstKind::AtomicFunctionSkeleton,
                    ],
                    found,
                ));
            }
        }

        Ok(())
    }

    /// Initializes the symbol table for an atomic function skeleton.
    ///
    /// This function processes an AST syntax of kind `AtomicFunctionSkeleton` by performing several key steps:
    /// - Verifies that the AST syntax is of the correct kind (`AtomicFunctionSkeleton`).
    /// - Checks that the syntax has at least two children: the function symbol and the list of arguments.
    /// - Validates that the first child is a `FunctionSymbol` syntax.
    /// - Creates a new nested scope for the function and initializes symbols for its arguments.
    /// - Extracts the arguments and computes their arity.
    /// - Adds a function declaration to the symbol table with the associated types and arguments.
    ///
    /// # Parameters
    /// - `node_ref`: The AST syntax reference corresponding to the `AtomicFunctionSkeleton`.
    /// - `ast`: Reference to the complete AST (`ArenaAst`) containing the syntax.
    /// - `scope`: The current scope in which the function is declared; used for symbol resolution.
    /// - `types`: A vector of type_checker identifiers associated with the function.
    ///
    /// # Returns
    /// - `Ok(())` on successful processing and symbol table update.
    /// - `Err(ParserInternalError)` if any validation or symbol table operation fails, such as:
    ///   - The syntax is not an `AtomicFunctionSkeleton`.
    ///   - The syntax has fewer than two children.
    ///   - The first child is not a `FunctionSymbol`.
    ///
    /// # Example
    /// ```rust
    /// let ast_node = /* obtain AtomicFunctionSkeleton syntax reference */;
    /// let current_scope = /* current scope context */;
    /// let types = vec!["int".to_string(), "bool".to_string()];
    ///
    /// let result = symbol_table.init_from_atomic_function_skeleton(ast_node, &ast, current_scope, types);
    /// match result {
    ///     Ok(()) => println!("Function initialized successfully"),
    ///     Err(e) => eprintln!("Error initializing function: {:?}", e),
    /// }
    /// ```
    ///
    /// # Error Handling
    /// The function returns a `ParserInternalError` describing any encountered issue, such as unexpected
    /// AST syntax kinds or structural problems in the `AtomicFunctionSkeleton` syntax.
    ///
    /// # Notes
    /// - Assumes `init_from_typed_list` correctly initializes and validates argument types.
    /// - This function is integral to managing function symbol declarations and their scopes
    ///   within the symbol table system.
    fn init_from_atomic_function_skeleton(
        &mut self,
        node_ref: &NodeRef<AstNode>,
        ast: &Ast,
        scope: Scope,
        types: Type,
    ) -> Result<(), SymbolTableError> {
        let node = node_ref.node();

        // Retrieve the first child and validate it as a 'FunctionSymbol'
        let functor_id = node.try_child(0)?;
        let functor_ref = ast.arena().try_node_ref(functor_id)?;
        if functor_ref.node().kind() != AstKind::FunctionSymbol {
            return Err(SymbolTableError::unexpected_ast_kind(
                functor_ref.id(),
                vec![AstKind::FunctionSymbol],
                functor_ref.node().kind(),
            ));
        }

        // Retrieve and process the arguments list
        let arguments_id = node.try_child(1)?;
        let arguments = &ast.arena().try_node_ref(arguments_id)?;

        // Initialize the symbol table for the arguments
        self.init_from_typed_list(
            arguments,
            ast,
            Scope::new(node_ref.id(), Some(&scope)),
        )?;

        // Extract the arguments and calculate the arity
        let arguments = self.extract_arguments_from_typed_list(arguments, ast)?;

        // Add the declaration to the symbol table
        self.add_declaration_symbol(
            &functor_ref,
            ast,
            scope.clone(),
            Some(types),
            Some(arguments),
        )?;

        Ok(())
    }


    /// Initializes the symbol table for an action definition.
    ///
    /// This function processes an AST syntax representing an action or method definition, which can be
    /// one of `ActionDef`, `DurativeActionDef`, or `MethodDef`. It extracts the action/method name,
    /// parameters, and body, adding the appropriate symbols to the symbol table.
    ///
    /// # Parameters
    /// - `node_ref`: The AST syntax reference corresponding to the action definition.
    /// - `ast`: Reference to the AST (`ArenaAst`) containing the syntax.
    /// - `scope`: The current scope in which the action or method is defined.
    ///
    /// # Returns
    /// - `Ok(())` if the initialization completes successfully.
    /// - `Err(ParserInternalError)` if the AST syntax does not have the expected structure or kind.
    ///
    /// # AST Structure
    /// The syntax must have exactly three children:
    /// 1. **Name**: The identifier of the action or method, added as a declaration symbol.
    /// 2. **Parameters**: The parameter list, which is recursively processed.
    /// 3. **Body**: The body of the action or method, which is recursively processed.
    ///
    /// # Example
    /// ```rust
    /// // Assuming `node_ref`, `ast`, and `scope` are correctly initialized:
    /// symbol_table.init_from_action_def(node_ref, &ast, scope)?;
    /// ```
    ///
    /// This function delegates the main work to `init_from_def`, passing in the expected syntax kinds,
    /// expected number of children, and a flag indicating the presence of a body.
    fn init_from_action_def(
        &mut self,
        node_ref: &NodeRef<AstNode>,
        ast: &Ast,
        scope: Scope,
    ) -> Result<(), SymbolTableError> {
        self.init_from_def(
            node_ref,
            ast,
            scope,
            &[
                AstKind::ActionDef,
                AstKind::DurativeActionDef,
                AstKind::MethodDef,
            ],
            3,    // ActionDef has 3 children (name, parameters, body)
            true, // It has a body
        )
    }

    /// Initializes the symbol table for a method definition.
    ///
    /// This function verifies that the given AST syntax is of kind `MethodDef` and contains exactly
    /// three children representing the method name, its parameters, and its body. It then processes
    /// these components to update the symbol table accordingly.
    ///
    /// # Parameters
    /// - `node_ref`: The AST syntax reference corresponding to the method definition.
    /// - `ast`: Reference to the AST (`ArenaAst`) that contains the syntax.
    /// - `scope`: The current scope in which the method is defined.
    ///
    /// # Returns
    /// - `Ok(())` if the initialization completes successfully.
    /// - `Err(ParserInternalError)` if the syntax kind or structure is invalid.
    ///
    /// # AST Structure
    /// The syntax is expected to have three children:
    /// 1. **Name**: The method's identifier, added as a declaration symbol.
    /// 2. **Parameters**: The method parameters, recursively processed.
    /// 3. **Body**: The method body, recursively processed.
    ///
    /// # Example
    /// ```rust
    /// // Assuming `node_ref`, `ast`, and `scope` are initialized:
    /// symbol_table.init_from_method_def(node_ref, &ast, scope)?;
    /// ```
    ///
    /// This function delegates the main processing to `init_from_def`.
    fn init_from_method_def(
        &mut self,
        node_ref: &NodeRef<AstNode>,
        ast: &Ast,
        scope: Scope,
    ) -> Result<(), SymbolTableError> {
        self.init_from_def(
            node_ref,
            ast,
            scope,
            &[AstKind::MethodDef],
            3,    // MethodDef has 3 children (name, parameters, body)
            true, // It has a body
        )
    }

    /// Initializes the symbol table for a durative action definition.
    ///
    /// This function verifies that the given AST syntax is of kind `DurativeActionDef` and contains
    /// exactly three children representing the action's name, parameters, and body. It processes
    /// these components to update the symbol table accordingly.
    ///
    /// # Parameters
    /// - `node_ref`: The AST syntax reference corresponding to the durative action definition.
    /// - `ast`: Reference to the AST (`ArenaAst`) containing the syntax.
    /// - `scope`: The current scope in which the durative action is defined.
    ///
    /// # Returns
    /// - `Ok(())` if the initialization completes successfully.
    /// - `Err(ParserInternalError)` if the syntax kind or structure is invalid.
    ///
    /// # AST Structure
    /// The syntax is expected to have three children:
    /// 1. **Name**: The durative action's identifier, added as a declaration symbol.
    /// 2. **Parameters**: The action parameters, recursively processed.
    /// 3. **Body**: The action body, recursively processed.
    ///
    /// # Example
    /// ```rust
    /// // Assuming `node_ref`, `ast`, and `scope` are initialized:
    /// symbol_table.init_from_durative_action_def(node_ref, &ast, scope)?;
    /// ```
    ///
    /// This function delegates the main processing to `init_from_def`.
    fn init_from_durative_action_def(
        &mut self,
        node_ref: &NodeRef<AstNode>,
        ast: &Ast,
        scope: Scope,
    ) -> Result<(), SymbolTableError> {
        self.init_from_def(
            node_ref,
            ast,
            scope,
            &[AstKind::DurativeActionDef],
            3,    // DurativeActionDef has 3 children (name, parameters, body)
            true, // It has a body
        )
    }

    /// Initializes the symbol table for a task definition.
    ///
    /// This function validates that the given AST syntax is of kind `TaskDef` and contains exactly
    /// two children: the task name and its parameters. Since task definitions do not have a body,
    /// only these components are processed and added to the symbol table.
    ///
    /// # Parameters
    /// - `node_ref`: The AST syntax reference corresponding to the task definition.
    /// - `ast`: Reference to the AST (`ArenaAst`) containing the syntax.
    /// - `scope`: The current scope in which the task is defined.
    ///
    /// # Returns
    /// - `Ok(())` if the initialization completes successfully.
    /// - `Err(ParserInternalError)` if the syntax kind or structure is invalid.
    ///
    /// # AST Structure
    /// The syntax is expected to have two children:
    /// 1. **Name**: The task's identifier, added as a declaration symbol.
    /// 2. **Parameters**: The task parameters, recursively processed.
    ///
    /// # Example
    /// ```rust
    /// // Assuming `node_ref`, `ast`, and `scope` are initialized:
    /// symbol_table.init_from_task_def(node_ref, &ast, scope)?;
    /// ```
    ///
    /// This function delegates the main processing to `init_from_def`.
    fn init_from_task_def(
        &mut self,
        node_ref: &NodeRef<AstNode>,
        ast: &Ast,
        scope: Scope,
    ) -> Result<(), SymbolTableError> {
        self.init_from_def(
            node_ref,
            ast,
            scope,
            &[AstKind::TaskDef],
            2,     // TaskDef has only 2 children (name, parameters)
            false, // No body for tasks
        )
    }

    /// Initializes a definition syntax (`ActionDef`, `DurativeActionDef`, `MethodDef`, or `TaskDef`)
    /// by extracting its name, parameters, and optionally its body, updating the symbol table accordingly.
    ///
    /// This function performs the following steps:
    /// - Validates that the AST syntax is one of the expected kinds.
    /// - Confirms the syntax has the expected number of children.
    /// - Extracts the definition name and adds it as a declaration in the current scope.
    /// - Recursively processes the parameter list (`TypedList`), adding parameter symbols.
    /// - If the definition has a body (e.g., actions and methods), recursively processes the body.
    ///
    /// # Parameters
    /// - `node_ref`: Reference to the AST syntax representing the definition.
    /// - `ast`: The AST arena containing all nodes.
    /// - `scope`: The current symbol table scope for resolving symbols and declarations.
    /// - `valid_kinds`: Slice of valid AST kinds that this function can process.
    /// - `expected_children`: The exact number of child nodes expected (2 for tasks, 3 for actions/methods).
    /// - `has_body`: Indicates if the definition syntax includes a body that needs processing.
    ///
    /// # Returns
    /// - `Ok(())` if initialization completes successfully.
    /// - `Err(ParserInternalError)` if validation fails or processing encounters an unexpected AST structure.
    ///
    /// # AST Structure
    /// The syntax's children are expected as follows:
    /// 1. **Name** — the identifier of the definition.
    /// 2. **Parameters** — a `TypedList` syntax representing parameters.
    /// 3. **Body** (optional) — the body of the definition (only if `has_body` is true).
    ///
    /// # Example
    /// ```rust
    /// symbol_table.init_from_def(
    ///     node_ref,
    ///     &ast,
    ///     scope,
    ///     &[AstKind::ActionDef, AstKind::MethodDef],
    ///     3,
    ///     true,
    /// )?;
    /// ```
    fn init_from_def(
        &mut self,
        node_ref: &NodeRef<AstNode>,
        ast: &Ast,
        scope: Scope,
        _valid_kinds: &[AstKind],
        _expected_children: usize,
        has_body: bool,
    ) -> Result<(), SymbolTableError> {

        let syntax_tree = ast.arena();
        let node = node_ref.node();

        // First child: definition name, add to symbol table
        let name_id = node.try_child(0)?;
        let name = syntax_tree.try_node_ref(name_id)?;

        // Second child: parameters, recursively initialize the symbol table
        let parameters_def_id = node.try_child(1)?;
        let parameters_def = &syntax_tree.try_node_ref(parameters_def_id)?;
        let parameters_id = parameters_def.node().try_child(0)?;
        let parameters = &syntax_tree.try_node_ref(parameters_id)?;
        self.init_from_typed_list(
            parameters,
            ast,
            Scope::new(node_ref.id(), Some(&scope)),
        )?;

        let parameters =
            self.extract_arguments_from_typed_list(parameters, ast)?;

        self.add_declaration_symbol(
            &name,
            ast,
            scope.clone(),
            None,
            Some(parameters),
        )?;

        // Third child: body (if applicable)
        if has_body {
            let body_id = node.try_child(2)?;
            let body = &syntax_tree.try_node_ref(body_id)?;
            self.init_from(
                body,
                ast,
                Scope::new(node_ref.id(), Some(&scope)),
            )?;
        }

        Ok(())
    }

    /// Initializes the syntax state from an `AtomicFormula`, `FunctionTerm`, or `Task` AST syntax.
    ///
    /// This function processes an AST syntax expected to represent either an atomic formula,
    /// a function term, or a task. It verifies that the syntax has at least one child (the symbol),
    /// registers the usage of that symbol, then recursively processes all children as arguments.
    ///
    /// # Parameters
    /// - `node_ref`: Reference to the AST syntax representing the atomic formula or function term.
    /// - `ast`: The AST arena containing all nodes.
    /// - `scope`: The current scope used for symbol resolution and symbol usage registration.
    ///
    /// # Returns
    /// - `Ok(())` if the syntax and its children are successfully processed.
    /// - `Err(ParserInternalError)` if the syntax kind is invalid, lacks children, or if recursive processing fails.
    ///
    /// # Errors
    /// Returns an error if:
    /// - The AST syntax kind is not one of `AtomicFormula`, `FunctionTerm`, or `Task`.
    /// - The syntax has no children (at least one child is expected as the symbol).
    /// - Any recursive call to `init_from` returns an error.
    ///
    /// # Example
    /// ```rust
    /// symbol_table.init_from_atomic_formula(node_ref, &ast, scope)?;
    /// ```
    fn init_from_atomic_formula(
        &mut self,
        node_ref: &NodeRef<AstNode>,
        ast: &Ast,
        scope: Scope,
    ) -> Result<(), SymbolTableError> {
        // Retrieve and register the first child (symbol)
        self.add_symbol_usage(node_ref, ast, scope.clone())?; // should be removed

        // Process the remaining children (arguments)
        for child in node_ref.node().children() {
            self.init_from(&ast.arena().try_node_ref(*child)?, ast, scope.clone())?;
        }

        Ok(())
    }

    /// Initializes the symbol table for a quantified expr (`Exists` or `Forall`) in the AST.
    ///
    /// This function verifies that the AST syntax is a quantified expr of kind
    /// `Exists` or `Forall` and that it has exactly two children:
    /// 1. A variable declaration (or typed list)
    /// 2. An inner expr.
    ///
    /// It then recursively initializes the symbol table for both the variable declarations and
    /// the inner expr, creating a new nested scope for these initializations.
    ///
    /// # Parameters
    /// - `node_ref`: Reference to the AST syntax representing the quantified expr.
    /// - `ast`: The AST arena containing all nodes.
    /// - `scope`: The current scope in which the quantified expr resides.
    ///
    /// # Returns
    /// - `Ok(())` if the symbol table initialization succeeds.
    /// - `Err(ParserInternalError)` if the syntax is not a quantified expr, if
    ///   the number of children is incorrect, or if initialization of children fails.
    ///
    /// # Errors
    /// Returns a `ParserInternalError` if:
    /// - The syntax kind is not `Exists` or `Forall`.
    /// - The syntax does not have exactly two children.
    /// - Initialization of the typed list or inner expr fails.
    ///
    /// # Example
    /// ```rust
    /// let result = symbol_table.init_from_quantified_expression(node_ref, &ast, scope);
    /// match result {
    ///     Ok(_) => println!("Quantified expr processed successfully"),
    ///     Err(e) => eprintln!("Error initializing quantified expr: {}", e),
    /// }
    /// ```
    fn init_from_quantified_expression(
        &mut self,
        node_ref: &NodeRef<AstNode>,
        ast: &Ast,
        scope: Scope,
    ) -> Result<(), SymbolTableError> {

        let syntax_tree = ast.arena();
        let node = node_ref.node();

        // Retrieve the children (variables and inner expr)
        let variables_id = node.try_child(0)?;
        let variables = &syntax_tree.try_node_ref(variables_id)?;
        let expression_id = node.try_child(1)?;
        let expression = &syntax_tree.try_node_ref(expression_id)?;

        // Initialize the symbol table for the variables (first child)
        self.init_from_typed_list(
            variables,
            ast,
            Scope::new(node_ref.id(), Some(&scope)),
        )?;

        // Initialize the symbol table for the inner expr (second child)
        self.init_from(
            expression,
            ast,
            Scope::new(node_ref.id(), Some(&scope)),
        )?;

        Ok(())
    }

    /// Initializes the symbol table for an atomic formula skeleton in the AST.
    ///
    /// This function verifies that the AST syntax has the correct structure for an atomic formula skeleton,
    /// which consists of a predicate followed by its arguments. Specifically, it ensures:
    /// 1. The syntax has exactly two children: a predicate and an argument list.
    /// 2. The first child is of kind `Predicate`.
    ///
    /// Then, it recursively initializes the symbol table for the argument list, extracts the arguments,
    /// and adds the predicate declaration with its arguments to the symbol table.
    ///
    /// # Parameters
    /// - `node_ref`: A reference to the AST syntax representing the atomic formula skeleton.
    /// - `ast`: The arena containing all AST nodes.
    /// - `scope`: The current scope for symbol resolution.
    ///
    /// # Returns
    /// - `Ok(())` if initialization is successful.
    /// - `Err(ParserInternalError)` if the syntax does not meet expected structure or if processing fails.
    ///
    /// # Example
    /// ```rust
    /// let result = symbol_table.init_from_atomic_formula_skeleton(node_ref, &ast, scope);
    /// match result {
    ///     Ok(_) => println!("Symbol table initialized successfully"),
    ///     Err(e) => eprintln!("Error initializing atomic formula skeleton: {}", e),
    /// }
    /// ```
    fn init_from_atomic_formula_skeleton(
        &mut self,
        node_ref: &NodeRef<AstNode>,
        ast: &Ast,
        scope: Scope,
    ) -> Result<(), SymbolTableError> {

        let syntax_tree = ast.arena();
        let node = node_ref.node();

        // Ensure the first child is of kind 'Predicate'
        let predicate_id = node.try_child(0)?;
        let predicate = &syntax_tree.try_node_ref(predicate_id)?;

        // Retrieve and process arguments
        let arguments_id = node.try_child(1)?;
        let arguments = &syntax_tree.try_node_ref(arguments_id)?;
        self.init_from_typed_list(
            arguments,
            ast,
            Scope::new(node_ref.id(), Some(&scope)),
        )?;

        let arguments =
            self.extract_arguments_from_typed_list(arguments, ast)?;
        self.add_declaration_symbol(
            predicate,
            ast,
            scope.clone(),
            None,
            Some(arguments),
        )?;

        Ok(())
    }

    /// Extracts arguments from a `TypedList` AST syntax, which consists of a list of typed symbols.
    ///
    /// This function processes the AST syntax representing a `TypedList`. A `TypedList` contains
    /// multiple typed items, each typically representing one or more variables/constants along
    /// with their types. The function recursively extracts all such typed symbols and collects them
    /// into a flat vector.
    ///
    /// # Parameters
    /// - `node_ref`: The AST syntax representing a `TypedList`.
    /// - `ast`: The arena containing all AST nodes.
    ///
    /// # Returns
    /// - `Ok(Vec<TypedSymbol>)`: A vector of typed symbols extracted from the list.
    /// - `Err(ParserInternalError)`: If the AST syntax is not a `TypedList` or if processing any
    ///   child syntax fails.
    ///
    /// # Errors
    /// This function returns an error if:
    /// - The provided syntax is not of kind `TypedList`.
    /// - Any of the typed items in the list fail to be extracted correctly.
    ///
    /// # Example
    /// ```rust
    /// let arguments = symbol_table.extract_arguments_from_typed_list(node_ref, &ast)?;
    /// for arg in arguments {
    ///     println!("Argument: {:?} with type_checker {:?}", arg.name, arg.type_);
    /// }
    /// ```
    fn extract_arguments_from_typed_list(
        &mut self,
        node_ref: &NodeRef<AstNode>,
        ast: &Ast,
    ) -> Result<TypedList, SymbolTableError> {

        let mut typed_arguments = TypedList::new();
        for typed_item_id in node_ref.node().children() {
            let typed_item_ref = &ast.arena().try_node_ref(*typed_item_id)?;
            typed_arguments.extend(self.extract_arguments_from_typed_item(typed_item_ref, ast)?);
        }
        Ok(typed_arguments)
    }

    /// Extracts `TypedSymbol`s from a `TypedItem` AST syntax.
    ///
    /// A `TypedItem` typically has:
    /// - A first child syntax: either a `Constant` or `Variable`.
    /// - An optional second child syntax: the associated type_checker(s).
    ///
    /// This function validates the structure, extracts the type_checker information,
    /// and returns a vector of `TypedSymbol`s containing the name and associated types.
    ///
    /// # Errors
    /// Returns a `ParserInternalError` if:
    /// - The syntax is not of kind `TypedItem`.
    /// - The first child is not a `Constant` or `Variable`.
    /// - The number of children is not 1 or 2.
    /// - Extracting the type_checker(s) fails.
    ///
    /// # Example
    /// ```rust
    /// let typed_symbols = self.extract_arguments_from_typed_item(typed_item_ref, &ast)?;
    /// for ts in typed_symbols {
    ///     println!("Symbol: {}, Types: {:?}", ts.name, ts.type_);
    /// }
    /// ```
    fn extract_arguments_from_typed_item(
        &mut self,
        typed_item_ref: &NodeRef<AstNode>,
        ast: &Ast,
    ) -> Result<TypedList, SymbolTableError> {

        let syntax_tree = ast.arena();
        let node = typed_item_ref.node();

        // Check the number of children to extract types or fallback to empty vector
        let types = match node.children().len() {
            1 => Type::new(),
            2 => {
                let ty_id = node.try_child(1)?;
                self.extract_type(&syntax_tree.try_node_ref(ty_id)?, ast)?
            },
            n => {
                return Err(SymbolTableError::invalid_typed_item_arity(
                    typed_item_ref.id(),
                    n,
                ));
            }
        };

        let mut typed_arguments = TypedList::new();
        let elt_id = node.try_child(0)?;
        let elt = syntax_tree.try_node_ref(elt_id)?;

        // Ensure the first child is either a Constant or Variable node
        match elt.node().kind() {
            AstKind::Constant | AstKind::Variable => {
                let symbol_ref = ast.arena().try_symbol_ref(elt.id())?;
                let name = symbol_ref.ident();
                typed_arguments.push(TypedSymbol::new(name, types.clone()));
            }
            found => {
                // Return error if the node kind is unexpected
                return Err(SymbolTableError::UnexpectedAstKind {
                    expected: vec![AstKind::Constant, AstKind::Variable],
                    found,
                    node_id: elt.id(),
                });
            }
        }

        Ok(typed_arguments)
    }


    /// Extracts type_checker names from a `Type` AST syntax without recording symbol usage.
    ///
    /// This function validates that the given AST syntax is of kind `Type` and
    /// extracts all contained primitive type_checker identifiers.
    ///
    /// # Arguments
    ///
    /// * `type_ref` - A reference to an AST syntax expected to be of kind `Type`.
    /// * `ast` - The AST arena containing all nodes.
    ///
    /// # Returns
    ///
    /// Returns a vector of type_checker identifiers (`Ident`) wrapped in `Ok` if successful,
    /// or a `ParserInternalError` if the syntax is invalid or contains unexpected children.
    ///
    /// # Errors
    ///
    /// Returns an error if the syntax is not of kind `Type` or if any child syntax is not of kind `PrimitiveType`.
    fn extract_type(
        &mut self,
        type_ref: &NodeRef<AstNode>,
        ast: &Ast,
    ) -> Result<Type, SymbolTableError> {

        let arena = ast.arena();

        let mut super_types = Type::new();

        for ty_id in type_ref.node().children() {
            let ty_ref = arena.try_node_ref(*ty_id)?;

            // Expecting each child to be a PrimitiveType node
            if ty_ref.node().kind() == AstKind::PrimitiveType {
                let symbol_ref = arena.try_symbol_ref(ty_ref.id())?;
                let name = symbol_ref.ident();
                super_types.add_type(name);
            } else {
                // Return an error if an unexpected AST node kind is found
                return Err(SymbolTableError::UnexpectedAstKind {
                    expected: vec![AstKind::PrimitiveType],
                    found: ty_ref.node().kind(),
                    node_id: ty_ref.id(),
                });
            }
        }

        Ok(super_types)
    }


    /// Initializes type_checker information and records symbol usage in the given scope.
    ///
    /// This function validates that the AST syntax is of kind `Type`, extracts the contained
    /// primitive type_checker identifiers, and registers each as a symbol usage within the specified scope.
    ///
    /// # Arguments
    ///
    /// * `type_ref` - A reference to an AST syntax expected to be of kind `Type`.
    /// * `ast` - The AST arena containing all nodes.
    /// * `scope` - The scope in which the type_checker symbols are used.
    ///
    /// # Returns
    ///
    /// Returns a vector of type_checker identifiers (`Ident`) wrapped in `Ok` if successful,
    /// or a `ParserInternalError` if the syntax is invalid or contains unexpected children.
    ///
    /// # Errors
    ///
    /// Returns an error if the syntax is not of kind `Type` or if any child syntax is not of kind
    /// `PrimitiveType`.
    fn init_from_type(
        &mut self,
        type_ref: &NodeRef<AstNode>,
        ast: &Ast,
        scope: Scope,
    ) -> Result<Type, SymbolTableError> {
        let super_types = self.extract_type(type_ref, ast)?; // Reuse `extract_type` to get type_checker names

        // Register each type_checker as a symbol usage in the given scope
        for ty in type_ref.node().children() {
            let ty_ref = ast.arena().try_node_ref(*ty)?;
            self.add_symbol_usage(&ty_ref, ast, scope.clone())?;
        }

        Ok(super_types)
    }

    /// Initializes the symbol table from a tagged task definition in the AST.
    ///
    /// This function validates that the AST syntax is of kind `TaggedTask` and contains exactly two children:
    /// an identifier and the associated task. It registers the identifier as a declaration symbol
    /// within the given scope, then initializes the symbol table for the referenced task.
    ///
    /// # Arguments
    ///
    /// * `node_ref` - A reference to the AST syntax representing the tagged task.
    /// * `ast` - The AST arena containing all nodes.
    /// * `scope` - The current scope where the symbol declarations and usages apply.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if the initialization is successful,
    /// or a `ParserInternalError` if the AST structure is invalid or unexpected.
    ///
    /// # Errors
    ///
    /// Returns an error if the syntax is not of kind `TaggedTask`, does not have exactly two children,
    /// or if the children are not of expected kinds (`TaskID` for the first and `Task` for the second).
    fn init_from_tagged_task(
        &mut self,
        node_ref: &NodeRef<AstNode>,
        ast: &Ast,
        scope: Scope,
    ) -> Result<(), SymbolTableError> {

        let syntax_tree= ast.arena();
        let node = node_ref.node();

        let tag_id = node.try_child(0)?;
        let tag = syntax_tree.try_node_ref(tag_id)?;

        // Add the task identifier as a declaration symbol
        self.add_declaration_symbol(&tag, ast, scope.clone(), None, None)?;

        // Process the actual task
        let task_id = node.try_child(1)?;
        let task = syntax_tree.try_node_ref(task_id)?;
        //Self::assert_ast_kind(task.syntax(), &[AstKind::Task])?;

        self.init_from_atomic_formula(&task, ast, scope.clone())?;

        Ok(())
    }

    /// Initializes a task ordering constraint from the given AST syntax.
    ///
    /// This function validates that the AST syntax is of kind `TaskOrderingConstraint` and contains
    /// exactly two children, each representing a `TaskID`. It registers both task identifiers as
    /// symbol usages within the given scope.
    ///
    /// # Arguments
    ///
    /// * `node_ref` - A reference to the AST syntax representing the task ordering constraint.
    /// * `ast` - The AST arena containing all nodes.
    /// * `scope` - The current scope used for symbol tracking.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if the initialization is successful,
    /// or a `ParserInternalError` if the AST syntax is invalid or children are not as expected.
    ///
    /// # Errors
    ///
    /// Returns an error if the syntax is not of kind `TaskOrderingConstraint`,
    /// if it does not have exactly two children,
    /// or if either child is not of kind `TaskID`.
    fn init_from_task_ordering_constraint(
        &mut self,
        node_ref: &NodeRef<AstNode>,
        ast: &Ast,
        scope: Scope,
    ) -> Result<(), SymbolTableError> {

        let syntax_tree = ast.arena();
        let t1_id = node_ref.node().try_child(0)?;
        let t1 = syntax_tree.try_node_ref(t1_id)?;
        self.add_symbol_usage(&t1, ast, scope.clone())?;

        let t2_id = node_ref.node().try_child(1)?;
        let t2 = syntax_tree.try_node_ref(t2_id)?;
        self.add_symbol_usage(&t2, ast, scope.clone())?;

        Ok(())
    }
}
