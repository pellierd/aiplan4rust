//! Module responsible for building a [`SymbolTable`] from an abstract syntax tree (AST).
//!
//! This module provides the [`SymbolTableBuilder`] struct, which encapsulates the logic
//! for traversing an [`ArenaAst`] and populating a [`SymbolTable`] with symbol
//! declarations and usages found in the syntax tree.
//!
//! The builder handles different AST node kinds such as domain names, problem names,
//! typed lists, declarations, actions, formulas, and HTN constructs. It maintains
//! scope information and symbol origins, ensuring that the resulting symbol table
//! accurately reflects the structure and semantics of the parsed input.
//!
//! # Usage
//!
//! ```rust,no_run
//! let mut builder = SymbolTableBuilder::new();
//! let symbol_table = builder.build(&ast)?;
//! ```
//!
//! This modular design isolates symbol table construction from other compiler phases,
//! enabling better error handling and easier testing.

use crate::aiplan4rust::core::arena::ArenaNode;
use crate::aiplan4rust::semantic::symbol::{Declaration, Scope, SymbolEntry, SymbolOrigin, Usage};
use crate::aiplan4rust::semantic::symbol_table::{SymbolTableError, SymbolTableOrigin};
use crate::aiplan4rust::semantic::SymbolTable;
use crate::aiplan4rust::syntax::ast::{AstNode, Ast, AstKind};
use crate::aiplan4rust::lang::{Type, TypedSymbol, TypedList};
use crate::aiplan4rust::syntax::tree::NodeRef;

/// A builder for constructing a [`SymbolTable`] from an abstract syntax arena (AST).
///
/// `SymbolTableBuilder` encapsulates the logic for traversing an [`ArenaAst`]
/// and populating a [`SymbolTable`] with symbols extracted from the arena. It
/// identifies the root syntax kind (e.g., Domain or Problem), determines the source
/// of the symbol table, and initializes it accordingly.
///
/// This pattern separates the concerns of AST traversal and symbol table
/// construction, allowing better testability and error handling.
///
/// # Example
///
/// ```rust,no_run
/// let mut builder = SymbolTableBuilder::new();
/// let symbol_table = builder.build(&ast)?;
/// ```
pub(crate) struct SymbolTableBuilder {
    table: SymbolTable,
}


impl SymbolTableBuilder {
    /// Creates a new `SymbolTableBuilder` with an empty symbol table.
    ///
    /// The internal symbol table is initialized with [`SymbolTableOrigin::Unknown`]
    /// as the default origin. The actual origin (`Domain` or `Problem`) will be set
    /// during the build process when the root node of the AST is analyzed.
    ///
    /// # Example
    ///
    /// ```rust
    /// let builder = SymbolTableBuilder::new();
    /// ```
    pub fn new() -> Self {
        SymbolTableBuilder {
            table: SymbolTable::new(),
        }
    }

    /// Returns a shared reference to the internal symbol table.
    ///
    /// This method is primarily intended for internal use during construction to
    /// inspect the current state of the symbol table without allowing modification.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// let builder = SymbolTableBuilder::new();
    /// let table_ref = builder.table();
    /// // Inspect or query `table_ref` as needed.
    /// ```
    fn table(&self) -> &SymbolTable {
        &self.table
    }

    /// Returns a mutable reference to the internal symbol table.
    ///
    /// This method allows modifying the symbol table, such as inserting new symbols,
    /// updating declarations, or adding usages, while building the table.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// let mut builder = SymbolTableBuilder::new();
    /// let table_mut = builder.table_mut();
    /// // Modify `table_mut` as part of the build process.
    /// ```
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
        // Attempt to retrieve the root node of the AST, returning error if none exists
        let root_ref = ast.arena().try_root_node_ref()?;
        let root_node = root_ref.node();

