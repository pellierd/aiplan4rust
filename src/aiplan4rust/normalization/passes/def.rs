use crate::aiplan4rust::arena::ArenaNode;
use crate::aiplan4rust::diagnostic::{Diagnostic, DiagnosticManager, Provider};
use crate::aiplan4rust::interner::SymbolInterner;
use crate::aiplan4rust::lang::SymbolId;
use crate::aiplan4rust::normalization::passes::NormalizationPassError;
use crate::aiplan4rust::syntax::ast::{Ast, AstKind, AstNode};
use crate::aiplan4rust::syntax::Span;
use crate::aiplan4rust::tree::{NodeId, Tree};
use std::collections::hash_map::Entry;
use std::collections::{HashMap, HashSet};

/// Orchestrates the normalization of a specific definition block (e.g., :types, :functions).
///
/// This is the main entry point for the normalization pass. It follows a two-step
/// process to ensure user-facing feedback is generated before the AST is modified.
///
/// # Workflow
///
/// 1. **Diagnostic Phase**: Scans the definition list to identify duplicates and
///    reports them as warnings via the `DiagnosticManager`. This is done on the
///    original, unmodified AST to ensure accurate source spans.
/// 2. **Mutation Phase**: Performs the actual merging of duplicates. It collapses
///    nodes with identical signatures and combines their type hierarchies.
///
/// # Arguments
///
/// * `ast` - The mutable Abstract Syntax Tree to normalize.
/// * `diagnostic_manager` - Collector for semantic warnings and errors.
/// * `kind` - The specific PDDL section to target (e.g., `AstKind::Types`).
///
/// # Returns
///
/// Returns `Ok(true)` if the AST was structurally modified (duplicates merged),
/// or `Ok(false)` if no changes were necessary.
pub fn normalize_def(
    ast: &mut Ast,
    diagnostic_manager: &mut DiagnosticManager,
    kind: AstKind,
) -> Result<bool, NormalizationPassError> {
    // 1. Attempt to locate the unique node for the requested definition section.
    let def_id = match ast.find_node_id_of_kind(kind) {
        Some(id) => id,
        // If the section doesn't exist (e.g., no :functions), we exit early.
        None => return Ok(false),
    };

    // 2. Perform a read-only pass to collect and report duplication warnings.
    // This must happen BEFORE merging to preserve the original source pointers.
    report_duplicated_declaration_warning(def_id, ast, diagnostic_manager, kind)?;

    // 3. Perform the in-place AST transformation to merge the duplicates.
    let modified = merge_duplicate_declarations(def_id, ast)?;

    //verify_tree_integrity(ast.syntax_tree())?;
    Ok(modified)
}

