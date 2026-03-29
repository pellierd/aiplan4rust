use crate::aiplan4rust::diagnostic::Diagnostic;
use crate::aiplan4rust::diagnostic::DiagnosticManager;
use crate::aiplan4rust::diagnostic::Provider;
use crate::aiplan4rust::interner::SymbolInterner;
use crate::aiplan4rust::lang::{LiteralId, SymbolId};
use crate::aiplan4rust::semantic::checks::{CheckContext, SemanticCheckError};
use crate::aiplan4rust::semantic::symbol::Declaration;
use crate::aiplan4rust::semantic::symbol::SymbolKind;

use crate::SymbolTable;
use bimap::BiMap;
use std::collections::HashMap;
use std::collections::HashSet;

/// Checks the type hierarchy for inheritance cycles and emits diagnostics if any are found.
///
/// This function analyzes the inheritance graph of PDDL types to detect circular dependencies
/// (e.g., Type A inherits from B, and B inherits from A). It uses the unified context
/// to access the symbol table and metadata required for diagnostic reporting.
///
/// # Parameters
///
/// - `context`: A reference to the [`CheckContext`] providing access to the symbol table,
///   interner, and diagnostic metadata (provider, source ID).
/// - `diagnostic_manager`: A mutable reference to the [`DiagnosticManager`] where
///   detected cycles will be reported.
///
/// # Returns
///
/// - `Ok(true)`: The type hierarchy is directed and acyclic (valid).
/// - `Ok(false)`: One or more inheritance cycles were detected and reported.
/// - `Err(SemanticCheckError)`: An internal error occurred during graph construction
///   or cycle detection.
///
/// # Algorithm
///
/// 1. **Collection**: Retrieves all `PrimitiveType` declarations from the root scope.
/// 2. **Indexing**: Maps each type name to a unique numeric index using a bidirectional map.
/// 3. **Adjacency**: Constructs a directed matrix representing direct parent-child relationships.
/// 4. **Closure**: Computes the transitive closure to expose indirect inheritance paths.
/// 5. **Detection**: Identifies cycles using Johnson’s algorithm for elementary cycles.
/// 6. **Reporting**: Filters redundant cycles and emits detailed diagnostics via the manager.
///
/// # Example
///
/// ```rust
/// let check_ctx = context.as_check_context(Provider::Analyzer);
/// let is_valid = check_type_hierarchy(&check_ctx, &mut diagnostic_manager)?;
/// ```
///
/// [`CheckContext`]: crate::semantics::CheckContext
/// [`DiagnosticManager`]: crate::diagnostics::DiagnosticManager
pub fn check_type_hierarchy(
    context: &CheckContext,
    symbol_table: &mut SymbolTable,
    diagnostic_manager: &mut DiagnosticManager,
) -> Result<bool, SemanticCheckError> {
    // Step 1: Collect all type_checker declarations from the root scope (PrimitiveType only)
    let types = symbol_table.collect_declarations(
        None,
        Some(&SymbolKind::PrimitiveType),
        Some(&symbol_table.root_scope()),
    );

    // Step 2: Build a bidirectional mapping between type_checker names and unique numeric indices
    let type_bimap = build_type_bimap(&types);

    // Step 3: Construct the inheritance adjacency matrix (direct parent-child relationships)
    let mut hierarchy = build_type_adjacency_matrix(&type_bimap, &types)?;

    // Step 4: Compute the transitive closure to reveal indirect inheritance paths
    compute_transitive_closure(&mut hierarchy);

    // Step 5: Detect cycles in the type_checker graph using Johnson’s algorithm
    let all_cycles = johnson_find_cycles(&hierarchy);

    // Step 6: Filter out trivial/self cycles and remove redundant ones
    let filtered_cycles = filter_cycles(all_cycles);

    // Step 7: Emit diagnostics for each meaningful cycle found in the hierarchy
    report_cyclic_type_declaration_error(
        &filtered_cycles,
        &type_bimap,
        &types,
        context.source(),
        context.provider(),
        diagnostic_manager,
    )?;

    // Return true if no cycles were found; false if diagnostics were emitted
    Ok(filtered_cycles.is_empty())
}

