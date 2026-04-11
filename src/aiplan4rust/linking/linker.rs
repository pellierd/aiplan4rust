//! Module responsible for performing the semantic linking phase of the AIPlan4Rust compilation pipeline.
//!
//! The linking phase connects the semantic contexts of a syntax domain and a problem,
//! resolving identifiers, verifying consistency, and producing a combined linked semantic context.
//!
//! This module provides the `Linker` struct which:
//! - **Unifies Identifier Spaces**: Merges string interners from domain and problem contexts.
//! - **Remaps Problem Symbols**: Aligns problem AST and symbol tables with the global interner.
//! - **Cross-Context Analysis**: Performs semantic and structural consistency checks between domain and problem.
//! - **Produces Linked Results**: Encapsulates the final [`LinkedSemanticContext`] and collected diagnostics.
//!
//! # Key Types
//!
//! - [`Linker`]: Main struct performing the linking process.
//! - [`LinkerResult`]: Encapsulates the success or failure of the linking, including diagnostics.
//! - [`LinkedSemanticContext`]: The final unified semantic model (Domain + Problem).
//!
//! # Key Functions
//!
//! - [`Linker::link`]: High-level entry point for full semantic linking.
//! - [`perform_problem_analysis`]: Core coordinator for cross-context validation logic.
//!
//! # Usage Example
//!
//! ```rust
//! let mut linker = Linker::new();
//! // domain_res and problem_res are AnalyzerResult instances
//! let result = linker.link(domain_res, problem_res)?;
//!
//! if let Some(linked_context) = result.context() {
//!     // Access unified interner, domain, and problem contexts...
//! }
//! ````

use crate::aiplan4rust::diagnostic::{DiagnosticManager, Provider};
use crate::aiplan4rust::interner::{InternerMergeResult, SymbolInterner};
use crate::aiplan4rust::lang::{LiteralId, Requirement};
use crate::aiplan4rust::linking::error::LinkingError;
use crate::aiplan4rust::linking::finalization::FinalizationContext;
use crate::aiplan4rust::linking::{finalization, LinkedSemanticContext, LinkerResult};
use crate::aiplan4rust::semantic::checks::CheckContext;
use crate::aiplan4rust::semantic::passes::PassContext;
use crate::aiplan4rust::semantic::{passes, AnalyzerResult, SemanticContext};
use crate::aiplan4rust::semantic::{SymbolTable, TypeChecker};
use crate::aiplan4rust::syntax::ast::AstNode;
use crate::aiplan4rust::tree::{NodeId, Tree};
use crate::aiplan4rust::{linking, semantic};
use crate::Severity;
use std::collections::HashMap;

/// The `Linker` struct orchestrates the semantic linking between domain and problem contexts.
///
/// It manages a [`DiagnosticManager`] to collect errors and warnings during the multi-step
/// linking process: unification of interners, identifier remapping, and cross-context
/// semantic verification.
///
/// # Internal Workflow
/// The `Linker` follows a strict pipeline to ensure memory safety and borrow checker
/// compliance while performing mutable transformations on the contexts:
/// 1. Technical unification of string interners.
/// 2. Isolation and analysis of symbol tables.
/// 3. Construction of the final unified semantic context.
///
/// # Example
///
/// ```rust
/// let mut linker = Linker::new();
/// let result = linker.link(domain_result, problem_result)?;
///
/// if result.is_success() {
///     println!("Linking successful!");
/// } else {
///     for diag in result.diagnostic_manager().diagnostics() {
///         println!("{:?}", diag);
///     }
/// }
/// ```
#[derive(Debug)]
pub struct Linker {
    /// Internal manager for collecting diagnostics during the linking phase.
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