#[cfg(debug_assertions)]
fn verify_tree_integrity(syntax_tree: &Tree<AstNode>) -> Result<(), NormalizationPassError> {
    // On itère sur les IDs des nœuds dans l'ordre de traversée
    for (node_id, node) in syntax_tree.preorder().ids() {
        for &child_id in node.children() {
            let child = syntax_tree.try_node(child_id)?;

            // On compare l'ID du parent stocké dans l'enfant
            // avec l'ID du nœud actuel (node_id)
            assert_eq!(
                child.parent(),
                Some(node_id),
                "Incohérence détectée : l'enfant {:?} prétend avoir le parent {:?} au lieu de {:?}",
                child_id,
                child.parent(),
                node_id
            );
        }
    }
    Ok(())
}
/// Validates that duplicated declarations have compatible return types or supertypes.
///
/// In PDDL, the `number` type is primitive and incompatible with object types.
/// This function enforces two rules:
/// 1. A symbol cannot change its return type from `number` to something else (or vice versa) in a duplicate.
/// 2. The `number` type cannot be part of an `either` block (it must be the sole return type).
///
/// # Arguments
///
/// * `seen` - A reference to the [`HashMap`] containing all gathered [`Occurrences`],
///   indexed by their unique [`DefinitionKey`].
/// * `ast` - A reference to the [`Ast`] used to resolve the global `number` symbol ID.
/// * `diagnostic_manager` - A mutable reference to the [`DiagnosticManager`] where
///   incompatibility errors will be registered.
///
/// # Errors
///
/// Returns a [`NormalizationPassError`] if the validation process encounters
/// unexpected AST states.
/// Validates that duplicated declarations have compatible type signatures.
///
/// This check ensures that symbols (functions, constants, etc.) maintain
/// consistent typing across multiple declarations and respect PDDL's
/// primitive type constraints (notably for `number`).
///
/// # Arguments
///
/// * `seen` - The map of collected occurrences grouped by unique signature.
/// * `kind` - The [`AstKind`] of the definitions being validated, used for
///   homogeneous error reporting.
/// * `ast` - The AST used to resolve the built-in `number` type and source ID.
/// * `diagnostic_manager` - The manager where incompatibility errors are recorded.
fn report_declaration_incompatibility(
    seen: &HashMap<DefinitionKey, Occurrences>,
    kind: AstKind,
    ast: &Ast,
    diagnostic_manager: &mut DiagnosticManager,
) -> Result<(), NormalizationPassError> {
    // Retrieve the 'number' symbol ID once.
    let number_id = SymbolInterner::NUMBER_SYMBOL_ID;

    for (key, occurrences) in seen {
        // --- Optimization (Hoisting) ---
        let first_types = &occurrences.first_supertypes;
        let first_has_number = first_types.contains(&number_id);

        // PDDL Rule: 'number' is a primitive type and cannot be part of an 'either' block.
        let first_is_invalid_either = first_has_number && first_types.len() > 1;

        for (dupe_types, dupe_span) in &occurrences.duplicates {
            let dupe_has_number = dupe_types.contains(&number_id);
            let dupe_is_invalid_either = dupe_has_number && dupe_types.len() > 1;

            // A conflict occurs if:
            // 1. Type mismatch (one is 'number', the other is an object/type).
            // 2. The original declaration was an invalid '(either number ...)'.
            // 3. The current duplicate is an invalid '(either number ...)'.
            let conflict = (first_has_number != dupe_has_number)
                || first_is_invalid_either
                || dupe_is_invalid_either;

            if conflict {
                // Convert HashSets to sorted Vecs for deterministic error reporting.
                // This ensures the order of types in the message is always the same.
                let mut expected: Vec<_> = first_types.iter().copied().collect();
                let mut found: Vec<_> = dupe_types.iter().copied().collect();

                expected.sort();
                found.sort();

                // Pass the collected type lists to the diagnostic.
                // This allows the renderer to show: "Expected (either a b), found number".
                diagnostic_manager.add_diagnostic(
                    Diagnostic::error_incompatible_type_declarations(
                        key.symbol,
                        kind,
                        expected,               // expected_types
                        found,                  // found_types
                        *dupe_span,             // offending_span (Copy trait used)
                        occurrences.first_span, // original_span
                        Provider::Normalizer,
                        ast.source_id(),
                    ),
                );
            }
        }
    }

    Ok(())
}

/// Scans the definition block for duplicates and reports them as warnings.
///
/// This function identifies symbols that have been declared multiple times with the
/// same signature. It aggregates all duplicate occurrences to produce a grouped
/// diagnostic message, helping the user clean up the PDDL source.
///
/// # Arguments
///
/// * `def_id` - The [`NodeId`] of the definition section (e.g., `:types` or `:functions`)
///   used as the root for the duplication scan.
/// * `ast` - A reference to the [`Ast`]. It is used as a read-only provider to
///   resolve node hierarchies and symbol identifiers.
/// * `diagnostic_manager` - A mutable reference to the [`DiagnosticManager`].
///   Required to register and store the generated warning diagnostics.
/// * `kind` - The [`AstKind`] of the current section. This allows the diagnostic
///   to specify the context of the duplication (e.g., "duplicate function").
///
/// # Errors
///
/// Returns a [`NormalizationPassError`] if the AST structure is inconsistent or
/// if a required child node cannot be retrieved.
fn report_duplicated_declaration_warning(
    def_id: NodeId,
    ast: &Ast,
    diagnostic_manager: &mut DiagnosticManager,
    kind: AstKind,
) -> Result<(), NormalizationPassError> {
    // 1. Group all declarations by their unique signature (identity)
    let seen = collect_duplicated_declarations(def_id, ast)?;

    for (key, occurrences) in seen {
        // Only trigger a warning if at least one duplicate exists
        if !occurrences.duplicates.is_empty() {
            // 2. Optimization: Pre-calculate capacity to avoid multiple re-allocations
            let total_elements = occurrences
                .duplicates
                .iter()
                .map(|(types, _)| types.len())
                .sum();

            let mut duplicate_types = Vec::with_capacity(total_elements);
            let mut duplicate_spans = Vec::with_capacity(total_elements);

            // 3. Flatten the nested structure for the diagnostic engine
            // Each individual type in a duplicate declaration is mapped to the same source span.
            for (supertypes, span) in occurrences.duplicates {
                for st in supertypes {
                    duplicate_types.push(st);
                    // Clone is necessary as the same span is reused for each type in 'either' blocks
                    duplicate_spans.push(span.clone());
                }
            }

            // 4. Create and register the warning using the first occurrence as the anchor
            let warning = Diagnostic::warning_duplicate_declaration(
                key.symbol,
                kind,
                duplicate_types,
                duplicate_spans,
                Provider::Normalizer,
                ast.source_id(),
                occurrences.first_span,
            );
            diagnostic_manager.add_diagnostic(warning);
        }
    }
    Ok(())
}