/// Reports diagnostics for cyclic typing declarations detected in the typing hierarchy.
///
/// For each detected cycle (represented as a vector of typing indices), this function reconstructs
/// the corresponding typing declarations and emits an error diagnostic describing the cycle and its
/// origin within the source code.
///
/// # Parameters
///
/// - `cycles`: A slice of typing cycles, where each cycle is a list of indices corresponding to
///   declared types forming a loop.
/// - `type_bimap`: A bidirectional map between typing identifiers (`Ident`) and their unique indices,
///   used to resolve cycles back to declarations.
/// - `types`: A list of references to `Declaration` objects representing all known types.
/// - `source`: The interned `Literal` representing the name of the source file where
///   the declarations originate.
/// - `provider`: The `Provider` identifying the compiler or analysis stage that reports this diagnostic
///   (e.g., `Provider::SemanticAnalyzer`).
/// - `diagnostic_manager`: A mutable reference to the `DiagnosticManager` used to collect and report diagnostics.
///
/// # Returns
///
/// - `Ok(())` if all diagnostics were successfully reported.
/// - `Err(SemanticCheckError)` if the cycle could not be resolved into valid declarations,
///   indicating a potential internal inconsistency.
///
/// # Errors
///
/// Returns `SemanticCheckError::empty_cycle_detail` if no declarations could be resolved for a cycle,
/// which likely indicates a bug in the analysis phase or an invalid state in the typing resolution.
///
/// # Example
///
/// ```rust
/// report_cyclic_type_declaration_error(
///     &cycles,
///     &type_bimap,
///     &type_declarations,
///     source_literal,
///     Provider::SemanticAnalyzer,
///     &mut diagnostic_manager,
/// )?;
/// ```
fn report_cyclic_type_declaration_error(
    cycles: &[Vec<usize>],
    type_bimap: &BiMap<SymbolId, usize>,
    types: &Vec<&Declaration>,
    source: LiteralId,
    provider: Provider,
    diagnostic_manager: &mut DiagnosticManager,
) -> Result<(), SemanticCheckError> {
    // Build a fast lookup map from symbol names to declarations
    let type_map: HashMap<SymbolId, &Declaration> = types
        .iter()
        .map(|&decl| (decl.symbol_ident(), decl))
        .collect();

    // Process each cycle to generate detailed diagnostic information
    for cycle in cycles {
        let mut cycle_detail = Vec::with_capacity(cycle.len());

        // Convert type_checker indices to symbols, and then to their declarations
        for &index in cycle {
            if let Some(symbol) = type_bimap.get_by_right(&index) {
                if let Some(declaration) = type_map.get(symbol) {
                    cycle_detail.push((*declaration).clone()); // Clone to own the declaration
                }
            }
        }

        // If no valid declarations were found, report an internal error
        if cycle_detail.is_empty() {
            return Err(SemanticCheckError::empty_cycle_detail());
        }

        // Use the span of the first declaration in the cycle for the diagnostic location
        let first_span = cycle_detail[0].span().clone();

        // Emit a diagnostic describing the cyclic type_checker declarations
        let error =
            Diagnostic::error_cyclic_type_declaration(cycle_detail, provider, source, first_span);

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
///   `graph[i][j] == true` indicates a directed edge from syntax `i` to syntax `j`.
///
/// # Returns
/// A vector of cycles. Each cycle is a vector of syntax indices representing the path.
/// Each cycle starts and ends at the same syntax.
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
        circuit(
            s,
            s,
            graph,
            &mut blocked,
            &mut block_map,
            &mut stack,
            &mut cycles,
        );
        blocked.fill(false);
        for bm in block_map.iter_mut() {
            bm.clear();
        }
    }

    cycles
}

