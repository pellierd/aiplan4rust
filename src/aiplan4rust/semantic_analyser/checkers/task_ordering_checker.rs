use crate::aiplan4rust::error::ErrorManager;
use crate::aiplan4rust::error::ParserErrorKind;
use crate::aiplan4rust::error::ParsingError;
use crate::aiplan4rust::frontend::ParserInternalError;
use crate::aiplan4rust::parser::syntax_tree::SyntaxNodeKind;
use crate::aiplan4rust::semantic_analyser::heap_syntax_tree::HeapSyntaxNode;
use crate::aiplan4rust::semantic_analyser::heap_syntax_tree::HeapSyntaxTree;
use crate::aiplan4rust::semantic_analyser::AnnotatedSyntaxTree;

use std::collections::HashMap;

/// Checks the task ordering constraints in the provided annotated syntax tree and detects any
/// cyclic dependencies.
///
/// This function analyzes the annotated syntax tree and checks for task ordering constraints
/// defined within it. If any cyclic dependencies are detected in the task ordering constraints, an
/// error is reported to the `ErrorManager`. It uses the transitive closure to compute all possible
/// orderings and verifies if there are any cycles in the matrix of task orderings.
///
/// # Arguments
///
/// * `tree` - A reference to the `AnnotatedSyntaxTree` which contains the symbol table and the
///   abstract syntax tree (AST). The function will traverse this tree to find task ordering
///   constraints and check for cycles.
/// * `errors` - A mutable reference to the `ErrorManager` where errors are collected and reported.
///   If a cyclic dependency is found, it will be added as an error.
///
/// # Returns
///
/// Returns a `Result<bool, ParserInternalError>`:
/// - `Ok(true)` if no cyclic task ordering constraints are found.
/// - `Ok(false)` if at least one cyclic dependency is detected.
/// - `Err(ParserInternalError)` if an internal error occurs while extracting task IDs, building the
///   matrix, or checking for cycles.
///
/// # Example
///
/// ```rust
/// let tree = ...; // An AnnotatedSyntaxTree containing the task ordering constraints
/// let mut errors = ErrorManager::new();
/// match check(&tree, &mut errors) {
///     Ok(true) => println!("No cyclic dependencies detected."),
///     Ok(false) => println!("Cyclic dependencies detected."),
///     Err(e) => eprintln!("An error occurred: {}", e),
/// }
/// ```
///
/// # Notes
///
/// - The function traverses the AST looking for nodes of type `TaskOrderingConstraintDef`, which
///   define task ordering constraints.
/// - The matrix of task orderings is built from the task IDs, and a transitive closure is
///   calculated to determine all direct and indirect ordering relationships.
/// - If a cycle is detected in the transitive closure, it is reported as an error with the relevant
///   line and column information.
///
/// # Errors
/// If the function encounters any internal errors during the process of extracting task IDs or
/// building the matrix, it will return a `ParserInternalError`.
///
pub fn check(
    tree: &AnnotatedSyntaxTree,
    errors: &mut ErrorManager,
) -> Result<bool, ParserInternalError> {
    let mut checked = true;
    let symbol_table = tree.symbol_table();
    let ast_table = tree.syntax_tree();

    for ast in ast_table.values() {
        match ast.kind() {
            SyntaxNodeKind::TaskOrderingConstraintDef => {
                let (line, column) = ast.span().start_position();
                let task_ids = extract_task_ids(ast, ast_table)?;
                let mut matrix = build_task_order_matrix(&task_ids)?;
                transitive_closure(&mut matrix);
                if is_cyclic(&matrix)? {
                    checked = false;
                    let (line, column) = ast.span().start_position();
                    let content = "Cyclic task ordering constraint detected.";
                    let error = ParsingError::new(
                        ParserErrorKind::ParseError,
                        Some(tree.filename().clone()),
                        line,
                        column,
                        content.to_string(),
                    );
                    errors.add_error(error);
                }
            }
            _ => {}
        }
    }

    Ok(checked)
}

