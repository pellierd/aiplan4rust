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
//! - [`Result`]: Encapsulates linking output and diagnostics.
//!
//! # Key Functions
//!
//! - [`Linker::link`]: Performs full semantic linking and verification.
//! - [`perform_linking_checks`]: Runs semantic and structural verification logic.
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

use std::mem::take;

use crate::aiplan4rust::diagnostic::{DiagnosticManager, Provider, Severity};
use crate::aiplan4rust::interner::InternerMergeResult;
use crate::aiplan4rust::lang::SymbolId;
use crate::aiplan4rust::linking::error::LinkingError;
use crate::aiplan4rust::linking::{LinkedSemanticContext, LinkerResult};
use crate::aiplan4rust::semantic::checks::CheckContext;
use crate::aiplan4rust::semantic::symbol::{Declaration, Symbol, SymbolKind, SymbolOrigin, Usage};
use crate::aiplan4rust::semantic::{passes, AnalyzerResult};
use crate::aiplan4rust::semantic::{SemanticContext, SymbolTable, TypeChecker};
use crate::aiplan4rust::{linking, semantic};

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
        match (
            domain.take_semantic_context(),
            problem.take_semantic_context(),
        ) {
            (Some(domain_ctx), Some(mut problem_ctx)) => {
                // Step 1: Merge the string interners from domain and problem to form a global interner
                let mut result = InternerMergeResult::from_domain_and_problem(
                    domain_ctx.interner(),
                    problem_ctx.interner(),
                );
                let global_interner = result.take_interner();

                // Step 2: Remap identifiers in the problem's AST and symbol table to the global interner space
                let ident_map = result.take_symbol_map();
                let literal_map = result.take_literal_map();
                problem_ctx.remap(&ident_map, &literal_map)?;

                // Collect diagnostics from domain and problem diagnostic managers
                self.diagnostic_manager
                    .add_diagnostic_from(domain.take_diagnostic_manager());
                let mut problem_diag_mgr = problem.take_diagnostic_manager();
                problem_diag_mgr.remap(&ident_map, &literal_map)?;
                self.diagnostic_manager
                    .add_diagnostic_from(problem_diag_mgr);

                // Step 3: Resolve external references in the problem with respect to the domain
                resolve_external_references(&domain_ctx, &mut problem_ctx)?;

                // --- NOUVELLE ÉTAPE : SIMPLIFICATION DU PROBLÈME ---
                // 1. On prépare la hiérarchie du domaine (qui est la référence)
                let type_hierarchy = domain_ctx.symbol_table().to_type_hierarchy();
                let type_checker = TypeChecker::new(&type_hierarchy);

                // 2. On simplifie la table des symboles du problème
                // Maintenant que le problème connaît les types du domaine,
                // on peut réduire les (either A B) du problème.
                passes::simplify_symbol_table(
                    &type_checker,
                    problem_ctx.symbol_table_mut(),
                    &mut self.diagnostic_manager,
                )?;

                // Step 4: Create a check context for the problem using the global interner
                // and perform semantic and structural linking checks on the problem
                let mut total_declared = domain_ctx.declared_requirements().clone();
                total_declared.extend(problem_ctx.declared_requirements());

                let problem_check_ctx = CheckContext::new(
                    problem_ctx.syntax_tree(),
                    problem_ctx.symbol_table(),
                    &global_interner,
                    problem_ctx.source_id(),
                    &total_declared,
                    problem_ctx.required_requirements(),
                    problem_ctx.requirement_triggers(),
                );
                perform_linking_checks(
                    &domain_ctx,
                    &problem_check_ctx,
                    &mut self.diagnostic_manager,
                )?;

                // Step 5: If errors, return early with diagnostics only
                if self
                    .diagnostic_manager
                    .has_diagnostics_of_severity(Severity::Error)
                {
                    return Ok(LinkerResult::failure(
                        take(&mut self.diagnostic_manager),
                        global_interner,
                    ));
                }

                // Step 7: Construct the final linked semantic context
                let semantic_context =
                    LinkedSemanticContext::new(domain_ctx, problem_ctx, global_interner)?;

                // Adapte selon ton API
                // Step 8: Return the result with the semantic context and diagnostics
                Ok(LinkerResult::success(
                    semantic_context,
                    take(&mut self.diagnostic_manager),
                ))
            }
            _ => {
                let domain_interner = domain.take_interner();
                let problem_interner = problem.take_interner();
                let mut result = InternerMergeResult::from_domain_and_problem(
                    &domain_interner,
                    &problem_interner,
                );
                let global_interner = result.take_interner();
                self.diagnostic_manager
                    .add_diagnostic_from(domain.take_diagnostic_manager());
                let mut problem_diag_mgr = problem.take_diagnostic_manager();
                problem_diag_mgr.remap(result.symbol_map(), result.literal_map())?;
                Ok(LinkerResult::failure(
                    take(&mut self.diagnostic_manager),
                    global_interner,
                ))
            }
        }
    }
}