/// Recursive helper function that searches for all elementary cycles starting from syntax `s`.
///
/// This function performs a depth-first search from the current syntax `v`, exploring all
/// paths to find cycles that begin and end at `s`. It uses the "blocked" set and "block_map"
/// to avoid unnecessary traversals of paths that cannot lead to new cycles, improving
/// efficiency according to Johnson's algorithm.
///
/// # Parameters
/// - `v`: The current syntax being visited.
/// - `s`: The start syntax of the cycle search (the "root" of the current search).
/// - `graph`: The directed graph represented as an adjacency matrix.
/// - `blocked`: A mutable boolean slice marking nodes that are temporarily blocked to prevent
///   revisiting.
/// - `block_map`: A mutable structure mapping nodes to lists of nodes that caused their blockage.
/// - `stack`: The current path stack of nodes visited in the search.
/// - `cycles`: The mutable collection where found cycles are appended.
///
/// # Returns
/// Returns `true` if at least one cycle involving syntax `s` was found during this search
/// (including cycles passing through `v`), `false` otherwise.
///
/// # Behavior
/// - Marks syntax `v` as blocked to prevent revisiting in the current search branch.
/// - Explores all neighbors `w` of `v`.
/// - If a neighbor `w` equals the start syntax `s`, a cycle is found and appended to `cycles`.
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

/// Unblocks syntax `u` and recursively unblocks all nodes that depend on it.
///
/// This function is part of Johnson's algorithm mechanism to manage the "blocked" set.
/// When a cycle involving syntax `u` is found, this function removes the block on `u`
/// and recursively unblocks all nodes in `block_map[u]` that were waiting on `u` to be
/// unblocked, allowing those nodes to be explored in subsequent searches.
///
/// # Parameters
/// - `u`: The syntax to unblock.
/// - `blocked`: Mutable boolean slice tracking the blocked status of each syntax.
/// - `block_map`: Mutable structure mapping nodes to the list of nodes that caused their blockage.
///
/// # Behavior
/// - Sets `blocked[u]` to `false`.
/// - Iteratively pops nodes from `block_map[u]`.
/// - For each such syntax `w`, if it is still blocked, recursively unblocks `w`.
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
/// - `cycles`: A vector of cycles, each cycle is a vector of syntax indices.
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
/// - `arr`: Slice of syntax indices representing a cycle.
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
    cycle
        .iter()
        .cycle()
        .skip(start)
        .take(cycle.len())
        .cloned()
        .collect()
}

/// Computes the transitive closure of a boolean adjacency matrix using the Floyd-Warshall algorithm.
///
/// This function updates the given square matrix in place. After execution, `matrix[i][j]`
/// will be `true` if there exists any path (direct or indirect) from syntax `i` to syntax `j`.
///
/// This is useful in type_checker systems to compute all inherited types (i.e., whether a type_checker
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