    /// Performs the complete semantic linking process between a domain and a problem.
    ///
    /// This is the primary entry point of the [`Linker`]. It unifies the identifier spaces,
    /// executes cross-context semantic analysis, and packages the results into a
    /// [`LinkerResult`].
    ///
    /// ### Linking Pipeline
    /// 1. **Technical Unification**: Merges the string interners and remaps the problem's
    ///    identifiers and diagnostics to a shared global space.
    /// 2. **Context Extraction**: Retrieves the semantic contexts from the analyzer results.
    ///    If either is missing, it returns a failure result immediately.
    /// 3. **Semantic Analysis**: Performs deep checks (binding, types, requirements) between
    ///    the domain and the problem.
    /// 4. **Severity Validation**: Evaluates collected diagnostics. While warnings are
    ///    tolerated, any diagnostic with `Severity::Error` triggers a failure return.
    /// 5. **Context Construction**: Upon success, wraps the unified contexts and interner
    ///    into a [`LinkedSemanticContext`].
    ///
    /// # Arguments
    /// * `domain` - The result from the domain analysis.
    /// * `problem` - The result from the problem analysis.
    ///
    /// # Returns
    /// * `Ok(LinkerResult)` - A result containing either the successfully linked context
    ///   or a collection of diagnostics in case of semantic failure.
    /// * `Err(LinkingError)` - If a fatal internal error occurs (e.g., AST corruption).
    pub fn link(
        &mut self,
        mut domain: AnalyzerResult,
        mut problem: AnalyzerResult,
    ) -> Result<LinkerResult, LinkingError> {
        // 1. Technical unification (Interner + Remap + Diagnostics)
        let global_interner = self.unify_problem_interner(&mut domain, &mut problem)?;

        // 2. Extract contexts with early return if either is None
        let (Some(mut dc), Some(mut pc)) = (
            domain.take_semantic_context(),
            problem.take_semantic_context(),
        ) else {
            return Ok(LinkerResult::failure(
                std::mem::take(&mut self.diagnostic_manager),
                global_interner,
            ));
        };

        // 3. Semantic analysis (populates the diagnostic manager)
        self.perform_linking_analysis(&mut dc, &mut pc, &global_interner)?;

        // 4. Severity check (tolerate Warnings, block on Errors)
        if self
            .diagnostic_manager
            .has_diagnostics_of_severity(Severity::Error)
        {
            return Ok(LinkerResult::failure(
                std::mem::take(&mut self.diagnostic_manager),
                global_interner,
            ));
        }
        /*let mut domain_ast = dc.take_syntax_tree();
        let mut domain_table = dc.take_symbol_table();
        let domain_source = dc.source();
        self.finalize(
            &mut domain_ast,
            &mut domain_table,
            domain_source,
            &global_interner,
        )?;
        dc.set_syntax_tree(domain_ast);
        dc.set_symbol_table(domain_table);*/

        // 5. Success: Construct the linked semantic context
        let mut context = LinkedSemanticContext::new(dc, pc, global_interner)?;

        Ok(LinkerResult::success(
            context,
            std::mem::take(&mut self.diagnostic_manager),
        ))
    }

    /// Orchestrates the semantic and structural analysis between domain and problem contexts.
    ///
    /// This method prepares the environment for cross-context validation by unifying requirements
    /// and isolating symbol tables to avoid mutable borrow conflicts. It then delegates the
    /// actual check logic to [`perform_problem_analysis`].
    ///
    /// ### Workflow
    /// 1. **Requirement Merging**: Combines requirements from both domain and problem to
    ///    ensure the problem analysis respects the full set of declared features.
    /// 2. **Table Isolation**: Temporarily extracts (via `std::mem::take`) the symbol tables
    ///    from the contexts. This allows the checker to mutably access the tables while
    ///    immutably referencing the rest of the contexts.
    /// 3. **Context Preparation**: Creates ephemeral `CheckContext` instances for both
    ///    sides using the unified global interner.
    /// 4. **Cross-Analysis**: Executes the core semantic checks (binding, type checking, etc.).
    /// 5. **Restoration**: Re-inserts the symbol tables back into their respective
    ///    semantic contexts regardless of analysis success.
    ///
    /// # Arguments
    /// * `domain_ctx` - The semantic context of the domain.
    /// * `problem_ctx` - The semantic context of the problem.
    /// * `global_interner` - The unified interner containing symbols from both sides.
    ///
    /// # Returns
    /// * `Ok(())` - Analysis completed (check `self.diagnostic_manager` for semantic errors).
    fn perform_linking_analysis(
        &mut self,
        domain_ctx: &mut SemanticContext,
        problem_ctx: &mut SemanticContext,
        global_interner: &SymbolInterner,
    ) -> Result<(), LinkingError> {
        // 1. Prepare cumulative requirements (Domain + Problem)
        let mut total_declared = domain_ctx.declared_requirements().clone();
        total_declared.extend(problem_ctx.declared_requirements());

        // 2. Isolate symbol tables
        // We use std::mem::take to extract the tables and leave empty ones
        // in the contexts. This releases mutable borrows on dc and pc.
        let mut domain_table = std::mem::take(domain_ctx.symbol_table_mut());
        let mut problem_table = std::mem::take(problem_ctx.symbol_table_mut());

        // 3. Create ephemeral check contexts
        let domain_check_ctx = CheckContext::new(
            domain_ctx.syntax_tree(),
            global_interner,
            domain_ctx.source(),
            Provider::Linker,
            domain_ctx.declared_requirements(),
        );

        let problem_check_ctx = CheckContext::new(
            problem_ctx.syntax_tree(),
            global_interner,
            problem_ctx.source(),
            Provider::Linker,
            &total_declared,
        );

        // 4. Execute cross-analysis
        // We pass the extracted tables (mutable) and the requirement triggers
        let inferred_requirements = perform_problem_analysis(
            &domain_check_ctx,
            &problem_check_ctx,
            &mut domain_table,
            &mut problem_table,
            &mut self.diagnostic_manager,
        )?;

        // 5. Update the Problem context with the results of the inference pass
        // This transitions the problem context from "pending" to "analyzed
        problem_ctx.set_requirement_triggers(inferred_requirements);

        // 6. Restore tables to their respective contexts
        domain_ctx.set_symbol_table(domain_table);
        problem_ctx.set_symbol_table(problem_table);

        Ok(())
    }

