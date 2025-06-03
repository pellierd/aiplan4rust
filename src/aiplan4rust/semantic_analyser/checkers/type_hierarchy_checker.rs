use crate::aiplan4rust::diagnostic::Diagnostic;
use crate::aiplan4rust::diagnostic::DiagnosticKind;
use crate::aiplan4rust::diagnostic::DiagnosticManager;
use crate::aiplan4rust::diagnostic::DiagnosticSource;
use crate::aiplan4rust::frontend::ParserInternalError;
use crate::aiplan4rust::parser::lexer::token::OBJECT_TYPE;
use crate::aiplan4rust::semantic_analyser::AnnotatedSyntaxTree;
use crate::aiplan4rust::semantic_analyser::SymbolTable;
use crate::aiplan4rust::semantic_analyser::symbol::Declaration;
use crate::aiplan4rust::semantic_analyser::symbol::SymbolKind;

use std::collections::HashMap;
use std::collections::HashSet;
use indexmap::IndexSet;
use bimap::BiMap;

/// Performs a full check on the syntax tree:
/// merges duplicated type declarations and verifies inheritance cycles.
///
/// # Arguments
/// - `syntax_tree`: Mutable reference to the annotated syntax tree.
/// - `diagnostic_manager`: Mutable reference to the diagnostic manager.
///
/// # Returns
/// Returns `Ok(true)` if no critical errors were found, or an error otherwise.
pub fn check(
    syntax_tree: &mut AnnotatedSyntaxTree,
    diagnostic_manager: &mut DiagnosticManager,
) -> Result<bool, ParserInternalError> {

    // Step 1: Merge duplicated type declarations and collect the unified type map
    let types = merge_type_declarations(syntax_tree, diagnostic_manager)?;

    // Step 2: Check the type hierarchy for inheritance cycles, emitting diagnostics if any
    Ok(check_type_hierarchy(&types, syntax_tree, diagnostic_manager)?)
}

/// Checks the type hierarchy for inheritance cycles and emits diagnostics if any are found.
///
/// This function analyzes the type inheritance graph to detect circular dependencies among
/// types. If cycles are found, it emits detailed diagnostics for each involved type using the
/// provided `DiagnosticManager`.
///
/// # Parameters
/// - `types`: A reference to a map of type declarations keyed by their symbol names.
/// - `syntax_tree`: An immutable reference to the annotated syntax tree, used to retrieve source
///   spans for precise diagnostic reporting.
/// - `diagnostic_manager`: A mutable reference to the diagnostic manager responsible for
///   collecting and reporting errors.
///
/// # Returns
/// - `Ok(true)` if no inheritance cycles were detected.
/// - `Ok(false)` if one or more inheritance cycles were found and diagnostics were emitted.
/// - `Err(ParserInternalError)` if an unexpected internal error occurs during verification.
///
/// # Behavior
/// The function performs the following steps:
/// 1. Builds a bidirectional mapping from type names to unique graph indices.
/// 2. Constructs the direct inheritance adjacency matrix representing the type hierarchy.
/// 3. Computes the transitive closure to capture indirect inheritance relationships.
/// 4. Detects all cycles using Johnson’s algorithm for elementary circuits.
/// 5. Filters out trivial or redundant cycles to avoid duplicate diagnostics.
/// 6. Emits diagnostics describing each detected cycle via the diagnostic manager.
///
/// # Errors
/// This function returns an error only if an unexpected internal inconsistency or failure
/// occurs during the verification process, such as missing data in the syntax tree.
fn check_type_hierarchy(
    types: &HashMap<String, Declaration>,
    syntax_tree: &AnnotatedSyntaxTree,
    diagnostic_manager: &mut DiagnosticManager,
) -> Result<bool, ParserInternalError> {
    // Step 1: Build a mapping from type names to unique indices in the graph
    let type_bimap = build_type_bimap(types);

    // Step 2: Build an adjacency matrix representing type inheritance
    let mut hierarchy = build_type_adjacency_matrix(&type_bimap, types)?;

    // Step 3: Compute the transitive closure to make indirect inheritance explicit
    compute_transitive_closure(&mut hierarchy);

    // Step 4: Detect all cycles using Johnson’s algorithm
    let all_cycles = johnson_find_cycles(&hierarchy);

    // Step 5: Remove trivial/self cycles or redundant ones
    let filtered_cycles = filter_cycles(all_cycles);

    // Step 6: Emit diagnostics for each detected cycle
    emit_cyclic_type_declaration_error(
        &filtered_cycles,
        &type_bimap,
        types,
        syntax_tree,
        diagnostic_manager,
    )?;

    Ok(filtered_cycles.is_empty())
}