/// Builds a direct inheritance adjacency matrix from a set of type_checker declarations.
///
/// This function constructs a square boolean matrix representing the direct
/// inheritance relationships between types. Each type_checker is assigned a unique index
/// via the `type_bimap`, and the matrix is constructed such that:
///
/// - `matrix[i][j] == true` if the type_checker at index `i` **directly inherits** from the type_checker at index
///   `j`.
/// - `matrix[i][j] == false` otherwise.
///
/// If a type_checker declaration does not specify any parent types, it is assumed to
/// implicitly inherit from the special `"object"` type_checker (if it exists in the map).
///
/// # Parameters
///
/// - `type_bimap`: A mapping from type_checker names to their unique integer indices.
///   This must include all types used in the declarations, and should include `"object"`
///   for correct handling of root types.
/// - `declarations`: A map of type_checker names to their `Declaration` objects. Each declaration
///   may include a list of parent types (i.e., superclasses or supertypes).
///
/// # Returns
///
/// A `Result` containing:
/// - On success: A square matrix `matrix[i][j]` where each row and column corresponds to a type_checker,
///   as defined in `type_bimap`. The matrix has the following meaning:
///     - `matrix[i][j] == true` ⇒ type_checker at index `i` inherits directly from type_checker at index `j`
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
    type_bimap: &BiMap<SymbolId, usize>,
    declarations: &Vec<&Declaration>,
) -> Result<Vec<Vec<bool>>, SemanticCheckError> {
    let n = type_bimap.len();

    // Preallocate adjacency matrix n x n with false
    let mut matrix = vec![vec![false; n]; n];

    // Get index of the special "object" typing once
    let object_index = type_bimap
        .get_by_left(&SymbolInterner::OBJECT_SYMBOL_ID)
        .copied();

    for declaration in declarations {
        let Some(&type_idx) = type_bimap.get_by_left(&declaration.symbol_ident()) else {
            // If typing is not found in map, just skip (consistency assumption)
            continue;
        };

        // Check typing index bounds
        if type_idx >= n {
            return Err(SemanticCheckError::type_index_out_of_bounds(
                type_idx,
                declaration.symbol_ident(),
                n - 1,
            ));
        }

        match declaration.ty() {
            Some(parents) => {
                for parent in parents.iter() {
                    if let Some(&parent_idx) = type_bimap.get_by_left(parent) {
                        // Check parent index bounds
                        if parent_idx >= n {
                            return Err(SemanticCheckError::parent_index_out_of_bounds(
                                parent_idx,
                                parent.clone(),
                                n - 1,
                            ));
                        }
                        matrix[type_idx][parent_idx] = true;
                    }
                }
            }
            None => {
                if let Some(j) = object_index {
                    if j >= n {
                        return Err(SemanticCheckError::object_index_out_of_bounds(j, n - 1));
                    }
                    matrix[type_idx][j] = true;
                }
            }
        }
    }

    Ok(matrix)
}

/// Builds a `BiMap` that assigns a unique index to each type_checker name found in the declarations.
///
/// This function avoids unnecessary `String` cloning by checking membership before insertion.
/// It collects:
/// - All declared types (keys in `declarations`)
/// - All parent types referenced in each declaration (if any)
/// - The special `"object"` type_checker, added if not already present
///
/// The returned `BiMap<String, usize>` enables:
/// - Efficient lookup from type_checker name to index (`left` map)
/// - Efficient reverse lookup from index to type_checker name (`right` map)
///
/// # Arguments
///
/// * `declarations` - A `HashMap` mapping type_checker names to their `Declaration` objects.
///
/// # Returns
///
/// A `BiMap<String, usize>` mapping type_checker names to unique indices assigned in insertion order.
///
/// # Example
///
/// ```rust
/// let type_index_map = build_type_index_map(&declarations);
/// let index = type_index_map.get_by_left("robot").unwrap();
/// let name = type_index_map.get_by_right(*index).unwrap();
/// ```
fn build_type_bimap(declarations: &Vec<&Declaration>) -> BiMap<SymbolId, usize> {
    // Create an empty BiMap to store type_checker names (String) and their unique indices (usize)
    let mut temp_map: BiMap<SymbolId, usize> = BiMap::new();

    // Iterate over each type_checker declaration in the input map
    for declaration in declarations {
        // If the type_checker name is not already in the BiMap, insert it with a new unique index
        if !temp_map.contains_left(&declaration.symbol_ident()) {
            let len = temp_map.len(); // Current size of the map used as next index
            temp_map.insert(declaration.symbol_ident().clone(), len); // Insert the type_checker name with the index
        }

        // If the declaration has parent types (e.g., inherited types)
        if let Some(parents) = declaration.ty() {
            // Iterate over each parent type_checker
            for parent in parents.iter() {
                // Insert the parent type_checker into the map if it's not already present
                if !temp_map.contains_left(parent) {
                    let len = temp_map.len(); // Get next index based on current size
                    temp_map.insert(parent.clone(), len); // Insert parent type_checker with index
                }
            }
        }
    }

    // Ensure the special OBJECT_TYPE is present in the map; add if missing
    if !temp_map.contains_left(&SymbolInterner::OBJECT_SYMBOL_ID) {
        let len = temp_map.len(); // Next index for insertion
        temp_map.insert(SymbolInterner::OBJECT_SYMBOL_ID, len); // Insert OBJECT_TYPE as a key
    }

    // Return the completed BiMap mapping type_checker names to unique indices
    temp_map
}