        // Match the root node kind and configure the symbol table's origin and root ID accordingly
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
                // Return an error if the root node kind is not Domain or Problem
                return Err(SymbolTableError::unexpected_ast_kind(
                    root_ref.id(),
                    vec![AstKind::Domain, AstKind::Problem],
                    found,
                ));
            }
        }

        // Initialize the symbol table by recursively processing the AST starting from the root
        self.initialize_from_ast(&root_ref, ast)?;

        // Return the fully constructed symbol table, replacing the internal table with an empty one
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
    /// - `Forall`, `Exists`: Handle quantified expressions and logical scopes.
    /// - `MethodDef`, `TaskDef`: Initialize HTN method and task definitions.
    /// - `Task`, `TaggedTask`: Handle HTN individual tasks and tagged tasks.
    /// - `TaskOrderingConstraint`: Initialize constraints between HTN tasks.
    /// - Other nodes: Recursively process all child nodes.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if all nodes and symbols are successfully processed, or
    /// a `SymbolTableError` if an error occurs at any step.
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
        // Match on the AST node kind to determine the appropriate processing logic
        match node_ref.node().kind() {
            // For domain and problem names, add declaration symbols directly without recursion
            AstKind::DomainName
            | AstKind::ProblemName => {
                self.add_declaration_symbol(node_ref, ast, scope.clone(), None, None)?;
            }

            // For typed lists, use specialized initialization for typed symbols
            AstKind::TypedList => {
                self.init_from_typed_list(node_ref, ast, scope.clone())?;
            }

            // For primitive types, constants, and variables, record symbol usages
            AstKind::PrimitiveType
            | AstKind::Constant
            | AstKind::Variable => {
                self.add_symbol_usage(node_ref, ast, scope.clone())?;
            }

            // For action definitions, initialize action symbols and related info
            AstKind::ActionDef => {
                self.init_from_action_def(node_ref, ast, scope.clone())?;
            }

            // For durative action definitions, handle timing-related action data
            AstKind::DurativeActionDef => {
                self.init_from_durative_action_def(node_ref, ast, scope.clone())?;
            }

            // For atomic formula skeletons, apply custom handling for partial formulas
            AstKind::AtomicFormulaSkeleton => {
                self.init_from_atomic_formula_skeleton(node_ref, ast, scope.clone())?;
            }

            // For atomic formulas and function terms, recurse into their structure
            AstKind::AtomicFormula | AstKind::FunctionTerm => {
                self.init_from_atomic_formula(node_ref, ast, scope.clone())?;
            }

            // For quantifiers (forall, exists), handle logical scoping and variable declarations
            AstKind::Forall | AstKind::Exists => {
                self.init_from_quantified_expression(node_ref, ast, scope.clone())?;
            }

            // For HTN method definitions, initialize method-specific symbols and scopes
            AstKind::MethodDef => {
                self.init_from_method_def(node_ref, ast, scope.clone())?;
            }

            // For HTN task definitions, initialize task-specific symbols and scopes
            AstKind::TaskDef => {
                self.init_from_task_def(node_ref, ast, scope.clone())?;
            }

            // For HTN task instances, handle as atomic formulas
            AstKind::Task => {
                self.init_from_atomic_formula(node_ref, ast, scope.clone())?;
            }

            // For tagged HTN tasks, handle extended metadata
            AstKind::TaggedTask => {
                self.init_from_tagged_task(node_ref, ast, scope.clone())?;
            }

            // For HTN task ordering constraints, initialize constraint symbols
            AstKind::TaskOrderingConstraint => {
                self.init_from_task_ordering_constraint(node_ref, ast, scope.clone())?;
            }

            // For any other kinds, recursively process all child nodes to cover nested syntax
            _ => {
                for child in node_ref.node().children() {
                    self.init_from(&ast.arena().try_node_ref(*child)?, ast, scope.clone())?;
                }
            }
        }

        // Indicate successful completion
        Ok(())
    }


    /// Adds a new symbol declaration to the symbol table.
    ///
    /// This function extracts the symbol's name and kind from the given AST syntax node
    /// by using `try_symbol_ref`. It validates the AST syntax kind (implicitly via
    /// `try_symbol_ref`), then either updates an existing symbol in the symbol table by
    /// adding a new declaration or creates a new symbol with the declaration and inserts it.
    ///
    /// # Arguments
    ///
    /// * `node_ref` - A reference to the AST syntax node representing the symbol declaration.
    /// * `ast` - The abstract syntax arena (`ArenaAst`) containing the syntax.
    /// * `scope` - The current scope in which the symbol is declared; this affects visibility and resolution.
    /// * `types` - Optional vector of identifier types associated with the symbol declaration.
    /// * `arguments` - Optional vector of typed symbols representing the symbol's arguments, for functions.
    ///
    /// # Returns
    ///
    /// * `Ok(())` if the symbol declaration is successfully added or updated in the symbol table.
    /// * `Err(SymbolTableError)` if the AST syntax kind is invalid, symbol extraction fails,
    ///   or if insertion into the symbol table encounters an error.
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

        // Extract the symbol reference from the AST node ID.
        // This retrieves symbol metadata such as the identifier name and kind.
        let symbol_ref = ast.arena().try_symbol_ref(node_ref.id())?;

        // Obtain the symbol's identifier (name) from the symbol reference.
        let ident = symbol_ref.ident();

        // Determine the origin context of this symbol declaration.
        // This helps track where the symbol was declared, for error reporting and resolution.
        let origin = SymbolOrigin::from(self.table().origin());

        // Check if the symbol already exists in the symbol table.
        // If it exists, add a new declaration to the existing symbol.
        if let Some(symbol) = self.table_mut().get_symbol_mut(ident) {
            // Create a new declaration instance for this symbol with provided metadata.
            let declaration = Declaration::new(
                symbol_ref,
                scope,
                origin,
                types,
                arguments,
                node_ref.node().span().clone(),  // Source code span for error diagnostics.
                node_ref.id(),                   // AST node identifier.
                None                            // Optional additional data (currently None).
            );
            // Append this declaration to the existing symbol's declarations list.
            symbol.add_declaration(declaration);
        } else {
            // If the symbol does not exist, create a new symbol entry.
            let mut symbol = SymbolEntry::new(ident);

            // Create a new declaration for the new symbol.
            let declaration = Declaration::new(
                symbol_ref,
                scope,
                origin,
                types,
                arguments,
                node_ref.node().span().clone(),
                node_ref.id(),
                None,
            );

            // Add the declaration to the symbol.
            symbol.add_declaration(declaration);

            // Insert the new symbol entry into the symbol table.
            self.table_mut().insert_symbol(ident, symbol);
        }

        // Return success indicating the symbol declaration was added properly.
        Ok(())
    }


    /// Adds the usage of a symbol found in the given AST syntax to the symbol table.
    ///
    /// This function processes AST nodes representing symbol usages, such as domain names,
    /// problem names, constants, variables, atomic formulas, function terms, and tasks.
    /// It first validates the AST syntax kind to ensure it is appropriate for symbol usage.
    /// For certain syntax kinds that represent complex expressions (e.g., atomic formulas),
    /// it extracts the first child node to retrieve the symbol reference.
    ///
    /// The symbol's identifier and kind are extracted, and the function updates
    /// the symbol table accordingly:
    /// - If the symbol already exists, the new usage is appended.
    /// - Otherwise, a new symbol entry is created and the usage recorded.
    ///
    /// # Arguments
    ///
    /// * `node_ref` - A reference to the AST node representing the symbol usage.
    /// * `ast` - Reference to the complete AST arena (`ArenaAst`).
    /// * `scope` - The current scope in which the symbol usage occurs, controlling visibility.
    ///
    /// # Returns
    ///
    /// * `Ok(())` if the usage was successfully recorded.
    /// * `Err(SymbolTableError)` if the AST kind is invalid, structure is incorrect,
    ///   or symbol extraction fails.
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
        // Determine the symbol reference based on the AST node kind.
        // For complex kinds (AtomicFormula, FunctionTerm, Task), extract the first child node.
        let symbol_ref = if matches!(
        node_ref.node().kind(),
        AstKind::AtomicFormula | AstKind::FunctionTerm | AstKind::Task
    ) {
            // Retrieve the first child node ID
            let first_child_id = node_ref.node().children()[0];
            // Get a reference to the first child node
            let first_node_ref = ast.arena().try_node_ref(first_child_id)?;
            // Extract the symbol reference from the first child node
            ast.arena().try_symbol_ref(first_node_ref.id())?
        } else {
            // For other kinds, get the symbol reference directly from this node
            ast.arena().try_symbol_ref(node_ref.id())?
        };

        // Extract the identifier (name) of the symbol
        let ident = symbol_ref.ident();

        // Retrieve the origin of the symbol (context/source of declaration)
        let origin = SymbolOrigin::from(self.table().origin());

        // If the symbol already exists in the symbol table, add a new usage record
        if let Some(symbol) = self.table_mut().get_symbol_mut(ident) {
            let usage = Usage::new(
                symbol_ref,
                scope,
                origin,
                node_ref.node().span().clone(), // Source span for error reporting/tracking
                node_ref.id(),                   // AST node ID
            );
            symbol.add_usage(usage);
        } else {
            // Otherwise, create a new symbol entry and record the first usage
            let mut symbol = SymbolEntry::new(ident);
            let usage = Usage::new(
                symbol_ref,
                scope,
                origin,
                node_ref.node().span().clone(),
                node_ref.id(),
            );
            symbol.add_usage(usage);
            // Insert the new symbol entry into the symbol table
            self.table_mut().insert_symbol(ident, symbol);
        }

        Ok(())
    }


    /// Initializes the symbol table from a `TypedList` AST syntax node.
    ///
    /// This function processes a `TypedList` node by iterating over its children, each representing
    /// a typed item such as a type declaration, constant, or variable. It recursively initializes
    /// symbol declarations for each child within the given scope.
    ///
    /// # Parameters
    ///
    /// * `node_ref` - Reference to the `TypedList` AST node to process.
    /// * `ast` - Reference to the full AST arena (`ArenaAst`) containing the syntax.
    /// * `scope` - The current scope in which the symbols are being declared.
    ///
    /// # Returns
    ///
    /// * `Ok(())` if all children are successfully processed and the symbol table is updated accordingly.
    /// * `Err(SymbolTableError)` if the node is not of the expected kind or any child node fails to process.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The AST node is not a `TypedList`.
    /// - Any child node cannot be processed as a typed item.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// let typed_list_node = ast.try_node_ref(node_id)?;
    /// symbol_table.init_from_typed_list(typed_list_node, &ast, current_scope)?;
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

        for &child_id in children {
            let child_ref = ast.arena().try_node_ref(child_id)?;
            self.init_from_typed_item(&child_ref, ast, scope.clone())?;
        }

        Ok(())
    }

    /// Initializes symbol declarations from a `TypedItem` AST syntax node.
    ///
    /// A `TypedItem` typically represents a declaration associating one or more symbols
    /// (e.g., variables or constants) with an optional type annotation. For example:
    /// ```text
    /// (?x ?y - location)  // symbols: ?x, ?y; type: location
    /// (?z)                // symbol: ?z; no associated type
    /// ```
    ///
    /// This function parses the symbols and their optional type, then registers them
    /// in the symbol table within the given scope.
    ///
    /// # Parameters
    ///
    /// * `node_ref` — Reference to the `TypedItem` AST node to process.
    /// * `ast` — Reference to the AST arena (`ArenaAst`) containing the full syntax.
    /// * `scope` — The current symbol table scope where declarations are added. It may be cloned
    ///   for nested or recursive calls.
    ///
    /// # Behavior
    ///
    /// - Validates that the node kind is `TypedItem`.
    /// - Examines the number of children:
    ///   - If there is exactly one child, treats it as an untyped declaration (empty type list).
    ///   - If there are exactly two children, parses the second child as the type annotation.
    ///   - Returns an error if the number of children differs from 1 or 2.
    /// - Processes the first child as the list of symbols, associating them with the parsed types.
    /// - Adds each symbol declaration to the symbol table under the given scope.
    ///
    /// # Errors
    ///
    /// Returns `SymbolTableError` if:
    /// - The node kind is not `TypedItem`.
    /// - The number of children is invalid (not 1 or 2).
    /// - The type annotation fails to parse correctly.
    /// - Adding declarations to the symbol table fails.
    ///
    /// # Example
    ///
    /// ```text
    /// // Declare ?x and ?y with type 'location'
    /// (?x ?y - location)
    ///
    /// // Declare ?z with no associated type
    /// (?z)
    /// ```
    ///
    fn init_from_typed_item(
        &mut self,
        node_ref: &NodeRef<AstNode>,
        ast: &Ast,
        scope: Scope,
    ) -> Result<(), SymbolTableError> {
        let syntax_tree = ast.arena();
        let node = node_ref.node();

        // Determine the types associated with the symbols
        let types = match node.arity() {
            1 => Type::new(), // No type annotation, empty type vector
            2 => {
                // Parse the type annotation from the second child
                let ty_id = node.try_child(1)?;
                self.init_from_type(&syntax_tree.try_node_ref(ty_id)?, ast, scope.clone())?
            },
            n => {
                // Invalid number of children for TypedItem node
                return Err(SymbolTableError::invalid_typed_item_arity(node_ref.id(), n));
            }
        };

        // Process the first child node as the list of symbols to declare, associating with extracted types
        let elt_id = node.try_child(0)?;
        self.init_from_typed_item_elements(&syntax_tree.try_node_ref(elt_id)?, ast, scope, types)
    }


    /// Helper function to process an individual element of a `TypedList`.
    ///
    /// This function handles different kinds of typed elements, adding them to the symbol table
    /// according to their kind and associated types. Supported kinds include:
    /// - `PrimitiveType`
    /// - `Constant`
    /// - `Variable`
    /// - `AtomicFunctionSkeleton` (recursively initialized)
    ///
    /// # Parameters
    ///
    /// * `node_ref` — Reference to the AST node representing the element to process. Expected kinds:
    ///   `PrimitiveType`, `Constant`, `Variable`, or `AtomicFunctionSkeleton`.
    /// * `ast` — Reference to the AST arena containing the syntax.
    /// * `scope` — The current symbol table scope where the element should be declared.
    /// * `types` — Vector of type checker identifiers associated with the element.
    ///
    /// # Behavior
    ///
    /// - Validates that the node kind is one of the accepted types.
    /// - Adds a declaration symbol for primitive types, constants, and variables using the provided types.
    /// - For `AtomicFunctionSkeleton`, recursively initializes its symbol table entries.
    /// - Returns an error if the node kind is unexpected.
    ///
    /// # Returns
    ///
    /// * `Ok(())` if the element is processed successfully.
    /// * `Err(SymbolTableError)` if the AST kind is invalid or any symbol table operation fails.
    fn init_from_typed_item_elements(
        &mut self,
        node_ref: &NodeRef<AstNode>,
        ast: &Ast,
        scope: Scope,
        types: Type,
    ) -> Result<(), SymbolTableError> {
        // Match on the AST node kind to determine processing logic
        match node_ref.node().kind() {
            AstKind::PrimitiveType | AstKind::Constant | AstKind::Variable => {
                // Add a declaration symbol with the provided types for simple typed elements
                self.add_declaration_symbol(node_ref, ast, scope.clone(), Some(types.clone()), None)?;
            }

            AstKind::AtomicFunctionSkeleton => {
                // Recursively initialize symbols for atomic function skeleton elements
                self.init_from_atomic_function_skeleton(node_ref, ast, scope.clone(), types.clone())?;
            }

            found => {
                // Return an error for unexpected AST kinds instead of panicking
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
    /// This function processes an AST node of kind `AtomicFunctionSkeleton` by:
    /// - Verifying the node kind is `AtomicFunctionSkeleton`.
    /// - Ensuring the node has at least two children: the function symbol and its argument list.
    /// - Validating that the first child is a `FunctionSymbol`.
    /// - Creating a nested scope for the function and initializing the symbol table for its arguments.
    /// - Extracting argument symbols and computing their arity.
    /// - Adding a function declaration to the symbol table with associated types and arguments.
    ///
    /// # Parameters
    ///
    /// * `node_ref` — Reference to the AST node representing the `AtomicFunctionSkeleton`.
    /// * `ast` — Reference to the AST arena containing all syntax nodes.
    /// * `scope` — The current scope in which the function is declared.
    /// * `types` — A vector of type checker identifiers associated with the function.
    ///
    /// # Returns
    ///
    /// * `Ok(())` if initialization succeeds.
    /// * `Err(SymbolTableError)` if validation fails, such as:
    ///   - Node kind is not `AtomicFunctionSkeleton`.
    ///   - Node has fewer than two children.
    ///   - First child is not a `FunctionSymbol`.
    ///
    /// # Example
    ///
    /// ```rust
    /// let ast_node = /* AST node for AtomicFunctionSkeleton */;
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
    ///
    /// Returns a `ParserInternalError` if the node kind or structure is invalid.
    ///
    /// # Notes
    ///
    /// - Relies on `init_from_typed_list` to initialize and validate argument symbols.
    /// - Critical for managing function declarations and scoping within the symbol table.
    fn init_from_atomic_function_skeleton(
        &mut self,
        node_ref: &NodeRef<AstNode>,
        ast: &Ast,
        scope: Scope,
        types: Type,
    ) -> Result<(), SymbolTableError> {
        let node = node_ref.node();

        // Retrieve and validate the first child as a FunctionSymbol
        let functor_id = node.try_child(0)?;
        let functor_ref = ast.arena().try_node_ref(functor_id)?;
        if functor_ref.node().kind() != AstKind::FunctionSymbol {
            return Err(SymbolTableError::unexpected_ast_kind(
                functor_ref.id(),
                vec![AstKind::FunctionSymbol],
                functor_ref.node().kind(),
            ));
        }

        // Retrieve the argument list node (second child)
        let arguments_id = node.try_child(1)?;
        let arguments = &ast.arena().try_node_ref(arguments_id)?;

        // Initialize symbols for the argument list in a new nested scope
        self.init_from_typed_list(
            arguments,
            ast,
            Scope::new(node_ref.id(), Some(&scope)),
        )?;

        // Extract typed argument symbols and compute arity
        let arguments = self.extract_arguments_from_typed_list(arguments, ast)?;

        // Add the function declaration symbol with types and arguments
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
    /// This function processes an AST node representing an action or method definition,
    /// such as `ActionDef`, `DurativeActionDef`, or `MethodDef`. It extracts the name,
    /// parameters, and body, adding the relevant symbols to the symbol table.
    ///
    /// # Parameters
    ///
    /// * `node_ref` — Reference to the AST node for the action definition.
    /// * `ast` — Reference to the AST arena containing all syntax nodes.
    /// * `scope` — The current scope in which the action or method is defined.
    ///
    /// # Returns
    ///
    /// * `Ok(())` if initialization completes successfully.
    /// * `Err(SymbolTableError)` if the AST node does not have the expected kind or structure.
    ///
    /// # AST Structure
    ///
    /// The node is expected to have exactly three children:
    /// 1. **Name** — The identifier of the action or method, added as a declaration symbol.
    /// 2. **Parameters** — The parameter list, recursively processed.
    /// 3. **Body** — The body of the action or method, recursively processed.
    ///
    /// # Example
    ///
    /// ```rust
    /// // Assuming `node_ref`, `ast`, and `scope` are properly initialized:
    /// symbol_table.init_from_action_def(node_ref, &ast, scope)?;
    /// ```
    ///
    /// This function delegates the core work to `init_from_def`, indicating that the
    /// definition includes a body.
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
            true, // The definition includes a body
        )
    }

    /// Initializes the symbol table for a method definition.
    ///
    /// This function verifies that the given AST node is of kind `MethodDef` and contains exactly
    /// three children representing the method's name, parameters, and body. It processes these
    /// components to update the symbol table accordingly.
    ///
    /// # Parameters
    ///
    /// * `node_ref` — Reference to the AST node representing the method definition.
    /// * `ast` — Reference to the AST arena containing all nodes.
    /// * `scope` — The current scope in which the method is defined.
    ///
    /// # Returns
    ///
    /// * `Ok(())` if the initialization completes successfully.
    /// * `Err(SymbolTableError)` if the AST node kind or structure is invalid.
    ///
    /// # AST Structure
    ///
    /// The node is expected to have exactly three children:
    /// 1. **Name** — The method's identifier, added as a declaration symbol.
    /// 2. **Parameters** — The method parameters, recursively processed.
    /// 3. **Body** — The method body, recursively processed.
    ///
    /// # Example
    ///
    /// ```rust
    /// // Given `node_ref`, `ast`, and `scope`
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
            true, // Method definitions have a body
        )
    }

    /// Initializes the symbol table for a durative action definition.
    ///
    /// This function verifies that the given AST node is of kind `DurativeActionDef`
    /// and contains exactly three children representing the action's name, parameters, and body.
    /// It processes these components to update the symbol table accordingly.
    ///
    /// # Parameters
    ///
    /// * `node_ref` — Reference to the AST node representing the durative action definition.
    /// * `ast` — Reference to the AST arena containing all nodes.
    /// * `scope` — The current scope in which the durative action is defined.
    ///
    /// # Returns
    ///
    /// * `Ok(())` if the initialization completes successfully.
    /// * `Err(SymbolTableError)` if the AST node kind or structure is invalid.
    ///
    /// # AST Structure
    ///
    /// The node is expected to have exactly three children:
    /// 1. **Name** — The durative action's identifier, added as a declaration symbol.
    /// 2. **Parameters** — The action parameters, recursively processed.
    /// 3. **Body** — The action body, recursively processed.
    ///
    /// # Example
    ///
    /// ```rust
    /// // Given `node_ref`, `ast`, and `scope`
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
            true, // Durative actions have a body
        )
    }

    /// Initializes the symbol table for a task definition.
    ///
    /// This function verifies that the given AST node is of kind `TaskDef`
    /// and contains exactly two children: the task name and its parameters.
    /// Since task definitions do not include a body, only these components are processed
    /// and added to the symbol table.
    ///
    /// # Parameters
    ///
    /// * `node_ref` — Reference to the AST node representing the task definition.
    /// * `ast` — Reference to the AST arena containing all nodes.
    /// * `scope` — The current scope in which the task is defined.
    ///
    /// # Returns
    ///
    /// * `Ok(())` if the initialization succeeds.
    /// * `Err(SymbolTableError)` if the AST node kind or structure is invalid.
    ///
    /// # AST Structure
    ///
    /// The node is expected to have exactly two children:
    /// 1. **Name** — The identifier of the task, added as a declaration symbol.
    /// 2. **Parameters** — The task parameters, recursively processed.
    ///
    /// # Example
    ///
    /// ```rust
    /// // Given `node_ref`, `ast`, and `scope`
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
            false, // Tasks do not have a body
        )
    }

    /// Initializes a definition syntax node (e.g., `ActionDef`, `DurativeActionDef`, `MethodDef`, or `TaskDef`)
    /// by extracting its name, parameters, and optionally its body, updating the symbol table accordingly.
    ///
    /// This function performs these steps:
    /// - Validates the AST node kind externally (not within this function).
    /// - Checks the node has the expected children count externally (not within this function).
    /// - Extracts the definition name and registers it as a declaration in the current scope.
    /// - Recursively initializes the symbol table for the parameter list (`TypedList`).
    /// - Extracts the typed parameters from the parameter list.
    /// - Adds the definition symbol with its parameters to the current scope.
    /// - If `has_body` is true, recursively initializes the symbol table for the body.
    ///
    /// # Parameters
    ///
    /// * `node_ref` — Reference to the AST node representing the definition.
    /// * `ast` — The AST arena containing all nodes.
    /// * `scope` — Current symbol table scope for resolving symbols and declarations.
    /// * `has_body` — Indicates if the definition includes a body that requires initialization.
    ///
    /// # Returns
    ///
    /// * `Ok(())` if initialization completes successfully.
    /// * `Err(SymbolTableError)` if any AST traversal or symbol table operation fails.
    ///
    /// # AST Node Children Structure
    ///
    /// 1. **Name** — Identifier node of the definition.
    /// 2. **Parameters** — A `TypedList` node containing the parameters.
    /// 3. **Body** (optional) — The body of the definition, present only if `has_body` is `true`.
    ///
    /// # Example
    ///
    /// ```rust
    /// symbol_table.init_from_def(
    ///     node_ref,
    ///     &ast,
    ///     scope,
    ///     true,
    /// )?;
    /// ```
    fn init_from_def(
        &mut self,
        node_ref: &NodeRef<AstNode>,
        ast: &Ast,
        scope: Scope,
        has_body: bool,
    ) -> Result<(), SymbolTableError> {
        let syntax_tree = ast.arena();
        let node = node_ref.node();

        // Extract the definition name (first child) and prepare to add it as a declaration
        let name_id = node.try_child(0)?;
        let name = syntax_tree.try_node_ref(name_id)?;

        // Extract the parameters definition node (second child), then the parameter list inside it
        let parameters_def_id = node.try_child(1)?;
        let parameters_def = &syntax_tree.try_node_ref(parameters_def_id)?;
        let parameters_id = parameters_def.node().try_child(0)?;
        let parameters = &syntax_tree.try_node_ref(parameters_id)?;

        // Initialize the symbol table for the parameters in a nested scope
        self.init_from_typed_list(parameters, ast, Scope::new(node_ref.id(), Some(&scope)))?;

        // Extract typed symbols from parameters for declaration
        let parameters = self.extract_arguments_from_typed_list(parameters, ast)?;

        // Add the definition name as a symbol declaration with its parameters
        self.add_declaration_symbol(&name, ast, scope.clone(), None, Some(parameters))?;

        // If present, recursively initialize the body of the definition
        if has_body {
            let body_id = node.try_child(2)?;
            let body = &syntax_tree.try_node_ref(body_id)?;
            self.init_from(body, ast, Scope::new(node_ref.id(), Some(&scope)))?;
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

    /// Initializes the symbol table for a quantified expression (`Exists` or `Forall`) in the AST.
    ///
    /// A quantified expression must follow this structure:
    /// 1. A `TypedList` node representing the quantified variables.
    /// 2. An expression node representing the body of the quantifier.
    ///
    /// This function:
    /// - Validates the kind of the node (must be `Exists` or `Forall`).
    /// - Verifies that exactly two children are present.
    /// - Creates a new nested scope for the quantified variables.
    /// - Initializes symbol information for the typed variables.
    /// - Recursively initializes the symbol table for the inner expression.
    ///
    /// # Parameters
    ///
    /// * `node_ref` - Reference to the quantified expression AST node.
    /// * `ast` - The AST arena containing all nodes and symbols.
    /// * `scope` - The current scope within which this quantified expression is nested.
    ///
    /// # Returns
    ///
    /// * `Ok(())` on successful initialization.
    /// * `Err(SymbolTableError)` if the AST is malformed or initialization fails.
    ///
    /// # Errors
    ///
    /// Returns a `SymbolTableError` if:
    /// - The node kind is not `Exists` or `Forall`.
    /// - The node does not have exactly two children.
    /// - Initialization of the variable list or expression fails.
    ///
    /// # Example
    ///
    /// ```rust
    /// symbol_table.init_from_quantified_expression(&quant_node, &ast, current_scope)?;
    /// ```
    fn init_from_quantified_expression(
        &mut self,
        node_ref: &NodeRef<AstNode>,
        ast: &Ast,
        scope: Scope,
    ) -> Result<(), SymbolTableError> {
        let syntax_tree = ast.arena();
        let node = node_ref.node();

        // Ensure the node is a quantified expression: `Exists` or `Forall`
        match node.kind() {
            AstKind::Exists | AstKind::Forall => {},
            other => {
                return Err(SymbolTableError::UnexpectedAstKind {
                    expected: vec![AstKind::Exists, AstKind::Forall],
                    found: other,
                    node_id: node_ref.id(),
                });
            }
        }

        // Step 1: Extract the two required children: variable declarations and inner expression
        let variables_id = node.try_child(0)?;
        let variables = &syntax_tree.try_node_ref(variables_id)?;

        let expression_id = node.try_child(1)?;
        let expression = &syntax_tree.try_node_ref(expression_id)?;

        // Step 2: Create a nested scope for the quantified variables
        let nested_scope = Scope::new(node_ref.id(), Some(&scope));

        // Step 3: Initialize symbol table entries for the quantified variables
        self.init_from_typed_list(variables, ast, nested_scope.clone())?;

        // Step 4: Recursively initialize the inner expression within the same nested scope
        self.init_from(expression, ast, nested_scope)?;

        Ok(())
    }


    /// Initializes the symbol table for an atomic formula skeleton from the AST.
    ///
    /// An atomic formula skeleton consists of a `Predicate` followed by a `TypedList`
    /// of arguments. This function performs the following:
    ///
    /// 1. Validates that the node has exactly two children.
    /// 2. Ensures the first child is a `Predicate`.
    /// 3. Initializes symbol information for the argument list.
    /// 4. Registers the predicate as a declaration symbol, with its associated typed arguments.
    ///
    /// # Arguments
    ///
    /// * `node_ref` - Reference to the AST node representing the atomic formula skeleton.
    /// * `ast` - The AST arena providing access to nodes and symbols.
    /// * `scope` - The current scope used for symbol resolution.
    ///
    /// # Returns
    ///
    /// * `Ok(())` if symbol table construction succeeds.
    /// * `Err(SymbolTableError)` if the AST structure is unexpected or initialization fails.
    ///
    /// # Errors
    ///
    /// Returns [`SymbolTableError`] in the following cases:
    /// - If child nodes cannot be retrieved.
    /// - If the first child is not of kind `Predicate`.
    /// - If argument list extraction fails or is malformed.
    ///
    /// # Example
    ///
    /// ```rust
    /// symbol_table.init_from_atomic_formula_skeleton(&formula_node, &ast, current_scope)?;
    /// ```
    fn init_from_atomic_formula_skeleton(
        &mut self,
        node_ref: &NodeRef<AstNode>,
        ast: &Ast,
        scope: Scope,
    ) -> Result<(), SymbolTableError> {
        let syntax_tree = ast.arena();
        let node = node_ref.node();

        // Step 1: Get and validate the predicate node (first child)
        let predicate_id = node.try_child(0)?;
        let predicate = &syntax_tree.try_node_ref(predicate_id)?;

        if predicate.node().kind() != AstKind::Predicate {
            return Err(SymbolTableError::UnexpectedAstKind {
                expected: vec![AstKind::Predicate],
                found: predicate.node().kind(),
                node_id: predicate.id(),
            });
        }

        // Step 2: Get the argument list node (second child)
        let arguments_id = node.try_child(1)?;
        let arguments = &syntax_tree.try_node_ref(arguments_id)?;

        // Step 3: Initialize symbols from the argument list (typed variables/constants)
        self.init_from_typed_list(
            arguments,
            ast,
            Scope::new(node_ref.id(), Some(&scope)),
        )?;

        // Step 4: Extract typed arguments from the list
        let extracted_arguments = self.extract_arguments_from_typed_list(arguments, ast)?;

        // Step 5: Register the predicate symbol declaration with its arguments
        self.add_declaration_symbol(
            predicate,
            ast,
            scope,
            None,
            Some(extracted_arguments),
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

    /// Extracts a list of `TypedSymbol`s from a `TypedItem` AST node.
    ///
    /// A `TypedItem` syntax node is expected to have:
    /// - A first child of kind `Constant` or `Variable` representing the symbol name.
    /// - An optional second child of kind `Type` representing associated types.
    ///
    /// This function validates the structure of the node, extracts the name and types,
    /// and returns a list containing the resulting [`TypedSymbol`]s.
    ///
    /// # Arguments
    ///
    /// * `typed_item_ref` - Reference to the `TypedItem` node.
    /// * `ast` - The AST arena providing access to nodes and symbol table.
    ///
    /// # Returns
    ///
    /// A [`TypedList`] containing one typed symbol extracted from the node.
    ///
    /// # Errors
    ///
    /// Returns [`SymbolTableError`] in the following cases:
    /// - The node has an unexpected number of children (not 1 or 2).
    /// - The first child is not a `Constant` or `Variable`.
    /// - A type extraction fails.
    ///
    /// # Example
    ///
    /// ```rust
    /// let typed_symbols = self.extract_arguments_from_typed_item(&typed_item_ref, &ast)?;
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

        // Step 1: Extract the type information if present
        let types = match node.arity() {
            1 => Type::new(), // No type specified → assume empty (default) type
            2 => {
                let ty_id = node.try_child(1)?; // Get the type node
                let ty_node_ref = syntax_tree.try_node_ref(ty_id)?;
                self.extract_type(&ty_node_ref, ast)? // Extract types from it
            }
            n => {
                // Invalid arity for a TypedItem node
                return Err(SymbolTableError::invalid_typed_item_arity(
                    typed_item_ref.id(),
                    n,
                ));
            }
        };

        // Step 2: Extract the identifier (Constant or Variable)
        let elt_id = node.try_child(0)?;
        let elt = syntax_tree.try_node_ref(elt_id)?;

        let mut typed_arguments = TypedList::new();

        match elt.node().kind() {
            AstKind::Constant | AstKind::Variable => {
                let symbol_ref = syntax_tree.try_symbol_ref(elt.id())?;
                let name = symbol_ref.ident();

                // Create a TypedSymbol with extracted name and associated types
                typed_arguments.push(TypedSymbol::new(name, types));
            }
            found => {
                // Unexpected kind for symbol part of TypedItem
                return Err(SymbolTableError::UnexpectedAstKind {
                    expected: vec![AstKind::Constant, AstKind::Variable],
                    found,
                    node_id: elt.id(),
                });
            }
        }

        Ok(typed_arguments)
    }

    /// Extracts primitive type_checker identifiers from a `Type` AST node without recording symbol usage.
    ///
    /// This function is used to interpret a type declaration from the AST. It expects the node to be of kind `Type`,
    /// and each of its children to be of kind `PrimitiveType`. The extracted identifiers are collected into a [`Type`] object.
    ///
    /// Unlike `init_from_type`, this function does **not** register symbol usage in the current scope—
    /// it only performs static extraction and validation.
    ///
    /// # Arguments
    ///
    /// * `type_ref` - A reference to the AST node expected to represent a type (`AstKind::Type`).
    /// * `ast` - The complete AST arena, used to access nodes and symbol interners.
    ///
    /// # Returns
    ///
    /// Returns a [`Type`] structure containing all extracted primitive identifiers on success.
    ///
    /// # Errors
    ///
    /// Returns [`SymbolTableError::UnexpectedAstKind`] if:
    /// - Any child node is not of kind `PrimitiveType`.
    /// - A referenced node or symbol is invalid in the arena.
    fn extract_type(
        &mut self,
        type_ref: &NodeRef<AstNode>,
        ast: &Ast,
    ) -> Result<Type, SymbolTableError> {
        let arena = ast.arena();
        let mut super_types = Type::new();

        // Iterate over each child of the Type node (expected to be PrimitiveType)
        for ty_id in type_ref.node().children() {
            let ty_ref = arena.try_node_ref(*ty_id)?;

            // Expect each child to be of kind PrimitiveType
            if ty_ref.node().kind() == AstKind::PrimitiveType {
                let symbol_ref = arena.try_symbol_ref(ty_ref.id())?; // Get the symbol associated with this type
                let name = symbol_ref.ident();
                super_types.add_type(name);
            } else {
                // Return a semantic error when the structure does not match expectations
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
    /// This function processes an AST node of kind `Type`, which is expected to list one or more
    /// `PrimitiveType` identifiers (e.g., type names). It extracts all these identifiers,
    /// constructs a `Type` object containing them, and registers each as a symbol usage in the provided scope.
    ///
    /// # Arguments
    ///
    /// * `type_ref` - A reference to the AST node expected to be of kind `Type`.
    /// * `ast` - The arena-backed AST structure used for looking up child nodes and interning.
    /// * `scope` - The current lexical scope where type usages should be recorded.
    ///
    /// # Returns
    ///
    /// Returns a [`Type`] object containing all extracted type identifiers if successful.
    ///
    /// # Errors
    ///
    /// Returns a [`SymbolTableError`] if:
    /// - The `type_ref` node is invalid or has children that are not of kind `PrimitiveType`.
    /// - A child node reference is invalid or symbol usage registration fails.
    fn init_from_type(
        &mut self,
        type_ref: &NodeRef<AstNode>,
        ast: &Ast,
        scope: Scope,
    ) -> Result<Type, SymbolTableError> {
        // --- Extract the type identifiers using existing logic ---
        let super_types = self.extract_type(type_ref, ast)?; // Handles structure & kind checking internally

        // --- Register each primitive type as a symbol usage ---
        for ty in type_ref.node().children() {
            let ty_ref = ast.arena().try_node_ref(*ty)?;  // Get reference to each type node
            self.add_symbol_usage(&ty_ref, ast, scope.clone())?; // Track usage in the current scope
        }

        Ok(super_types)
    }


    /// Initializes the symbol table from a tagged task definition in the AST.
    ///
    /// A `TaggedTask` defines a task with an explicit identifier. This function validates the structure
    /// of the AST node, ensures it has exactly two children (a task identifier and the task itself),
    /// and then:
    /// 1. Registers the identifier as a symbol declaration in the current scope.
    /// 2. Initializes symbol references within the inner task definition.
    ///
    /// # Arguments
    ///
    /// * `node_ref` - Reference to the AST node of kind `TaggedTask`.
    /// * `ast` - The full abstract syntax tree containing all arena-managed nodes.
    /// * `scope` - The current lexical scope for symbol declarations and references.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if initialization is successful.
    ///
    /// # Errors
    ///
    /// Returns a [`SymbolTableError`] if:
    /// - The `TaggedTask` node does not contain exactly two children.
    /// - A child node is not valid or cannot be referenced.
    /// - Adding the declaration or initializing the task fails.
    fn init_from_tagged_task(
        &mut self,
        node_ref: &NodeRef<AstNode>,
        ast: &Ast,
        scope: Scope,
    ) -> Result<(), SymbolTableError> {
        let syntax_tree = ast.arena();
        let node = node_ref.node();

        // --- Extract the tag identifier child (expected to be a TaskID) ---
        let tag_id = node.try_child(0)?;                  // Error if missing
        let tag = syntax_tree.try_node_ref(tag_id)?;      // Error if invalid node reference

        // --- Register the tag as a declaration symbol in the current scope ---
        self.add_declaration_symbol(&tag, ast, scope.clone(), None, None)?; // May fail if duplicate, invalid kind, etc.

        // --- Extract the task definition (second child) ---
        let task_id = node.try_child(1)?;                 // Error if missing
        let task = syntax_tree.try_node_ref(task_id)?;    // Error if invalid reference

        // --- Initialize symbols for the task formula (likely a predicate or action expression) ---
        self.init_from_atomic_formula(&task, ast, scope.clone())?; // Recursively builds symbol table for inner task

        Ok(())
    }


    /// Initializes a task ordering constraint from the given AST syntax node.
    ///
    /// This method expects the AST node to be of kind `TaskOrderingConstraint`, which must
    /// have exactly two child nodes, each representing a `TaskID`. It registers both task
    /// identifiers as symbol usages within the provided scope using the symbol table.
    ///
    /// This ensures that all task references used in ordering constraints are properly tracked
    /// for later semantic validation.
    ///
    /// # Arguments
    ///
    /// * `node_ref` - A reference to the AST node representing the `TaskOrderingConstraint`.
    /// * `ast` - The complete abstract syntax tree (arena-based) containing all nodes.
    /// * `scope` - The current lexical scope used to register symbol usages.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if both task identifiers are correctly extracted and registered.
    ///
    /// # Errors
    ///
    /// Returns a [`SymbolTableError`] if:
    /// - One of the expected child nodes is missing.
    /// - The referenced child nodes are of an unexpected kind.
    /// - Registering the symbol usage fails.
    fn init_from_task_ordering_constraint(
        &mut self,
        node_ref: &NodeRef<AstNode>,
        ast: &Ast,
        scope: Scope,
    ) -> Result<(), SymbolTableError> {
        let syntax_tree = ast.arena();

        // --- Extract the first child node (expected to be a TaskID) ---
        let t1_id = node_ref.node().try_child(0)?; // Error if no first child
        let t1 = syntax_tree.try_node_ref(t1_id)?; // Error if invalid node reference
        self.add_symbol_usage(&t1, ast, scope.clone())?; // Registers t1 as a symbol usage

        // --- Extract the second child node (also expected to be a TaskID) ---
        let t2_id = node_ref.node().try_child(1)?; // Error if no second child
        let t2 = syntax_tree.try_node_ref(t2_id)?; // Error if invalid node reference
        self.add_symbol_usage(&t2, ast, scope.clone())?; // Registers t2 as a symbol usage

        Ok(())
    }
}