/// Performs semantic and structural linking checks between a domain and a problem.
///
/// This function runs a sequence of verification logic to ensure the compatibility and
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
///     - Type correctness of logic (`check_typed_expressions`)
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
fn perform_linking_checks(
    domain: &SemanticContext,
    problem: &CheckContext,
    diagnostic_manager: &mut DiagnosticManager,
) -> Result<bool, LinkingError> {
    let type_hierarchy = domain.symbol_table().to_type_hierarchy();

    let type_checker = TypeChecker::new(&type_hierarchy);

    // Check that the domain name matches the problem's declared domain
    linking::checks::check_domain_name(domain, problem, Provider::Linker, diagnostic_manager)?;

    // 2. On vérifie que les types utilisés dans le PROBLÈME existent dans le DOMAINE
    // On réutilise la fonction du domaine !
    let mut check = semantic::checks::check_symbol_types(
        problem,         // On scanne la table du problème
        &type_hierarchy, // Mais on valide par rapport à la hiérarchie du domaine
        diagnostic_manager,
    )?;

    // Check for duplicate symbol declarations across domain and problem
    check &= linking::checks::check_cross_declared_symbols(
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

    // If structural checks passed, perform type_checker-dependent semantic checks
    if check {
        // Initialize a type_checker checker with the domain's symbol table
        // Validate signatures of declared symbols
        semantic::checks::check_symbol_signatures(problem, &type_checker, diagnostic_manager)?;

        // Verify the type_checker correctness of logic in the problem
        semantic::checks::check_typed_expressions(
            problem,
            &type_checker,
            Provider::Linker,
            diagnostic_manager,
        )?;

        // Check task ordering constraints in the problem
        semantic::checks::check_task_ordering(problem, Provider::Linker, diagnostic_manager)?;

        // Check for any requirement violations
        semantic::checks::check_requirement_violations(
            problem,
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
fn resolve_external_references(
    domain: &SemanticContext,
    problem: &mut SemanticContext,
) -> Result<(), LinkingError> {
    let mut declared = Vec::new();
    let mut undeclared = Vec::new();
    let mut to_verify = Vec::new();

    collect_declared_and_undeclared_symbols(
        problem,
        domain.symbol_table(),
        &mut declared,
        &mut undeclared,
        &mut to_verify,
    )?;

    if !to_verify.is_empty() {
        let hierarchy = domain.symbol_table().to_type_hierarchy();
        let type_checker = TypeChecker::new(&hierarchy);
        let interner = domain.interner();

        for (dom_decl, prob_decl) in to_verify {
            if let (Some(dom_type), Some(prob_type)) = (dom_decl.ty(), prob_decl.ty()) {
                // Règle de sous-typage stricte
                match type_checker.is_any_subtype_of(dom_type, prob_type) {
                    Ok(true) => {
                        // Succès : Le problème confirme ou spécialise le domaine.
                        // On ne fait rien, on laisse la déclaration du problème telle quelle.
                    }
                    _ => {
                        let symbol_name = interner
                            .resolve_symbol(prob_decl.symbol().id())
                            .unwrap_or("unknown");
                        let dom_type_str = interner
                            .resolve_symbol(dom_type.members()[0])
                            .unwrap_or("?");
                        let prob_type_str = interner
                            .resolve_symbol(prob_type.members()[0])
                            .unwrap_or("?");

                        panic!(
                            "\n[Linking Error] Incompatible redefinition for symbol '{}':\n\
                             - Domain expects:  {}\n\
                             - Problem defined: {}\n\
                             => To fix this for UM-Translog, change the domain constant to a parent type (e.g., Truck).",
                            symbol_name, dom_type_str, prob_type_str
                        );
                    }
                }
            }
        }
    }

    // Injection des constantes du domaine dans le contexte du problème
    for (symbol_name, declaration) in declared {
        problem.add_declaration(symbol_name, declaration);
    }

    Ok(())
}

fn collect_declared_and_undeclared_symbols<'a>(
    problem: &'a SemanticContext,
    domain_symbol_table: &'a SymbolTable,
    declared: &mut Vec<(SymbolId, Declaration)>,
    undeclared: &mut Vec<(SymbolId, &'a Usage)>,
    to_verify: &mut Vec<(&'a Declaration, &'a Declaration)>,
) -> Result<bool, LinkingError> {
    let problem_symbol_table = problem.symbol_table();
    let mut all_resolved = true;

    for symbol in problem_symbol_table.values() {
        let symbol_ident = symbol.ident();

        // --- MODIFICATION ICI : On exclut le nom du domaine et du problème ---
        let problem_decls: Vec<&Declaration> = symbol
            .declarations()
            .iter()
            .filter(|d| {
                let k = d.symbol().kind();
                k != SymbolKind::DomainName && k != SymbolKind::ProblemName
            })
            .collect();

        if problem_decls.is_empty() {
            for usage in symbol.usages() {
                let kind = usage.symbol_kind();

                // On ignore aussi ces types dans les usages pour le linking
                if kind == SymbolKind::DomainName || kind == SymbolKind::ProblemName {
                    continue;
                }

                let dom_decl_opt = domain_symbol_table.resolve_declaration(
                    &symbol_ident,
                    &kind,
                    &domain_symbol_table.root_scope(),
                )?;

                if let Some(dom_decl) = dom_decl_opt {
                    if !declared
                        .iter()
                        .any(|(id, d)| *id == symbol_ident && d.symbol().kind() == kind)
                    {
                        let mut linked_decl = dom_decl.clone();
                        linked_decl.set_origin(SymbolOrigin::Domain);
                        linked_decl.set_imported_scope(Some(dom_decl.scope().clone()));
                        linked_decl.set_scope(problem.symbol_table().root_scope().clone());
                        declared.push((symbol_ident, linked_decl));
                    }
                } else {
                    undeclared.push((symbol_ident, usage));
                    all_resolved = false;
                }
            }
        } else if problem_decls.len() == 1 {
            let prob_decl = problem_decls[0];
            let kind = prob_decl.symbol().kind();

            let dom_decl_opt = domain_symbol_table.resolve_declaration(
                &symbol_ident,
                &kind,
                &domain_symbol_table.root_scope(),
            )?;

            if let Some(dom_decl) = dom_decl_opt {
                to_verify.push((dom_decl, prob_decl));
            }
        } else {
            return Err(LinkingError::duplicate_symbol_declaration(Symbol::new(
                symbol_ident,
                problem_decls[0].symbol().kind(),
            )));
        }
    }

    Ok(all_resolved)
}
