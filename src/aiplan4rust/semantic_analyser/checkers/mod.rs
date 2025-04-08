/// The `checker` module provides various checks and validation mechanisms for the semantic analysis
/// phase of the compiler. It ensures that the program adheres to the language's semantic rules,
/// including type correctness, symbol declarations, and task ordering. Each submodule addresses a
/// specific aspect of the semantic analysis process.
///
/// # Submodules
/// - `atomic_formula_checker`: Validates the correctness of atomic formulas used in expressions.
/// - `functional_expression_checker`: Ensures that functional expressions are correctly formed and
///   typed.
/// - `symbol_declaration_checker`: Verifies that all symbols (variables, functions, etc.) are
///   declared before use.
/// - `task_ordering_checker`: Checks the ordering of tasks to ensure that dependencies and
///   execution orders are respected.
/// - `type_checker`: Performs type checking on expressions and ensures type consistency across the
///   program.
/// - `undeclared_symbol_checker`: Detects any usage of symbols that have not been declared in the
///   program.
/// - `unused_symbol_checker`: Identifies symbols that are declared but never used, helping to clean
///   up the code.
///
/// # Usage
/// The `checker` module is typically used by the compiler during the semantic analysis phase, and
/// can be accessed by importing the relevant submodules. Here's an example of how to use the
/// `TypeChecker`:
///
pub mod atomic_formula_checker;
pub mod functional_expression_checker;
pub mod symbol_declaration_checker;
pub mod task_ordering_checker;
pub mod type_checker;
pub mod undeclared_symbol_checker;
pub mod unused_symbol_checker;

pub use type_checker::TypeChecker;