/// Emits diagnostics for cyclic type declarations found in the type hierarchy.
///
/// For each detected cycle (represented as a vector of type indices), this function
/// reconstructs detailed cycle information by retrieving type symbols, declarations,
/// and source code spans from the syntax tree. It then creates and adds a diagnostic
/// message describing the cycle.
///
/// # Parameters
/// - `cycles`: A slice of cycles, each cycle is a vector of type indices representing a cycle in
///   the hierarchy.
/// - `type_bimap`: A bidirectional map between type names and their assigned indices.
/// - `types`: A map from type names to their `Declaration` objects.
/// - `syntax_tree`: The annotated syntax tree from which to retrieve source spans.
/// - `diagnostic_manager`: The manager to which diagnostics will be reported.
///
/// # Returns
/// Returns `Ok(())` if diagnostics were emitted successfully for all cycles.
/// Returns a `ParserInternalError` if required span information is missing or cycle data is empty.
fn emit_cyclic_type_declaration_error(
    cycles: &[Vec<usize>],
    type_bimap: &BiMap<String, usize>,
    types: &HashMap<String, Declaration>,
    syntax_tree: &AnnotatedSyntaxTree,
    diagnostic_manager: &mut DiagnosticManager,
) -> Result<(), ParserInternalError> {

    for cycle in cycles {
        // Prepare a vector to hold detailed cycle info: (symbol, declaration, span)
        let mut cycle_detail = Vec::with_capacity(cycle.len());

        for &index in cycle {
            if let Some(symbol) = type_bimap.get_by_right(&index) {
                if let Some(declaration) = types.get(symbol) {
                    // Retrieve the annotated syntax node for the declaration's AST
                    let node = syntax_tree
                        .get_entry(declaration.ast())
                        .ok_or(ParserInternalError::new(format!(
                            "Span for declaration '{}' not found in syntax tree",
                            symbol
                        )))?;

                    // Extract the span from the syntax node
                    let span = node.span().clone();

                    cycle_detail.push((symbol.clone(), declaration.clone(), span));
                }
            }
        }

        // Ensure the cycle detail is not empty before proceeding
        if cycle_detail.is_empty() {
            return Err(ParserInternalError::new("Cycle detail cannot be empty".to_string()));
        }

        // Use the span of the first element in the cycle for the diagnostic location
        let first_span = cycle_detail[0].2.clone();

        // Create and add the diagnostic about the cyclic type declaration
        let error = Diagnostic::new(
            DiagnosticKind::CyclicTypeDeclarationError { cycle: cycle_detail },
            DiagnosticSource::SemanticAnalyzer,
            syntax_tree.filename().clone(),
            first_span,
        );

        diagnostic_manager.add_diagnostic(error);
    }

    Ok(())
}


/// Finds all elementary cycles in a directed graph using Johnson's algorithm.
///
/// This function detects all simple cycles (no repeated nodes except the start/end)
/// in a directed graph represented as a boolean adjacency matrix.
///
/// # Parameters
/// - `graph`: A reference to a square boolean adjacency matrix where
///   `graph[i][j] == true` indicates a directed edge from node `i` to node `j`.
///
/// # Returns
/// A vector of cycles. Each cycle is a vector of node indices representing the path.
/// Each cycle starts and ends at the same node.
///
/// # Complexity
/// Worst-case time complexity is exponential in the number of nodes, but Johnson's
/// algorithm is efficient for sparse graphs with relatively few cycles.
fn johnson_find_cycles(graph: &[Vec<bool>]) -> Vec<Vec<usize>> {
    let n = graph.len();
    let mut blocked = vec![false; n];
    let mut block_map = vec![Vec::new(); n];
    let mut stack = Vec::new();
    let mut cycles = Vec::new();

    for s in 0..n {
        circuit(s, s, graph, &mut blocked, &mut block_map, &mut stack, &mut cycles);
        blocked.fill(false);
        for bm in block_map.iter_mut() {
            bm.clear();
        }
    }

    cycles
}

