use std::collections::HashMap;
use crate::aiplan4rust::diagnostic::{DiagnosticManager, Severity, Provider};
use crate::aiplan4rust::frontend::ParserInternalError;
use crate::aiplan4rust::linking::{resolution, LinkedSemanticContext};
use crate::aiplan4rust::linking::LinkerResult;
use crate::aiplan4rust::semantic::{SemanticContext, SymbolTable, TypeChecker};
use crate::aiplan4rust::{linking, semantic};

use std::mem::take;
use crate::aiplan4rust::interner::{InternerMergeResult, StringInterner};
use crate::aiplan4rust::semantic::checks::CheckContext;
use crate::aiplan4rust::syntax::elements::Ident;

/// The `Linker` is responsible for performing the linking phase
/// of the AIPlan4Rust compilation pipeline.
///
/// It takes care of:
/// 1. Resolving symbols between the domain and problem definitions,
/// 2. Performing semantic and structural consistency checks,
/// 3. Producing a `LinkerResult` which includes the linked domain/problem
///    pair (`LinkedSemanticContext`) and diagnostic information.
///
/// # Example
/// ```rust
/// let mut linker = Linker::new();
/// let result = linker.link(domain_context, problem_context)?;
/// if let Some(linked) = result.task() {
///     // use the linked planning task...
/// }
/// ```
#[derive(Debug)]
pub struct Linker {
    diagnostic_manager: DiagnosticManager,
}

impl Linker {
    /// Creates a new instance of the `Linker`.
    pub fn new() -> Self {
        Self {
            diagnostic_manager: DiagnosticManager::new(),
        }
    }

    /// Returns an immutable reference to the internal `DiagnosticManager`,
    /// which contains diagnostics collected during the linking process.
    pub fn diagnostic_manager(&self) -> &DiagnosticManager {
        &self.diagnostic_manager
    }

    /// Performs semantic linking between a domain and a problem context.
    ///
    /// This function carries out the following steps:
    ///
    /// 1. Merges string interners from the domain and the problem to create a global interner.
    /// 2. Remaps identifiers in the problem's AST and symbol table to correspond to the global interner.
    /// 3. Resolves external references in the problem against the domain.
    /// 4. Creates a verification context (`CheckContext`) using the problem's AST, symbol table,
    ///    global interner, source name, and requirements.
    /// 5. Performs semantic and structural linking checks on the problem.
    /// 6. Finalizes the linking process and returns the result, while managing diagnostics.
    ///
    /// # Arguments
    ///
    /// * `domain` - The semantic context representing the domain (reference context).
    /// * `problem` - The semantic context of the problem to be linked.
    ///
    /// # Returns
    ///
    /// * `Ok(LinkerResult)` if linking succeeds.
    /// * `Err(ParserInternalError)` if an error occurs during resolution or verification.
    ///
    /// # Notes
    ///
    /// Identifier remapping ensures symbol consistency within the unified identifier space.
    /// The diagnostic manager collects errors or warnings encountered during linking.
    ///
    pub fn link(
        &mut self,
        domain: SemanticContext,
        mut problem: SemanticContext,
    ) -> Result<LinkerResult, ParserInternalError> {

        // Step 1: Merge the string interners from domain and problem to form a global interner
        let mut result = InternerMergeResult::from_domain_and_problem(
            domain.interner(),
            problem.interner()
        );
        let global_interner = result.take_interner();

        // Step 2: Remap identifiers in the problem's AST and symbol table to the global interner space
        let problem_ident_map = result.take_problem_ident_map();
        remap_problem_idents(&mut problem, &problem_ident_map);

        // Step 3: Resolve external references in the problem with respect to the domain
        resolution::resolve_external_references(&domain, &mut problem)?;

        // Step 4: Create a check context for the problem using the global interner
        // and Pperform semantic and structural linking checks on the problem
        let problem_ctx = CheckContext::new(
            problem.ast(),
            problem.symbol_table(),
            &global_interner,
            problem.source_name(),
            problem.requirements(),
        );
        perform_linking_checks(&domain, &problem_ctx, &mut self.diagnostic_manager)?;

        // Step 6: Finalize and return the linking result, reporting diagnostics if any
        finalize_linking_result(domain, problem, &mut self.diagnostic_manager)
    }
}

