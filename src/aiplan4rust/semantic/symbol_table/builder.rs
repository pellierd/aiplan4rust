
use crate::aiplan4rust::frontend::ParserInternalError;
use crate::aiplan4rust::tree::{TreeArena, NodeRef};
use crate::aiplan4rust::semantic::symbol::SymbolOrigin;
use crate::aiplan4rust::semantic::symbol::{Declaration, Scope, SymbolEntry,Usage};
use crate::aiplan4rust::semantic::symbol_table::SymbolTableOrigin;
use crate::aiplan4rust::semantic::{AstArenaNode, SymbolTable};
use crate::aiplan4rust::syntax::ast::AstKind;
use crate::aiplan4rust::lang::Type;
use crate::aiplan4rust::lang::TypedSymbol;
use crate::aiplan4rust::lang::TypedList;

/// `Comparator` is an enum that represents the different types of comparisons
/// that can be made between values, specifically for validating the number of children
/// in an Abstract Syntax Tree (AST) node.
///
/// This enum provides a variety of comparison operations that allow you to check:
/// - equality
/// - inequality
/// - relative magnitude (less than, greater than)
/// - inclusive comparisons (less than or equal, greater than or equal)
///
/// It is primarily used to validate the number of children a specific AST node should have
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
/// `Comparator` makes it easy to express common comparison operations in a type-safe manner.
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