/// Recursive helper function that searches for all elementary cycles starting from node `s`.
///
/// This function performs a depth-first search from the current node `v`, exploring all
/// paths to find cycles that begin and end at `s`. It uses the "blocked" set and "block_map"
/// to avoid unnecessary traversals of paths that cannot lead to new cycles, improving
/// efficiency according to Johnson's algorithm.
///
/// # Parameters
/// - `v`: The current node being visited.
/// - `s`: The start node of the cycle search (the "root" of the current search).
/// - `graph`: The directed graph represented as an adjacency matrix.
/// - `blocked`: A mutable boolean slice marking nodes that are temporarily blocked to prevent
///   revisiting.
/// - `block_map`: A mutable structure mapping nodes to lists of nodes that caused their blockage.
/// - `stack`: The current path stack of nodes visited in the search.
/// - `cycles`: The mutable collection where found cycles are appended.
///
/// # Returns
/// Returns `true` if at least one cycle involving node `s` was found during this search
/// (including cycles passing through `v`), `false` otherwise.
///
/// # Behavior
/// - Marks node `v` as blocked to prevent revisiting in the current search branch.
/// - Explores all neighbors `w` of `v`.
/// - If a neighbor `w` equals the start node `s`, a cycle is found and appended to `cycles`.
/// - Otherwise, recursively continues the search from unblocked neighbors.
/// - If any cycle is found, calls `unblock` on `v` to allow revisits along other branches.
/// - If no cycle found from `v`, updates `block_map` to keep track of dependencies causing
///   blockage.
/// - Pops `v` from `stack` before returning to previous recursion level.
fn circuit(
    v: usize,
    s: usize,
    graph: &[Vec<bool>],
    blocked: &mut [bool],
    block_map: &mut [Vec<usize>],
    stack: &mut Vec<usize>,
    cycles: &mut Vec<Vec<usize>>,
) -> bool {
    let mut found_cycle = false;
    stack.push(v);
    blocked[v] = true;

    for w in 0..graph.len() {
        if graph[v][w] {
            if w == s {
                // Cycle found
                let mut cycle = stack.clone();
                cycle.push(s);
                cycles.push(cycle);
                found_cycle = true;
            } else if !blocked[w] {
                if circuit(w, s, graph, blocked, block_map, stack, cycles) {
                    found_cycle = true;
                }
            }
        }
    }

    if found_cycle {
        unblock(v, blocked, block_map);
    } else {
        for w in 0..graph.len() {
            if graph[v][w] && !block_map[w].contains(&v) {
                block_map[w].push(v);
            }
        }
    }

    stack.pop();
    found_cycle
}

/// Unblocks node `u` and recursively unblocks all nodes that depend on it.
///
/// This function is part of Johnson's algorithm mechanism to manage the "blocked" set.
/// When a cycle involving node `u` is found, this function removes the block on `u`
/// and recursively unblocks all nodes in `block_map[u]` that were waiting on `u` to be
/// unblocked, allowing those nodes to be explored in subsequent searches.
///
/// # Parameters
/// - `u`: The node to unblock.
/// - `blocked`: Mutable boolean slice tracking the blocked status of each node.
/// - `block_map`: Mutable structure mapping nodes to the list of nodes that caused their blockage.
///
/// # Behavior
/// - Sets `blocked[u]` to `false`.
/// - Iteratively pops nodes from `block_map[u]`.
/// - For each such node `w`, if it is still blocked, recursively unblocks `w`.
/// - Clears `block_map[u]` in the process.
fn unblock(u: usize, blocked: &mut [bool], block_map: &mut [Vec<usize>]) {
    blocked[u] = false;
    while let Some(w) = block_map[u].pop() {
        if blocked[w] {
            unblock(w, blocked, block_map);
        }
    }
}

/// Filters cycles to remove:
/// 1. Cycles of length 1 (self-loops),
/// 2. Duplicate cycles that are rotations of each other.
///
/// # Arguments
/// - `cycles`: A vector of cycles, each cycle is a vector of node indices.
///
/// # Returns
/// A filtered vector where self-loops and rotated duplicates are removed.
fn filter_cycles(cycles: Vec<Vec<usize>>) -> Vec<Vec<usize>> {
    let mut seen = HashSet::new();
    let mut filtered = Vec::new();

    for cycle in cycles {
        if cycle.len() <= 1 {
            // Skip cycles of length 1 (self-loops)
            continue;
        }
        // Compute the canonical (minimal lex rotation) form of the cycle
        let c = canonical_cycle(&cycle);
        if !seen.contains(&c) {
            // If this canonical cycle hasn't been seen before, add it to the set and results
            seen.insert(c.clone());
            filtered.push(c);
        }
    }

    filtered
}