    /// This method performs the technical synchronization required to merge two independent
    /// semantic contexts. It aligns the problem's internal IDs with the domain's ID space
    /// to ensure consistent symbol resolution during the linking phase.
    ///
    /// ### Process
    /// 1. **Interner Merging**: Computes a new `SymbolInterner` containing the union of
    ///    all symbols from both contexts and generates mapping tables.
    /// 2. **Context Remapping**: Updates the problem's semantic context (AST and Symbol Table)
    ///    so that all existing identifiers point to their new IDs in the global interner.
    /// 3. **Diagnostic Alignment**: Collects diagnostics from both results, remapping
    ///    those from the problem to maintain correct source references.
    ///
    /// # Arguments
    /// * `domain_res` - The analyzer result of the domain (used as the primary ID reference).
    /// * `problem_res` - The analyzer result of the problem to be remapped and unified.
    ///
    /// # Returns
    /// * `Ok(SymbolInterner)` - The unified global interner.
    fn unify_problem_interner(
        &mut self,
        domain_res: &mut AnalyzerResult,
        problem_res: &mut AnalyzerResult,
    ) -> Result<SymbolInterner, LinkingError> {
        // Step 1: Direct merging of interners (Computation)
        let mut result = InternerMergeResult::from_domain_and_problem(
            domain_res.interner(),
            problem_res.interner(),
        );

        let global_interner = result.take_interner();
        let ident_map = result.take_symbol_map();
        let literal_map = result.take_literal_map();

        // Step 2: Remap the semantic context (if present)
        if let Some(ctx) = problem_res.semantic_context_mut() {
            ctx.remap(&ident_map, &literal_map)?;
        }

        // Step 3: Merge and Remap diagnostics
        self.diagnostic_manager
            .add_diagnostic_from(domain_res.take_diagnostic_manager());

        let mut problem_diag_mgr = problem_res.take_diagnostic_manager();
        problem_diag_mgr.remap(&ident_map, &literal_map)?;
        self.diagnostic_manager
            .add_diagnostic_from(problem_diag_mgr);

        Ok(global_interner)
    }

    fn finalize(
        &mut self,
        ast: &mut Tree<AstNode>,
        symbol_table: &SymbolTable,
        source: LiteralId,
        interner: &SymbolInterner,
    ) -> Result<(), LinkingError> {
        let context = FinalizationContext::new(interner, source, Provider::Linker);
        finalization::types::finalize(&context, symbol_table, ast)?;

        Ok(())
    }
}

