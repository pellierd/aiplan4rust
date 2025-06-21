use crate::aiplan4rust::diagnostic::Diagnostic;
use crate::aiplan4rust::diagnostic::Provider;
use crate::aiplan4rust::diagnostic::DiagnosticKind;
use crate::aiplan4rust::diagnostic::DiagnosticManager;
use crate::aiplan4rust::frontend::ParserInternalError;
use crate::aiplan4rust::semantic::symbol::Declaration;
use crate::aiplan4rust::semantic::symbol::Scope;
use crate::aiplan4rust::semantic::symbol::SymbolKind;

use std::collections::HashMap;
use std::collections::HashSet;
use bimap::BiMap;
use crate::aiplan4rust::semantic::SemanticContext;
use crate::aiplan4rust::syntax::elements::Ident;
use crate::aiplan4rust::syntax::StringInterner;

/// Checks the type hierarchy for inheritance cycles and emits diagnostics if any are found.
///
/// This function analyzes the type inheritance graph to detect circular dependencies among
/// declared PDDL types. If any cycles are found, it emits detailed diagnostics for each involved
/// type using the provided `DiagnosticManager`.
///
/// # Parameters
/// - `ast_old`: A reference to the `AnnotatedSyntaxTree`, which provides access to all
///   declared types and their source context.
/// - `source`: The `DiagnosticSource` identifying the current analysis phase (e.g., semantic check).
/// - `diagnostic_manager`: A mutable reference to the `DiagnosticManager` that will collect and emit
///   error diagnostics.
///
/// # Returns
/// - `Ok(true)`: No type inheritance cycles were found; the type hierarchy is valid.
/// - `Ok(false)`: One or more cycles were detected and reported via diagnostics.
/// - `Err(ParserInternalError)`: An internal error occurred, such as a missing declaration
///   or unresolved reference, preventing the analysis from completing.
///
/// # Algorithm Steps
/// 1. Collect all `PrimitiveType` declarations from the root scope.
/// 2. Assign each type a unique numeric index via a bidirectional map.
/// 3. Construct a directed adjacency matrix representing direct inheritance relationships.
/// 4. Compute the transitive closure of the graph to expose indirect inheritance.
/// 5. Detect cycles in the graph using Johnson’s algorithm.
/// 6. Filter trivial or duplicate cycles.
/// 7. Emit detailed diagnostics for each remaining cycle.
///
/// # Errors
/// This function may return a `ParserInternalError` if critical internal data is missing
/// (such as symbol declarations or span information), or if structural assumptions about
/// the type graph are violated.
///
/// # Example
/// ```rust
/// let result = check_type_hierarchy(
///     &ast_old,
///     DiagnosticSource::SemanticAnalyzer,
///     &mut diagnostic_manager,
/// );
/// match result {
///     Ok(true) => println!("No type cycles detected."),
///     Ok(false) => println!("Cycles detected in type hierarchy."),
///     Err(err) => eprintln!("Internal error: {:?}", err),
/// }
pub fn check_type_hierarchy(
    context: &SemanticContext,
    source: Provider,
    diagnostic_manager: &mut DiagnosticManager,
) -> Result<bool, ParserInternalError> {

    // Step 1: Collect all type declarations from the root scope (PrimitiveType only)
    let types = context
        .symbol_table()
        .collect_declarations(
            None,
            Some(&SymbolKind::PrimitiveType),
            Some(&Scope::root()),
        );

    // Step 2: Build a bidirectional mapping between type names and unique numeric indices
    let type_bimap = build_type_bimap(&types);

    // Step 3: Construct the inheritance adjacency matrix (direct parent-child relationships)
    let mut hierarchy = build_type_adjacency_matrix(&type_bimap, &types)?;

    // Step 4: Compute the transitive closure to reveal indirect inheritance paths
    compute_transitive_closure(&mut hierarchy);

    // Step 5: Detect cycles in the type graph using Johnson’s algorithm
    let all_cycles = johnson_find_cycles(&hierarchy);

    // Step 6: Filter out trivial/self cycles and remove redundant ones
    let filtered_cycles = filter_cycles(all_cycles);

    // Step 7: Emit diagnostics for each meaningful cycle found in the hierarchy
    report_cyclic_type_declaration_error(
        &filtered_cycles,
        &type_bimap,
        &types,
        context.source_name(),
        source,
        diagnostic_manager,
    )?;

    // Return true if no cycles were found; false if diagnostics were emitted
    Ok(filtered_cycles.is_empty())
}