/// Finds the lexicographically minimal rotation of a slice using Booth's algorithm.
///
/// This rotation serves as a canonical form to detect cycles that are rotations of each other.
///
/// # Arguments
/// - `arr`: Slice of node indices representing a cycle.
///
/// # Returns
/// The starting index of the minimal rotation.
fn booth_algorithm(arr: &[usize]) -> usize {
    let n = arr.len();
    let mut i = 0; // Candidate start index of minimal rotation
    let mut j = 1; // Candidate start index to compare with i
    let mut k = 0; // Offset for comparison

    // Loop until either index reaches the end or all elements compared
    while i < n && j < n && k < n {
        if arr[(i + k) % n] == arr[(j + k) % n] {
            // If elements are equal, move comparison offset forward
            k += 1;
            continue;
        }
        if arr[(i + k) % n] > arr[(j + k) % n] {
            // If element at i+k is greater, move i forward beyond this mismatch
            i = i + k + 1;
            if i <= j {
                // Ensure i is always ahead of j
                i = j + 1;
            }
        } else {
            // Else move j forward beyond this mismatch
            j = j + k + 1;
            if j <= i {
                // Ensure j is always ahead of i
                j = i + 1;
            }
        }
        // Reset comparison offset
        k = 0;
    }
    // Return the minimum index as start of minimal rotation
    std::cmp::min(i, j)
}

/// Computes the canonical rotation (lexicographically minimal) of a cycle using Booth's algorithm.
///
/// # Arguments
/// - `cycle`: A slice representing the cycle.
///
/// # Returns
/// A new `Vec<usize>` containing the lex minimal rotation of the cycle.
fn canonical_cycle(cycle: &[usize]) -> Vec<usize> {
    if cycle.is_empty() {
        // Return empty vector if input cycle is empty
        return Vec::new();
    }
    // Find the index of minimal rotation
    let start = booth_algorithm(cycle);
    // Collect the cycle starting from minimal rotation index, wrapping around with .cycle()
    cycle.iter()
        .cycle()
        .skip(start)
        .take(cycle.len())
        .cloned()
        .collect()
}

/// Computes the transitive closure of a boolean adjacency matrix using the Floyd-Warshall algorithm.
///
/// This function updates the given square matrix in place. After execution, `matrix[i][j]`
/// will be `true` if there exists any path (direct or indirect) from node `i` to node `j`.
///
/// This is useful in type systems to compute all inherited types (i.e., whether a type
/// transitively inherits from another).
///
/// # Parameters
///
/// - `matrix`: A mutable reference to a square `n × n` boolean matrix. Initially,
///   `matrix[i][j]` should be `true` if there is a direct edge (or relationship)
///   from `i` to `j`.
///
/// # Algorithm
///
/// This is the classic Floyd-Warshall algorithm adapted for boolean reachability graphs.
/// It has a time complexity of **O(n³)**, where `n` is the number of types.
///
/// # Example
/// ```rust
/// let mut matrix = vec![
///     vec![false, true, false],
///     vec![false, false, true],
///     vec![false, false, false],
/// ];
/// compute_transitive_closure(&mut matrix);
/// assert!(matrix[0][2]); // 0 → 1 → 2
/// ```
fn compute_transitive_closure(matrix: &mut Vec<Vec<bool>>) {
    let n = matrix.len();

    for k in 0..n {
        for i in 0..n {
            if matrix[i][k] {
                for j in 0..n {
                    if matrix[k][j] && !matrix[i][j] {
                        matrix[i][j] = true;
                    }
                }
            }
        }
    }
}

