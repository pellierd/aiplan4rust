use crate::aiplan4rust::frontend::ParserInternalError;
use crate::aiplan4rust::semantics::ast_table::{AstEntry, AstTable};
use crate::aiplan4rust::semantics::scope::Scope;
use crate::aiplan4rust::semantics::symbol::{
    Declaration, FilterableSymbol, Symbol, SymbolKind, Usage,
};
use crate::aiplan4rust::semantics::typed_symbol::TypedSymbol;
use crate::aiplan4rust::syntax::ast::BinaryComp;
use crate::aiplan4rust::syntax::ast::{AstKind, Requirement};
use crate::aiplan4rust::syntax::token::TOTAL_TIME;
use linked_hash_map::LinkedHashMap;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::hash::Hash;

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

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
/// A table of symbols used by the aiplan4rust.
///
/// This structure maintains an ordered mapping from unique keys to symbols.
/// It provides methods to insert, retrieve, and serialize symbols during parsing.
pub struct SymbolTable {
    symbols: LinkedHashMap<String, Symbol>,
}

impl SymbolTable {
    /// Creates a new, empty `SymbolTable`.
    ///
    /// # Returns
    ///
    /// A new instance of `SymbolTable` with no symbols.
    pub fn new() -> Self {
        SymbolTable {
            symbols: LinkedHashMap::new(),
        }
    }

    /// Inserts a symbol into the symbol table using a unique key.
    ///
    /// # Arguments
    /// * `key` - A unique key for the symbol.
    /// * `symbol` - The symbol to insert.
    pub fn insert_symbol(&mut self, key: String, symbol: Symbol) {
        self.symbols.insert(key, symbol);
    }

    /// Retrieves an immutable reference to a symbol by its key.
    ///
    /// # Arguments
    /// * `name` - The key for the symbol.
    ///
    /// # Returns
    /// An `Option` with a reference to the symbol if it exists.
    pub fn get_symbol(&self, name: &str) -> Option<&Symbol> {
        self.symbols.get(name)
    }

    /// Retrieves a mutable reference to a symbol by its key.
    ///
    /// # Arguments
    /// * `name` - The key for the symbol.
    ///
    /// # Returns
    /// An `Option` with a mutable reference if the symbol exists.
    pub fn get_symbol_mut(&mut self, name: &str) -> Option<&mut Symbol> {
        self.symbols.get_mut(name)
    }

    /// Returns an iterator over all symbols in the table.
    ///
    /// # Returns
    /// An iterator yielding references to all symbols.
    pub fn values(&self) -> impl Iterator<Item = &Symbol> {
        self.symbols.values()
    }

    /// Returns a mutable iterator over all symbols in the table.
    ///
    /// # Returns
    /// An iterator yielding mutable references to all symbols.
    pub fn values_mut(&mut self) -> impl Iterator<Item = &mut Symbol> {
        self.symbols.iter_mut().map(|(_, symbol)| symbol)
    }

    pub fn get_symbols_with_declaration_by_filter(
        &self,
        symbol_name: Option<&str>,
        kind: Option<&SymbolKind>,
        scope: Option<&Scope>,
    ) -> Vec<&Symbol> {
        let mut result = Vec::new();

        // Parcourir tous les symboles
        for symbol in self.symbols.values() {
            // Vérifier si le nom correspond, si spécifié
            if let Some(name) = symbol_name {
                if symbol.name() != name {
                    continue;
                }
            }

            // Récupérer les déclarations du symbole
            let declarations = symbol.declarations();

            // Vérifier si le filtre sur les déclarations correspond à quelque chose
            let filtered_declarations = SymbolTable::get_by_filter(kind, scope, declarations);

            // Si des déclarations correspondantes sont trouvées, ajouter le symbole à la réponse
            if !filtered_declarations.is_empty() {
                result.push(symbol);
            }
        }

        result
    }

    pub fn get_symbols_with_usage_by_filter(
        &self,
        symbol_name: Option<&str>,
        kind: Option<&SymbolKind>,
        scope: Option<&Scope>,
    ) -> Vec<&Symbol> {
        let mut result = Vec::new();

        // Parcourir tous les symboles
        for symbol in self.symbols.values() {
            // Vérifier si le nom correspond, si spécifié
            if let Some(name) = symbol_name {
                if symbol.name() != name {
                    continue;
                }
            }

            // Récupérer les déclarations du symbole
            let declarations = symbol.usages();

            // Vérifier si le filtre sur les déclarations correspond à quelque chose
            let filtered_usages = SymbolTable::get_by_filter(kind, scope, declarations);

            // Si des déclarations correspondantes sont trouvées, ajouter le symbole à la réponse
            if !filtered_usages.is_empty() {
                result.push(symbol);
            }
        }

        result
    }

    pub fn get_declarations_by_filter(
        &self,
        symbol_name: Option<&str>,
        kind: Option<&SymbolKind>,
        scope: Option<&Scope>,
    ) -> Vec<&Declaration> {
        let mut result = Vec::new();

        for symbol in self.symbols.values() {
            if let Some(name) = symbol_name {
                if symbol.name() != name {
                    continue;
                }
            }
            let declarations = symbol.declarations();
            result.extend(SymbolTable::get_by_filter(kind, scope, declarations));
        }
        result
    }

    pub fn get_usages_by_filter(
        &self,
        symbol_name: Option<&str>,
        kind: Option<&SymbolKind>,
        scope: Option<&Scope>,
    ) -> Vec<&Usage> {
        let mut result = Vec::new();

        for symbol in self.symbols.values() {
            if let Some(name) = symbol_name {
                if symbol.name() != name {
                    continue;
                }
            }

            // Passer les usages à la fonction générique
            let usages = symbol.usages();
            result.extend(SymbolTable::get_by_filter(kind, scope, usages));
        }

        result
    }