/// Scans a definition list to collect all occurrences of duplicated declarations.
///
/// This function performs a read-only pass over the AST to group declarations by their
/// identity ([`DefinitionKey`]). It captures both the initial declaration and all
/// subsequent duplicates, including their respective supertypes and source locations.
///
/// # Arguments
///
/// * `def_id` - The [`NodeId`] of the parent definition block (e.g., `:types`, `:functions`).
///   The function expects this node to contain a `TypedList` as its first child.
/// * `ast` - A reference to the [`Ast`]. Used to traverse the tree, resolve nodes,
///   and extract identifiers or spans without modifying the original structure.
///
/// # Returns
///
/// A [`HashMap`] where each key represents a unique signature ([`DefinitionKey`]),
/// and the value ([`Occurrences`]) contains the metadata needed to generate
/// detailed diagnostic warnings.
///
/// # Errors
///
/// Returns a [`NormalizationPassError`] if:
/// * The AST structure does not match expectations (missing children).
/// * A node that should contain an identifier fails to provide a [`SymbolId`].
fn collect_duplicated_declarations(
    def_id: NodeId,
    ast: &Ast,
) -> Result<HashMap<DefinitionKey, Occurrences>, NormalizationPassError> {
    let syntax_tree = ast.syntax_tree();

    // 1. Navigate to the TypedList node containing the individual declarations.
    let typed_list_id = syntax_tree.try_node(def_id)?.try_child(0)?;
    let typed_list = syntax_tree.try_node(typed_list_id)?;

    // Map to group occurrences by their unique signature (name + arguments).
    let mut seen: HashMap<DefinitionKey, Occurrences> = HashMap::new();

    // 2. Iterate through each child in the list (usually 'TypedItem' nodes).
    for typed_item_id in typed_list.children() {
        let item = syntax_tree.try_node(*typed_item_id)?;

        // Generate the unique identity key.
        // Note: This involves AST traversal and potential Vec allocation for arguments.
        let key = DefinitionKey::from_node(item, ast)?;

        // 3. Extract supertypes or return types associated with this specific declaration.
        let super_idents = match item.get_child(1) {
            Some(ty_id) => {
                let ty_node = syntax_tree.try_node(ty_id)?;
                // Pre-allocate the set to avoid re-allocations during insertion.
                let mut set = HashSet::with_capacity(ty_node.children().len());
                for id in ty_node.children() {
                    // Collect the SymbolId of each type identifier.
                    set.insert(syntax_tree.try_node(*id)?.try_ident()?);
                }
                set
            }
            // Case for untyped declarations (defaults to an empty set).
            None => HashSet::new(),
        };

        // 4. Retrieve the source location (Span) of the declared symbol.
        // We clone it here to store it within the Occurrences structure.
        let span = syntax_tree.try_node(item.try_child(0)?)?.span().clone();

        // 5. Group the data using the Entry API to minimize HashMap lookups.
        match seen.entry(key) {
            // If the key exists, this is a duplicate: append it to the list.
            Entry::Occupied(mut entry) => {
                entry.get_mut().duplicates.push((super_idents, span));
            }
            // If the key is new, initialize the Occurrences record.
            Entry::Vacant(entry) => {
                entry.insert(Occurrences {
                    first_supertypes: super_idents,
                    first_span: span,
                    duplicates: Vec::new(),
                });
            }
        }
    }

    Ok(seen)
}