/// A builder for constructing a [`Table`] from an abstract syntax tree (AST).
///
/// `SymbolTableBuilder` encapsulates the logic for traversing an [`ArenaAst`]
/// and populating a `SymbolTable` with symbols extracted from the tree. It
/// identifies the root node type (e.g., Domain or Problem), determines the source
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
            table: SymbolTable::new(SymbolTableOrigin::Unknown),
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

    /// Builds a complete [`Table`] from the given abstract syntax tree.
    ///
    /// This method:
    /// - Retrieves the root node of the AST.
    /// - Ensures that the root is either a `Domain` or `Problem`.
    /// - Sets the table's source accordingly.
    /// - Initializes the table using the AST contents.
    ///
    /// # Arguments
    ///
    /// * `ast` - A reference to the [`ArenaAst`] representing the parsed syntax tree.
    ///
    /// # Errors
    ///
    /// Returns a [`ParserInternalError`] if:
    /// - The AST has no root node.
    /// - The root node is not a valid entry point (Domain or Problem).
    /// - An error occurs during symbol initialization.
    ///
    /// # Returns
    ///
    /// A fully initialized `SymbolTable` on success.
    pub fn build(&mut self, ast: &TreeArena<AstArenaNode>) -> Result<SymbolTable, ParserInternalError> {
        // Retrieve the root of the AST and handle the case where it is missing
        let root_ref = ast.root_node_ref().ok_or_else(|| {
            ParserInternalError::new("AST root node is missing".to_string())
        })?;
        let root_node = root_ref.node();

        // Determine the root kind and set the source of the symbol table
        match root_node.kind() {
            AstKind::Domain => {
                self.table_mut().set_origin(SymbolTableOrigin::Domain);
            }
            AstKind::Problem => {
                self.table_mut().set_origin(SymbolTableOrigin::Problem);
            }
            _ => {
                return Err(ParserInternalError::new(format!(
                    "Invalid AST: root node is not a Domain or Problem, found: {}",
                    root_node.kind()
                )));
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
    /// Depending on the node kind, it may add symbol declarations, register symbol usages,
    /// or perform special handling for constructs such as actions and atomic formulas.
    ///
    /// # Arguments
    ///
    /// * `node_ref` - A reference to the AST node from which initialization begins.
    ///                This node acts as the root for the current scope of symbol processing.
    /// * `ast` - A reference to the entire AST (`ArenaAst`), used for node lookups and context.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` on success, or a [`ParserInternalError`] if an error occurs during
    /// symbol initialization, such as invalid node types or semantic errors.
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
        node_ref: &NodeRef<AstArenaNode>,
        ast: &TreeArena<AstArenaNode>,
    ) -> Result<(), ParserInternalError> {
        let scope = Scope::new(node_ref.id(), None);
        self.init_from(node_ref, ast, scope)?;
        Ok(())
    }

    /// Recursively initializes the symbol table by processing AST nodes.
    ///
    /// This function examines the kind of the AST node referenced by `node_ref`
    /// and performs symbol table operations accordingly. It handles declarations,
    /// symbol usages, and special AST constructs such as actions, formulas, and HTN tasks.
    /// For unhandled node kinds, it recursively processes their child nodes.
    ///
    /// # Arguments
    ///
    /// * `node_ref` - The reference to the AST node currently being processed.
    /// * `ast` - The full abstract syntax tree (`ArenaAst`) that contains the node.
    /// * `scope` - The current scope, which manages symbol visibility and hierarchical
    ///   structure within the AST.
    ///
    /// # Behavior by AST node kind
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
    /// This method returns an error if symbol registration or node processing fails.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// let mut builder = SymbolTableBuilder::new();
    /// builder.init_from(root_node_ref, &ast, initial_scope)?;
    /// ```
    fn init_from(
        &mut self,
        node_ref: &NodeRef<AstArenaNode>,
        ast: &TreeArena<AstArenaNode>,
        scope: Scope,
    ) -> Result<(), ParserInternalError> {
        // Determine the type of the AST node and apply appropriate processing
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

            // Default case: If the AST node is not explicitly handled, process its children
            // recursively
            _ => {
                for child in node_ref.node().children() {
                    self.init_from(&ast.try_node_ref(*child)?, ast, scope.clone())?;
                }
            }
        }

        // Successfully completed processing the AST node
        Ok(())
    }

    /// Adds a new symbol declaration to the symbol table.
    ///
    /// This function extracts the symbol's name and kind from the given AST node by
    /// using `try_symbol_ref`. It first asserts that the AST node's kind is valid for a
    /// declaration using `assert_ast_kind`. Then, it either updates an existing symbol
    /// in the symbol table by adding a new declaration or creates a new symbol with
    /// the declaration and inserts it into the table.
    ///
    /// # Arguments
    ///
    /// * `node_ref` - A reference to the AST node representing the symbol declaration.
    /// * `ast` - The abstract syntax tree (`ArenaAst`) containing the node.
    /// * `scope` - The current scope in which the symbol is declared, affecting visibility.
    /// * `types` - Optional vector of identifier types associated with the symbol declaration.
    /// * `arguments` - Optional vector of typed symbols representing the symbol's arguments.
    ///
    /// # Returns
    ///
    /// * `Ok(())` - When the symbol declaration is successfully added to the symbol table.
    /// * `Err(ParserInternalError)` - If the AST node kind is invalid, symbol extraction fails,
    ///   or insertion into the table encounters an error.
    ///
    /// # Panics
    ///
    /// This function will return an error if the AST node kind does not match expected
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
        node_ref: &NodeRef<AstArenaNode>,
        ast: &TreeArena<AstArenaNode>,
        scope: Scope,
        types: Option<Type>,
        arguments: Option<TypedList>,
    ) -> Result<(), ParserInternalError> {
        // Assert that the AST kind is valid
        /*Self::assert_ast_kind(
            node_ref.node(),
            &[
                AstKind::DomainName,
                AstKind::PrimitiveType,
                AstKind::ProblemName,
                AstKind::Constant,
                AstKind::Variable,
                AstKind::Predicate,
                AstKind::FunctionSymbol,
                AstKind::ActionSymbol,
                AstKind::DASymbol,
                // Add for HDDL
                AstKind::MethodSymbol,
                AstKind::TaskSymbol,
                AstKind::TaskID,
            ],
        )?;*/

        // Extract the symbol information from the AST
        let symbol_ref = ast.try_symbol_ref(node_ref.id())?;
        let ident = symbol_ref.ident();

        // Check if the symbol is already in the symbol table and add a declaration
        let origin = SymbolOrigin::from(self.table().origin());
        if let Some(symbol) = self.table_mut().get_symbol_mut(ident) {
            let declaration =
                Declaration::new(symbol_ref, scope, origin, types, arguments, node_ref.node().span().clone(), node_ref.id());
            symbol.add_declaration(declaration);
        } else {
            // Create a new symbol and add the declaration to it
            let mut symbol = SymbolEntry::new(ident);
            let declaration =
                Declaration::new(symbol_ref, scope, origin, types, arguments, node_ref.node().span().clone(), node_ref.id());
            symbol.add_declaration(declaration);
            self.table_mut().insert_symbol(ident, symbol); // Insert the new symbol into the table
        }

        Ok(())
    }

    /// Adds the usage of a symbol found in the given AST node to the symbol table.
    ///
    /// This function processes AST nodes representing symbol usages, such as domain names,
    /// problem names, constants, variables, atomic formulas, function terms, and tasks.
    /// It first validates the AST node kind to ensure it is appropriate for symbol usage.
    /// For certain node kinds that represent complex expr (e.g., atomic formulas),
    /// it also verifies the presence of at least one child node.
    ///
    /// The symbol's identifier and kind are extracted, and the function then updates
    /// the symbol table: if the symbol already exists, the usage information is appended;
    /// otherwise, a new symbol entry is created with the usage data.
    ///
    /// # Arguments
    ///
    /// * `node_ref` - A reference to the AST node representing the symbol usage.
    /// * `ast` - The full abstract syntax tree (`ArenaAst`) containing the node.
    /// * `scope` - The scope context in which the symbol is used, which controls visibility.
    ///
    /// # Returns
    ///
    /// * `Ok(())` - Indicates the symbol usage was successfully recorded.
    /// * `Err(ParserInternalError)` - Returned if the AST node kind is invalid, the node’s
    ///   structure is incorrect (e.g., missing children), or if symbol extraction fails.
    ///
    /// # Panics
    ///
    /// The function will return an error if the AST node kind does not match any of the
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
        node_ref: &NodeRef<AstArenaNode>,
        ast: &TreeArena<AstArenaNode>,
        scope: Scope,
    ) -> Result<(), ParserInternalError> {
        // Assert that the AST node is of a valid kind for symbol usage.
        /*Self::assert_ast_kind(
            node_ref.node(),
            &[
                AstKind::DomainName,
                AstKind::ProblemName,
                AstKind::Constant,
                AstKind::Variable,
                AstKind::AtomicFormula,
                AstKind::FunctionTerm,
                AstKind::Task,
            ],
        )?;*/

        // Handle specific cases for AtomicFormula and FunctionTerm, which need to have a child.
        /*if matches!(
                node_ref.node().kind(),
                AstKind::AtomicFormula | AstKind::FunctionTerm | AstKind::Task
            ) {
            Self::assert_ast_children_number(node_ref.node(), 1, Comparator::GreaterEq)?;
        }*/

        let symbol_ref = if matches!(
            node_ref.node().kind(),
            AstKind::AtomicFormula | AstKind::FunctionTerm | AstKind::Task
        ) {
            let first_node_ref = ast.try_node_ref(node_ref.node().children()[0])?;
            ast.try_symbol_ref(first_node_ref.id())?
        } else {
            ast.try_symbol_ref(node_ref.id())?
        };

        // Extract symbol name and type based on the AST node's kind.
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

    /// Initializes the symbol table from a `TypedList` AST node.
    ///
    /// This function processes a `TypedList` node in the AST by iterating over its children,
    /// which represent typed elements such as types, constants, variables, or other symbols.
    /// It handles the node structure by:
    /// - Returning early if there are no children (nothing to process).
    /// - Recursively processing each child node as a typed item.
    ///
    /// # Arguments
    ///
    /// * `node_ref` - A reference to the `TypedList` AST node to process.
    /// * `ast` - The full abstract syntax tree (`ArenaAst`) containing the node.
    /// * `scope` - The current scope within which the symbols are being initialized.
    ///
    /// # Returns
    ///
    /// * `Ok(())` if all children are successfully processed and the symbol table is updated.
    /// * `Err(ParserInternalError)` if the node is not of the expected kind or if any child node
    ///   fails to process correctly.
    ///
    /// # Errors
    ///
    /// Returns an error if the AST node kind is not `TypedList` or if any of the child nodes cannot
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
        node_ref: &NodeRef<AstArenaNode>,
        ast: &TreeArena<AstArenaNode>,
        scope: Scope,
    ) -> Result<(), ParserInternalError> {
        // Ensure the AST node is of the expected type 'TypedList'
        //Self::assert_ast_kind(node_ref.node(), &[AstKind::TypedList])?;

        let children = node_ref.node().children();

        if children.is_empty() {
            return Ok(());
        }

        for typed_item in children.iter() {
            self.init_from_typed_item(&ast.try_node_ref(*typed_item)?, ast, scope.clone())?;
        }

        Ok(())
    }

    /// Initializes symbol declarations from a `TypedItem` syntax node.
    ///
    /// A `TypedItem` node typically represents a declaration where a list of symbols (e.g.,
    /// constants or variables) is associated with a type (e.g., `?x ?y - location`). This function
    /// parses both the symbol list and the type annotation and registers the symbols in the
    /// internal symbol table.
    ///
    /// # Parameters
    ///
    /// - `node_ref`: A reference to the `TypedItem` AST node to process.
    /// - `ast`: The full abstract syntax tree (`ArenaAst`) containing the node.
    /// - `scope`: The current scope in which the symbols are declared. This is cloned as needed
    ///   to maintain correct scoping during recursive calls.
    ///
    /// # Behavior
    ///
    /// - Verifies that the node kind is `TypedItem`.
    /// - Extracts the children of the node:
    ///   - If there is only one child, treats it as an untyped declaration (empty type list).
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
    /// - The node is not of kind `TypedItem`.
    /// - The number of children is invalid (not 1 or 2).
    /// - Parsing the type annotation fails.
    /// - Adding the declarations to the symbol table fails.
    ///
    /// # Example
    ///
    /// ```text
    /// (?x ?y - location)  // symbols: ?x, ?y; type: location
    /// (?z)                // symbol: ?z; no associated type
    /// ```
    fn init_from_typed_item(
        &mut self,
        node_ref: &NodeRef<AstArenaNode>,
        ast: &TreeArena<AstArenaNode>,
        scope: Scope,
    ) -> Result<(), ParserInternalError> {
        // Ensure the node is of the expected kind
        //Self::assert_ast_kind(node_ref.node(), &[AstKind::TypedItem])?;

        // Get its children
        let children = node_ref.node().children();

        // Match on the number of children to extract the types or fallback to an empty vector
        let types = match children.len() {
            1 => Type::new(),
            2 => self.init_from_type(&ast.try_node_ref(children[1])?, ast, scope.clone())?,
            _ => {
                return Err(ParserInternalError::new(format!(
                    "TypedItem node has unexpected number of children: {}",
                    children.len()
                )))
            }
        };
        // Process the first element of the pair
        self.init_from_typed_item_elements(&ast.try_node_ref(children[0])?, ast, scope, types)
    }

    /// Helper function to process an individual element of the `TypedList`.
    ///
    /// This function adds the element to the symbol table based on its type, such as
    /// `PrimitiveType`, `Constant`, `Variable`, or handles nested elements like
    /// `AtomicFunctionSkeleton`.
    ///
    /// # Parameters
    ///
    /// - `node_ref`: The AST node representing the element to process. This node is expected
    ///   to be one of the accepted kinds (e.g., `PrimitiveType`, `Constant`, `Variable`, or `AtomicFunctionSkeleton`).
    /// - `ast`: The full abstract syntax tree (`ArenaAst`) that contains the node.
    /// - `scope`: The current scope in which the symbol is being declared or used.
    /// - `types`: A vector of type identifiers associated with the element.
    ///
    /// # Behavior
    ///
    /// - Validates that the AST node kind is one of the expected types.
    /// - If the element is a constant, variable, or primitive type, it adds a declaration symbol
    ///   to the symbol table with the provided types.
    /// - If the element is an atomic function skeleton, it recursively initializes it accordingly.
    /// - The function assumes all other node kinds are invalid and will panic if encountered.
    ///
    /// # Returns
    ///
    /// - Returns `Ok(())` on successful processing.
    /// - Returns an error if the AST node kind is invalid or any symbol table operation fails.
    fn init_from_typed_item_elements(
        &mut self,
        node_ref: &NodeRef<AstArenaNode>,
        ast: &TreeArena<AstArenaNode>,
        scope: Scope,
        types: Type,
    ) -> Result<(), ParserInternalError> {
        // Validate that the node kind is one of the expected AST kinds
        /*Self::assert_ast_kind(
            node_ref.node(),
            &[
                AstKind::PrimitiveType,
                AstKind::Constant,
                AstKind::Variable,
                AstKind::AtomicFunctionSkeleton,
            ],
        )?;*/

        match node_ref.node().kind() {
            AstKind::PrimitiveType | AstKind::Constant | AstKind::Variable => {
                // For constants and variables, add a declaration symbol with the provided types
                self.add_declaration_symbol(node_ref, ast, scope.clone(), Some(types.clone()), None)?;
            }

            AstKind::AtomicFunctionSkeleton => {
                // Recursively initialize atomic function skeleton elements
                self.init_from_atomic_function_skeleton(node_ref, ast, scope.clone(), types.clone())?;
            }

            _ => {
                // This should never happen due to the earlier assertion
                unreachable!("Unexpected AST node type: {:?}", node_ref.node().kind());
            }
        }

        Ok(())
    }

    /// Initializes the symbol table for an atomic function skeleton.
    ///
    /// This function processes an AST node of kind `AtomicFunctionSkeleton` by performing several key steps:
    /// - Verifies that the AST node is of the correct kind (`AtomicFunctionSkeleton`).
    /// - Checks that the node has at least two children: the function symbol and the list of arguments.
    /// - Validates that the first child is a `FunctionSymbol` node.
    /// - Creates a new nested scope for the function and initializes symbols for its arguments.
    /// - Extracts the arguments and computes their arity.
    /// - Adds a function declaration to the symbol table with the associated types and arguments.
    ///
    /// # Parameters
    /// - `node_ref`: The AST node reference corresponding to the `AtomicFunctionSkeleton`.
    /// - `ast`: Reference to the complete AST (`ArenaAst`) containing the node.
    /// - `scope`: The current scope in which the function is declared; used for symbol resolution.
    /// - `types`: A vector of type identifiers associated with the function.
    ///
    /// # Returns
    /// - `Ok(())` on successful processing and symbol table update.
    /// - `Err(ParserInternalError)` if any validation or symbol table operation fails, such as:
    ///   - The node is not an `AtomicFunctionSkeleton`.
    ///   - The node has fewer than two children.
    ///   - The first child is not a `FunctionSymbol`.
    ///
    /// # Example
    /// ```rust
    /// let ast_node = /* obtain AtomicFunctionSkeleton node reference */;
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
    /// AST node kinds or structural problems in the `AtomicFunctionSkeleton` node.
    ///
    /// # Notes
    /// - Assumes `init_from_typed_list` correctly initializes and validates argument types.
    /// - This function is integral to managing function symbol declarations and their scopes
    ///   within the symbol table system.
    fn init_from_atomic_function_skeleton(
        &mut self,
        node_ref: &NodeRef<AstArenaNode>,
        ast: &TreeArena<AstArenaNode>,
        scope: Scope,
        types: Type,
    ) -> Result<(), ParserInternalError> {
        // Check that the AST node is of the expected type 'Function'
        //Self::assert_ast_kind(node_ref.node(), &[AstKind::AtomicFunctionSkeleton])?;

        // Ensure the node has at least two children (function symbol and arguments)
        //Self::assert_ast_children_number(node_ref.node(), 2, Comparator::GreaterEq)?;

        let children = node_ref.node().children();

        // Retrieve the first child and validate it as a 'FunctionSymbol'
        let functor_ref = ast.try_node_ref(children[0])?;
        if functor_ref.node().kind() != AstKind::FunctionSymbol {
            return Err(ParserInternalError::new(format!(
                "First child of 'Function' must match the expected kind. Encountered: '{:?}'",
                functor_ref.node().kind()
            )))
        }

        let arguments = &ast.try_node_ref(children[1])?;
        // Initialize the symbol table for the arguments;
        self.init_from_typed_list(
            arguments,
            ast,
            Scope::new(node_ref.id(), Some(&scope)),
        )?;

        // Extract the arguments and calculate the arity
        let arguments =
            self.extract_arguments_from_typed_list(arguments, ast)?;

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
    /// This function processes an AST node representing an action or method definition, which can be
    /// one of `ActionDef`, `DurativeActionDef`, or `MethodDef`. It extracts the action/method name,
    /// parameters, and body, adding the appropriate symbols to the symbol table.
    ///
    /// # Parameters
    /// - `node_ref`: The AST node reference corresponding to the action definition.
    /// - `ast`: Reference to the AST (`ArenaAst`) containing the node.
    /// - `scope`: The current scope in which the action or method is defined.
    ///
    /// # Returns
    /// - `Ok(())` if the initialization completes successfully.
    /// - `Err(ParserInternalError)` if the AST node does not have the expected structure or kind.
    ///
    /// # AST Structure
    /// The node must have exactly three children:
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
    /// This function delegates the main work to `init_from_def`, passing in the expected node kinds,
    /// expected number of children, and a flag indicating the presence of a body.
    fn init_from_action_def(
        &mut self,
        node_ref: &NodeRef<AstArenaNode>,
        ast: &TreeArena<AstArenaNode>,
        scope: Scope,
    ) -> Result<(), ParserInternalError> {
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
    /// This function verifies that the given AST node is of kind `MethodDef` and contains exactly
    /// three children representing the method name, its parameters, and its body. It then processes
    /// these components to update the symbol table accordingly.
    ///
    /// # Parameters
    /// - `node_ref`: The AST node reference corresponding to the method definition.
    /// - `ast`: Reference to the AST (`ArenaAst`) that contains the node.
    /// - `scope`: The current scope in which the method is defined.
    ///
    /// # Returns
    /// - `Ok(())` if the initialization completes successfully.
    /// - `Err(ParserInternalError)` if the node kind or structure is invalid.
    ///
    /// # AST Structure
    /// The node is expected to have three children:
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
        node_ref: &NodeRef<AstArenaNode>,
        ast: &TreeArena<AstArenaNode>,
        scope: Scope,
    ) -> Result<(), ParserInternalError> {
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
    /// This function verifies that the given AST node is of kind `DurativeActionDef` and contains
    /// exactly three children representing the action's name, parameters, and body. It processes
    /// these components to update the symbol table accordingly.
    ///
    /// # Parameters
    /// - `node_ref`: The AST node reference corresponding to the durative action definition.
    /// - `ast`: Reference to the AST (`ArenaAst`) containing the node.
    /// - `scope`: The current scope in which the durative action is defined.
    ///
    /// # Returns
    /// - `Ok(())` if the initialization completes successfully.
    /// - `Err(ParserInternalError)` if the node kind or structure is invalid.
    ///
    /// # AST Structure
    /// The node is expected to have three children:
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
        node_ref: &NodeRef<AstArenaNode>,
        ast: &TreeArena<AstArenaNode>,
        scope: Scope,
    ) -> Result<(), ParserInternalError> {
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
    /// This function validates that the given AST node is of kind `TaskDef` and contains exactly
    /// two children: the task name and its parameters. Since task definitions do not have a body,
    /// only these components are processed and added to the symbol table.
    ///
    /// # Parameters
    /// - `node_ref`: The AST node reference corresponding to the task definition.
    /// - `ast`: Reference to the AST (`ArenaAst`) containing the node.
    /// - `scope`: The current scope in which the task is defined.
    ///
    /// # Returns
    /// - `Ok(())` if the initialization completes successfully.
    /// - `Err(ParserInternalError)` if the node kind or structure is invalid.
    ///
    /// # AST Structure
    /// The node is expected to have two children:
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
        node_ref: &NodeRef<AstArenaNode>,
        ast: &TreeArena<AstArenaNode>,
        scope: Scope,
    ) -> Result<(), ParserInternalError> {
        self.init_from_def(
            node_ref,
            ast,
            scope,
            &[AstKind::TaskDef],
            2,     // TaskDef has only 2 children (name, parameters)
            false, // No body for tasks
        )
    }

    /// Initializes a definition node (`ActionDef`, `DurativeActionDef`, `MethodDef`, or `TaskDef`)
    /// by extracting its name, parameters, and optionally its body, updating the symbol table accordingly.
    ///
    /// This function performs the following steps:
    /// - Validates that the AST node is one of the expected kinds.
    /// - Confirms the node has the expected number of children.
    /// - Extracts the definition name and adds it as a declaration in the current scope.
    /// - Recursively processes the parameter list (`TypedList`), adding parameter symbols.
    /// - If the definition has a body (e.g., actions and methods), recursively processes the body.
    ///
    /// # Parameters
    /// - `node_ref`: Reference to the AST node representing the definition.
    /// - `ast`: The AST tree containing all nodes.
    /// - `scope`: The current symbol table scope for resolving symbols and declarations.
    /// - `valid_kinds`: Slice of valid AST kinds that this function can process.
    /// - `expected_children`: The exact number of child nodes expected (2 for tasks, 3 for actions/methods).
    /// - `has_body`: Indicates if the definition node includes a body that needs processing.
    ///
    /// # Returns
    /// - `Ok(())` if initialization completes successfully.
    /// - `Err(ParserInternalError)` if validation fails or processing encounters an unexpected AST structure.
    ///
    /// # AST Structure
    /// The node's children are expected as follows:
    /// 1. **Name** — the identifier of the definition.
    /// 2. **Parameters** — a `TypedList` node representing parameters.
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
        node_ref: &NodeRef<AstArenaNode>,
        ast: &TreeArena<AstArenaNode>,
        scope: Scope,
        _valid_kinds: &[AstKind],
        _expected_children: usize,
        has_body: bool,
    ) -> Result<(), ParserInternalError> {
        // Ensure the AST node is of the correct kind
        //Self::assert_ast_kind(node_ref.node(), valid_kinds)?;

        // Ensure the node has the expected number of children
        //Self::assert_ast_children_number(node_ref.node(), expected_children, Comparator::Equal)?;

        let children = node_ref.node().children();

        // First child: definition name, add to symbol table
        let name = ast.try_node_ref(children[0])?;

        // Second child: parameters, recursively initialize the symbol table
        let parameters = &ast.try_node_ref(children[1])?;
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
            let body = &ast.try_node_ref(children[2])?;
            self.init_from(
                body,
                ast,
                Scope::new(node_ref.id(), Some(&scope)),
            )?;
        }

        Ok(())
    }

    /// Initializes the syntax state from an `AtomicFormula`, `FunctionTerm`, or `Task` AST node.
    ///
    /// This function processes an AST node expected to represent either an atomic formula,
    /// a function term, or a task. It verifies that the node has at least one child (the symbol),
    /// registers the usage of that symbol, then recursively processes all children as arguments.
    ///
    /// # Parameters
    /// - `node_ref`: Reference to the AST node representing the atomic formula or function term.
    /// - `ast`: The AST tree containing all nodes.
    /// - `scope`: The current scope used for symbol resolution and symbol usage registration.
    ///
    /// # Returns
    /// - `Ok(())` if the node and its children are successfully processed.
    /// - `Err(ParserInternalError)` if the node kind is invalid, lacks children, or if recursive processing fails.
    ///
    /// # Errors
    /// Returns an error if:
    /// - The AST node kind is not one of `AtomicFormula`, `FunctionTerm`, or `Task`.
    /// - The node has no children (at least one child is expected as the symbol).
    /// - Any recursive call to `init_from` returns an error.
    ///
    /// # Example
    /// ```rust
    /// symbol_table.init_from_atomic_formula(node_ref, &ast, scope)?;
    /// ```
    fn init_from_atomic_formula(
        &mut self,
        node_ref: &NodeRef<AstArenaNode>,
        ast: &TreeArena<AstArenaNode>,
        scope: Scope,
    ) -> Result<(), ParserInternalError> {
        // Ensure the AST node is of the correct kind (AtomicFormula or FunctionTerm)
        /*Self::assert_ast_kind(
            node_ref.node(),
            &[
                AstKind::AtomicFormula,
                AstKind::FunctionTerm,
                AstKind::Task,
            ],
        )?;*/

        // Ensure the node has at least one child (the symbol)
        //Self::assert_ast_children_number(node_ref.node(), 1, Comparator::GreaterEq)?;

        // Retrieve and register the first child (symbol)
        self.add_symbol_usage(node_ref, ast, scope.clone())?; // should be removed

        // Process the remaining children (arguments)
        for child in node_ref.node().children() {
            self.init_from(&ast.try_node_ref(*child)?, ast, scope.clone())?;
        }

        Ok(())
    }

    /// Initializes the symbol table for a quantified expr (`Exists` or `Forall`) in the AST.
    ///
    /// This function verifies that the AST node is a quantified expr of kind
    /// `Exists` or `Forall` and that it has exactly two children:
    /// 1. A variable declaration (or typed list)
    /// 2. An inner expr.
    ///
    /// It then recursively initializes the symbol table for both the variable declarations and
    /// the inner expr, creating a new nested scope for these initializations.
    ///
    /// # Parameters
    /// - `node_ref`: Reference to the AST node representing the quantified expr.
    /// - `ast`: The AST tree containing all nodes.
    /// - `scope`: The current scope in which the quantified expr resides.
    ///
    /// # Returns
    /// - `Ok(())` if the symbol table initialization succeeds.
    /// - `Err(ParserInternalError)` if the node is not a quantified expr, if
    ///   the number of children is incorrect, or if initialization of children fails.
    ///
    /// # Errors
    /// Returns a `ParserInternalError` if:
    /// - The node kind is not `Exists` or `Forall`.
    /// - The node does not have exactly two children.
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
        node_ref: &NodeRef<AstArenaNode>,
        ast: &TreeArena<AstArenaNode>,
        scope: Scope,
    ) -> Result<(), ParserInternalError> {
        // Check if the AST node is of kind 'Exists' or 'Forall'
        //Self::assert_ast_kind(node_ref.node(), &[AstKind::Exists, AstKind::Forall])?;

        // Ensure the AST has exactly 2 children (variables and inner expr)
        //Self::assert_ast_children_number(node_ref.node(), 2, Comparator::Equal)?;

        let children = node_ref.node().children();
        // Retrieve the children (variables and inner expr)
        let variables = &ast.try_node_ref(children[0])?;
        let expression = &ast.try_node_ref(children[1])?;

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
    /// This function verifies that the AST node has the correct structure for an atomic formula skeleton,
    /// which consists of a predicate followed by its arguments. Specifically, it ensures:
    /// 1. The node has exactly two children: a predicate and an argument list.
    /// 2. The first child is of kind `Predicate`.
    ///
    /// Then, it recursively initializes the symbol table for the argument list, extracts the arguments,
    /// and adds the predicate declaration with its arguments to the symbol table.
    ///
    /// # Parameters
    /// - `node_ref`: A reference to the AST node representing the atomic formula skeleton.
    /// - `ast`: The tree containing all AST nodes.
    /// - `scope`: The current scope for symbol resolution.
    ///
    /// # Returns
    /// - `Ok(())` if initialization is successful.
    /// - `Err(ParserInternalError)` if the node does not meet expected structure or if processing fails.
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
        node_ref: &NodeRef<AstArenaNode>,
        ast: &TreeArena<AstArenaNode>,
        scope: Scope,
    ) -> Result<(), ParserInternalError> {
        let children = node_ref.node().children();

        // Ensure the AST has at least two children (predicate and arguments)
        //Self::assert_ast_children_number(node_ref.node(), 2, Comparator::Equal)?;

        // Ensure the first child is of kind 'Predicate'
        let predicate = &ast.try_node_ref(children[0])?;
        //Self::assert_ast_kind(predicate.node(), &[AstKind::Predicate])?;

        // Retrieve and process arguments
        let arguments = &ast.try_node_ref(children[1])?;
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

    /// Extracts arguments from a `TypedList` AST node, which consists of a list of typed symbols.
    ///
    /// This function processes the AST node representing a `TypedList`. A `TypedList` contains
    /// multiple typed items, each typically representing one or more variables/constants along
    /// with their types. The function recursively extracts all such typed symbols and collects them
    /// into a flat vector.
    ///
    /// # Parameters
    /// - `node_ref`: The AST node representing a `TypedList`.
    /// - `ast`: The tree containing all AST nodes.
    ///
    /// # Returns
    /// - `Ok(Vec<TypedSymbol>)`: A vector of typed symbols extracted from the list.
    /// - `Err(ParserInternalError)`: If the AST node is not a `TypedList` or if processing any
    ///   child node fails.
    ///
    /// # Errors
    /// This function returns an error if:
    /// - The provided node is not of kind `TypedList`.
    /// - Any of the typed items in the list fail to be extracted correctly.
    ///
    /// # Example
    /// ```rust
    /// let arguments = symbol_table.extract_arguments_from_typed_list(node_ref, &ast)?;
    /// for arg in arguments {
    ///     println!("Argument: {:?} with type {:?}", arg.name, arg.type_);
    /// }
    /// ```
    fn extract_arguments_from_typed_list(
        &mut self,
        node_ref: &NodeRef<AstArenaNode>,
        ast: &TreeArena<AstArenaNode>,
    ) -> Result<TypedList, ParserInternalError> {
        // Ensure the AST node is of kind TypedList
        //Self::assert_ast_kind(node_ref.node(), &[AstKind::TypedList])?;

        let mut typed_arguments = TypedList::new();
        for typed_item_id in node_ref.node().children() {
            let typed_item_ref = &ast.try_node_ref(*typed_item_id)?;
            typed_arguments.extend(self.extract_arguments_from_typed_item(typed_item_ref, ast)?);
        }
        Ok(typed_arguments)
    }

    /// Extracts `TypedSymbol`s from a `TypedItem` AST node.
    ///
    /// A `TypedItem` typically has:
    /// - A first child node: either a `Constant` or `Variable`.
    /// - An optional second child node: the associated type(s).
    ///
    /// This function validates the structure, extracts the type information,
    /// and returns a vector of `TypedSymbol`s containing the name and associated types.
    ///
    /// # Errors
    /// Returns a `ParserInternalError` if:
    /// - The node is not of kind `TypedItem`.
    /// - The first child is not a `Constant` or `Variable`.
    /// - The number of children is not 1 or 2.
    /// - Extracting the type(s) fails.
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
        typed_item_ref: &NodeRef<AstArenaNode>,
        ast: &TreeArena<AstArenaNode>,
    ) -> Result<TypedList, ParserInternalError> {
        // Ensure the node is of the correct kind
        //Self::assert_ast_kind(typed_item_ref.node(), &[AstKind::TypedItem])?;

        let children = typed_item_ref.node().children();

        // Extract types if available, or use an empty vector
        let types = match children.len() {
            1 => Type::new(),
            2 => self.extract_type(&ast.try_node_ref(children[1])?, ast)?,
            _ => {
                return Err(ParserInternalError::new(format!(
                    "TypedItem must have 1 or 2 children, got {}",
                    children.len()
                )))
            }
        };

        let mut typed_arguments = TypedList::new();
        let elt = ast.try_node_ref(children[0])?;

        match elt.node().kind() {
            AstKind::Constant | AstKind::Variable => {
                let symbol_ref = ast.try_symbol_ref(elt.id())?;
                let name = symbol_ref.ident();
                typed_arguments.push(TypedSymbol::new(name, types.clone()));
            }
            _ => {
                return Err(ParserInternalError::new(format!(
                    "Expected Constant or Variable in TypedItem, found {:?}",
                    elt.node().kind()
                )));
            }
        }

        Ok(typed_arguments)
    }

    /// Extracts type names from a `Type` AST node without recording symbol usage.
    ///
    /// This function validates that the given AST node is of kind `Type` and
    /// extracts all contained primitive type identifiers.
    ///
    /// # Arguments
    ///
    /// * `type_ref` - A reference to an AST node expected to be of kind `Type`.
    /// * `ast` - The AST tree containing all nodes.
    ///
    /// # Returns
    ///
    /// Returns a vector of type identifiers (`Ident`) wrapped in `Ok` if successful,
    /// or a `ParserInternalError` if the node is invalid or contains unexpected children.
    ///
    /// # Errors
    ///
    /// Returns an error if the node is not of kind `Type` or if any child node is not of kind `PrimitiveType`.
    fn extract_type(
        &mut self,
        type_ref: &NodeRef<AstArenaNode>,
        ast: &TreeArena<AstArenaNode>,
    ) -> Result<Type, ParserInternalError> {
        // Ensure the provided AST node is of kind `Type`
        //Self::assert_ast_kind(type_ref.node(), &[AstKind::Type])?;

        let mut super_types = Type::new();
        for ty in type_ref.node().children() {
            let ty_ref = ast.try_node_ref(*ty)?;
            if let AstKind::PrimitiveType = ty_ref.node().kind() {
                let symbol_ref = ast.try_symbol_ref(ty_ref.id())?;
                let name = symbol_ref.ident();
                super_types.add_type(name);
            } else {
                return Err(ParserInternalError::new(
                    "Unexpected AST node inside Type".to_string(),
                ));
            }
        }

        Ok(super_types)
    }

    /// Initializes type information and records symbol usage in the given scope.
    ///
    /// This function validates that the AST node is of kind `Type`, extracts the contained
    /// primitive type identifiers, and registers each as a symbol usage within the specified scope.
    ///
    /// # Arguments
    ///
    /// * `type_ref` - A reference to an AST node expected to be of kind `Type`.
    /// * `ast` - The AST tree containing all nodes.
    /// * `scope` - The scope in which the type symbols are used.
    ///
    /// # Returns
    ///
    /// Returns a vector of type identifiers (`Ident`) wrapped in `Ok` if successful,
    /// or a `ParserInternalError` if the node is invalid or contains unexpected children.
    ///
    /// # Errors
    ///
    /// Returns an error if the node is not of kind `Type` or if any child node is not of kind
    /// `PrimitiveType`.
    fn init_from_type(
        &mut self,
        type_ref: &NodeRef<AstArenaNode>,
        ast: &TreeArena<AstArenaNode>,
        scope: Scope,
    ) -> Result<Type, ParserInternalError> {
        //Self::assert_ast_kind(type_ref.node(), &[AstKind::Type])?;
        let super_types = self.extract_type(type_ref, ast)?; // Reuse `extract_type` to get type names

        // Register each type as a symbol usage in the given scope
        for ty in type_ref.node().children() {
            let ty_ref = ast.try_node_ref(*ty)?;
            self.add_symbol_usage(&ty_ref, ast, scope.clone())?;
        }

        Ok(super_types)
    }

    /// Initializes the symbol table from a tagged task definition in the AST.
    ///
    /// This function validates that the AST node is of kind `TaggedTask` and contains exactly two children:
    /// an identifier and the associated task. It registers the identifier as a declaration symbol
    /// within the given scope, then initializes the symbol table for the referenced task.
    ///
    /// # Arguments
    ///
    /// * `node_ref` - A reference to the AST node representing the tagged task.
    /// * `ast` - The AST tree containing all nodes.
    /// * `scope` - The current scope where the symbol declarations and usages apply.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if the initialization is successful,
    /// or a `ParserInternalError` if the AST structure is invalid or unexpected.
    ///
    /// # Errors
    ///
    /// Returns an error if the node is not of kind `TaggedTask`, does not have exactly two children,
    /// or if the children are not of expected kinds (`TaskID` for the first and `Task` for the second).
    fn init_from_tagged_task(
        &mut self,
        node_ref: &NodeRef<AstArenaNode>,
        ast: &TreeArena<AstArenaNode>,
        scope: Scope,
    ) -> Result<(), ParserInternalError> {
        // Ensure the AST node is a tagged task
        //Self::assert_ast_kind(node_ref.node(), &[AstKind::TaggedTask])?;
        //Self::assert_ast_children_number(node_ref.node(), 2, Comparator::Equal)?;

        let children = node_ref.node().children();
        let task_id = ast.try_node_ref(children[0])?;
        //Self::assert_ast_kind(task_id.node(), &[AstKind::TaskID])?;

        // Add the task identifier as a declaration symbol
        self.add_declaration_symbol(&task_id, ast, scope.clone(), None, None)?;

        // Process the actual task
        let task = ast.try_node_ref(children[1])?;
        //Self::assert_ast_kind(task.node(), &[AstKind::Task])?;

        self.init_from_atomic_formula(&task, ast, scope.clone())?;

        Ok(())
    }

    /// Initializes a task ordering constraint from the given AST node.
    ///
    /// This function validates that the AST node is of kind `TaskOrderingConstraint` and contains
    /// exactly two children, each representing a `TaskID`. It registers both task identifiers as
    /// symbol usages within the given scope.
    ///
    /// # Arguments
    ///
    /// * `node_ref` - A reference to the AST node representing the task ordering constraint.
    /// * `ast` - The AST tree containing all nodes.
    /// * `scope` - The current scope used for symbol tracking.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if the initialization is successful,
    /// or a `ParserInternalError` if the AST node is invalid or children are not as expected.
    ///
    /// # Errors
    ///
    /// Returns an error if the node is not of kind `TaskOrderingConstraint`,
    /// if it does not have exactly two children,
    /// or if either child is not of kind `TaskID`.
    fn init_from_task_ordering_constraint(
        &mut self,
        node_ref: &NodeRef<AstArenaNode>,
        ast: &TreeArena<AstArenaNode>,
        scope: Scope,
    ) -> Result<(), ParserInternalError> {
        // Ensure the AST node is a tagged task
        /*Self::assert_ast_kind(
            node_ref.node(),
            &[AstKind::TaskOrderingConstraint],
        )?;*/
        //Self::assert_ast_children_number(node_ref.node(), 2, Comparator::Equal)?;

        let children = node_ref.node().children();
        let t1 = ast.try_node_ref(children[0])?;
        //Self::assert_ast_kind(t1.node(), &[AstKind::TaskID])?;
        self.add_symbol_usage(&t1, ast, scope.clone())?;

        let t2 = ast.try_node_ref(children[1])?;
        //Self::assert_ast_kind(t2.node(), &[AstKind::TaskID])?;
        self.add_symbol_usage(&t2, ast, scope.clone())?;

        Ok(())
    }

    /// Asserts that the AST node's kind is contained within the provided set of valid kinds.
    ///
    /// This function checks if the kind of the provided AST node matches one of the valid kinds
    /// in the `valid_kinds` slice. If the AST node's kind is one of the allowed kinds, the function
    /// returns `Ok(())`. Otherwise, it returns a `ParserInternalError` specifying the invalid
    /// node's kind and the expected kinds.
    ///
    /// # Parameters
    /// - `node`: The AST node to check. This node should have a specific kind that needs validation.
    /// - `valid_kinds`: A slice of valid AST kinds that the node is allowed to have.
    ///
    /// # Returns
    /// - `Ok(())` if the AST node's kind matches one of the valid kinds.
    /// - `Err(ParserInternalError)` if the AST node's kind is not valid, including an error message
    ///   with the actual and expected kinds.
    ///
    /// # Example
    /// ```rust
    /// let result = Self::assert_ast_kind(node, &[AstKind::Predicate, AstKind::DomainName]);
    /// match result {
    ///     Ok(()) => println!("Valid AST node!"),
    ///     Err(e) => println!("Error: {}", e),
    /// }
    /// ```
    fn assert_ast_kind(
        node: &AstArenaNode,
        valid_kinds: &[AstKind],
    ) -> Result<(), ParserInternalError> {
        match node.kind() {
            // Case where the AST node's kind is one of the defined types (Predicate, DomainName, etc.)
            AstKind::DomainName
            | AstKind::ProblemName
            | AstKind::Requirement
            | AstKind::PrimitiveType
            | AstKind::Constant
            | AstKind::Variable
            | AstKind::Predicate
            | AstKind::FunctionSymbol
            | AstKind::ActionSymbol
            | AstKind::DASymbol
            | AstKind::PrefName
            | AstKind::Number
            | AstKind::Assign
            | AstKind::FComp
            | AstKind::Metric
            | AstKind::Operation
            | AstKind::Parallel
            | AstKind::Serial
            // Add for HDDL
            | AstKind::MethodSymbol
            | AstKind::TaskSymbol
            | AstKind::TaskID => {
                // Check if the AST node's kind matches one of the valid kinds
                if valid_kinds.iter().any(|_k| matches!(node.kind(), _k)) {
                    Ok(())
                } else {
                    Err(ParserInternalError::new(format!(
                        "Unexpected AST node '{:?}'. Expected one of {:?}.",
                        node.kind(),
                        valid_kinds
                    )))
                }
            }
            // Standard case: check if the node's kind is in valid_kinds
            kind if valid_kinds.contains(&kind) => Ok(()),
            // Case where the type is not expected
            kind => Err(ParserInternalError::new(format!(
                "Unexpected AST node '{:?}'. Expected one of {:?}.",
                kind, valid_kinds
            ))),
        }
    }

    /// Verifies that the number of children in an AST node satisfies a specified comparison
    /// with an expected length.
    ///
    /// This function checks if the number of children (direct descendants) of the given AST node
    /// matches the expected length according to the provided `Comparator`.
    /// If the comparison fails, it returns a `ParserInternalError` detailing the mismatch.
    ///
    /// # Parameters
    /// - `node`: The AST node whose children count is being validated.
    /// - `expected_len`: The expected number of children.
    /// - `comparator`: The comparison operator used to validate the children count:
    ///   - `Comparator::Equal`: number of children should be exactly equal to `expected_len`.
    ///   - `Comparator::NotEqual`: number of children should not be equal to `expected_len`.
    ///   - `Comparator::Less`: number of children should be less than `expected_len`.
    ///   - `Comparator::Greater`: number of children should be greater than `expected_len`.
    ///   - `Comparator::LessEq`: number of children should be less than or equal to `expected_len`.
    ///   - `Comparator::GreaterEq`: number of children should be greater than or equal to `expected_len`.
    ///
    /// # Returns
    /// - `Ok(())` if the number of children satisfies the comparison.
    /// - `Err(ParserInternalError)` if the validation fails, including details of the expected and actual counts.
    ///
    /// # Example
    /// ```rust
    /// let node = get_some_ast_node();
    /// Self::assert_ast_children_number(&node, 3, Comparator::GreaterEq).unwrap(); // passes if node has 3 or more children
    /// Self::assert_ast_children_number(&node, 2, Comparator::Equal).unwrap_err(); // fails if node doesn't have exactly 2 children
    /// ```
    fn assert_ast_children_number(
        node: &AstArenaNode,
        expected_len: usize,
        comparator: Comparator,
    ) -> Result<(), ParserInternalError> {
        let children_len = node.children().len();

        let is_valid = match comparator {
            Comparator::Equal => children_len == expected_len,
            Comparator::Less => children_len < expected_len,
            Comparator::Greater => children_len > expected_len,
            Comparator::LessEq => children_len <= expected_len,
            Comparator::GreaterEq => children_len >= expected_len,
            Comparator::NotEqual => children_len != expected_len,
        };

        if is_valid {
            Ok(())
        } else {
            Err(ParserInternalError::new(format!(
                "Expected {} children for AST node of kind '{:?}', found {}. Expected comparison: '{:?}'.",
                expected_len,
                node.kind(),
                children_len,
                comparator
            )))
        }
    }
}