/// Remaps identifiers in the problem's AST and symbol table using the provided mapping.
///
/// This function updates all identifiers in the problem’s AST and symbol table
/// to their corresponding identifiers in the global interner space according to
/// the `problem_ident_map`.
///
/// **Note:** This function does not modify or replace the underlying string interner itself;
/// it only updates the identifier references within the problem to align with the global interner.
///
/// # Arguments
///
/// * `problem` - Mutable reference to the problem semantic context.
/// * `problem_ident_map` - A mapping from the problem's local identifiers to the global identifiers.
fn remap_problem_idents(
    problem: &mut SemanticContext,
    problem_ident_map: &HashMap<Ident, Ident>,
) {
    problem.ast_mut().remap_idents(problem_ident_map);
    problem.symbol_table_mut().remap_idents(problem_ident_map);
}
/// Performs semantic and structural linking checks between a domain and a problem.
///
/// This function runs a sequence of verification passes to ensure the compatibility and
/// coherence between a domain and a problem during the linking phase. It emits diagnostics
/// (warnings and errors) via the provided `DiagnosticManager`.
///
/// The checks are performed in two phases:
///
/// 1. **Structural Checks** (always executed):
///     - Domain and problem name consistency (`check_domain_name`)
///     - Duplicate symbol declarations (`check_cross_declared_symbols`)
///     - Undeclared symbol usages (`check_undeclared_symbols`)
///     - Unused symbol declarations (`check_unused_symbols`)
///
/// 2. **Type-Dependent Checks** (executed only if no errors found in phase 1):
///     - Signature validation of declared symbols (`check_declared_symbol_signatures`)
///     - Type correctness of expressions (`check_typed_expressions`)
///     - Task ordering consistency (`check_task_ordering`)
///     - Requirement compliance (`check_requirement_violations`)
///
/// # Arguments
///
/// * `domain` - A reference to the domain's `SemanticContext`.
/// * `problem` - A reference to the problem's `SemanticContext`.
/// * `diagnostic_manager` - A mutable reference to the `DiagnosticManager` to collect diagnostics.
///
/// # Returns
///
/// Returns `Ok(true)` if all checks passed successfully without critical errors, or `Ok(false)`
/// if some checks failed but no internal error occurred. Returns `Err` if an internal error
/// (e.g., inconsistent state or invalid assumptions) occurs during the process.
///
/// # Errors
///
/// Returns `ParserInternalError` if an internal semantic or resolution error prevents
/// the checks from completing.
///
/// # Example
///
/// ```rust
/// let mut diagnostics = DiagnosticManager::default();
/// let result = perform_linking_checks(&domain_ctx, &problem_ctx, &mut diagnostics)?;
/// if !result {
///     eprintln!("Some linking checks failed");
/// }
/// ```
///
pub fn perform_linking_checks(
    domain: &SemanticContext,
    problem: &CheckContext,
    diagnostic_manager: &mut DiagnosticManager,
) -> Result<bool, ParserInternalError> {

    linking::checks::check_domain_name(domain, problem, Provider::Linker, diagnostic_manager)?;

    let mut check = linking::checks::check_cross_declared_symbols(domain, problem, Provider::Linker, diagnostic_manager)?;
    check &= semantic::checks::check_undeclared_symbols(problem, &[], Provider::Linker, diagnostic_manager)?;
    check &= semantic::checks::check_unused_symbols(problem, &[], Provider::Linker, diagnostic_manager)?;

    if check {
        let type_checker = TypeChecker::new(&domain.symbol_table());

        semantic::checks::check_declared_symbol_signatures(problem, &type_checker, diagnostic_manager)?;
        semantic::checks::check_typed_expressions(problem, &type_checker, Provider::Linker, diagnostic_manager)?;
        semantic::checks::check_task_ordering(problem, Provider::Linker, diagnostic_manager)?;

        let mut requirements = domain.requirements().clone();
        requirements.extend(problem.requirements().clone());

        semantic::checks::check_requirement_violations(
            problem,
            &requirements,
            Provider::Linker,
            diagnostic_manager,
        )?;
    }

    Ok(check)
}

/// Finalizes the result of the linking process by examining diagnostics
/// and determining whether a valid `LinkedSemanticContext` can be returned.
///
/// If any diagnostic with severity `Error` is present in the diagnostic manager,
/// this function returns a `LinkerResult` without a `LinkedSemanticContext` (`None`),
/// indicating that linking failed due to unrecoverable issues.
///
/// Otherwise, a new `LinkedSemanticContext` is created from the provided `domain`
/// and `problem`, and returned within the `LinkerResult`.
///
/// In both cases, the function takes ownership of the `diagnostic_manager`'s contents,
/// transferring any collected diagnostics into the result.
///
/// # Arguments
///
/// * `domain` - The fully resolved domain context.
/// * `problem` - The fully resolved problem context.
/// * `diagnostic_manager` - A mutable reference to the diagnostic manager holding any diagnostics emitted during linking.
///
/// # Returns
///
/// A `Result<LinkerResult, ParserInternalError>`:
/// - `Ok(LinkerResult)` containing either a valid `LinkedSemanticContext` or `None` (if errors are present).
/// - Any internal error from earlier stages is propagated as `Err`.
///
/// # Example
///
/// ```ignore
/// let result = finalize_linking_result(domain, problem, &mut diagnostic_manager)?;
/// if result.task().is_some() {
///     println!("Linking successful!");
/// }
/// ```
fn finalize_linking_result(
    mut domain: SemanticContext,
    mut problem: SemanticContext,
    diagnostic_manager: &mut DiagnosticManager,
) -> Result<LinkerResult, ParserInternalError> {

    let domain_table = domain.take_symbol_table();
    let problem_table = problem.take_symbol_table();
    let global_table = SymbolTable::merge(domain_table, problem_table)?;

    // If there are any errors in the diagnostics, return a result without a planning task.
    if diagnostic_manager.has_diagnotics_of_severity(Severity::Error) {
        Ok(LinkerResult::new(None, take(diagnostic_manager)))
    } else {
        // All checks passed — build the final linked planning task.
        let semantic_context = LinkedSemanticContext::new(domain, problem);

        // Return the linked task with the collected diagnostics.
        Ok(LinkerResult::new(Some(semantic_context), take(diagnostic_manager)))
    }
}