/// Extracts all TaskID values from the given syntax tree node and its children.
///
/// This function traverses the syntax tree starting from the provided node, recursively extracting
/// all TaskID values found within it. The function searches for nodes of type `TaskID` and adds the
/// associated string identifiers to a vector. If a node does not contain a `TaskID`, the function
/// recursively searches its children.
///
/// # Arguments
///
/// * `node` - A reference to the `AstEntry` node from which the extraction starts. This node may
///   have child nodes containing `TaskID` values.
/// * `tree` - A reference to the `AstTable` that stores the entire syntax tree. The function will
///   use this to retrieve child nodes and access their information.
///
/// # Returns
///
/// The function returns a `Result`:
/// - `Ok(Vec<&'a String>)`: A vector of references to the `String` identifiers of the `TaskID`
///   nodes found within the tree. These are ordered as they appear in the tree.
/// - `Err(ParserInternalError)`: If a child node cannot be found in the tree or if an error occurs
///   during extraction, an error is returned with a message describing the issue.
///
/// # Example
///
/// ```rust
/// let node = ...;  // An AstEntry node in the syntax tree
/// let tree = ...;  // An AstTable containing the entire syntax tree
/// let result = extract_task_ids(node, tree);
///
/// match result {
///     Ok(task_ids) => {
///         for task_id in task_ids {
///             println!("Found task ID: {}", task_id);
///         }
///     }
///     Err(e) => {
///         eprintln!("Error extracting task IDs: {}", e);
///     }
/// }
/// ```
///
/// # Notes
///
/// - The function performs a depth-first traversal of the tree, so the task IDs will be extracted
///   in the order they appear in the tree, from top to bottom.
/// - If a node does not directly contain a `TaskID`, the function will recursively search through
///   its child nodes.
fn extract_task_ids<'a>(
    node: &'a HeapSyntaxNode,
    tree: &'a HeapSyntaxTree,
) -> Result<Vec<&'a String>, ParserInternalError> {
    let mut vec_task_id = Vec::new();
    for child_index in node.children() {
        let child_node = match tree.get_entry(*child_index) {
            Some(child) => child,
            None => {
                return Err(ParserInternalError::new(format!(
                    "Entry not found for child index: {}",
                    child_index
                )))
            }
        };
        match child_node.kind() {
            SyntaxNodeKind::TaskID(id) => {
                vec_task_id.push(id);
            }
            _ => {
                // Recursively handle non-TaskID children
                let nested = extract_task_ids(child_node, tree)?;
                vec_task_id.extend(nested);
            }
        }
    }
    Ok(vec_task_id)
}

/// Builds a matrix representing the order constraints between tasks.
///
/// This function takes a slice of task IDs and creates a matrix that represents the ordering
/// constraints between them. Each pair of consecutive task IDs in the slice is treated as a
/// constraint where the first task in the pair must precede the second task. The matrix is built
/// with the task IDs mapped to unique indices, where `matrix[i][j]` is `true` if task `i` must
/// precede task `j`. The matrix is square, with a size equal to the number of unique task IDs.
///
/// # Arguments
///
/// * `task_ids` - A vector of task IDs (`&String`). The vector should have an even length, as it is
///   expected to contain pairs of task IDs representing ordering constraints.
///
/// # Returns
///
/// This function returns a `Result` with:
/// - `Ok(Vec<Vec<bool>>)`: A matrix where each element represents whether one task must precede
///   another. Each row/column corresponds to a unique task ID.
/// - `Err(ParserInternalError)`: If the length of the task IDs slice is odd, or if another error
///   occurs.
///
/// # Example
///
/// ```rust
/// let task_ids = vec![
///     "task1".to_string(),
///     "task2".to_string(),
///     "task3".to_string(),
///     "task4".to_string(),
/// ];
/// let matrix = build_task_order_matrix(&task_ids);
///
/// // matrix will contain the following:
/// // {
/// //     task1 => 0,
/// //     task2 => 1,
/// //     task3 => 2,
/// //     task4 => 3,
/// // }
/// // and matrix will look like:
/// // [
/// //     [false, true, false, false],
/// //     [false, false, false, false],
/// //     [false, false, false, true],
/// //     [false, false, false, false],
/// // ]
/// ```
///
/// # Errors
///
/// - If the length of the `task_ids` slice is not even, the function will return a
///   `ParserInternalError` indicating that the length must be even.
///
/// # Notes
///
/// - Each consecutive pair of task IDs in the input slice represents an ordering constraint where
///   the first task must precede the second.
fn build_task_order_matrix(task_ids: &Vec<&String>) -> Result<Vec<Vec<bool>>, ParserInternalError> {
    // Ensure the number of task IDs is even, as we expect pairs of tasks
    if task_ids.len() % 2 != 0 {
        return Err(ParserInternalError::new(
            "task_ids length must be even".to_string(),
        ));
    }

    // Build a map from task IDs to unique indices
    let map = build_task_index_map(&task_ids);
    let size = map.len(); // Number of unique tasks
    let mut matrix = vec![vec![false; size]; size]; // Initialize the matrix with false values

    // Iterate over the task IDs in pairs (task1, task2), (task3, task4), etc.
    for pair in task_ids.chunks(2) {
        // Get the indices of the task IDs in the map
        let i = map[pair[0]];
        let j = map[pair[1]];

        // Set the matrix entry to true for the ordering constraint
        matrix[i][j] = true;
    }

    // Return the resulting matrix
    Ok(matrix)
}