    pub fn get_by_filter<'a, T: FilterableSymbol>(
        kind: Option<&SymbolKind>,
        scope: Option<&Scope>,
        items: &'a [T],
    ) -> Vec<&'a T> {
        let mut result = Vec::new();
        for item in items {
            // Filtrage par type de symbole (kind)
            if let Some(k) = kind {
                if item.kind() != k {
                    continue;
                }
            }

            // Filtrage par scope
            if let Some(s) = scope {
                if !s.starts_with(item.scope()) {
                    continue;
                }
            }
            result.push(item);
        }

        result
    }

    ///////////////////////////////////////////////////////////////////////////////////////////////
    // From this point the code is dedicated to the initialization of the symbol table from an AST
    ///////////////////////////////////////////////////////////////////////////////////////////////
    fn get_ast_entry(index: usize, ast: &AstTable) -> Result<&AstEntry, ParserInternalError> {
        ast.get_entry(index).ok_or_else(|| {
            ParserInternalError::new(format!("AstEntry not found for index: {}", index))
        })
    }

    /// This function initializes the symbol table based on the Abstract Syntax Tree (AST) nodes.
    /// It processes different AST kinds and either adds declaration symbols, symbol usages, or
    /// handles specific cases like actions and atomic formulas.
    ///
    /// # Arguments
    /// * `ast` - A reference to the AST node to be processed. It can be a variety of types
    ///   depending on the node.
    /// * `scope` - The scope in which the symbol is being declared or used. It helps manage
    ///   variable or function visibility.
    pub fn initialize_from_ast(
        &mut self,
        index: usize,
        index_table: &AstTable,
    ) -> Result<(), ParserInternalError> {
        let ast = index_table.get_entry(index).unwrap();
        //let ast = SymbolTable::get_ast_entry(index, index_table)?;
        self.init_from(ast, index, index_table, Scope::new(index, None))?;

        Ok(())
    }

    /// This function initializes the symbol table based on the Abstract Syntax Tree (AST) nodes.
    /// It processes different AST kinds and either adds declaration symbols, symbol usages, or
    /// handles specific cases like actions and atomic formulas.
    ///
    /// # Arguments
    /// * `ast` - A reference to the AST node to be processed. It can be a variety of types
    ///   depending on the node.
    /// * `index` - The index of the AST entry, used to uniquely identify the node in the symbol table.
    /// * `index_table` - A reference to the table that maps AST entries to their respective indices.
    /// * `scope` - The scope in which the symbol is being declared or used. It helps manage
    ///   variable or function visibility, ensuring that symbols are correctly handled in the context
    ///   of the program structure.
    ///
    /// # Returns
    /// * `Result<(), ParserInternalError>` - Returns `Ok(())` on success, or an error if any problem
    ///   is encountered during symbol table initialization.
    fn init_from(
        &mut self,
        ast: &AstEntry,
        index: usize,
        index_table: &AstTable,
        scope: Scope,
    ) -> Result<(), ParserInternalError> {
        // Determine the type of the AST node and apply appropriate processing
        match ast.kind() {
            // Handle declarations: These simply register symbols without additional processing
            AstKind::DomainName(_) | AstKind::ProblemName(_) | AstKind::Requirement(_) => {
                self.add_declaration_symbol(ast, index, index_table, scope.clone(), None, None)?;
            }

            // Handle typed lists: Requires specialized initialization logic
            AstKind::TypedList => {
                self.init_from_typed_list(ast, index, index_table, scope.clone())?;
            }

            // Handle primitive types, constants, and variables: Register them as symbol usages
            AstKind::PrimitiveType(_) | AstKind::Constant(_) | AstKind::Variable(_) => {
                self.add_symbol_usage(ast, index, index_table, scope.clone())?;
            }

            // Handle action definitions
            AstKind::ActionDef => {
                self.init_from_action_def(ast, index, index_table, scope.clone())?;
            }

            // Handle durative actions, which include timing constraints
            AstKind::DurativeActionDef => {
                self.init_from_durative_action_def(ast, index, index_table, scope.clone())?;
            }

            // Handle atomic formula skeletons: Requires custom symbol table handling
            AstKind::AtomicFormulaSkeleton => {
                self.init_from_atomic_formula_skeleton(ast, index, index_table, scope.clone())?;
            }

            // Handle atomic formulas and function terms: These require recursive processing
            AstKind::AtomicFormula | AstKind::FunctionTerm => {
                self.init_from_atomic_formula(ast, index, index_table, scope.clone())?;
            }

            // Handle quantified expressions (`Forall` and `Exists`): Need special treatment for
            // logical scopes
            AstKind::Forall | AstKind::Exists => {
                self.init_from_quantified_expression(ast, index, index_table, scope.clone())?;
            }

            // Handle hierarchical task network (HTN) method definitions
            AstKind::MethodDef => {
                self.init_from_method_def(ast, index, index_table, scope.clone())?;
            }

            // Handle task definitions in HTN planning
            AstKind::TaskDef => {
                self.init_from_task_def(ast, index, index_table, scope.clone())?;
            }

            // Handle individual task references in HTN planning
            AstKind::Task => {
                self.init_from_atomic_formula(ast, index, index_table, scope.clone())?;
            }

            // Handle tagged tasks, which include additional metadata in HTN planning
            AstKind::TaggedTask => {
                self.init_from_tagged_task(ast, index, index_table, scope.clone())?;
            }

            // Handle task ordering constraints in HTN planning
            AstKind::TaskOrderingConstraint(_) => {
                self.init_from_task_ordering_constraint(ast, index, index_table, scope.clone())?;
            }

            // Default case: If the AST node is not explicitly handled, process its children
            // recursively
            _ => {
                for child_index in ast.children() {
                    let child = SymbolTable::get_ast_entry(*child_index, index_table)?;
                    self.init_from(child, *child_index, index_table, scope.clone())?;
                }
            }
        }

        // Successfully completed processing the AST node
        Ok(())
    }

    /// Adds a new symbol declaration to the symbol table. This function first extracts
    /// the symbol name and kind from the AST node using the `extract_symbol_info` function.
    /// It then verifies the AST node's kind using `assert_ast_kind` to ensure it's one of the valid types.
    /// If the symbol is already present, it adds a new declaration to the existing symbol;
    /// otherwise, it creates a new symbol and inserts it into the symbol table.
    ///
    /// # Arguments
    ///
    /// * `ast` - A reference to a boxed `Ast` object representing the AST node.
    /// * `scope` - The current scope in which the symbol is declared.
    /// * `types` - Optional vector of types associated with the declaration.
    /// * `arguments` - Optional vector of typed symbols representing the arguments of the symbol.
    ///
    /// # Returns
    ///
    /// * `Ok(())` - If the symbol is successfully added.
    /// * `Err(ParserInternalError)` - If any error occurs during the symbol extraction or insertion process.
    fn add_declaration_symbol(
        &mut self,
        ast: &AstEntry,
        index: usize,
        index_table: &AstTable,
        scope: Scope,
        types: Option<Vec<String>>,
        arguments: Option<Vec<TypedSymbol<String>>>,
    ) -> Result<(), ParserInternalError> {
        // Assert that the AST kind is valid
        Self::assert_ast_kind(
            ast,
            &[
                AstKind::DomainName(String::new()),
                AstKind::PrimitiveType(String::new()),
                AstKind::ProblemName(String::new()),
                AstKind::Requirement(Requirement::Strips),
                AstKind::Constant(String::new()),
                AstKind::Variable(String::new()),
                AstKind::Predicate(String::new()),
                AstKind::FunctionSymbol(String::new()),
                AstKind::ActionSymbol(String::new()),
                AstKind::DASymbol(String::new()),
                // Add for HDDL
                AstKind::MethodSymbol(String::new()),
                AstKind::TaskSymbol(String::new()),
                AstKind::TaskID(String::new()),
            ],
        )?;

        // Extract the symbol information from the AST
        let (name, kind) = Self::extract_symbol(ast, index_table)?;

        // Check if the symbol is already in the symbol table and add a declaration
        if let Some(symbol) = self.get_symbol_mut(&name) {
            let declaration =
                Declaration::new(index, kind, scope, index_table.source(), types, arguments);
            symbol.add_declaration(declaration);
        } else {
            // Create a new symbol and add the declaration to it
            let mut symbol = Symbol::new(&name);
            let declaration =
                Declaration::new(index, kind, scope, index_table.source(), types, arguments);
            symbol.add_declaration(declaration);
            self.insert_symbol(name, symbol); // Insert the new symbol into the table
        }

        Ok(())
    }

    /// Adds the usage of a symbol in the given AST node, updating or inserting it into the symbol
    /// table.
    ///
    /// This function handles different kinds of AST nodes (such as domain names, problem names,
    /// requirements, constants, variables, and formulas) by extracting the relevant symbol
    /// information. It checks if the symbol already exists in the symbol table, adding the usage
    /// information to it, or creates a new symbol entry if it doesn't exist.
    ///
    /// # Parameters
    /// - `ast`: The AST node representing the symbol to be added or updated.
    /// - `scope`: The scope in which the symbol is being used.
    ///
    /// # Returns
    /// - `Ok(())` if the symbol usage was added successfully.
    /// - `Err(ParserInternalError)` if an error occurred, such as an unexpected AST node type or
    ///   invalid child node structure.
    fn add_symbol_usage(
        &mut self,
        ast: &AstEntry,
        index: usize,
        index_table: &AstTable,
        scope: Scope,
    ) -> Result<(), ParserInternalError> {
        // Assert that the AST node is of a valid kind for symbol usage.
        Self::assert_ast_kind(
            ast,
            &[
                AstKind::DomainName(String::new()),
                AstKind::ProblemName(String::new()),
                AstKind::Constant(String::new()),
                AstKind::Variable(String::new()),
                AstKind::AtomicFormula,
                AstKind::FunctionTerm,
                AstKind::Task,
            ],
        )?;

        // Handle specific cases for AtomicFormula and FunctionTerm, which need to have a child.
        if matches!(
            ast.kind(),
            AstKind::AtomicFormula | AstKind::FunctionTerm | AstKind::Task
        ) {
            Self::assert_ast_children_number(ast, 1, Comparator::GreaterEq)?;
        }

        // Extract symbol name and type based on the AST node's kind.
        let (name, kind) = Self::extract_symbol(ast, index_table)?;

        // If the symbol exists, add the usage; otherwise, create a new symbol.
        if let Some(symbol) = self.get_symbol_mut(&name) {
            let usage = Usage::new(index, kind, scope, index_table.source());
            symbol.add_usage(usage);
        } else {
            let mut symbol = Symbol::new(&name);
            let usage = Usage::new(index, kind, scope, index_table.source());
            symbol.add_usage(usage);
            self.insert_symbol(name, symbol);
        }
        Ok(())
    }

    /// Extracts the symbol name and its corresponding symbol kind from the AST.
    ///
    /// This function processes various AST node types, extracting relevant information about the
    /// symbol represented by the node (e.g., its name and kind). It supports different symbol
    /// categories, including domain and problem elements, function and predicate symbols, task and
    /// method symbols, and special cases such as `AtomicFormula` and `FunctionTerm`.
    ///
    /// If the AST node type is unexpected or has an invalid structure, the function
    /// returns an error.
    ///
    /// # Arguments
    ///
    /// * `ast` - A reference to an `AstEntry` representing the AST node.
    /// * `index_table` - A reference to an `AstTable` used for resolving child nodes.
    ///
    /// # Returns
    ///
    /// * `Ok((String, SymbolKind))` - A tuple containing the symbol name and its kind.
    /// * `Err(ParserInternalError)` - If the AST node kind is unrecognized or has an invalid
    ///   structure.
    ///
    /// # Supported AST Node Kinds
    ///
    /// The function handles the following AST node types:
    ///
    /// - **Basic Symbol Types:**
    ///   - `DomainName` → `SymbolKind::DomainName`
    ///   - `PrimitiveType` → `SymbolKind::PrimitiveType`
    ///   - `ProblemName` → `SymbolKind::ProblemName`
    ///   - `Requirement` → `SymbolKind::Requirement`
    ///   - `Constant` → `SymbolKind::Constant`
    ///   - `Variable` → `SymbolKind::Variable`
    ///   - `FunctionSymbol` → `SymbolKind::Function`
    ///   - `Predicate` → `SymbolKind::Predicate`
    ///   - `ActionSymbol` → `SymbolKind::Action`
    ///   - `DASymbol` → `SymbolKind::DASymbol`
    ///
    /// - **Hierarchical Task Network (HTN) and HDDL Support:**
    ///   - `MethodSymbol` → `SymbolKind::Method`
    ///   - `TaskSymbol` → `SymbolKind::Task`
    ///   - `TaskID` → `SymbolKind::TaskID`
    ///
    /// - **Structured Symbols (Handled via First Child Resolution):**
    ///   - `AtomicFormula`
    ///   - `FunctionTerm`
    ///   - `Task`
    ///
    ///   These require a valid first child, which must be one of:
    ///   - `Predicate` → `SymbolKind::Predicate`
    ///   - `FunctionSymbol` → `SymbolKind::Function`
    ///   - `TaskSymbol` → `SymbolKind::Task`
    ///   - `TotalTime` (Special case, treated as `SymbolKind::Function`)
    ///
    /// # Errors
    ///
    /// The function returns a `ParserInternalError` in the following cases:
    ///
    /// * The AST node type is unrecognized.
    /// * An `AtomicFormula`, `FunctionTerm`, or `Task` node has no children when they are expected.
    /// * The first child of an `AtomicFormula`, `FunctionTerm`, or `Task` is not a `Predicate`,
    /// `FunctionSymbol`, or `TaskSymbol`.

    fn extract_symbol(
        ast: &AstEntry,
        index_table: &AstTable,
    ) -> Result<(String, SymbolKind), ParserInternalError> {
        match ast.kind() {
            // Handling different AST node kinds and returning appropriate symbol information
            AstKind::DomainName(name) => Ok((name.to_string(), SymbolKind::DomainName)),
            AstKind::PrimitiveType(name) => Ok((name.to_string(), SymbolKind::PrimitiveType)),
            AstKind::ProblemName(name) => Ok((name.to_string(), SymbolKind::ProblemName)),
            AstKind::Requirement(requirement) => {
                Ok((requirement.to_string(), SymbolKind::Requirement))
            }
            AstKind::Constant(name) => Ok((name.to_string(), SymbolKind::Constant)),
            AstKind::Variable(name) => Ok((name.to_string(), SymbolKind::Variable)),
            AstKind::FunctionSymbol(name) => Ok((name.to_string(), SymbolKind::Function)),
            AstKind::Predicate(name) => Ok((name.to_string(), SymbolKind::Predicate)),
            AstKind::ActionSymbol(name) => Ok((name.to_string(), SymbolKind::Action)),
            AstKind::DASymbol(name) => Ok((name.to_string(), SymbolKind::DASymbol)),

            // Add on for HDDL support
            AstKind::MethodSymbol(name) => Ok((name.to_string(), SymbolKind::Method)),
            AstKind::TaskSymbol(name) => Ok((name.to_string(), SymbolKind::Task)),
            AstKind::TaskID(name) => Ok((name.to_string(), SymbolKind::TaskID)),

            // Special case for AtomicFormula and FunctionTerm: handle their children
            AstKind::AtomicFormula | AstKind::FunctionTerm | AstKind::Task => {
                let children = ast.children();
                if children.is_empty() {
                    return Err(ParserInternalError::new(format!(
                        "{} must have children, but none found.",
                        ast.kind()
                    )));
                }
                let first_child = SymbolTable::get_ast_entry(children[0], index_table)?;
                match first_child.kind() {
                    AstKind::Predicate(s) => Ok((s.to_string(), SymbolKind::Predicate)),
                    AstKind::FunctionSymbol(s) => Ok((s.to_string(), SymbolKind::Function)),
                    AstKind::TaskSymbol(s) => Ok((s.to_string(), SymbolKind::Task)),
                    // Deal special TotalTime symbol as a classical function symbol
                    AstKind::TotalTime => Ok((TOTAL_TIME.to_string(), SymbolKind::Function)),
                    // Error if the first child is not a Predicate or FunctionSymbol
                    _ => Err(ParserInternalError::new(format!(
                        "First child of {} must be a Predicate or FunctionSymbol, found: {:?}",
                        ast.kind(),
                        first_child.kind()
                    ))),
                }
            }
            // Error case for unsupported AST kinds
            _ => Err(ParserInternalError::new(format!(
                "Unexpected symbol kind encountered: {:?}",
                ast.kind()
            ))),
        }
    }

    /// Initializes the symbol table from a `TypedList` AST node.
    ///
    /// This function processes a `TypedList` node in the AST, adding its elements (such as types,
    /// constants, variables, etc.)
    /// to the symbol table. It handles the node in three cases:
    /// 1. No children: It returns early without processing.
    /// 2. Exactly two children: It processes them without recursion.
    /// 3. More than two children: It recursively processes additional `TypedList` nodes.
    ///
    /// # Parameters
    /// - `ast`: A reference to a boxed `Ast` node of type `TypedList`.
    /// - `scope`: The scope in which the symbol table is being initialized.
    ///
    /// # Returns
    /// - `Ok(())`: If all nodes are processed successfully and the symbol table is updated.
    /// - `Err(ParserInternalError)`: If there is an unexpected AST node or an invalid structure
    ///   encountered.
    ///
    /// # Error Handling
    /// If the function encounters an unexpected AST structure or node type (such as a missing child
    /// or an unsupported node type), it will return a `ParserInternalError` with a description of
    /// the problem.
    fn init_from_typed_list(
        &mut self,
        ast: &AstEntry,
        _index: usize,
        index_table: &AstTable,
        scope: Scope,
    ) -> Result<(), ParserInternalError> {
        // Ensure the AST node is of the expected type 'TypedList'
        Self::assert_ast_kind(ast, &[AstKind::TypedList])?;

        let children = ast.children();

        // Case 1: No children, return early
        if children.is_empty() {
            return Ok(());
        }

        // Ensure we have at least two children for valid processing
        Self::assert_ast_children_number(ast, 2, Comparator::Greater)?;

        // Process the types (second child)
        let types_entry = SymbolTable::get_ast_entry(children[1], index_table)?;
        let types = self.init_from_type(types_entry, children[1], index_table, scope.clone())?;

        // Case 2: Exactly two children (no recursion needed)
        let element_entry = SymbolTable::get_ast_entry(children[0], index_table)?;
        if children.len() == 2 {
            // Process the first element using a helper function
            self.init_from_typed_list_element(
                element_entry,
                children[0],
                index_table,
                scope.clone(),
                types,
            )?;
            return Ok(());
        }

        // Case 3: More than two children (recursive processing)
        // Process the first element using a helper function
        self.init_from_typed_list_element(
            element_entry,
            children[0],
            index_table,
            scope.clone(),
            types,
        )?;

        // Process the next TypedList (third child)
        let next_typed_list = SymbolTable::get_ast_entry(children[2], index_table)?;
        self.init_from_typed_list(next_typed_list, children[2], index_table, scope)?;

        Ok(())
    }

    /// Helper function to process an individual element of the TypedList.
    ///
    /// This function adds the element to the symbol table based on its type (e.g., PrimitiveType,
    /// Constant, Variable, etc.). It can also handle nested elements like AtomicFunctionSkeleton.
    ///
    /// # Parameters
    /// - `element`: The AST node representing the element (could be PrimitiveType, Constant, etc.).
    /// - `scope`: The scope in which the symbol table is being updated.
    /// - `types`: The types associated with the element.
    /// Helper function to process an individual element of the TypedList.
    ///
    /// This function adds the element to the symbol table based on its type (e.g., PrimitiveType,
    /// Constant, Variable, etc.). It can also handle nested elements like AtomicFunctionSkeleton.
    ///
    /// # Parameters
    /// - `element`: The AST node representing the element (could be PrimitiveType, Constant, etc.).
    /// - `scope`: The scope in which the symbol table is being updated.
    /// - `types`: The types associated with the element.
    fn init_from_typed_list_element(
        &mut self,
        element: &AstEntry,
        index: usize,
        index_table: &AstTable,
        scope: Scope,
        types: Vec<String>,
    ) -> Result<(), ParserInternalError> {
        // Ensure the element is one of the expected types before proceeding
        Self::assert_ast_kind(
            element,
            &[
                AstKind::PrimitiveType(String::new()),
                AstKind::Constant(String::new()),
                AstKind::Variable(String::new()),
                AstKind::AtomicFunctionSkeleton,
            ],
        )?;

        match element.kind() {
            // Process PrimitiveType, Constant, or Variable
            AstKind::PrimitiveType(_) | AstKind::Constant(_) | AstKind::Variable(_) => {
                self.add_declaration_symbol(element, index, index_table, scope, Some(types), None)?;
            }
            // Handle AtomicFunctionSkeleton recursively
            AstKind::AtomicFunctionSkeleton => {
                self.init_from_atomic_function_skeleton(element, index, index_table, scope, types)?;
            }
            _ => {
                // Handle any unexpected cases, though the assertion should prevent them
                unreachable!("Unexpected AST node type: {:?}", element.kind());
            }
        }

        Ok(())
    }

    /// Initializes the symbol table for an atomic function skeleton.
    ///
    /// This function processes an AST node of type `AtomicFunctionSkeleton` and performs
    /// the following tasks:
    /// - Validates that the AST node is of type `AtomicFunctionSkeleton`.
    /// - Ensures the node has at least two children: the function symbol and the list of arguments.
    /// - Validates that the first child is of type `FunctionSymbol` and extracts it.
    /// - Creates a new scope for the function and initializes the symbol table for the function's
    ///   arguments.
    /// - Extracts the arguments and calculates their arity.
    /// - Adds the function declaration to the symbol table with the provided types and arguments.
    ///
    /// # Parameters
    /// - `ast`: A reference to a boxed `Ast` node of type `AtomicFunctionSkeleton`.
    /// - `scope`: The current scope in which the function is defined. This scope is used for symbol
    ///   resolution and declarations.
    /// - `types`: A vector of strings representing the types associated with the function.
    ///
    /// # Returns
    /// - `Ok(())`: If the function is successfully processed and the symbol table is updated.
    /// - `Err(ParserInternalError)`: If any validation fails (e.g., invalid AST node type or
    ///   incorrect number of children).
    ///
    /// # Example
    /// ```rust
    /// let ast = // ... get the AST node for AtomicFunctionSkeleton
    /// let scope = // ... obtain the scope
    /// let types = vec!["int".to_string(), "bool".to_string()];
    /// let result = symbol_table.init_symbol_table_from_atomic_function_skeleton(&ast, scope, types);
    /// match result {
    ///     Ok(_) => println!("Function initialized successfully"),
    ///     Err(e) => eprintln!("Error: {:?}", e),
    /// }
    /// ```
    ///
    /// # Error Handling
    /// If the function encounters an AST node that doesn't match the expected structure or if any
    /// required components are missing, it will return a `ParserInternalError` with a message
    /// specifying the problem. For example, if the first child of the `AtomicFunctionSkeleton` is
    /// not a `FunctionSymbol`, an error will be raised detailing the encountered type.
    ///
    /// # Notes
    /// - This function assumes that the `init_symbol_table_from_typed_list` method handles the
    ///   argument's type validation.
    /// - The arguments' types and their arity are extracted and used to update the symbol table,
    ///   ensuring proper symbol resolution.
    /// - This method is part of a larger symbol table management system for function definitions
    ///   and their scope resolution.
    fn init_from_atomic_function_skeleton(
        &mut self,
        ast: &AstEntry,
        index: usize,
        index_table: &AstTable,
        scope: Scope,
        types: Vec<String>,
    ) -> Result<(), ParserInternalError> {
        // Check that the AST node is of the expected type 'Function'
        Self::assert_ast_kind(ast, &[AstKind::AtomicFunctionSkeleton])?;

        // Ensure the node has at least two children (function symbol and arguments)
        Self::assert_ast_children_number(ast, 2, Comparator::GreaterEq)?;

        let children = ast.children();

        // Retrieve the first child and validate it as a 'FunctionSymbol'
        let functor = SymbolTable::get_ast_entry(children[0], index_table)?;
        match functor.kind() {
            AstKind::FunctionSymbol(s) => s,
            _ => {
                return Err(ParserInternalError::new(format!(
                    "First child of 'Function' must match the expected kind. Encountered: '{:?}'",
                    functor.kind()
                )))
            }
        };

        let arguments = SymbolTable::get_ast_entry(children[1], index_table)?;

        // Initialize the symbol table for the arguments;
        self.init_from_typed_list(
            arguments,
            index,
            index_table,
            Scope::new(index, Some(&scope)),
        )?;

        // Extract the arguments and calculate the arity
        let arguments =
            self.extract_arguments_from_typed_list(arguments, children[1], index_table)?;

        // Add the declaration to the symbol table
        self.add_declaration_symbol(
            functor,
            index,
            index_table,
            scope.clone(),
            Some(types),
            Some(arguments),
        )?;

        Ok(())
    }

    /// Initializes the symbol table for an action definition.
    ///
    /// This function processes an `ActionDef`, `DurativeActionDef`, or `MethodDef` AST node.
    /// It extracts the action name, parameters, and body, adding relevant symbols to the table.
    ///
    /// # Parameters
    /// - `ast`: The AST node representing the action definition.
    /// - `index`: The index of the node in the AST table.
    /// - `index_table`: The table containing all AST nodes.
    /// - `scope`: The current scope in which the action is being defined.
    ///
    /// # Returns
    /// - `Ok(())` if the initialization succeeds.
    /// - `Err(ParserInternalError)` if the AST structure is invalid.
    ///
    /// # AST Structure
    /// The function expects the node to have exactly three children:
    /// 1. **Name**: The action/method name, which is added as a declaration symbol.
    /// 2. **Parameters**: The parameter list, which is recursively processed.
    /// 3. **Body**: The action/method body, which is recursively processed.
    ///
    /// # Example
    /// ```rust
    /// // Assuming `ast`, `index`, `index_table`, and `scope` are correctly initialized:
    /// parser.init_from_action_def(&ast, index, &index_table, scope)?;
    /// ```
    ///
    /// This function delegates to `init_from_definition` for common logic.
    fn init_from_action_def(
        &mut self,
        ast: &AstEntry,
        index: usize,
        index_table: &AstTable,
        scope: Scope,
    ) -> Result<(), ParserInternalError> {
        self.init_from_def(
            ast,
            index,
            index_table,
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
    /// This function ensures that the given AST node is a `MethodDef` and has
    /// the expected number of children (name, parameters, and body). It then
    /// processes these components accordingly.
    ///
    /// # Arguments
    ///
    /// * `ast` - A reference to the AST entry representing the method definition.
    /// * `index` - The index of the AST entry.
    /// * `index_table` - A reference to the AST table containing all nodes.
    /// * `scope` - The current scope in which the method definition resides.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if the initialization is successful, or a `ParserInternalError`
    /// if an issue occurs.
    fn init_from_method_def(
        &mut self,
        ast: &AstEntry,
        index: usize,
        index_table: &AstTable,
        scope: Scope,
    ) -> Result<(), ParserInternalError> {
        self.init_from_def(
            ast,
            index,
            index_table,
            scope,
            &[AstKind::MethodDef],
            3,    // MethodDef has 3 children (name, parameters, body)
            true, // It has a body
        )
    }

    /// Initializes the symbol table for a durative action definition.
    ///
    /// This function ensures that the given AST node is a `DurativeActionDef` and
    /// has the expected number of children (name, parameters, and body). It then
    /// processes these components accordingly.
    ///
    /// # Arguments
    ///
    /// * `ast` - A reference to the AST entry representing the durative action.
    /// * `index` - The index of the AST entry.
    /// * `index_table` - A reference to the AST table containing all nodes.
    /// * `scope` - The current scope in which the durative action definition resides.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if the initialization is successful, or a `ParserInternalError`
    /// if an issue occurs.
    fn init_from_durative_action_def(
        &mut self,
        ast: &AstEntry,
        index: usize,
        index_table: &AstTable,
        scope: Scope,
    ) -> Result<(), ParserInternalError> {
        self.init_from_def(
            ast,
            index,
            index_table,
            scope,
            &[AstKind::DurativeActionDef],
            3,    // DurativeActionDef has 3 children (name, parameters, body)
            true, // It has a body
        )
    }

    /// Initializes the symbol table for a task definition.
    ///
    /// This function ensures that the given AST node is a `TaskDef` and has
    /// the expected number of children (name and parameters). Since a task
    /// definition does not include a body, the function processes only these
    /// components.
    ///
    /// # Arguments
    ///
    /// * `ast` - A reference to the AST entry representing the task definition.
    /// * `index` - The index of the AST entry.
    /// * `index_table` - A reference to the AST table containing all nodes.
    /// * `scope` - The current scope in which the task definition resides.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if the initialization is successful, or a `ParserInternalError`
    /// if an issue occurs.
    fn init_from_task_def(
        &mut self,
        ast: &AstEntry,
        index: usize,
        index_table: &AstTable,
        scope: Scope,
    ) -> Result<(), ParserInternalError> {
        self.init_from_def(
            ast,
            index,
            index_table,
            scope,
            &[AstKind::TaskDef],
            2,     // TaskDef has only 2 children (name, parameters)
            false, // No body for tasks
        )
    }

    /// Initializes a definition node (ActionDef, DurativeActionDef, MethodDef, or TaskDef)
    /// by extracting its name and parameters, and optionally its body.
    ///
    /// # Arguments
    /// - `ast`: The AST entry to process.
    /// - `index`: The index of the node in the AST table.
    /// - `index_table`: Reference to the AST table.
    /// - `scope`: The current scope for symbol resolution.
    /// - `valid_kinds`: A slice of valid `AstKind` values.
    /// - `expected_children`: The number of children expected (2 for `TaskDef`, 3 for actions).
    /// - `has_body`: Whether the definition has a body (true for actions, false for tasks).
    ///
    /// # Returns
    /// Returns `Ok(())` on success or a `ParserInternalError` if validation fails.
    fn init_from_def(
        &mut self,
        ast: &AstEntry,
        index: usize,
        index_table: &AstTable,
        scope: Scope,
        valid_kinds: &[AstKind],
        expected_children: usize,
        has_body: bool,
    ) -> Result<(), ParserInternalError> {
        // Ensure the AST node is of the correct kind
        Self::assert_ast_kind(ast, valid_kinds)?;

        // Ensure the node has the expected number of children
        Self::assert_ast_children_number(ast, expected_children, Comparator::Equal)?;

        let children = ast.children();

        // First child: definition name, add to symbol table
        let name = SymbolTable::get_ast_entry(children[0], index_table)?;

        // Second child: parameters, recursively initialize the symbol table
        let parameters = SymbolTable::get_ast_entry(children[1], index_table)?;
        self.extract_arguments_from_typed_list(parameters, children[1], index_table)?;
        self.init_from_typed_list(
            parameters,
            children[1],
            index_table,
            Scope::new(index, Some(&scope)),
        )?;

        let parameters =
            self.extract_arguments_from_typed_list(parameters, children[1], index_table)?;
        self.add_declaration_symbol(
            name,
            children[0],
            index_table,
            scope.clone(),
            None,
            Some(parameters),
        )?;

        // Third child: body (if applicable)
        if has_body {
            let body = SymbolTable::get_ast_entry(children[2], index_table)?;
            self.init_from(
                body,
                children[2],
                index_table,
                Scope::new(index, Some(&scope)),
            )?;
        }

        Ok(())
    }

    /// Initializes the parser state from an atomic formula or function term AST node.
    ///
    /// This function processes an `AstEntry` of type `AtomicFormula` or `FunctionTerm`. It ensures that
    /// the node has at least one child, representing the symbol. The function first registers the
    /// symbol usage, then recursively processes the remaining children, which represent the arguments.
    ///
    /// # Arguments
    /// * `ast` - A reference to the `AstEntry` to process.
    /// * `_index` - The index of the AST node in the table (not used in this function).
    /// * `index_table` - A reference to the `AstTable`, which contains the AST nodes.
    /// * `scope` - The current scope used for symbol resolution.
    ///
    /// # Returns
    /// Returns `Ok(())` if the AST node is successfully processed. Otherwise, returns a `ParserInternalError`.
    ///
    /// # Errors
    /// This function returns an error if:
    /// * The `ast` is not of type `AtomicFormula` or `FunctionTerm`.
    /// * The `ast` has no children (it must have at least one, representing the symbol).
    /// * Any recursive call to `init_from` fails.
    fn init_from_atomic_formula(
        &mut self,
        ast: &AstEntry,
        index: usize,
        index_table: &AstTable,
        scope: Scope,
    ) -> Result<(), ParserInternalError> {
        // Ensure the AST node is of the correct kind (AtomicFormula or FunctionTerm)
        Self::assert_ast_kind(
            ast,
            &[AstKind::AtomicFormula, AstKind::FunctionTerm, AstKind::Task],
        )?;

        // Ensure the node has at least one child (the symbol)
        Self::assert_ast_children_number(ast, 1, Comparator::GreaterEq)?;

        // Retrieve and register the first child (symbol)
        self.add_symbol_usage(ast, index, index_table, scope.clone())?;

        // Process the remaining children (arguments)
        for child_index in ast.children() {
            let child = SymbolTable::get_ast_entry(*child_index, index_table)?;
            self.init_from(child, *child_index, index_table, scope.clone())?;
        }

        Ok(())
    }

    /// Initializes the symbol table for a quantified expression in the AST.
    ///
    /// This function validates that the given AST node is of a quantified expression type (either
    /// `Exists` or `Forall`). It then processes the children of the AST node. The first child
    /// should represent a variable declaration, and the second child should be an inner expression.
    /// The function recursively initializes the symbol table for both the variable declaration and
    /// the inner expression.
    ///
    /// # Parameters
    /// - `ast`: A reference to a boxed `Ast` node representing the quantified expression. The node
    ///   should be of kind `Exists` or `Forall`.
    /// - `scope`: The current scope to be passed to the symbol table initialization.
    ///
    /// # Returns
    /// - `Ok(())`: If the initialization is successful.
    /// - `Err(ParserInternalError)`: If any errors occur, such as an unexpected AST node type,
    ///   incorrect number of children, or issues initializing the symbol table.
    ///
    /// # Errors
    /// - `ParserInternalError`: If the AST node type is not `Exists` or `Forall`, or if there are
    ///   issues with the children (e.g., the number of children is not exactly 2).
    ///   The error message will specify the expected node type and the number of children.
    ///
    /// # Example
    /// ```rust
    /// let ast = ...; // An Ast node representing a quantified expression
    /// let scope = ...; // The current scope
    /// let result = symbol_table.init_symbol_table_from_quantified_expression(ast, scope);
    /// match result {
    ///     Ok(_) => println!("Symbol table initialized successfully"),
    ///     Err(e) => eprintln!("Error: {}", e),
    /// }
    /// ```
    fn init_from_quantified_expression(
        &mut self,
        ast: &AstEntry,
        index: usize,
        index_table: &AstTable,
        scope: Scope,
    ) -> Result<(), ParserInternalError> {
        // Check if the AST node is of kind 'Exists' or 'Forall'
        Self::assert_ast_kind(ast, &[AstKind::Exists, AstKind::Forall])?;

        // Ensure the AST has exactly 2 children (variables and inner expression)
        Self::assert_ast_children_number(ast, 2, Comparator::Equal)?;

        let children = ast.children();
        // Retrieve the children (variables and inner expression)
        let variables = SymbolTable::get_ast_entry(children[0], index_table)?;
        let expression = SymbolTable::get_ast_entry(children[1], index_table)?;

        // Initialize the symbol table for the variables (first child)
        self.init_from_typed_list(
            variables,
            children[0],
            index_table,
            Scope::new(index, Some(&scope)),
        )?;

        // Initialize the symbol table for the inner expression (second child)
        self.init_from(
            expression,
            children[1],
            index_table,
            Scope::new(index, Some(&scope)),
        )?;

        Ok(())
    }

    /// Initializes the symbol table for an atomic formula skeleton in the AST.
    ///
    /// This function validates that the given AST node has the expected structure for an atomic
    /// formula, which includes a predicate and at least one argument. It checks the following:
    /// 1. The node has at least two children: a predicate and its arguments.
    /// 2. The first child is of the `Predicate` kind.
    /// The function then recursively initializes the symbol table for the arguments of the predicate.
    ///
    /// # Parameters
    /// - `ast`: A reference to a boxed `Ast` node representing the atomic formula skeleton. It should
    ///   have at least two children: a predicate and its arguments.
    /// - `scope`: The current scope to be passed to the symbol table initialization.
    ///
    /// # Returns
    /// - `Ok(())`: If the initialization is successful.
    /// - `Err(ParserInternalError)`: If any validation errors occur, such as an incorrect number of children
    ///   or an invalid AST node type.
    ///
    /// # Example
    /// ```rust
    /// let ast = ...; // An Ast node representing an atomic formula skeleton
    /// let scope = ...; // The current scope
    /// let result = symbol_table.init_symbol_table_from_atomic_formula_skeleton(ast, scope);
    /// match result {
    ///     Ok(_) => println!("Symbol table initialized successfully"),
    ///     Err(e) => eprintln!("Error: {}", e),
    /// }
    /// ```
    fn init_from_atomic_formula_skeleton(
        &mut self,
        ast: &AstEntry,
        index: usize,
        index_table: &AstTable,
        scope: Scope,
    ) -> Result<(), ParserInternalError> {
        let children = ast.children();

        // Ensure the AST has at least two children (predicate and arguments)
        Self::assert_ast_children_number(ast, 2, Comparator::Equal)?;

        // Ensure the first child is of kind 'Predicate'
        let predicate = SymbolTable::get_ast_entry(children[0], index_table)?;
        Self::assert_ast_kind(predicate, &[AstKind::Predicate(String::new())])?;

        // Retrieve and process arguments
        let arguments = SymbolTable::get_ast_entry(children[1], index_table)?;
        self.init_from_typed_list(
            arguments,
            children[1],
            index_table,
            Scope::new(index, Some(&scope)),
        )?;

        let arguments =
            self.extract_arguments_from_typed_list(arguments, children[1], index_table)?;
        self.add_declaration_symbol(
            predicate,
            children[0],
            index_table,
            scope.clone(),
            None,
            Some(arguments),
        )?;

        Ok(())
    }

    /// Extracts arguments from a TypedList AST node, which consists of a list of symbols with
    /// types.
    ///
    /// This function processes the AST node representing a `TypedList` and recursively extracts the
    /// variable or constant names, along with their corresponding types. It handles three distinct
    /// cases:
    /// - If the list is empty, it returns an empty vector.
    /// - If the list contains exactly two children, it processes them and returns a vector with a
    ///   single entry.
    /// - If the list contains three children, it processes the first two and recursively processes
    ///   the third child.
    ///
    /// # Parameters
    /// - `ast`: The AST node representing a TypedList, containing a sequence of children to be
    ///   processed.
    ///
    /// # Returns
    /// - `Ok(Vec<TypedSymbol<String>>)` if the extraction is successful. This will contain a vector
    ///   of `TypedSymbol` representing the extracted arguments (with their types).
    /// - `Err(ParserInternalError)` if the AST node is not of the expected type (`TypedList`), or
    ///   if the list's structure doesn't match the expected number of children or contains
    ///   unexpected kinds of elements.
    ///
    /// # Errors
    /// - Returns an error if the AST node is not of kind `TypedList`.
    /// - Returns an error if the first child is neither a `Variable` nor a `Constant`.
    /// - Returns an error if the second child cannot be processed as a list of primitive types.
    /// - Returns an error if the number of children is less than 2 or more than 3 in an unexpected
    ///   way.
    ///
    /// # Example
    /// ```
    /// let ast = // Some AST node of kind TypedList
    /// let result = self.extract_arguments_from_typed_list(&ast);
    /// match result {
    ///     Ok(arguments) => {
    ///         // Use extracted arguments
    ///     },
    ///     Err(e) => {
    ///         // Handle error
    ///     }
    /// }
    /// ```
    fn extract_arguments_from_typed_list(
        &mut self,
        ast: &AstEntry,
        _index: usize,
        index_table: &AstTable,
    ) -> Result<Vec<TypedSymbol<String>>, ParserInternalError> {
        // Ensure the AST node is of kind TypedList
        Self::assert_ast_kind(ast, &[AstKind::TypedList])?;

        let children = ast.children();

        // Case 1: No children, return an empty vector
        if children.is_empty() {
            return Ok(Vec::new());
        }

        // Ensure we have at least 2 children for valid processing
        Self::assert_ast_children_number(ast, 2, Comparator::Greater)?;

        // Extract the first child (must be a Variable or Constant)
        let element = SymbolTable::get_ast_entry(children[0], index_table)?;
        let name = match element.kind() {
            AstKind::Variable(name) | AstKind::Constant(name) => name.clone(),
            _ => {
                return Err(ParserInternalError::new(format!(
                    "Expected Variable or Constant as the first child, but found {:?}.",
                    element.kind()
                )))
            }
        };

        // Extract types from the second child
        let types_entry = SymbolTable::get_ast_entry(children[1], index_table)?;
        let types = self.extract_type(types_entry, children[1], index_table)?;

        // Case 2: Exactly two children, no recursion needed
        if children.len() == 2 {
            return Ok(vec![TypedSymbol::new(name, types)]);
        }

        // Case 3: Three children, recursive extraction
        let next_typed_list = SymbolTable::get_ast_entry(children[2], index_table)?;
        let mut results =
            self.extract_arguments_from_typed_list(next_typed_list, children[2], index_table)?;
        results.insert(0, TypedSymbol::new(name, types));

        Ok(results)
    }

    /// Extracts type names from an AST node without recording symbol usage.
    ///
    /// # Arguments
    ///
    /// * `types` - A reference to an AST node that must be of kind `Type`.
    ///
    /// # Returns
    ///
    /// A `Result` containing a vector of extracted type names or a `ParserInternalError` if the
    ///   node is invalid.
    fn extract_type(
        &mut self,
        types: &AstEntry,
        _index: usize,
        index_table: &AstTable,
    ) -> Result<Vec<String>, ParserInternalError> {
        // Ensure the provided AST node is of kind `Type`
        Self::assert_ast_kind(types, &[AstKind::Type])?;

        let mut super_types = Vec::new();
        for ty_index in types.children() {
            let ty = SymbolTable::get_ast_entry(*ty_index, index_table)?;
            if let AstKind::PrimitiveType(name) = ty.kind() {
                super_types.push(name.clone());
            } else {
                return Err(ParserInternalError::new(
                    "Unexpected AST node inside Type".to_string(),
                ));
            }
        }

        Ok(super_types)
    }

    /// Initializes type information and records symbol usage.
    ///
    /// This function extracts type names and additionally registers them as used symbols
    /// in the given scope.
    ///
    /// # Arguments
    ///
    /// * `types` - A reference to an AST node that must be of kind `Type`.
    /// * `scope` - The scope in which the type symbols are used.
    ///
    /// # Returns
    ///
    /// A `Result` containing a vector of extracted type names or a `ParserInternalError` if the
    /// node is invalid.
    fn init_from_type(
        &mut self,
        types: &AstEntry,
        index: usize,
        index_table: &AstTable,
        scope: Scope,
    ) -> Result<Vec<String>, ParserInternalError> {
        let super_types = self.extract_type(types, index, index_table)?; // Reuse `extract_type` to get type names

        // Register each type as a symbol usage in the given scope
        for ty_index in types.children() {
            let ty = SymbolTable::get_ast_entry(*ty_index, index_table)?;
            if matches!(ty.kind(), AstKind::PrimitiveType(_)) {
                self.add_symbol_usage(ty, *ty_index, index_table, scope.clone())?;
            }
        }

        Ok(super_types)
    }

    /// Initializes the symbol table from a tagged task definition in the AST.
    ///
    /// This function processes a tagged task by extracting its identifier and associated task,
    /// adding the identifier as a declaration, and then initializing the task itself.
    ///
    /// # Arguments
    /// * `ast` - A reference to the AST entry representing the tagged task.
    /// * `_index` - The index of the AST entry (not used in this function).
    /// * `index_table` - A reference to the AST table containing all parsed nodes.
    /// * `scope` - The scope in which the task symbols should be declared or used.
    ///
    /// # Returns
    /// * `Result<(), ParserInternalError>` - Returns `Ok(())` if successful, or an error if the AST
    ///   structure is unexpected.
    fn init_from_tagged_task(
        &mut self,
        ast: &AstEntry,
        _index: usize,
        index_table: &AstTable,
        scope: Scope,
    ) -> Result<(), ParserInternalError> {
        // Ensure the AST node is a tagged task
        Self::assert_ast_kind(ast, &[AstKind::TaggedTask])?;
        Self::assert_ast_children_number(ast, 2, Comparator::Equal)?;

        let children = ast.children();
        let task_id = SymbolTable::get_ast_entry(children[0], index_table)?;
        Self::assert_ast_kind(task_id, &[AstKind::TaskID(String::new())])?;

        // Add the task identifier as a declaration symbol
        self.add_declaration_symbol(task_id, children[0], index_table, scope.clone(), None, None)?;

        // Process the actual task
        let task = SymbolTable::get_ast_entry(children[1], index_table)?;
        Self::assert_ast_kind(task, &[AstKind::Task])?;

        self.init_from_atomic_formula(task, children[1], index_table, scope.clone())?;

        Ok(())
    }

    /// Initializes a task ordering constraint from the given AST node.
    ///
    /// This function processes an `AstEntry` representing a task ordering constraint,
    /// ensuring its validity and extracting the referenced task identifiers. It verifies
    /// that the AST node is correctly structured and that it contains exactly two child
    /// nodes representing ordered tasks.
    ///
    /// # Arguments
    ///
    /// * `ast` - A reference to an `AstEntry` representing the task ordering constraint.
    /// * `_index` - An unused index parameter.
    /// * `index_table` - A reference to an `AstTable` used for resolving AST entries.
    /// * `scope` - The current `Scope` used for symbol tracking.
    ///
    /// # Returns
    ///
    /// * `Ok(())` - If the task ordering constraint is correctly initialized.
    /// * `Err(ParserInternalError)` - If the AST node has an invalid structure or unexpected child
    ///   types.
    ///
    /// # Behavior
    ///
    /// 1. **Validation Checks:**
    ///    - Ensures that the AST node is of type `TaskOrderingConstraint(BinaryComp::Less)`.
    ///    - Ensures that the AST node has exactly **two children** (representing tasks).
    ///
    /// 2. **Extracting Task Identifiers:**
    ///    - Retrieves the first child and verifies it is of type `TaskID(String)`.
    ///    - Registers its usage in the symbol table.
    ///    - Retrieves the second child and verifies it is also of type `TaskID(String)`.
    ///    - Registers its usage in the symbol table.
    ///
    /// # Errors
    ///
    /// Returns a `ParserInternalError` if:
    /// - The AST node is not of kind `TaskOrderingConstraint(BinaryComp::Less)`.
    /// - The AST node does not have exactly two children.
    /// - Either child is not of kind `TaskID(String)`.
    fn init_from_task_ordering_constraint(
        &mut self,
        ast: &AstEntry,
        _index: usize,
        index_table: &AstTable,
        scope: Scope,
    ) -> Result<(), ParserInternalError> {
        // Ensure the AST node is a tagged task
        Self::assert_ast_kind(ast, &[AstKind::TaskOrderingConstraint(BinaryComp::Less)])?;
        Self::assert_ast_children_number(ast, 2, Comparator::Equal)?;

        let children = ast.children();
        let t1 = SymbolTable::get_ast_entry(children[0], index_table)?;
        Self::assert_ast_kind(t1, &[AstKind::TaskID(String::new())])?;
        self.add_symbol_usage(t1, children[0], index_table, scope.clone())?;

        let t2 = SymbolTable::get_ast_entry(children[1], index_table)?;
        Self::assert_ast_kind(t2, &[AstKind::TaskID(String::new())])?;
        self.add_symbol_usage(t2, children[1], index_table, scope.clone())?;

        Ok(())
    }

    /// Asserts that the AST node's kind is contained within the provided set of valid kinds.
    ///
    /// # Parameters
    /// - `ast`: The AST node to check.
    /// - `valid_kinds`: A slice of valid AST kinds that the node should match.
    ///
    /// # Returns
    /// - `Ok(())`: If the AST node's kind is valid.
    /// - `Err(ParserInternalError)`: If the AST node's kind is not valid.
    /// Asserts that the AST node's kind is contained within the provided set of valid kinds.
    ///
    /// This function checks if the kind of the provided AST node matches one of the valid kinds
    /// in the `valid_kinds` slice. If the AST node's kind is one of the allowed kinds, the function
    /// will return `Ok(())`. Otherwise, it will return an error with a message specifying the invalid
    /// node's kind and the list of expected valid kinds.
    ///
    /// # Parameters
    /// - `ast`: The AST node to check. This node should have a specific kind that needs to be
    ///   validated.
    /// - `valid_kinds`: A slice of valid AST kinds that the node should match. The slice can contain
    ///   any combination of the `AstKind` enum variants, such as `Predicate`, `DomainName`,
    ///   `ProblemName`, etc.
    ///
    /// # Returns
    /// - `Ok(())`: If the AST node's kind is valid and matches one of the kinds in `valid_kinds`.
    /// - `Err(ParserInternalError)`: If the AST node's kind is not valid, an error is returned. The
    ///   error message will include the actual node's kind and the expected valid kinds.
    ///
    /// # Example
    /// ```rust
    /// let result = Self::assert_ast_kind(ast, &[AstKind::Predicate(_), AstKind::DomainName(_)]);
    /// match result {
    ///     Ok(()) => println!("Valid AST node!"),
    ///     Err(e) => println!("Error: {}", e),
    /// }
    /// ```
    fn assert_ast_kind(ast: &AstEntry, valid_kinds: &[AstKind]) -> Result<(), ParserInternalError> {
        match ast.kind() {
            // Case where the AST node's kind is one of the defined types (Predicate, DomainName, etc.)
            AstKind::DomainName(_)
            | AstKind::ProblemName(_)
            | AstKind::Requirement(_)
            | AstKind::PrimitiveType(_)
            | AstKind::Constant(_)
            | AstKind::Variable(_)
            | AstKind::Predicate(_)
            | AstKind::FunctionSymbol(_)
            | AstKind::ActionSymbol(_)
            | AstKind::DASymbol(_)
            | AstKind::PrefName(_)
            | AstKind::Number(_)
            | AstKind::Assign(_)
            | AstKind::FComp(_)
            | AstKind::Metric(_)
            | AstKind::Operation(_)
            | AstKind::Parallel(_)
            | AstKind::Serial(_)
            // Add for HDDL
            | AstKind::MethodSymbol(_)
            | AstKind::TaskSymbol(_)
            | AstKind::TaskID(_) => {
                // Check if the AST node's kind matches one of the valid kinds
                if valid_kinds.iter().any(|_k| matches!(ast.kind(), _k)) {
                    Ok(())
                } else {
                    Err(ParserInternalError::new(format!(
                        "Unexpected AST node '{:?}'. Expected one of {:?}.",
                        ast.kind(),
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

    /// `assert_ast_children_number` verifies that the number of children in a given AST node
    /// satisfies a specified comparison with an expected length.
    ///
    /// This function checks if the number of children in the AST node (i.e., its direct descendants)
    /// matches the expected length based on the provided `Comparator` comparison type.
    /// If the comparison fails, it returns an error indicating the mismatch.
    ///
    /// # Parameters
    /// - `ast`: The AST node whose children are being checked.
    /// - `expected_len`: The expected number of children for the given AST node.
    /// - `comparator`: The comparison type used to validate the number of children relative to
    ///   `expected_len`. It can be one of the following:
    ///   - `Comparator::Equal`: The number of children should be exactly equal to `expected_len`.
    ///   - `Comparator::NotEqual`: The number of children should not be equal to `expected_len`.
    ///   - `Comparator::Less`: The number of children should be less than `expected_len`.
    ///   - `Comparator::Greater`: The number of children should be greater than `expected_len`.
    ///   - `Comparator::LessEq`: The number of children should be less than or equal to `expected_len`.
    ///   - `Comparator::GreaterEq`: The number of children should be greater than or equal to
    ///     `expected_len`.
    ///
    /// # Returns
    /// - `Ok(())` if the comparison is valid.
    /// - `Err(ParserInternalError)` if the comparison fails, indicating the expected and actual number
    ///   of children.
    ///
    /// # Errors
    /// This function returns a `ParserInternalError` if the comparison does not hold true, providing
    /// an error message with details about the AST node kind, the expected number of children,
    /// the actual number of children, and the comparison type used.
    ///
    /// # Example
    /// ```rust
    ///
    /// let ast = get_some_ast_node();
    /// Self::assert_ast_children_number(&ast, 3, Comparator::GreaterEq).unwrap(); // Passes if node has 3 or more children
    /// Self::assert_ast_children_number(&ast, 2, Comparator::Equal).unwrap_err(); // Fails if node doesn't have exactly 2 children
    /// ```
    ///
    /// # Notes
    /// This function provides a convenient way to enforce the expected structure of AST nodes during
    /// parsing, ensuring that the number of children aligns with the expectations set by the aiplan4rust
    /// logic.
    fn assert_ast_children_number(
        ast: &AstEntry,
        expected_len: usize,
        comparator: Comparator,
    ) -> Result<(), ParserInternalError> {
        let children_len = ast.children().len();

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
                ast.kind(),
                children_len,
                comparator
            )))
        }
    }
}

impl fmt::Display for SymbolTable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for symbol in self.symbols.values() {
            writeln!(f, "{}", symbol)?;
        }
        Ok(())
    }
}