/// Builds a direct inheritance adjacency matrix from a set of type declarations.
///
/// This function constructs a square boolean matrix representing the direct
/// inheritance relationships between types. Each type is assigned a unique index
/// via the `type_bimap`, and the matrix is constructed such that:
///
/// - `matrix[i][j] == true` if the type at index `i` **directly inherits** from the type at index
///   `j`.
/// - `matrix[i][j] == false` otherwise.
///
/// If a type declaration does not specify any parent types, it is assumed to
/// implicitly inherit from the special `"object"` type (if it exists in the map).
///
/// # Parameters
///
/// - `type_bimap`: A mapping from type names to their unique integer indices.
///   This must include all types used in the declarations, and should include `"object"`
///   for correct handling of root types.
/// - `declarations`: A map of type names to their `Declaration` objects. Each declaration
///   may include a list of parent types (i.e., superclasses or supertypes).
///
/// # Returns
///
/// A `Result` containing:
/// - On success: A square matrix `matrix[i][j]` where each row and column corresponds to a type,
///   as defined in `type_bimap`. The matrix has the following meaning:
///     - `matrix[i][j] == true` ⇒ type at index `i` inherits directly from type at index `j`
///     - `matrix[i][j] == false` ⇒ no direct inheritance between these two types
/// - On failure: `ParserInternalError` if an index is out of bounds (indicating inconsistency).
///
/// # Errors
///
/// Returns an error if any index obtained from `type_bimap` exceeds the matrix dimension,
/// which indicates a corrupted or inconsistent index map.
///
/// # Example
///
/// ```rust
/// let index_map = build_type_index_map(&declarations);
/// let adj_matrix = build_type_adjacency_matrix(&index_map, &declarations)?;
/// ```
fn build_type_adjacency_matrix(
    type_bimap: &BiMap<String, usize>,
    declarations: &HashMap<String, Declaration>,
) -> Result<Vec<Vec<bool>>, ParserInternalError> {
    let n = type_bimap.len();

    // Preallocate a square adjacency matrix of size n x n initialized with false
    let mut matrix = vec![vec![false; n]; n];

    // Get the index of the special "object" type once to reuse later
    let object_index = type_bimap.get_by_left(OBJECT_TYPE).copied();

    // Iterate over all declared types and their declarations
    for (type_name, decl) in declarations {
        // Try to get the index for the current type name from the bimap
        let Some(&type_idx) = type_bimap.get_by_left(type_name) else {
            // If the type is not found in the map (should not happen if map is consistent), skip
            continue;
        };

        // Validate type_idx is within matrix bounds
        if type_idx >= n {
            return Err(ParserInternalError::new(format!(
                "Index {} for type '{}' is out of bounds (max {})",
                type_idx, type_name, n - 1
            )));
        }

        match decl.types() {
            Some(parents) => {
                // For each parent type, set an edge in the adjacency matrix
                for parent in parents {
                    if let Some(&parent_idx) = type_bimap.get_by_left(parent) {
                        // Validate parent_idx is within bounds
                        if parent_idx >= n {
                            return Err(ParserInternalError::new(format!(
                                "Index {} for parent type '{}' is out of bounds (max {})",
                                parent_idx, parent, n - 1
                            )));
                        }
                        matrix[type_idx][parent_idx] = true; // type -> parent edge
                    }
                }
            }
            None => {
                // If no parent declared, implicitly link to the "object" type if present
                if let Some(j) = object_index {
                    if j >= n {
                        return Err(ParserInternalError::new(format!(
                            "Index {} for special object type is out of bounds (max {})",
                            j, n - 1
                        )));
                    }
                    matrix[type_idx][j] = true;
                }
            }
        }
    }

    Ok(matrix)
}

/// Builds a `BiMap` that assigns a unique index to each type name found in the declarations.
///
/// This function avoids unnecessary `String` cloning by checking membership before insertion.
/// It collects:
/// - All declared types (keys in `declarations`)
/// - All parent types referenced in each declaration (if any)
/// - The special `"object"` type, added if not already present
///
/// The returned `BiMap<String, usize>` enables:
/// - Efficient lookup from type name to index (`left` map)
/// - Efficient reverse lookup from index to type name (`right` map)
///
/// # Arguments
///
/// * `declarations` - A `HashMap` mapping type names to their `Declaration` objects.
///
/// # Returns
///
/// A `BiMap<String, usize>` mapping type names to unique indices assigned in insertion order.
///
/// # Example
///
/// ```rust
/// let type_index_map = build_type_index_map(&declarations);
/// let index = type_index_map.get_by_left("robot").unwrap();
/// let name = type_index_map.get_by_right(*index).unwrap();
/// ```
fn build_type_bimap(
    declarations: &HashMap<String, Declaration>,
) -> BiMap<String, usize> {
    // Create an empty BiMap to store type names (String) and their unique indices (usize)
    let mut temp_map: BiMap<String, usize> = BiMap::new();

    // Iterate over each type declaration in the input map
    for (type_name, decl) in declarations {
        // If the type name is not already in the BiMap, insert it with a new unique index
        if !temp_map.contains_left(type_name) {
            let len = temp_map.len();        // Current size of the map used as next index
            temp_map.insert(type_name.clone(), len); // Insert the type name with the index
        }

        // If the declaration has parent types (e.g., inherited types)
        if let Some(parents) = decl.types() {
            // Iterate over each parent type
            for parent in parents {
                // Insert the parent type into the map if it's not already present
                if !temp_map.contains_left(parent) {
                    let len = temp_map.len();      // Get next index based on current size
                    temp_map.insert(parent.clone(), len); // Insert parent type with index
                }
            }
        }
    }

    // Ensure the special OBJECT_TYPE is present in the map; add if missing
    if !temp_map.contains_left(OBJECT_TYPE) {
        let len = temp_map.len();                    // Next index for insertion
        temp_map.insert(OBJECT_TYPE.to_string(), len); // Insert OBJECT_TYPE as a key
    }

    // Return the completed BiMap mapping type names to unique indices
    temp_map
}