/// Builds a map from task IDs to unique indices.
///
/// This function creates a mapping from each task ID in the provided slice to a unique index.
/// Each task ID is associated with the first index it appears at in the slice. If a task ID appears
/// multiple times, it will always be mapped to the same index. The function ensures that each task
/// ID receives a distinct index, with no duplicates.
///
/// # Arguments
///
/// * `task_ids` - A slice of references to `String` values, each representing a task ID. The slice
///   can contain duplicates, but each task ID will only be assigned a unique index in the returned
///   map.
///
/// # Returns
///
/// This function returns a `HashMap` where:
/// - The keys are references to task IDs (`&String`).
/// - The values are unique indices (`usize`), starting from 0.
///
/// # Example
///
/// ```rust
/// let task_ids = vec![
///     "task1".to_string(),
///     "task2".to_string(),
///     "task1".to_string(),
/// ];
/// let map = build_task_index_map(&task_ids);
///
/// // map will contain:
/// // {
/// //     "task1" => 0,
/// //     "task2" => 1,
/// // }
/// ```
///
/// # Notes
///
/// - Task IDs that appear multiple times in the input slice will be assigned the same index.
/// - The returned map will have only unique task IDs as keys, with no duplicates.
/// - The function iterates over the slice once, and assigns indices sequentially based on the
///   order in which task IDs appear.
fn build_task_index_map<'a>(task_ids: &[&'a String]) -> HashMap<&'a String, usize> {
    let mut map = HashMap::new();
    let mut index = 0;
    for &task_id in task_ids {
        if !map.contains_key(task_id) {
            map.insert(task_id, index);
            index += 1;
        }
    }
    map
}

/// Computes the transitive closure of a matrix.
///
/// The transitive closure of a matrix represents the indirect dependencies between tasks or nodes.
/// This function modifies the input matrix such that if a path exists between two nodes
/// (even indirectly), the corresponding matrix entry will be set to `true`. In other words,
/// it updates the matrix to reflect all possible dependencies between tasks or nodes, including
/// those that are not direct.
///
/// This function applies the **Floyd-Warshall algorithm**, which is a well-known algorithm for
/// computing the transitive closure of a graph represented by an adjacency matrix.
///
/// **Note:** This function modifies the input matrix in place, so no new matrix is returned.
///
/// # Arguments
///
/// * `matrix` - A mutable reference to a 2D vector (`Vec<Vec<bool>>`) representing the matrix.
///   The matrix should be square (i.e., the number of rows is equal to the number of columns),
///   where `true` represents a dependency between two tasks (or nodes) and `false` means no direct
///   dependency.
///
/// # Example
///
/// ```rust
/// let mut matrix = vec![
///     vec![false, true, false],
///     vec![false, false, true],
///     vec![true, false, false],
/// ];
///
/// transitive_closure(&mut matrix);
///
/// // After the transitive closure, the matrix will reflect indirect dependencies.
/// // matrix will become:
/// // vec![
/// //     vec![true, true, true],
/// //     vec![true, true, true],
/// //     vec![true, true, true],
/// // ]
/// ```
///
/// # Notes
///
/// This function assumes that the matrix is square and that the diagonal elements represent direct
/// dependencies (or lack thereof). After computing the transitive closure, the matrix will
/// represent both direct and indirect dependencies between all nodes.
///
/// The function modifies the matrix in place, so you should make sure to pass a mutable reference
/// to it. If the matrix is not square, the behavior is undefined.
fn transitive_closure(matrix: &mut Vec<Vec<bool>>) {
    let n = matrix.len();
    for k in 0..n {
        for i in 0..n {
            for j in 0..n {
                if matrix[i][k] == true && matrix[k][j] == true {
                    matrix[i][j] = true;
                }
            }
        }
    }
}