/// Reports diagnostics for cyclic type declarations detected in the type hierarchy.
///
/// For each detected cycle (represented as a vector of type indices), this function reconstructs
/// detailed cycle information by mapping indices to their corresponding type declarations.
/// It then emits a diagnostic error describing the cycle and indicating its source location.
///
/// # Parameters
/// - `cycles`: A slice of cycles, where each cycle is a list of type indices forming a loop.
/// - `type_bimap`: A bidirectional map between type names and their unique numeric indices.
/// - `types`: A slice of references to `Declaration` objects representing all declared types.
/// - `filename`: The name of the source file where the declarations appear.
/// - `source`: The `DiagnosticSource` identifying the analysis phase that detected the cycle.
/// - `diagnostic_manager`: A mutable reference to the `DiagnosticManager` to which diagnostics are added.
///
/// # Returns
/// - `Ok(())` if all diagnostics were successfully emitted.
/// - `Err(ParserInternalError)` if a required declaration or mapping is missing,
///   preventing accurate diagnostic reporting.
///
/// # Errors
/// Returns an error if any cycle cannot be resolved to valid declarations,
/// indicating a potential internal inconsistency.
///
/// # Example
/// ```rust
/// let result = report_cyclic_type_declaration_error(
///     &cycles,
///     &type_bimap,
///     &type_declarations,
///     filename,
///     DiagnosticSource::SemanticAnalyzer,
///     &mut diagnostic_manager,
/// );
///
/// if let Err(e) = result {
///     eprintln!("Error reporting type cycles: {:?}", e);
/// }
/// ```
fn report_cyclic_type_declaration_error(
    cycles: &[Vec<usize>],
    type_bimap: &BiMap<Ident, usize>,
    types: &Vec<&Declaration>,
    filename: &str,
    source: Provider,
    diagnostic_manager: &mut DiagnosticManager,
) -> Result<(), ParserInternalError> {

    // Build a fast lookup map from symbol names to declarations
    let type_map: HashMap<Ident, &Declaration> = types
        .iter()
        .map(|&decl| (decl.symbol(), decl))
        .collect();

    // Process each cycle to generate detailed diagnostic information
    for cycle in cycles {
        let mut cycle_detail = Vec::with_capacity(cycle.len());

        // Convert type indices to symbols, and then to their declarations
        for &index in cycle {
            if let Some(symbol) = type_bimap.get_by_right(&index) {
                if let Some(declaration) = type_map.get(symbol) {
                    cycle_detail.push((*declaration).clone()); // Clone to own the declaration
                }
            }
        }

        // If no valid declarations were found, report an internal error
        if cycle_detail.is_empty() {
            return Err(ParserInternalError::new(
                "Cycle detail cannot be empty".to_string(),
            ));
        }

        // Use the span of the first declaration in the cycle for the diagnostic location
        let first_span = cycle_detail[0].span().clone();

        // Emit a diagnostic describing the cyclic type declarations
        let error = Diagnostic::new(
            DiagnosticKind::CyclicTypeDeclarationError { cycle: cycle_detail },
            source,
            filename.to_string(),
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
    type_bimap: &BiMap<Ident, usize>,
    declarations: &Vec<&Declaration>,
) -> Result<Vec<Vec<bool>>, ParserInternalError> {
    let n = type_bimap.len();

    // Preallocate a square adjacency matrix of size n x n initialized with false
    let mut matrix = vec![vec![false; n]; n];

    // Get the index of the special "object" type once to reuse later
    let object_index = type_bimap.get_by_left(&StringInterner::IDENT_OBJECT).copied();

    // Iterate over all declared types and their declarations
    for declaration in declarations {
        // Try to get the index for the current type name from the bimap
        let Some(&type_idx) = type_bimap.get_by_left(&declaration.symbol()) else {
            // If the type is not found in the map (should not happen if map is consistent), skip
            continue;
        };

        // Validate type_idx is within matrix bounds
        if type_idx >= n {
            return Err(ParserInternalError::new(format!(
                "Index {} for type '{}' is out of bounds (max {})",
                type_idx, declaration.symbol(), n - 1
            )));
        }

        match declaration.types() {
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
    declarations: &Vec<&Declaration>,
) -> BiMap<Ident, usize> {
    // Create an empty BiMap to store type names (String) and their unique indices (usize)
    let mut temp_map: BiMap<Ident, usize> = BiMap::new();

    // Iterate over each type declaration in the input map
    for declaration in declarations {
        // If the type name is not already in the BiMap, insert it with a new unique index
        if !temp_map.contains_left(&declaration.symbol()) {
            let len = temp_map.len();        // Current size of the map used as next index
            temp_map.insert(declaration.symbol().clone(), len); // Insert the type name with the index
        }

        // If the declaration has parent types (e.g., inherited types)
        if let Some(parents) = declaration.types() {
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
    if !temp_map.contains_left(&StringInterner::IDENT_OBJECT) {
        let len = temp_map.len();                    // Next index for insertion
        temp_map.insert(StringInterner::IDENT_OBJECT, len); // Insert OBJECT_TYPE as a key
    }

    // Return the completed BiMap mapping type names to unique indices
    temp_map
}
