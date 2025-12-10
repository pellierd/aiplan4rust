//! Module responsible for performing the semantic linking phase of the AIPlan4Rust compilation pipeline.
//!
//! The linking phase connects the semantic contexts of a syntax domain and a problem,
//! resolving identifiers, verifying consistency, and producing a combined linked semantic context.
//!
//! This module provides the `Linker` struct which:
//! - Merges string interners from domain and problem contexts to unify identifier spaces.
//! - Remaps identifiers in the problem to the global interner.
//! - Resolves external references from the problem against the domain.
//! - Performs semantic and structural consistency checks.
//! - Produces a `LinkerResult` encapsulating the linked semantic context and diagnostics.
//!
//! # Key Types
//!
//! - [`Linker`]: Main struct performing linking.
//! - [`LinkerResult`]: Encapsulates linking output and diagnostics.
//!
//! # Key Functions
//!
//! - [`Linker::link`]: Performs full semantic linking and verification.
//! - [`perform_linking_checks`]: Runs semantic and structural verification passes.
//!
//! # Usage Example
//!
//! ```rust
//! let mut linker = Linker::new();
//! let result = linker.link(domain_context, problem_context)?;
//! if let Some(linked_task) = result.context() {
//!     // Use the linked semantic context...
//! }
//! ```

use crate::aiplan4rust::diagnostic::{DiagnosticManager, Severity, Provider};
use crate::aiplan4rust::linking::{LinkedSemanticContext, LinkerResult};
use crate::aiplan4rust::semantic::{SemanticContext, SymbolTable, TypeChecker};
use crate::aiplan4rust::{linking, semantic};
use crate::aiplan4rust::interner::{InternerError, InternerMergeResult, Literal};
use crate::aiplan4rust::semantic::checks::CheckContext;
use crate::aiplan4rust::semantic::symbol::{Declaration, SymbolOrigin, Usage};
use crate::aiplan4rust::lang::Ident;
use crate::aiplan4rust::linking::error::LinkingError;
use crate::AnalyzerResult;

use std::collections::HashMap;
use std::mem::take;