/// Checks if a matrix contains a cycle.
///
/// A cycle is detected if any element on the diagonal of the matrix is `true`.
/// This function assumes that the matrix represents a graph of tasks or dependencies,
/// where a `true` value indicates a dependency between tasks. If a task is dependent
/// on itself (i.e., there is a `true` value on the diagonal), a cycle exists.
///
/// **Note:** The matrix should already have undergone a transitive closure calculation
/// before being passed to this function. The transitive closure ensures that all indirect
/// dependencies are captured, meaning that if a task indirectly depends on itself,
/// it will be reflected in the diagonal element.
///
/// # Arguments
///
/// * `matrix` - A reference to a 2D vector (`Vec<Vec<bool>>`) representing the matrix.
///   The matrix should be square (i.e., the number of rows is equal to the number of columns),
///   and the transitive closure should have been computed beforehand.
///
/// # Returns
///
/// * `Ok(true)` if a cycle is detected (i.e., a `true` value is found on the diagonal),
///   otherwise `Ok(false)` if no cycle is detected.
///
/// # Errors
///
/// This function returns a `ParserInternalError` if the matrix is not square.
///
/// # Examples
///
/// ```rust
/// let matrix = vec![
///     vec![true, false, false],
///     vec![false, true, false],
///     vec![false, false, true],
/// ];
/// assert_eq!(is_cyclic(&matrix), Ok(true));  // Cycle detected on the diagonal
///
/// let matrix_no_cycle = vec![
///     vec![false, true, false],
///     vec![false, false, true],
///     vec![true, false, false],
/// ];
/// assert_eq!(is_cyclic(&matrix_no_cycle), Ok(false));  // No cycle detected
/// ```
///
/// # Notes
///
/// This function assumes that the matrix has already been updated to reflect the transitive
/// closure. If you need to compute the transitive closure, you can use the `transitive_closure`
/// function before calling this function. The matrix is expected to be square, and if it is not, an
/// error will be returned.
///
/// The check for cycles is performed by inspecting the diagonal elements of the matrix.
/// If any of the diagonal elements are `true`, it indicates a cycle (self-dependency).
fn is_cyclic(matrix: &[Vec<bool>]) -> Result<bool, ParserInternalError> {
    if !is_square(matrix) {
        return Err(ParserInternalError::new("Matrix is not square".to_string()));
    }
    for i in 0..matrix.len() {
        if matrix[i][i] {
            return Ok(true);
        }
    }
    Ok(false)
}

/// Checks if a matrix is square.
///
/// A matrix is considered square if the number of rows is equal to the number of columns.
///
/// # Arguments
///
/// * `matrix` - A reference to a 2D vector (`Vec<Vec<bool>>`) representing the matrix.
///
/// # Returns
///
/// * `true` if the matrix is square (i.e., the number of rows equals the number of columns),
///   otherwise `false`.
///
/// # Examples
///
/// ```rust
/// let matrix = vec![
///     vec![true, false, true],
///     vec![false, true, false],
///     vec![true, true, true],
/// ];
/// assert_eq!(is_square(&matrix), true);
///
/// let non_square_matrix = vec![
///     vec![true, false],
///     vec![false, true],
/// ];
/// assert_eq!(is_square(&non_square_matrix), false);
/// ```
///
/// # Panics
///
/// This function will panic if the matrix contains rows of inconsistent lengths,
/// but this is avoided by using the `all` method to check all rows' lengths before returning.
///
/// # Notes
///
/// An empty matrix (with no rows) is considered non-square by this function.
fn is_square(matrix: &[Vec<bool>]) -> bool {
    let size = matrix.len();
    matrix.iter().all(|row| row.len() == size)
}