/// Merges duplicated primitive type declarations in the symbol table and emits diagnostics.
///
/// This function processes the symbol table of the provided `AnnotatedSyntaxTree`, merges
/// multiple primitive type declarations per symbol into one, and reports any duplicates
/// through diagnostics. It avoids borrow checker conflicts by decoupling mutation and
/// read-only access into two phases.
///
/// # Borrowing Strategy
/// Rust's borrowing rules prevent simultaneous mutable and immutable borrows of the
/// syntax tree. To work around this safely:
/// 1. **Merging Phase** (mutable borrow):
///    Calls [`collect_duplicated_type_declarations`] to mutate the symbol table,
///    consolidate duplicate primitive declarations, and collect raw diagnostic metadata
///    (without needing the syntax tree).
/// 2. **Diagnostic Phase** (immutable borrow):
///    After the mutable borrow ends, uses [`emit_duplicated_type_declaration_warning`]
///    to generate and add warnings to the `DiagnosticManager`, using the syntax tree
///    immutably to locate source spans.
///
/// # Parameters
/// - `syntax_tree`: A mutable reference to the `AnnotatedSyntaxTree` to process.
///   Its symbol table will be modified in-place.
/// - `diagnostic_manager`: The manager that will receive emitted warnings for any
///   duplicated type declarations found.
///
/// # Returns
/// A `Result` containing a `HashMap<String, Declaration>` that maps symbol names to their
/// merged primitive type declarations, if any were present.
///
/// # Errors
/// Returns a [`ParserInternalError`] if:
/// - The merging process fails unexpectedly (e.g., due to an internal invariant violation).
/// - Emitting diagnostics encounters an unrecoverable condition.
///
/// # Behavior Summary
/// - Consolidates all `SymbolKind::PrimitiveType` declarations in the symbol table.
/// - For any duplicates, merges their types into a single declaration and emits a warning.
/// - Leaves all other declaration kinds unchanged.
///
/// # See Also
/// - [`collect_duplicated_type_declarations`] — collects duplicates without needing AST spans.
/// - [`emit_duplicated_type_declaration_warning`] — emits diagnostics using collected metadata.

fn merge_type_declarations(
    syntax_tree: &mut AnnotatedSyntaxTree,
    diagnostic_manager: &mut DiagnosticManager
) -> Result<HashMap<String, Declaration>, ParserInternalError> {
    // Clone the filename from the syntax tree to use in diagnostics later.
    let filename = syntax_tree.filename().clone();

    // Mutably borrow the symbol table from the syntax tree to perform merging.
    let symbol_table = syntax_tree.symbol_table_mut();

    // Call the helper function that merges duplicated declarations and returns:
    // 1) the merged primitive declarations
    // 2) a list of raw diagnostic data tuples to be processed later.
    let (type_declarations, diagnostics) =
        collect_duplicated_type_declarations(symbol_table)?;

    // After the mutable borrow is released, emit diagnostics by converting raw data into actual
    // warnings, accessing the syntax tree immutably for span info.
    emit_duplicated_type_declaration_warning(diagnostics, syntax_tree, &filename, diagnostic_manager)?;

    // Return the map of merged primitive declarations.
    Ok(type_declarations)
}