/// Performs a comprehensive linking analysis between a Domain and a Problem.
///
/// This function acts as the central coordinator for structural and semantic
/// validation of the Problem AST against the Domain's definitions. It ensures
/// that the problem is not only syntactically correct but also semantically
/// consistent with the rules and objects defined in its associated domain.
///
/// ### Analysis Strategy: Adaptive Modes
/// The function operates in two distinct modes depending on the outcome of the
/// **Symbol Binding** phase:
///
/// 1. **Normal Mode (Full Analysis)**:
///    Triggered if all critical symbols (types, constants, predicates) are
///    successfully resolved. Performs deep checks including typed expressions
///    in `:init` and `:goal`, and cross-context name conflicts.
///
/// 2. **Degraded Mode (Surface Analysis)**:
///    Triggered if binding fails (e.g., orphan types or missing domain references).
///    Only executes non-dependent checks (domain name matching, structural
///    task network ordering) to provide maximum diagnostic feedback.
///
/// ### Analysis Phases
/// * **Symbol Binding**: Resolves problem-local symbols (objects) and links them
///   to domain-level declarations.
/// * **Metadata Consistency**: Verifies matching domain names.
/// * **Type Integrity**: Validates the problem's object hierarchy.
/// * **Deep Expression Analysis**: Performs type-checking on initial state
///   fluents and goal conditions.
/// * **Requirement Extraction**: **(Crucial)** Detects utilized PDDL features
///   within the problem AST *after* symbols have been merged, allowing for
///   accurate identification of fluents and requirements.
/// * **Requirement Compliance**: Validates inferred features against the
///   merged requirements set.
///
/// # Arguments
/// * `domain` - The domain's check context.
/// * `problem` - The problem's check context.
/// * `domain_table` - The resolved symbol table of the domain (reference).
/// * `problem_table` - The mutable symbol table of the problem, enriched during analysis.
/// * `diagnostic_manager` - Accumulates errors and warnings.
///
/// # Returns
/// * `Ok(HashMap<Requirement, Vec<NodeId>>)` - The map of requirements inferred
///   from the problem AST. Returns an empty map if analysis failed or was degraded.
/// * `Err(LinkingError)` - A fatal internal error prevented completion.
fn perform_problem_analysis(
    domain: &CheckContext,
    problem: &CheckContext,
    domain_table: &SymbolTable,
    problem_table: &mut SymbolTable,
    diagnostic_manager: &mut DiagnosticManager,
) -> Result<HashMap<Requirement, Vec<NodeId>>, LinkingError> {
    let type_hierarchy = domain_table.to_type_hierarchy();
    let type_checker = TypeChecker::new(&type_hierarchy);
    let mut inferred_reqs = HashMap::new();

    let pass_context = PassContext::new(
        problem.syntax_tree(),
        problem.interner(),
        problem.source(),
        Provider::Linker,
    );

    // --- PHASE 1: BINDING (Resolution Attempt) ---
    // Attempt to bind problem symbols to domain declarations.
    // resolve_symbols should return Ok(true) if all critical symbols are successfully linked.
    let binding_success = passes::resolve_symbols(
        &pass_context,
        problem_table,
        Some(&type_checker),
        Some(domain_table),
    )
    .is_ok();

    if binding_success {
        let mut is_valid = true;
        // ==========================================
        // NORMAL MODE: Full Semantic Analysis
        // ==========================================

        // 1. Metadata consistency (e.g., matching domain names)
        is_valid &= linking::checks::check_domain_name(
            domain,
            problem,
            domain_table,
            problem_table,
            diagnostic_manager,
        )?;

        // 2. Symbol usage (ensure utilized symbols are properly defined)
        is_valid &=
            semantic::checks::check_symbol_usage(problem, problem_table, &[], diagnostic_manager)?;

        // 3. Type validation (Objects vs. Domain Hierarchy)
        is_valid &= semantic::checks::check_symbol_types(
            problem,
            problem_table,
            &type_hierarchy,
            diagnostic_manager,
        )?;

        if is_valid {
            // 4. Name conflicts (e.g., illegal shadowing across scopes)
            linking::checks::check_cross_declared_symbols(
                domain,
                problem,
                domain_table,
                problem_table,
                diagnostic_manager,
            )?;

            // 5. Deep expression analysis (:init, :goal)
            semantic::checks::check_typed_expressions(
                problem,
                problem_table,
                &type_checker,
                diagnostic_manager,
            )?;

            // 6. Structural constraints (e.g., Task Network cycles)
            semantic::checks::check_task_ordering(problem, diagnostic_manager)?;

            // 7. Detect which PDDL features are actually used in the problem.
            inferred_reqs = passes::extract_required_requirements(&pass_context, &problem_table)?;

            // 8. Requirement compliance check
            // Note: problem (CheckContext) already contains the merged Domain + Problem requirements.
            semantic::checks::check_requirements(
                problem,        // CheckContext with merged requirements
                &inferred_reqs, // Inferred requirements and their evidence
                diagnostic_manager,
            )?;
        }
    } else {
        // ==========================================
        // DEGRADED MODE: Surface Analysis Only
        // ==========================================
        // Binding failed (orphan symbols detected).
        // We only execute checks that do not depend on successful resolution.

        // Still check the domain name to provide helpful feedback to the user
        let _ = linking::checks::check_domain_name(
            domain,
            problem,
            domain_table,
            problem_table,
            diagnostic_manager,
        );

        // Still check task ordering (pure graph-based analysis)
        let _ = semantic::checks::check_task_ordering(problem, diagnostic_manager);

        // Note: Resolution errors (symbols not found) are already
        // injected into the diagnostic_manager by `resolve_symbols`.
    }

    Ok(inferred_reqs)
}
