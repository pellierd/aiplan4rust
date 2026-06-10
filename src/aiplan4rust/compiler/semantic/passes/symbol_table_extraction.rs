//! Module responsible for building a [`SymbolTable`] from an abstract syntax tree (AST).
//!
//! This module provides the [`SymbolTableBuilder`] struct, which encapsulates the ops
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

use crate::aiplan4rust::compiler::semantic::passes::PassContext;
use crate::aiplan4rust::compiler::semantic::symbol::{
    Declaration, Scope, SymbolKind, SymbolOrigin, Usage,
};
use crate::aiplan4rust::compiler::semantic::symbol_table::{SymbolTableError, SymbolTableOrigin};
use crate::aiplan4rust::compiler::semantic::{SemanticError, SymbolTable};
use crate::aiplan4rust::compiler::syntax::ast::arena::{ArenaNode, NodeId};
use crate::aiplan4rust::compiler::syntax::ast::tree::NodeRef;
use crate::aiplan4rust::compiler::syntax::ast::{AstKind, AstNode};
use crate::aiplan4rust::compiler::syntax::lexer::token::NUMBER_TYPE;
use crate::aiplan4rust::support::lang::{SymbolId, Type, TypedList, TypedSymbol};

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
pub fn extract_symbol_table(context: &PassContext) -> Result<SymbolTable, SemanticError> {
    // Attempt to retrieve the root node of the AST, returning error if none exists
    let root_ref = context.syntax_tree().try_root_node_ref()?;
    let root_node = root_ref.node();

    let mut table = SymbolTable::new(context.interner(), context.syntax_tree().len());
    // Match the root node kind and configure the symbol table's origin and root ID accordingly
    match root_node.kind() {
        AstKind::Domain => {
            table.set_origin(SymbolTableOrigin::Domain);
            table.set_root_id(root_ref.id());
        }
        AstKind::Problem => {
            table.set_origin(SymbolTableOrigin::Problem);
            table.set_root_id(root_ref.id());
        }
        found => {
            // Return an error if the root node kind is not Domain or Problem
            return Err(SemanticError::unexpected_node_kind(
                root_ref.id(),
                vec![AstKind::Domain, AstKind::Problem],
                found,
            ));
        }
    }

    // Initialize the symbol table by recursively processing the AST starting from the root
    initialize_from_ast(context, &mut table, &root_ref)?;

    // Return the fully constructed symbol table, replacing the internal table with an empty one
    Ok(table)
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
    context: &PassContext,
    table: &mut SymbolTable,
    node_ref: &NodeRef<AstNode>,
) -> Result<(), SemanticError> {
    let scope = Scope::new(node_ref.id(), None);
    init_from(context, table, node_ref, scope)?;
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
/// - `TypedList`: Initialize symbol entries with specialized typed list ops.
/// - `PrimitiveType`, `Constant`, `Variable`: Register symbols as usages.
/// - `ActionDef`, `DurativeActionDef`: Initialize action-related symbols.
/// - `AtomicFormulaSkeleton`: Special symbol table handling for formula skeletons.
/// - `AtomicFormula`, `FunctionTerm`: Recursively initialize atomic formulas and function terms.
/// - `Forall`, `Exists`: Handle quantified logic and logical scopes.
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
    context: &PassContext,
    table: &mut SymbolTable,
    node_ref: &NodeRef<AstNode>,
    scope: Scope,
) -> Result<(), SemanticError> {
    // Match on the AST node kind to determine the appropriate processing ops
    match node_ref.node().kind() {
        // For domain and problem names, add declaration symbols directly without recursion
        AstKind::DomainName | AstKind::ProblemName => {
            add_declaration_symbol(
                context,
                table,
                node_ref,
                scope.clone(),
                None,
                None,
                None,
                None,
                false,
            )?;
        }

        // For typed lists, use specialized initialization for typed symbols
        AstKind::TypedList => {
            init_from_typed_list(context, table, node_ref, scope.clone())?;
        }

        // For primitive types, constants, and variables, record symbol usages
        AstKind::PrimitiveType | AstKind::Object | AstKind::Variable => {
            add_symbol_usage(context, table, node_ref, None, scope.clone())?;
        }

        // For action definitions, initialize action symbols and related info
        AstKind::ActionDef => {
            init_from_action_def(context, table, node_ref, scope.clone())?;
        }

        // For durative action definitions, handle timing-related action data
        AstKind::DurativeActionDef => {
            init_from_durative_action_def(context, table, node_ref, scope.clone())?;
        }

        // For atomic formula skeletons, apply custom handling for partial formulas
        AstKind::AtomicFormulaSkeleton => {
            init_from_atomic_formula_skeleton(context, table, node_ref, scope.clone())?;
        }

        // For atomic formulas and function terms, recurse into their structure
        AstKind::AtomicFormula | AstKind::Function => {
            init_from_atomic_formula(context, table, node_ref, scope.clone())?;
        }

        // For quantifiers (forall, exists), handle logical scoping and variable declarations
        AstKind::Forall | AstKind::Exists => {
            init_from_quantified_expression(context, table, node_ref, scope.clone())?;
        }

        // For HTN method definitions, initialize method-specific symbols and scopes
        AstKind::MethodDef => {
            init_from_method_def(context, table, node_ref, scope.clone())?;
        }

        // For HTN task definitions, initialize task-specific symbols and scopes
        AstKind::TaskDef => {
            init_from_task_def(context, table, node_ref, scope.clone())?;
        }

        // For HTN task instances, handle as atomic formulas
        AstKind::Task => {
            init_from_atomic_formula(context, table, node_ref, scope.clone())?;
        }

        // For tagged HTN tasks, handle extended metadata
        AstKind::LabeledTask => {
            init_from_tagged_task(context, table, node_ref, scope.clone())?;
        }

        // For HTN task ordering constraints, initialize constraint symbols
        AstKind::TaskOrderingConstraint => {
            init_from_task_ordering_constraint(context, table, node_ref, scope.clone())?;
        }
        // For axioms
        AstKind::DerivedDef => {
            init_from_derived_predicate_def(context, table, node_ref, scope.clone())?;
        }
        // For any other kinds, recursively process all child nodes to cover nested syntax
        _ => {
            for child in node_ref.node().children() {
                init_from(
                    context,
                    table,
                    &context.syntax_tree().try_node_ref(*child)?,
                    scope.clone(),
                )?;
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
pub fn add_declaration_symbol(
    context: &PassContext,
    table: &mut SymbolTable,
    node_ref: &NodeRef<AstNode>,
    scope: Scope,
    types: Option<Type<SymbolId>>,
    ty_node_ids: Option<Vec<NodeId>>,
    arguments: Option<TypedList<SymbolId, SymbolId>>,
    argument_node_ids: Option<Vec<NodeId>>,
    is_derived: bool,
) -> Result<(), SymbolTableError> {
    // 1. Extraction (Identique)
    let node = context.syntax_tree().try_node(node_ref.id())?;
    let symbol_ref = node.try_symbol()?;
    let ident = symbol_ref.id();

    // 2. Préparation des données (Identique)
    let origin = SymbolOrigin::from(table.origin());

    // 3. Création de la déclaration (Logique commune aux deux branches)
    let mut declaration = Declaration::new(
        symbol_ref,
        scope,
        origin,
        types,
        ty_node_ids,
        arguments,
        argument_node_ids,
        node_ref.node().span().clone(),
        node_ref.id(),
        None,
        None,
    );
    declaration.set_derived(is_derived);

    // 4. LE MÊME COMPORTEMENT :
    // Au lieu de faire le if/else manuellement ici, on appelle `add_declaration`.
    // Pourquoi c'est le même comportement ?
    // Parce que `table.add_declaration` fait exactement ceci :
    //    - Si le symbole existe -> il récupère l'entrée et ajoute la decl (ton `if`)
    //    - Si le symbole n'existe pas -> il crée l'entrée et ajoute la decl (ton `else`)
    //    - EN PLUS : il vérifie le cache pour éviter les erreurs E2011.

    table.add_declaration(ident, declaration)
}

/// Adds the usage of a symbol found in the given AST syntax to the symbol table.
///
/// This function processes AST nodes representing symbol usages, such as domain names,
/// problem names, constants, variables, atomic formulas, function terms, and tasks.
/// It first validates the AST syntax kind to ensure it is appropriate for symbol usage.
/// For certain syntax kinds that represent complex logic (e.g., atomic formulas),
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
pub fn add_symbol_usage(
    context: &PassContext,
    table: &mut SymbolTable,
    node_ref: &NodeRef<AstNode>,
    arguments: Option<Vec<NodeId>>,
    scope: Scope,
) -> Result<(), SymbolTableError> {
    // 1. Détermination du nœud cible et extraction du symbole (Identique)
    let (target_id, symbol_ref) = if matches!(
        node_ref.node().kind(),
        AstKind::AtomicFormula | AstKind::Function | AstKind::Task
    ) {
        let first_child_id = node_ref.node().children()[0];
        (
            first_child_id,
            context
                .syntax_tree()
                .try_node(first_child_id)?
                .try_symbol()?,
        )
    } else {
        (node_ref.id(), node_ref.node().try_symbol()?)
    };

    let ident = symbol_ref.id();
    let origin = SymbolOrigin::from(table.origin());

    // Récupération du span (Identique)
    let span = context
        .syntax_tree()
        .try_node_ref(target_id)?
        .node()
        .span()
        .clone();

    // 2. Préparation de l'usage (Identique)
    let usage = Usage::new(symbol_ref, scope, origin, span, target_id, arguments);

    // 3. Mise à jour de la Table via la nouvelle API
    // Strictement équivalent à ton if/else précédent mais avec :
    // - L'idempotence automatique (grâce au cache usages_index)
    // - L'utilisation de l'Entry API interne (plus rapide)
    // - La garantie que l'index de cache est créé
    table.add_usage(ident, usage)
}

/// Initializes the symbol table from a `TypedList` AST syntax node.
///
/// This function processes a `TypedList` node by iterating over its children, each representing
/// a typed item such as a typing declaration, constant, or variable. It recursively initializes
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
    context: &PassContext,
    table: &mut SymbolTable,
    node_ref: &NodeRef<AstNode>,
    scope: Scope,
) -> Result<(), SemanticError> {
    let children = node_ref.node().children();

    if children.is_empty() {
        return Ok(());
    }

    for &child_id in children {
        let child_ref = context.syntax_tree().try_node_ref(child_id)?;
        init_from_typed_item(context, table, &child_ref, scope.clone())?;
    }

    Ok(())
}

/// Initializes symbol declarations from a `TypedItem` AST syntax node.
///
/// A `TypedItem` typically represents a declaration associating one or more symbols
/// (e.g., variables or constants) with an optional typing annotation. For example:
/// ```text
/// (?x ?y - location)  // symbols: ?x, ?y; typing: location
/// (?z)                // symbol: ?z; no associated typing
/// ```
///
/// This function parses the symbols and their optional typing, then registers them
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
///   - If there is exactly one child, treats it as an untyped declaration (empty typing list).
///   - If there are exactly two children, parses the second child as the typing annotation.
///   - Returns an error if the number of children differs from 1 or 2.
/// - Processes the first child as the list of symbols, associating them with the parsed types.
/// - Adds each symbol declaration to the symbol table under the given scope.
///
/// # Errors
///
/// Returns `SymbolTableError` if:
/// - The node kind is not `TypedItem`.
/// - The number of children is invalid (not 1 or 2).
/// - The typing annotation fails to parse correctly.
/// - Adding declarations to the symbol table fails.
///
/// # Example
///
/// ```text
/// // Declare ?x and ?y with typing 'location'
/// (?x ?y - location)
///
/// // Declare ?z with no associated typing
/// (?z)
/// ```
///
fn init_from_typed_item(
    context: &PassContext,
    table: &mut SymbolTable,
    node_ref: &NodeRef<AstNode>,
    scope: Scope,
) -> Result<(), SemanticError> {
    let syntax_tree = context.syntax_tree();
    let node = node_ref.node();

    // 2. On traite les éléments
    let elt_id = node.try_child(0)?;
    let elt_ref = syntax_tree.try_node_ref(elt_id)?;
    let is_type_definition = elt_ref.node().kind() == AstKind::PrimitiveType;

    // 1. Initialisation par défaut (vide)
    let mut ty_node_ids = Vec::new();
    let mut types = Type::new();

    if node.arity() == 2 {
        let ty_id = node.try_child(1)?;
        let ty_node_ref = syntax_tree.try_node_ref(ty_id)?;

        // PROPRE : On utilise le retour de init_from_type qui contient déjà tout.
        // On n'a plus besoin de manipuler les enfants manuellement ici,
        // c'est extract_type qui s'en occupe.
        let (extracted_types, extracted_ids) = init_from_type(
            context,
            table,
            &ty_node_ref,
            scope.clone(),
            is_type_definition,
        )?;

        types = extracted_types;
        ty_node_ids = extracted_ids;

        // CAS PARTICULIER : Si extract_type n'a rien renvoyé (ex: le nœud de type existe
        // mais n'a pas d'enfants PrimitiveType), on utilise ty_id comme fallback.
        if ty_node_ids.is_empty() && !types.is_empty() {
            ty_node_ids = vec![ty_id];
        }
    }

    init_from_typed_item_elements(context, table, &elt_ref, scope, types, ty_node_ids)
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
/// * `types` — Vector of typing checker identifiers associated with the element.
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
    context: &PassContext,
    table: &mut SymbolTable,
    node_ref: &NodeRef<AstNode>,
    scope: Scope,
    types: Type<SymbolId>,
    ty_node_ids: Vec<NodeId>,
) -> Result<(), SemanticError> {
    // Match on the AST node kind to determine processing ops
    match node_ref.node().kind() {
        AstKind::PrimitiveType => {
            let symbol_ref = node_ref.node().try_symbol()?;
            let ident = symbol_ref.id();

            let mut promoted = false;

            // 1. On cherche si une "coquille vide" existe (lecture seule)
            if let Some(entry) = table.get_symbol(ident) {
                let old_id = entry.declarations().iter().find_map(|d| {
                    // d est maintenant un &Declaration
                    if d.symbol().kind() == SymbolKind::PrimitiveType && d.ty().is_none() {
                        // On récupère le NodeId via .source()
                        Some(d.source())
                    } else {
                        None
                    }
                });

                // 2. Si trouvée, on procède à la promotion via la nouvelle API
                if let Some(id) = old_id {
                    if let Ok(decl) = table.promote_declaration(id, node_ref.id()) {
                        // Mise à jour des données sémantiques sur l'objet existant
                        decl.set_ty(types.clone());
                        decl.set_type_sources(ty_node_ids.clone());
                        decl.set_span(node_ref.node().span().clone());

                        promoted = true;
                    }
                }
            }

            // 3. Si aucune promotion (cas standard ou premier passage), on crée normalement
            if !promoted {
                add_declaration_symbol(
                    context,
                    table,
                    node_ref,
                    scope.clone(),
                    Some(types.clone()),
                    Some(ty_node_ids),
                    None,
                    None,
                    false,
                )?;
            }
        }

        AstKind::Object | AstKind::Variable => {
            // Add a declaration symbol with the provided types for simple typed elements
            add_declaration_symbol(
                context,
                table,
                node_ref,
                scope.clone(),
                Some(types.clone()),
                Some(ty_node_ids),
                None,
                None,
                false,
            )?;
        }

        AstKind::AtomicFunctionSkeleton => {
            // Recursively initialize symbols for atomic function skeleton elements
            init_from_atomic_function_skeleton(
                context,
                table,
                node_ref,
                scope.clone(),
                types.clone(),
                ty_node_ids,
            )?;
        }

        found => {
            // Return an error for unexpected AST kinds instead of panicking
            return Err(SemanticError::unexpected_node_kind(
                node_ref.id(),
                vec![
                    AstKind::PrimitiveType,
                    AstKind::Object,
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
/// * `types` — A vector of typing checker identifiers associated with the function.
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
    context: &PassContext,
    table: &mut SymbolTable,
    node_ref: &NodeRef<AstNode>,
    scope: Scope,
    mut types: Type<SymbolId>,
    mut ty_node_ids: Vec<NodeId>, // On le rend mutable pour la cohérence
) -> Result<(), SemanticError> {
    let node = node_ref.node();

    // --- Default Type Injection & AST Consistency ---
    if types.is_empty() {
        types.add_type(context.interner().try_lookup_symbol(NUMBER_TYPE)?);
        // PROPRE : Si on injecte un type par défaut, on associe l'ID du nœud
        // de la fonction pour que le Finalizer sache où "pointer" ce type.
        if ty_node_ids.is_empty() {
            ty_node_ids.push(AstNode::NODE_ID_NUMBER);
        }
    }
    // ------------------------------------------------

    let functor_id = node.try_child(0)?;
    let functor_ref = context.syntax_tree().try_node_ref(functor_id)?;
    if functor_ref.node().kind() != AstKind::FunctionSymbol {
        return Err(SemanticError::unexpected_node_kind(
            functor_ref.id(),
            vec![AstKind::FunctionSymbol],
            functor_ref.node().kind(),
        ));
    }

    let arguments_id = node.try_child(1)?;
    let arguments = context.syntax_tree().try_node_ref(arguments_id)?;

    init_from_typed_list(
        context,
        table,
        &arguments,
        Scope::new(node_ref.id(), Some(&scope)),
    )?;

    // Step 4: Extraction du triplet complet (Args, IDs symboles, IDs types)
    let (args, ids, _arg_ty_ids) = extract_arguments_from_typed_list(context, &arguments)?;

    // Step 5: Enregistrement
    add_declaration_symbol(
        context,
        table,
        &functor_ref,
        scope.clone(),
        Some(types),
        Some(ty_node_ids),
        Some(args),
        Some(ids),
        // NOTE: Si add_declaration_symbol n'accepte pas arg_ty_ids,
        // il faudra s'assurer que la table des symboles peut stocker
        // les types des arguments de manière cohérente.
        false,
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
/// This function delegates the common work to `init_from_def`, indicating that the
/// definition includes a body.
fn init_from_action_def(
    context: &PassContext,
    table: &mut SymbolTable,
    node_ref: &NodeRef<AstNode>,
    scope: Scope,
) -> Result<(), SemanticError> {
    Ok(init_from_def(context, table, node_ref, scope, true)?)
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
    context: &PassContext,
    table: &mut SymbolTable,
    node_ref: &NodeRef<AstNode>,
    scope: Scope,
) -> Result<(), SemanticError> {
    Ok(init_from_def(context, table, node_ref, scope, true)?)
}

/// Initializes a durative action definition and safely injects the implicit `?duration` variable.
///
/// This method follows a prioritized initialization strategy:
/// 1. **Standard Initialization**: Calls `init_from_def` to process the action name,
///    parameters, and requirements, returning the internal `action_scope`.
/// 2. **Shadowing Check**: Verifies if the user has already defined a parameter named
///    `?duration`. If found, the injection is skipped to respect user-defined shadowing.
/// 3. **Type Resolution**: Resolves the `number` type ID. It prioritizes a user-defined
///    `number` type from the domain but falls back to the internal [`ParseContext::NODE_ID_NUMBER`]
///    if none exists.
/// 4. **Implicit Injection**: Registers the built-in `?duration` ([`ParseContext::NODE_ID_DURATION`])
///    into the action's local scope with the resolved type information.
///
/// # Arguments
/// * `node_ref` - Reference to the AST node defining the durative action.
/// * `ast` - The full Abstract Syntax Tree.
/// * `scope` - The parent scope in which this action is defined (usually the global domain scope).
///
/// # Errors
/// Returns a [`SemanticError`] if the action name is already defined or if symbol
/// registration fails due to an unexpected structural error in the AST.
fn init_from_durative_action_def(
    context: &PassContext,
    table: &mut SymbolTable,
    node_ref: &NodeRef<AstNode>, // L'ID de l'action (ex: 479)
    scope: Scope,
) -> Result<(), SemanticError> {
    Ok(init_from_def(context, table, node_ref, scope, true)?)
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
    context: &PassContext,
    table: &mut SymbolTable,
    node_ref: &NodeRef<AstNode>,
    scope: Scope,
) -> Result<(), SemanticError> {
    Ok(init_from_def(context, table, node_ref, scope, false)?)
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
    context: &PassContext,
    table: &mut SymbolTable,
    node_ref: &NodeRef<AstNode>,
    scope: Scope,
    has_body: bool,
) -> Result<(), SemanticError> {
    let syntax_tree = context.syntax_tree();
    let node = node_ref.node();

    // 1. On récupère le nom de la définition (ex: le nom de l'action)
    let name_id = node.try_child(0)?;
    let name = syntax_tree.try_node_ref(name_id)?;

    // 2. Navigation vers la liste des paramètres
    let parameters_def_id = node.try_child(1)?;
    let parameters_def = syntax_tree.try_node_ref(parameters_def_id)?;
    let parameters_id = parameters_def.node().try_child(0)?;
    let parameters = syntax_tree.try_node_ref(parameters_id)?;

    // 3. Initialisation sémantique des paramètres dans un scope imbriqué
    init_from_typed_list(
        context,
        table,
        &parameters,
        Scope::new(node_ref.id(), Some(&scope)),
    )?;

    // 4. Extraction complète (Triplet : Args, IDs des symboles, IDs des types)
    // C'est ici qu'on récupère 'ty_ids' pour s#22 et les autres.
    let (params, ids, _ty_ids) = extract_arguments_from_typed_list(context, &parameters)?;

    // 5. Enregistrement de la déclaration avec TOUTES les informations AST
    add_declaration_symbol(
        context,
        table,
        &name,
        scope.clone(),
        None,         // Le symbole de l'action lui-même n'a pas de type
        None,         // Donc pas d'ID de type pour le nom de l'action
        Some(params), // Les arguments typés
        Some(ids),    // Les IDs des variables/paramètres
        false,
    )?;

    // 6. Initialisation récursive du corps (body)
    if has_body {
        let body_id = node.try_child(2)?;
        let body = syntax_tree.try_node_ref(body_id)?;
        init_from(
            context,
            table,
            &body,
            Scope::new(node_ref.id(), Some(&scope)),
        )?;
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
    context: &PassContext,
    table: &mut SymbolTable,
    node_ref: &NodeRef<AstNode>,
    scope: Scope,
) -> Result<(), SemanticError> {
    let syntax_tree = context.syntax_tree();
    let node = node_ref.node();

    // Ensure the node is a quantified expression: `Exists` or `Forall`
    match node.kind() {
        AstKind::Exists | AstKind::Forall => {}
        other => {
            return Err(SemanticError::unexpected_node_kind(
                node_ref.id(),
                vec![AstKind::Exists, AstKind::Forall],
                other,
            ));
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
    init_from_typed_list(context, table, variables, nested_scope.clone())?;

    // Step 4: Recursively initialize the inner expression within the same nested scope
    init_from(context, table, expression, nested_scope)?;

    Ok(())
}

/// Initializes the symbol table for an atomic formula skeleton from the AST.
///
/// An atomic formula skeleton consists of a `PredicateSymbol` followed by a `TypedList`
/// of arguments. This function performs the following:
///
/// 1. Validates that the first child is a `PredicateSymbol`.
/// 2. Creates a local scope for the argument list to isolate variables (e.g., in a `:derived` body).
/// 3. Initializes symbol information for the argument list.
/// 4. Registers the predicate in the symbol table.
/// 5. If `is_derived` is true, promotes the symbol's kind to `DerivedPredicate` regardless
///    of its original syntactic kind in the AST.
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
/// * `Err(SemanticError)` if the AST structure is unexpected or initialization fails.
///
/// # Errors
///
/// Returns [`SemanticError`] in the following cases:
/// - If child nodes cannot be retrieved (invalid arity).
/// - If the first child is not of kind `PredicateSymbol`.
/// - If the argument list extraction or typed list initialization fails.
///
/// # Example
///
/// ```rust
/// // Standard predicate declaration
/// builder.init_from_atomic_formula_skeleton(&node, &ast, scope, false)?;
///
/// // Derived predicate definition
/// builder.init_from_atomic_formula_skeleton(&node, &ast, scope, true)?;
/// ```
fn init_from_atomic_formula_skeleton(
    context: &PassContext,
    table: &mut SymbolTable,
    node_ref: &NodeRef<AstNode>,
    scope: Scope,
) -> Result<(), SemanticError> {
    let syntax_tree = context.syntax_tree();
    let node = node_ref.node();

    // Step 1: Get and validate the predicate node (first child)
    let predicate_id = node.try_child(0)?;
    let predicate = syntax_tree.try_node_ref(predicate_id)?;

    if predicate.node().kind() != AstKind::PredicateSymbol {
        return Err(SemanticError::unexpected_node_kind(
            predicate.id(),
            vec![AstKind::PredicateSymbol],
            predicate.node().kind(),
        ));
    }

    // Step 2: Get the argument list node (second child)
    let arguments_id = node.try_child(1)?;
    let arguments = syntax_tree.try_node_ref(arguments_id)?;

    // Step 3: Initialize symbols from the argument list (typed variables/constants)
    // On crée le scope local pour les arguments (ex: les variables d'un prédicat)
    init_from_typed_list(
        context,
        table,
        &arguments,
        Scope::new(node_ref.id(), Some(&scope)),
    )?;

    // Step 4: Extract everything from the list (Sémantique, IDs symboles, IDs types)
    // C'est ici que le triplet (args, ids, ty_ids) devient vital
    let (args, ids, _ty_ids) = extract_arguments_from_typed_list(context, &arguments)?;

    // Step 5: Register the predicate symbol declaration
    // On passe enfin 'ty_ids' à add_declaration_symbol.
    // Même si le prédicat lui-même n'a pas de type (None),
    // ses arguments, eux, en ont un (ty_ids).
    add_declaration_symbol(
        context,
        table,
        &predicate,
        scope,
        None,       // Le prédicat n'a pas de type de retour (c'est un booléen)
        None,       // Donc pas d'ID de type de retour
        Some(args), // La liste des arguments typés
        Some(ids),  // Les IDs des variables
        false,      // is_derived
    )?;

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
    context: &PassContext,
    table: &mut SymbolTable,
    node_ref: &NodeRef<AstNode>,
    scope: Scope,
) -> Result<(), SemanticError> {
    let children = node_ref.node().children();

    // 1. HEAD MANAGEMENT (The Caller)
    // We treat the first child as the predicate/task head.
    if let Some(&predicate_id) = children.first() {
        let predicate_ref = context.syntax_tree().try_node_ref(predicate_id)?;

        // We clone the children IDs to provide the Usage with its full syntactic context.
        // This 'flattens' the AST relationship into the Symbol Table for easier type checking.
        let arguments = children[1..].to_vec();
        add_symbol_usage(
            context,
            table,
            &predicate_ref,
            Some(arguments),
            scope.clone(),
        )?;
    }

    // 2. ARGUMENTS MANAGEMENT (The Parameters)
    // Iterate through the remaining children. We use skip(1) to avoid
    // double-processing the head node, ensuring each argument is initialized
    // according to its specific AstKind (Variable, Object, etc.).
    for &child_id in children.iter().skip(1) {
        let child_ref = context.syntax_tree().try_node_ref(child_id)?;
        init_from(context, table, &child_ref, scope.clone())?;
    }

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
    context: &PassContext,
    node_ref: &NodeRef<AstNode>,
) -> Result<(TypedList<SymbolId, SymbolId>, Vec<NodeId>, Vec<NodeId>), SemanticError> {
    let mut typed_arguments = TypedList::new();
    let mut argument_node_ids = Vec::new();
    let mut all_ty_node_ids = Vec::new(); // <-- Le nouveau vecteur pour les IDs de types

    for typed_item_id in node_ref.node().children() {
        let typed_item_ref = context.syntax_tree().try_node_ref(*typed_item_id)?;

        // 1. On récupère le couple (Arguments, IDs de types) de l'item
        let (args, ty_node_ids) = extract_arguments_from_typed_item(context, &typed_item_ref)?;

        // 2. On collecte l'ID de chaque argument (ex: le nœud de la variable)
        // Note: Si un TypedItem contient plusieurs variables (ex: ?x ?y - type),
        // il faut s'assurer de pousser l'ID pour chaque argument extrait.
        let arg_id = typed_item_ref.node().try_child(0)?;
        for _ in 0..args.len() {
            argument_node_ids.push(arg_id);
            // On associe les IDs de types reçus à chaque argument de ce TypedItem
            all_ty_node_ids.extend(ty_node_ids.clone());
        }

        typed_arguments.extend(args);
    }

    // On retourne les 3 listes synchronisées
    Ok((typed_arguments, argument_node_ids, all_ty_node_ids))
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
/// - A typing extraction fails.
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
    context: &PassContext,
    typed_item_ref: &NodeRef<AstNode>,
) -> Result<(TypedList<SymbolId, SymbolId>, Vec<NodeId>), SemanticError> {
    // Retourne le couple (Sémantique, AST)
    let syntax_tree = context.syntax_tree();
    let node = typed_item_ref.node();

    // Étape 1 : Extraction du couple (Sémantique, IDs de nœuds)
    // On récupère les deux informations de extract_type sans en jeter aucune.
    let (types, ty_node_ids) = match node.arity() {
        1 => (Type::new(), Vec::new()), // Pas de type -> listes vides (0 vs 0) cohérentes
        2 => {
            let ty_id = node.try_child(1)?;
            let ty_node_ref = syntax_tree.try_node_ref(ty_id)?;
            // On suppose ici que extract_type a été modifiée pour renvoyer le tuple
            extract_type(context, &ty_node_ref)?
        }
        n => {
            return Err(SemanticError::invalid_node_arity(
                typed_item_ref.id(),
                AstKind::TypedItem,
                n,
                vec![1, 2],
            ));
        }
    };

    // Étape 2 : Extraction de l'identifiant (Objet ou Variable)
    let elt_id = node.try_child(0)?;
    let elt = syntax_tree.try_node_ref(elt_id)?;

    let mut typed_arguments = TypedList::new();

    match elt.node().kind() {
        AstKind::Object | AstKind::Variable => {
            let symbol_ref = elt.node().try_symbol()?;
            let name = symbol_ref.id();

            // On crée le TypedSymbol normalement (il ne porte que la sémantique)
            typed_arguments.push(TypedSymbol::new(name, types));
        }
        found => {
            return Err(SemanticError::unexpected_node_kind(
                elt.id(),
                vec![AstKind::Object, AstKind::Variable],
                found,
            ));
        }
    }

    // Étape 3 : On renvoie les arguments ET les IDs de types associés
    // C'est ce retour qui permettra à extract_arguments_from_typed_list de
    // remplir all_ty_node_ids proprement.
    Ok((typed_arguments, ty_node_ids))
}

/// Extracts primitive type_checker identifiers from a `Type` AST node without recording symbol usage.
///
/// This function is used to interpret a typing declaration from the AST. It expects the node to be of kind `Type`,
/// and each of its children to be of kind `PrimitiveType`. The extracted identifiers are collected into a [`Type`] object.
///
/// Unlike `init_from_type`, this function does **not** register symbol usage in the current scope—
/// it only performs static extraction and validation.
///
/// # Arguments
///
/// * `type_ref` - A reference to the AST node expected to represent a typing (`AstKind::Type`).
/// * `ast` - The complete AST arena, used to access nodes and symbol interners.
///
/// # Returns
///
/// Returns a [`Type`] structure containing all extracted primitive identifiers on success.
///
/// # Errors
///
/// Returns [`SymbolTableError::UnexpectedNodeKind`] if:
/// - Any child node is not of kind `PrimitiveType`.
/// - A referenced node or symbol is invalid in the arena.
fn extract_type(
    context: &PassContext,
    type_ref: &NodeRef<AstNode>,
) -> Result<(Type<SymbolId>, Vec<NodeId>), SemanticError> {
    // On renvoie un tuple
    let arena = context.syntax_tree();
    let mut super_types = Type::new();
    let mut node_ids = Vec::new();

    for ty_id in type_ref.node().children() {
        let ty_ref = arena.try_node_ref(*ty_id)?;

        if ty_ref.node().kind() == AstKind::PrimitiveType {
            let symbol_ref = ty_ref.node().try_symbol()?;
            super_types.add_type(symbol_ref.id());
            node_ids.push(*ty_id); // ON GARDE L'ID ICI
        } else {
            return Err(SemanticError::unexpected_node_kind(
                ty_ref.id(),
                vec![AstKind::PrimitiveType],
                ty_ref.node().kind(),
            ));
        }
    }

    Ok((super_types, node_ids))
}

/// Initializes type_checker information and records symbol usage in the given scope.
///
/// This function processes an AST node of kind `Type`, which is expected to list one or more
/// `PrimitiveType` identifiers (e.g., typing names). It extracts all these identifiers,
/// constructs a `Type` object containing them, and registers each as a symbol usage in the provided scope.
///
/// # Arguments
///
/// * `type_ref` - A reference to the AST node expected to be of kind `Type`.
/// * `ast` - The arena-backed AST structure used for looking up child nodes and interning.
/// * `scope` - The current lexical scope where typing usages should be recorded.
///
/// # Returns
///
/// Returns a [`Type`] object containing all extracted typing identifiers if successful.
///
/// # Errors
///
/// Returns a [`SymbolTableError`] if:
/// - The `type_ref` node is invalid or has children that are not of kind `PrimitiveType`.
/// - A child node reference is invalid or symbol usage registration fails.
fn init_from_type(
    context: &PassContext,
    table: &mut SymbolTable,
    type_ref: &NodeRef<AstNode>,
    scope: Scope,
    is_type_def: bool,
) -> Result<(Type<SymbolId>, Vec<NodeId>), SemanticError> {
    // 1. On extrait les types (ex: [object]) et leurs IDs de nœuds
    let (super_types, ty_node_ids) = extract_type(context, type_ref)?;

    // 2. Traitement de chaque type trouvé à droite du tiret '-'
    for &ty_id in &ty_node_ids {
        let ty_ref = context.syntax_tree().try_node_ref(ty_id)?;
        let symbol_ref = ty_ref.node().try_symbol()?;
        let ident = symbol_ref.id();

        if is_type_def {
            // On cherche si parmi les déclarations existantes, il y en a une de genre PrimitiveType
            let already_has_primitive_type = table
                .get_symbol(ident)
                .map(|entry| {
                    // .iter() remplace .values()
                    entry
                        .declarations()
                        .iter()
                        .any(|d| d.symbol().kind() == SymbolKind::PrimitiveType)
                })
                .unwrap_or(false);

            if !already_has_primitive_type {
                // On le déclare comme une racine (PrimitiveType sans parent)
                // Cela crée l'entrée manquante pour le SignatureChecker
                add_declaration_symbol(
                    context,
                    table,
                    &ty_ref,
                    scope.clone(),
                    None, // Pas de super-type (c'est une racine)
                    None,
                    None,
                    None,
                    false, // Non dérivé
                )?;
            } else {
                add_symbol_usage(context, table, &ty_ref, None, scope.clone())?;
            }
        } else {
            // Si le symbole existe déjà, on enregistre simplement son usage
            add_symbol_usage(context, table, &ty_ref, None, scope.clone())?;
        }
    }

    // 3. On renvoie le tuple complet pour que le TypedItem puisse l'associer aux enfants
    Ok((super_types, ty_node_ids))
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
    context: &PassContext,
    table: &mut SymbolTable,
    node_ref: &NodeRef<AstNode>,
    scope: Scope,
) -> Result<(), SemanticError> {
    let syntax_tree = context.syntax_tree();
    let node = node_ref.node();

    // --- Extract the tag identifier child (expected to be a TaskID) ---
    let tag_id = node.try_child(0)?; // Error if missing
    let tag = syntax_tree.try_node_ref(tag_id)?; // Error if invalid node reference

    // --- Register the tag as a declaration symbol in the current scope ---
    add_declaration_symbol(
        context,
        table,
        &tag,
        scope.clone(),
        None,
        None,
        None,
        None,
        false,
    )?; // May fail if duplicate, invalid kind, etc.

    // --- Extract the task definition (second child) ---
    let task_id = node.try_child(1)?; // Error if missing
    let task = syntax_tree.try_node_ref(task_id)?; // Error if invalid reference

    // --- Initialize symbols for the task formula (likely a predicate or action expression) ---
    init_from_atomic_formula(context, table, &task, scope.clone())?; // Recursively builds symbol table for inner task

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
    context: &PassContext,
    table: &mut SymbolTable,
    node_ref: &NodeRef<AstNode>,
    scope: Scope,
) -> Result<(), SymbolTableError> {
    let syntax_tree = context.syntax_tree();

    // --- Extract the first child node (expected to be a TaskID) ---
    let t1_id = node_ref.node().try_child(0)?; // Error if no first child
    let t1 = syntax_tree.try_node_ref(t1_id)?; // Error if invalid node reference
    add_symbol_usage(context, table, &t1, None, scope.clone())?; // Registers t1 as a symbol usage

    // --- Extract the second child node (also expected to be a TaskID) ---
    let t2_id = node_ref.node().try_child(1)?; // Error if no second child
    let t2 = syntax_tree.try_node_ref(t2_id)?; // Error if invalid node reference
    add_symbol_usage(context, table, &t2, None, scope.clone())?; // Registers t2 as a symbol usage

    Ok(())
}

/// Initializes the symbol table for a derived predicate definition.
///
/// This function handles the `:derived` PDDL structure by:
/// 1. Creating a single local scope for both the skeleton and the body.
/// 2. Manually extracting the predicate symbol and arguments from the skeleton.
/// 3. Registering the derived predicate in the parent (domain) scope.
/// 4. Initializing the body formula using the populated local scope.
///
/// # Arguments
///
/// * `node_ref` - Reference to the AST node for the derived definition.
/// * `ast` - The AST arena.
/// * `scope` - The parent scope (usually the global Domain).
fn init_from_derived_predicate_def(
    context: &PassContext,
    table: &mut SymbolTable,
    node_ref: &NodeRef<AstNode>,
    scope: Scope,
) -> Result<(), SemanticError> {
    let syntax_tree = context.syntax_tree();
    let node = node_ref.node();

    // 1. Scope unique pour la définition (pour que le body voie les paramètres)
    let derived_scope = Scope::new(node_ref.id(), Some(&scope));

    // 2. Accès au skeleton
    let skeleton_id = node.try_child(0)?;
    let skeleton_node = syntax_tree.try_node_ref(skeleton_id)?;

    let predicate_id = skeleton_node.node().try_child(0)?;
    let predicate_ref = syntax_tree.try_node_ref(predicate_id)?;

    let args_id = skeleton_node.node().try_child(1)?;
    let args_ref = syntax_tree.try_node_ref(args_id)?;

    // Initialisation sémantique dans le scope dérivé
    init_from_typed_list(context, table, &args_ref, derived_scope.clone())?;

    // --- CORRECTION : Extraction du triplet complet ---
    // On récupère 'ty_ids' (les IDs de types des arguments)
    let (args, ids, _ty_ids) = extract_arguments_from_typed_list(context, &args_ref)?;

    // Enregistrement du prédicat dérivé
    add_declaration_symbol(
        context,
        table,
        &predicate_ref,
        scope, // Le prédicat appartient au scope global (domain)
        None,  // Pas de type de retour pour un prédicat
        None,  // Pas d'ID de type de retour
        Some(args),
        Some(ids),
        // Note: Ici aussi, assure-toi que add_declaration_symbol
        // ou la structure interne traite 'ty_ids'.
        true, // is_derived = true
    )?;

    // 3. Process du corps de la formule
    let body_id = node.try_child(1)?;
    let body_ref = syntax_tree.try_node_ref(body_id)?;

    init_from(context, table, &body_ref, derived_scope)?;

    Ok(())
}