/// Merges primitive type declarations across all symbols in the symbol table and collects
/// duplicates.
///
/// This function processes every symbol's declaration set within the provided symbol table,
/// consolidating redundant primitive type declarations (e.g., multiple `(:type x - object)`).
/// It uses [`merge_symbol_type_declarations`] internally to perform the per-symbol merging logic,
/// and aggregates both the resulting unique primitive declarations and any metadata
/// required to later emit diagnostics for duplicates.
///
/// # Parameters
/// - `symbol_table`: A mutable reference to the full `SymbolTable`, where each symbol maps to
///   a set of declarations (`IndexSet<Declaration>`) that may contain duplicates.
///
/// # Returns
/// Returns a `Result` containing a tuple:
/// - `HashMap<String, Declaration>`: A map where each key is a symbol name, and the value is
///   the merged primitive declaration for that symbol, if one was found.
/// - `Vec<(String, Declaration, Declaration, usize)>`: A collection of tuples representing
///   duplicate primitive type declarations. Each tuple contains:
///     - the symbol name (`String`)
///     - the original declaration retained after merging
///     - the duplicate declaration that was merged
///     - the AST node ID of the duplicate (used to locate its span for diagnostics)
///
/// # Errors
/// Returns a [`ParserInternalError`] if the merging process fails for any symbol, which is
/// not expected under normal parsing conditions but may indicate internal logic issues.
///
/// # Notes
/// - This function **does not emit diagnostics directly**. It collects all necessary
///   metadata and defers diagnostic creation and reporting to the caller.
/// - Non-primitive declarations in the symbol table are unaffected and preserved.
///
/// # See Also
/// - [`merge_symbol_type_declarations`] — performs the actual merge logic for a single symbol.
fn collect_duplicated_type_declarations(
    symbol_table: &mut SymbolTable,
) -> Result<
    (
        HashMap<String, Declaration>,
        Vec<(String, Declaration, Declaration, usize)>,
    ),
    ParserInternalError,
> {
    let mut unique_primitive_declarations = HashMap::new();
    let mut diagnostics_data = Vec::new();

    // Iterate over each symbol and merge their primitive type declarations
    for (key, symbol) in symbol_table.iter_mut() {
        let declarations = symbol.declarations_mut();

        // Merge declarations and collect diagnostics related to duplicates
        let (maybe_primitive_decl, mut local_diagnostics) =
            merge_symbol_type_declarations(key, declarations)?;

        // Store merged primitive declarations keyed by symbol name
        if let Some(decl) = maybe_primitive_decl {
            unique_primitive_declarations.insert(key.clone(), decl);
        }

        // Append any diagnostic data gathered during merging
        diagnostics_data.append(&mut local_diagnostics);
    }

    Ok((unique_primitive_declarations, diagnostics_data))
}

/// Emits diagnostics for duplicated primitive type declarations.
///
/// This function takes diagnostic data collected earlier about duplicated declarations
/// and creates proper `Diagnostic` entries by accessing the syntax tree for span info.
///
/// # Parameters
/// - `diagnostics_data`: Vector of tuples containing:
///     - symbol name (String)
///     - first duplicated declaration (Declaration)
///     - second duplicated declaration (Declaration)
///     - AST node ID (usize) associated with the duplicate declaration
/// - `syntax_tree`: Reference to the `AnnotatedSyntaxTree` to fetch syntax node info.
/// - `filename`: The source filename, used for diagnostic reporting.
///
/// # Returns
/// Returns `Ok(())` if diagnostics were successfully emitted,
/// or an error if a required syntax node was missing.
fn emit_duplicated_type_declaration_warning(
    diagnostics_data: Vec<(String, Declaration, Declaration, usize)>,
    syntax_tree: &AnnotatedSyntaxTree,
    filename: &str,
    diagnostic_manager: &mut DiagnosticManager,
) -> Result<(), ParserInternalError> {
    for (symbol, decl1, decl2, ast_id) in diagnostics_data {
        // Attempt to retrieve the syntax node for the duplicate declaration
        let declaration_node = syntax_tree.get_entry(ast_id).ok_or_else(|| {
            ParserInternalError::new("Missing syntax node for declaration".to_string())
        })?;

        // Construct the diagnostic warning with relevant information
        let diagnostic = Diagnostic::new(
            DiagnosticKind::WarningDuplicatedTypeDeclaration {
                ty: symbol,
                declaration1: decl1,
                declaration2: decl2,
            },
            DiagnosticSource::SemanticAnalyzer,
            filename.to_string(),
            declaration_node.span().clone(),
        );

        // Add the diagnostic to the manager
        diagnostic_manager.add_diagnostic(diagnostic);
    }
    Ok(())
}