/// The `Linker` struct is responsible for performing the linking phase
/// between domain and problem semantic contexts.
///
/// Linking resolves identifiers, checks semantic and structural consistency,
/// and produces a combined [`LinkedSemanticContext`] along with diagnostics.
///
/// # Example
///
/// ```rust
/// let mut linker = Linker::new();
/// let result = linker.link(domain_context, problem_context)?;
/// if let Some(linked) = result.context() {
///     // use the linked semantic context...
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
    /// This method executes the following steps in order:
    ///
    /// 1. Merges the string interners from the domain and the problem contexts to produce a global interner,
    ///    ensuring consistent identifier representation across both contexts.
    /// 2. Remaps identifiers in the problem's AST and symbol table to align with the global interner's identifier space.
    /// 3. Resolves external references within the problem context against the domain context to establish correct linkages.
    /// 4. Creates a `CheckContext` for the problem using the global interner, along with its syntax tree, symbol table,
    ///    source name, and requirements.
    /// 5. Performs semantic and structural linking checks on the problem context within the merged environment,
    ///    recording any diagnostics encountered.
    /// 6. If errors of severity `Error` are detected, returns early with diagnostics and the global interner.
    /// 7. Otherwise, constructs a final linked semantic context combining domain and problem data, with unified
    ///    interners and symbol tables.
    /// 8. Returns the successful linking result, including the linked semantic context and collected diagnostics.
    ///
    /// # Arguments
    ///
    /// * `domain` - The analyzer result containing the semantic context of the domain (reference context).
    /// * `problem` - The analyzer result containing the semantic context of the problem to be linked.
    ///
    /// # Returns
    ///
    /// * `Ok(LinkerResult)` containing the linked semantic context and diagnostics if linking succeeds.
    /// * `Err(LinkingError)` if any error occurs during resolution or verification.
    ///
    /// # Notes
    ///
    /// * Identifier remapping is crucial to maintain symbol consistency within the combined identifier space.
    /// * The diagnostic manager accumulates errors and warnings during linking, which are included in the result.
    /// * In case either domain or problem contexts are missing semantic information, the function returns
    ///   a failure result with diagnostics and a merged interner, ensuring graceful error handling.
    pub fn link(
        &mut self,
        mut domain: AnalyzerResult,
        mut problem: AnalyzerResult,
    ) -> Result<LinkerResult, LinkingError> {
        match (domain.take_semantic_context(), problem.take_semantic_context()) {
            (Some(mut domain_ctx), Some(mut problem_ctx)) => {

                // Step 1: Merge the string interners from domain and problem to form a global interner
                let mut result = InternerMergeResult::from_domain_and_problem(
                    domain_ctx.interner(),
                    problem_ctx.interner(),
                );
                let global_interner = result.take_interner();

                // Step 2: Remap identifiers in the problem's AST and symbol table to the global interner space
                let ident_map = result.take_ident_map();
                let literal_map = result.take_literal_map();
                remap_problem(&mut problem_ctx, &ident_map, &literal_map)?;
                // Collect diagnostics from domain and problem diagnostic managers
                self.diagnostic_manager.add_diagnostic_from(domain.take_diagnostic_manager());
                let mut problem_diag_mgr = problem.take_diagnostic_manager();
                problem_diag_mgr.remap(&ident_map, &literal_map);
                self.diagnostic_manager.add_diagnostic_from(problem_diag_mgr);

                // Step 3: Resolve external references in the problem with respect to the domain
                resolve_external_references(&domain_ctx, &mut problem_ctx)?;

                // Step 4: Create a check context for the problem using the global interner
                // and perform semantic and structural linking checks on the problem
                let problem_check_ctx = CheckContext::new(
                    problem_ctx.syntax_tree(),
                    problem_ctx.symbol_table(),
                    &global_interner,
                    problem_ctx.source_id(),
                    problem_ctx.requirements(),
                );
                perform_linking_checks(&domain_ctx, &problem_check_ctx, &mut self.diagnostic_manager)?;

                // Step 5: If errors, return early with diagnostics only
                if self.diagnostic_manager.has_diagnostics_of_severity(Severity::Error) {
                    return Ok(LinkerResult::failure(take(&mut self.diagnostic_manager), global_interner));
                }

                // Step 7: Construct the final linked semantic context
                let semantic_context = LinkedSemanticContext::new(
                    domain_ctx.take_syntax_tree(),
                    problem_ctx.take_syntax_tree(),
                    domain_ctx.take_symbol_table(),
                    problem_ctx.take_symbol_table(),
                    global_interner,
                    domain_ctx.source_id(),
                    problem_ctx.source_id(),
                );

                // Step 8: Return the result with the semantic context and diagnostics
                Ok(LinkerResult::success(semantic_context, take(&mut self.diagnostic_manager)))
            }
            _ => {
                let domain_interner = domain.take_interner();
                let problem_interner = problem.take_interner();
                let mut result = InternerMergeResult::from_domain_and_problem(&domain_interner, &problem_interner);
                let global_interner = result.take_interner();
                Ok(LinkerResult::failure(take(&mut self.diagnostic_manager), global_interner))
            }
        }
    }
}