/// Merges duplicate declarations within a PDDL definition list (e.g., `:types`, `:constants`, `:functions`).
///
/// This function performs an in-place normalization of the AST by identifying items with
/// identical signatures (name and arguments) and merging their supertypes or return types.
///
/// # Algorithm
///
/// 1. Iterates through the children of the `TypedList` associated with `def_id`.
/// 2. Generates a [`DefinitionKey`] for each item to identify duplicates.
/// 3. When a duplicate is found:
///    - It moves all type identifiers from the duplicate to the first encountered instance.
///    - It performs an unstable sort and deduplication on the combined type list to ensure
///      a canonical, minimal representation.
/// 4. Removes all redundant duplicate nodes from the syntax tree.
///
/// # Arguments
///
/// * `def_id` - The [`NodeId`] of the definition block (e.g., the `:functions` node).
/// * `ast` - A mutable reference to the Abstract Syntax Tree.
///
/// # Returns
///
/// * `Ok(true)` if at least one duplicate was merged and the AST was modified.
/// * `Ok(false)` if no duplicates were found.
/// * `Err(NormalizationPassError)` if the AST structure is invalid or a node is missing.
///
/// # Performance Notes
///
/// - The function avoids full AST clones by using index-based iteration and targeted
///   node mutations.
/// - Sorting is performed using `sort_unstable` for maximum speed on `NodeId` primitives.
/// - Duplicate removal uses `retain` to modify the children vector in-place.
/// Merges duplicate declarations within a PDDL definition list (e.g., `:types`, `:constants`, `:functions`).
///
/// This function performs an in-place normalization of the AST by identifying items with
/// identical signatures (name and arguments) and merging their supertypes or return types.
pub fn merge_duplicate_declarations(
    def_id: NodeId,
    ast: &mut Ast,
) -> Result<bool, NormalizationPassError> {
    // 1. On accède au bloc de définition (ex: :types) et à sa TypedList sous-jacente.
    let typed_list_id = ast.syntax_tree().try_node(def_id)?.try_child(0)?;
    let child_count = ast.syntax_tree().try_node(typed_list_id)?.children().len();

    let mut modified = false;
    // Map pour suivre les définitions uniques (nom + arguments) -> NodeId de la première occurrence.
    let mut seen: HashMap<DefinitionKey, NodeId> = HashMap::new();
    // Set pour collecter les IDs à supprimer à la fin.
    let mut duplicates_to_remove = HashSet::new();

    for i in 0..child_count {
        // On récupère l'ID par index pour éviter de cloner tout le vecteur d'enfants à chaque tour.
        let item_id = ast.syntax_tree().try_node(typed_list_id)?.children()[i];

        // Génération de la clé unique (signature de la déclaration).
        let key = {
            let item_node = ast.syntax_tree().try_node(item_id)?;
            DefinitionKey::from_node(item_node, ast)?
        };

        // Si on a déjà vu cette signature...
        if let Some(&existing_item_id) = seen.get(&key) {
            // DOUBLON DÉTECTÉ : Fusion du doublon vers l'original existant.

            // Récupération des blocs 'Type' (index 1 dans un TypedItem).
            let current_super_id = ast.syntax_tree().try_node(item_id)?.try_child(1)?;
            let existing_super_id = ast.syntax_tree().try_node(existing_item_id)?.try_child(1)?;

            // 2. Extraction des types du doublon.
            let types_from_duplicate: Vec<NodeId> = ast
                .syntax_tree()
                .try_node(current_super_id)?
                .children()
                .to_vec();

            // --- POINT 2 : TRANSPLANTATION DES ENFANTS ---
            // On change le parent de chaque type déplacé vers le bloc de l'original.
            for &type_node_id in &types_from_duplicate {
                ast.syntax_tree_mut()
                    .try_node_mut(type_node_id)?
                    .set_parent(Some(existing_super_id));
            }

            // --- NETTOYAGE DU BLOC SOURCE (CURRENT_SUPER_ID) ---
            // On vide l'ancien bloc de types et on le détache pour éviter les racines orphelines.
            let current_super_node = ast.syntax_tree_mut().try_node_mut(current_super_id)?;
            current_super_node.set_parent(None); // Plus de parent
                                                 // ---------------------------------------------------

            // 3. Mutation du nœud original pour inclure les nouveaux types fusionnés.
            let existing_node = ast.syntax_tree_mut().try_node_mut(existing_super_id)?;
            let existing_children = existing_node.children_mut();
            existing_children.extend(types_from_duplicate);

            // 4. Nettoyage de la liste fusionnée (tri et dédoublonnage).
            existing_children.sort_unstable();
            existing_children.dedup();

            // --- POINT SÉCURITÉ : DÉTACHEMENT DU DOUBLON ---
            ast.syntax_tree_mut()
                .try_node_mut(item_id)?
                .set_parent(None);
            // -----------------------------------------------

            duplicates_to_remove.insert(item_id);
            modified = true;
        } else {
            // Première occurrence : on enregistre cet ID comme la référence.
            seen.insert(key, item_id);
        }
    }

    // 5. Phase finale : suppression effective des nœuds redondants dans la TypedList.
    if modified {
        // Sécurité supplémentaire : on s'assure que tout ce qui sort est orphelin.
        for &dup_id in &duplicates_to_remove {
            ast.syntax_tree_mut().try_node_mut(dup_id)?.set_parent(None);
        }

        let list_node = ast.syntax_tree_mut().try_node_mut(typed_list_id)?;
        list_node
            .children_mut()
            .retain(|id| !duplicates_to_remove.contains(id));
    }

    Ok(modified)
}