/// Merges primitive type declarations for a given symbol and collects duplicate diagnostics.
///
/// This function processes all declarations associated with a symbol, consolidating multiple
/// primitive type declarations (e.g., `(:type a b - object)`) into a single merged declaration.
/// If duplicates are found, it collects diagnostic metadata to be used later for warning emission.
/// Non-primitive declarations are left untouched.
///
/// The function is designed to **avoid borrow checker conflicts** by:
/// - collecting diagnostic data without accessing the syntax tree,
/// - deferring diagnostic creation and emission to the caller.
///
/// # Parameters
/// - `symbol`: The name of the symbol whose declarations are being processed.
/// - `declarations`: A mutable reference to an `IndexSet<Declaration>` that contains all
///   declarations for the given symbol.
///
/// # Returns
/// Returns a `Result` containing:
/// - `Option<Declaration>`: The merged primitive type declaration if one was found, otherwise
///   `None`.
/// - `Vec<(String, Declaration, Declaration, usize)>`: A list of diagnostic metadata, where each
///   tuple contains:
///     - the symbol name (`String`)
///     - the first (existing) primitive declaration
///     - the second (duplicate) primitive declaration
///     - the AST node ID of the duplicate declaration (for span lookup)
///
/// # Algorithm
/// 1. Take ownership of the original `declarations` by replacing them with an empty set.
/// 2. Iterate over each declaration:
///    - If it's not a primitive type, reinsert it as-is.
///    - If it's a primitive type:
///       - If no primitive declaration has been merged yet, store it.
///       - Otherwise, treat it as a duplicate:
///           - Record diagnostic metadata.
///           - Merge its type content (if any) into the existing declaration.
/// 3. Reinsert the final merged primitive declaration (if one exists).
/// 4. Update the original `declarations` with the new set.
/// 5. Return the merged declaration and diagnostic data.
///
/// # Errors
/// Returns a `ParserInternalError` only if unexpected invariants are violated (not expected under
/// normal use).
///
/// # Notes
/// - This function does not emit diagnostics directly. It only prepares the data required to do so
///   later.
/// - It assumes that `Declaration::types_mut()` and `Declaration::take_types()` are used to access
///   and move internal type data for merging.

fn merge_symbol_type_declarations(
    symbol: &str,
    declarations: &mut IndexSet<Declaration>,
) -> Result<(Option<Declaration>, Vec<(String, Declaration, Declaration, usize)>), ParserInternalError> {
    // Holds the merged primitive declaration, initially None
    let mut merged_primitive_decl: Option<Declaration> = None;
    // Create a new set to accumulate non-primitive declarations and the final merged one
    let mut new_declarations = IndexSet::with_capacity(declarations.len());
    // Vector to store data needed for diagnostics creation later on, without borrowing syntax_tree
    let mut diagnostics_data = Vec::new();

    // Replace the original declarations with an empty set, taking ownership of the old ones
    let old_declarations = std::mem::take(declarations);

    // Iterate over all old declarations
    for mut declaration in old_declarations {
        // If this declaration is not a primitive type, insert it directly into the new set
        if declaration.kind() != &SymbolKind::PrimitiveType {
            new_declarations.insert(declaration);
            continue; // Move to the next declaration
        }

        // If it is a primitive type declaration
        match &mut merged_primitive_decl {
            // If a merged primitive declaration already exists
            Some(existing_decl) => {
                // Store the information necessary for a diagnostic without directly borrowing
                // syntax_tree
                diagnostics_data.push((
                    symbol.to_string(),    // The symbol name
                    existing_decl.clone(), // The already merged declaration
                    declaration.clone(),   // The duplicate declaration found
                    declaration.ast(),     // AST node id for locating the source
                ));

                // Merge the types from the duplicate declaration into the existing merged
                // declaration
                match existing_decl.types_mut() {
                    Some(existing_types) => {
                        // If the new declaration has types, extend the existing types
                        if let Some(new_types) = declaration.take_types() {
                            existing_types.extend(new_types);
                        }
                    }
                    // Otherwise, replace the existing types with those from the new declaration
                    None => {
                        existing_decl.set_types(declaration.take_types());
                    }
                }
            }
            // If this is the first primitive declaration encountered, set it as the merged one
            None => {
                merged_primitive_decl = Some(declaration);
            }
        }
    }

    // If there is a merged primitive declaration, insert it into the new declarations set
    if let Some(decl) = &merged_primitive_decl {
        new_declarations.insert(decl.clone());
    }

    // Replace the original declarations with the updated set containing merged results
    *declarations = new_declarations;

    // Return the merged primitive declaration (if any) and the list of diagnostic data for later
    // use
    Ok((merged_primitive_decl, diagnostics_data))
}