/// Remaps identifiers and literals in the problem's AST and symbol table.
///
/// This function updates all identifier references within the problem's AST and symbol table
/// using `problem_ident_map`, and updates the source literal ID using `literal_map`.
/// It ensures that the problem’s identifiers and literals are aligned with the global interner,
/// facilitating consistent symbol resolution across linked semantic contexts.
///
/// **Important:** This function does **not** modify the string interner itself; it only updates
/// the identifier and literal references (indices/keys) in the problem's AST and symbol table.
///
/// # Arguments
///
/// * `problem` - A mutable reference to the problem semantic context whose identifiers
///   and source literal will be remapped.
/// * `ident_map` - A `HashMap` mapping local problem identifiers (`Ident`) to
///   their corresponding global identifiers.
/// * `literal_map` - A `HashMap` mapping local problem literals (`Literal`) to their
///   corresponding global literals.
///
/// # Returns
///
/// Returns `Ok(())` if remapping succeeded for both identifiers and literals, otherwise
/// returns a [`LinkingError`] encapsulating either a `SymbolTableError` or an `InternerError`.
pub fn remap_problem(
    problem: &mut SemanticContext,
    ident_map: &HashMap<Ident, Ident>,
    literal_map: &HashMap<Literal, Literal>,
) -> Result<(), LinkingError> {
    // Step 1: remap identifiers in AST
    problem.ast_mut().remap_idents(ident_map);

    // Step 2: remap identifiers in the symbol table
    problem.symbol_table_mut().remap_idents(ident_map)?;

    // Step 3: remap the source literal
    let new_source_id = literal_map
        .get(&problem.source_id())
        .ok_or_else(|| InternerError::missing_remap_literal(problem.source_id()))?;
    problem.set_source_id(*new_source_id);

    Ok(())
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
///     - Type correctness of expr (`check_typed_expressions`)
///     - Task ordering consistency (`check_task_ordering`)
///     - Requirement compliance (`check_requirement_violations`)
///
/// # Arguments
///
/// * `domain` - Reference to the domain's semantic context.
/// * `problem` - Reference to the problem's check context.
/// * `diagnostic_manager` - Mutable reference to collect diagnostics.
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
pub fn perform_linking_checks(
    domain: &SemanticContext,
    problem: &CheckContext,
    diagnostic_manager: &mut DiagnosticManager,
) -> Result<bool, LinkingError> {

    // Check that the domain name matches the problem's declared domain
    linking::checks::check_domain_name(domain, problem, Provider::Linker, diagnostic_manager)?;

    // Check for duplicate symbol declarations across domain and problem
    let mut check = linking::checks::check_cross_declared_symbols(
        domain,
        problem,
        Provider::Linker,
        diagnostic_manager,
    )?;

    // Check for undeclared symbols used in the problem
    check &= semantic::checks::check_undeclared_symbols(
        problem,
        &[],
        Provider::Linker,
        diagnostic_manager,
    )?;

    // Check for symbols declared but never used in the problem
    check &= semantic::checks::check_unused_symbols(
        problem,
        &[],
        Provider::Linker,
        diagnostic_manager,
    )?;

    // If structural checks passed, perform type_checker-dependent semantic checks
    if check {
        // Initialize a type_checker checker with the domain's symbol table
        let type_checker = TypeChecker::new(&domain.symbol_table());

        // Validate signatures of declared symbols
        semantic::checks::check_declared_symbol_signatures(
            problem,
            &type_checker,
            diagnostic_manager,
        )?;

        // Verify the type_checker correctness of expr in the problem
        semantic::checks::check_typed_expressions(
            problem,
            &type_checker,
            Provider::Linker,
            diagnostic_manager,
        )?;

        // Check task ordering constraints in the problem
        semantic::checks::check_task_ordering(problem, Provider::Linker, diagnostic_manager)?;

        // Merge requirements from domain and problem contexts
        let mut requirements = domain.requirements().clone();
        requirements.extend(problem.requirements().clone());

        // Check for any requirement violations
        semantic::checks::check_requirement_violations(
            problem,
            &requirements,
            Provider::Linker,
            diagnostic_manager,
        )?;
    }

    // Return whether all checks passed successfully
    Ok(check)
}

/// Resolves external references in the problem by injecting missing declarations from the domain.
///
/// This function updates the problem's symbol table by adding declarations found in the domain's
/// symbol table for symbols that are used but not declared in the problem. It ensures that the
/// problem's symbols have all necessary declarations available for subsequent semantic analysis.
///
/// The process involves:
/// - Collecting declared and undeclared symbols in the problem relative to the domain.
/// - For each declared symbol missing declarations, cloning and injecting the corresponding
///   declarations from the domain symbol table into the problem's symbol table.
///
/// # Arguments
///
/// * `domain` - Reference to the domain's semantic context.
/// * `problem` - Mutable reference to the problem's semantic context, to be updated.
///
/// # Returns
///
/// Returns `Ok(())` if external references are successfully resolved.
/// Returns `Err(ParserInternalError)` if any internal semantic error occurs during resolution.
///
/// # Example
///
/// ```ignore
/// resolve_external_references(&domain_context, &mut problem_context)?;
/// ```
pub fn resolve_external_references(
    domain: &SemanticContext,
    problem: &mut SemanticContext,
) -> Result<(), LinkingError> {
    // Collect declared and undeclared symbols in the problem relative to the domain symbol table
    let mut declared = Vec::new();
    let mut undeclared = Vec::new();

    collect_declared_and_undeclared_symbols(
        problem,
        domain.symbol_table(),
        &mut declared,
        &mut undeclared,
    )?;

    let problem_symbol_table = problem.symbol_table_mut();

    // For each declared symbol, inject the corresponding declaration into the problem symbol table
    for (symbol_name, declaration) in declared {
        if let Some(symbol) = problem_symbol_table.get_symbol_mut(symbol_name) {
            symbol.add_declaration(declaration);
        }
    }

    Ok(())
}

/// Collects symbol declarations from the domain symbol table for problem symbols that lack declarations,
/// and gathers symbols that remain undeclared.
///
/// This function does **not** mutate any symbol tables directly.
/// Instead, it fills the provided vectors:
/// - `declared`: collects `(symbol_name, declaration)` pairs to later add to the problem's symbol table.
/// - `undeclared`: collects `(symbol_name, usage)` pairs representing unresolved symbols.
///
/// The function iterates over all symbols in the problem's symbol table that currently have no declarations.
/// For each usage of such a symbol, it attempts to resolve a matching declaration in the domain's symbol table.
/// If found, it clones the declaration, marks it as originating from the domain,
/// and adds it to `declared`. Otherwise, it records the symbol and usage as `undeclared`.
///
/// # Arguments
///
/// * `problem` - Reference to the problem's semantic context.
/// * `domain_symbol_table` - Reference to the domain's symbol table for resolving declarations.
/// * `declared` - Mutable vector to collect declarations to add to problem symbols.
/// * `undeclared` - Mutable vector to collect unresolved symbols and their usages.
///
/// # Returns
///
/// Returns `Ok(true)` if all problem symbols were successfully resolved from the domain.
/// Returns `Ok(false)` if some symbols remain undeclared.
/// Returns `Err(ParserInternalError)` if any error occurs during symbol resolution.
///
/// # Example
///
/// ```ignore
/// let mut declared = Vec::new();
/// let mut undeclared = Vec::new();
/// let all_resolved = collect_declared_and_undeclared_symbols(
///     &problem_context,
///     &domain_symbol_table,
///     &mut declared,
///     &mut undeclared,
/// )?;
/// if !all_resolved {
///     // Handle diagnostics for undeclared symbols here
/// }
/// ```
fn collect_declared_and_undeclared_symbols<'a>(
    problem: &'a SemanticContext,
    domain_symbol_table: &'a SymbolTable,
    declared: &mut Vec<(Ident, Declaration)>,
    undeclared: &mut Vec<(Ident, &'a Usage)>,
) -> Result<bool, LinkingError> {
    let problem_symbol_table = problem.symbol_table();
    let mut all_resolved = true;

    for symbol in problem_symbol_table.values() {
        if symbol.declarations().is_empty() {
            for usage in symbol.usages() {
                let domain_declaration_option = domain_symbol_table.resolve_declaration(
                    &symbol.ident(),
                    &usage.symbol_kind(),
                    &domain_symbol_table.root_scope(),
                )?;

                if let Some(domain_declaration) = domain_declaration_option {
                    let mut domain_declaration = domain_declaration.clone();
                    domain_declaration.set_origin(SymbolOrigin::Domain);
                    domain_declaration.set_imported_scope(Some(domain_declaration.scope().clone()));
                    domain_declaration.set_scope(problem.symbol_table().root_scope().clone());
                    declared.push((symbol.ident(), domain_declaration));
                } else {
                    undeclared.push((symbol.ident(), usage));
                    all_resolved = false;
                }
            }
        }
    }

    Ok(all_resolved)
}