/// Internal structure to track the first occurrence and subsequent duplicates
/// of a definition during the normalization pass.
struct Occurrences {
    /// The supertypes/return types of the first encountered declaration.
    first_supertypes: HashSet<SymbolId>,
    /// The source location of the first declaration.
    first_span: Span,
    /// A list of subsequent declarations with their own types and spans.
    duplicates: Vec<(HashSet<SymbolId>, Span)>,
}

/// A unique identity key for a PDDL definition.
///
/// This key allows distinguishing between simple entities (Types, Objects, Constants)
/// and complex signatures (Functions, Predicates).
///
/// Two definitions are considered identical if they share the same symbol name
/// and the exact same argument type sequence (arity and types).
#[derive(Clone, Debug, Hash, PartialEq, Eq)]
struct DefinitionKey {
    /// The name of the defined symbol (e.g., 'truck' or 'distance').
    symbol: SymbolId,
    /// The ordered list of argument types. Empty for non-functional definitions.
    arg_types: Vec<SymbolId>,
}

impl DefinitionKey {
    /// Extracts a [`DefinitionKey`] from a `TypedItem` node.
    ///
    /// This method navigates the AST to find the symbol name and, if the item
    /// is an `AtomicFunctionSkeleton`, it collects all parameter types to
    /// build the signature.
    ///
    /// # Errors
    ///
    /// Returns a [`NormalizationPassError`] if:
    /// * A required child node is missing in the AST.
    /// * A node expected to be an identifier does not contain a valid [`SymbolId`].
    pub fn from_node(item_node: &AstNode, ast: &Ast) -> Result<Self, NormalizationPassError> {
        let syntax_tree = ast.syntax_tree();

        // In a TypedItem, the first child (index 0) is either an Identifier
        // or a complex structure like an AtomicFunctionSkeleton.
        let first_child_id = item_node.try_child(0)?;
        let first_child = syntax_tree.try_node(first_child_id)?;

        match first_child.kind() {
            // Structure: TypedItem -> AtomicFunctionSkeleton -> [FunctionSymbol, TypedList]
            AstKind::AtomicFunctionSkeleton => {
                // 1. Extract the function name from the FunctionSymbol node.
                let symbol_node_id = first_child.try_child(0)?;
                let symbol = syntax_tree.try_node(symbol_node_id)?.try_ident()?;

                // 2. Extract argument types from the TypedList (index 1 of the skeleton).
                let mut arg_types = Vec::with_capacity(4);
                if let Some(typed_list_id) = first_child.get_child(1) {
                    let typed_list = syntax_tree.try_node(typed_list_id)?;

                    // Iterate through each TypedItem in the arguments list.
                    for arg_item_id in typed_list.children() {
                        let arg_item = syntax_tree.try_node(*arg_item_id)?;

                        // In a PDDL argument (TypedItem), index 1 contains the Type block.
                        if let Some(type_node_id) = arg_item.get_child(1) {
                            let type_node = syntax_tree.try_node(type_node_id)?;

                            // Collect all PrimitiveTypes (handles single types and 'either' blocks).
                            for prim_id in type_node.children() {
                                arg_types.push(syntax_tree.try_node(*prim_id)?.try_ident()?);
                            }
                        }
                    }
                }
                Ok(Self { symbol, arg_types })
            }
            // Standard Case: TypedItem -> PrimitiveType / Identifier (Types, Objects, Constants).
            // The child is expected to be a leaf node containing the SymbolId.
            _ => Ok(Self {
                symbol: first_child.try_ident()?,
                arg_types: Vec::new(),
            }),
        }
    }
}
