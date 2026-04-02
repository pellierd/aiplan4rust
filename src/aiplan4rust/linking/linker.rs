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
use crate::aiplan4rust::interner::{InternerDisplay, InternerMergeResult};
use crate::aiplan4rust::linking::checks::perform_linking;
use crate::aiplan4rust::linking::error::LinkingError;
use crate::aiplan4rust::linking::{LinkedSemanticContext, LinkerResult};
use crate::aiplan4rust::semantic::checks::CheckContext;
use crate::aiplan4rust::semantic::passes::PassContext;
use crate::aiplan4rust::semantic::symbol_resolver::SymbolResolver;
use crate::aiplan4rust::semantic::{passes, AnalyzerResult};
use crate::aiplan4rust::semantic::{SymbolTable, TypeChecker};
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
            (Some(mut domain_ctx), Some(mut problem_ctx)) => {
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
                //resolve_external_references(&mut domain_ctx, &mut problem_ctx)?;

                // --- ÉTAPE : SIMPLIFICATION DU PROBLÈME ---

                // 1. On prépare la hiérarchie de référence (le Domaine)
                let type_hierarchy = domain_ctx.symbol_table().to_type_hierarchy();
                let type_checker = TypeChecker::new(&type_hierarchy);

                // 2. On simplifie la Table du Problème
                // On change 'finalize' pour qu'elle retourne les 'changes' comme on l'a fait avant
                let ctx =
                    PassContext::new(&global_interner, problem_ctx.source(), Provider::Linker);

                let changes = passes::symbol_table::finalize(
                    &ctx,
                    &type_checker,
                    problem_ctx.symbol_table_mut(), // On modifie la table du problème
                    &mut self.diagnostic_manager,
                )?;

                // 3. NOUVEAU : On synchronise l'AST du Problème
                // Si on a des changements, on les répercute sur l'AST pour que
                // le r-affichage (pretty print) du problème soit aussi propre que celui du domaine.
                if !changes.is_empty() {
                    passes::ast::finalize(&mut problem_ctx, &changes)?;
                }

                // Step 4: Create a check context for the problem using the global interner
                // and perform semantic and structural linking checks on the problem
                let mut total_declared = domain_ctx.declared_requirements().clone();
                total_declared.extend(problem_ctx.declared_requirements());

                // 1. On "prend" les tables (elles sont remplacées par des tables vides dans les contextes)
                // Cela libère domain_ctx et problem_ctx de tout emprunt mutable sur leurs tables.
                let mut domain_table = take(domain_ctx.symbol_table_mut());
                let mut problem_table = take(problem_ctx.symbol_table_mut());

                // 2. Maintenant, on peut créer les CheckContext sans conflit !
                // Rust autorise l'emprunt immuable de domain_ctx car domain_table est
                // maintenant une variable indépendante sur la pile.
                let domain_check_ctx = CheckContext::new(
                    domain_ctx.syntax_tree(),
                    &global_interner,
                    domain_ctx.source(),
                    Provider::Linker,
                    domain_ctx.declared_requirements(),
                    domain_ctx.required_requirements(),
                    domain_ctx.requirement_triggers(),
                );

                let problem_check_ctx = CheckContext::new(
                    problem_ctx.syntax_tree(),
                    &global_interner,
                    problem_ctx.source(),
                    Provider::Linker,
                    &total_declared,
                    problem_ctx.required_requirements(),
                    problem_ctx.requirement_triggers(),
                );

                // 3. On fait l'analyse avec les tables "volées" (et mutables !)
                perform_linking_checks(
                    &domain_check_ctx,
                    &problem_check_ctx,
                    &mut domain_table,
                    &mut problem_table,
                    &mut self.diagnostic_manager,
                )?;

                domain_ctx.set_symbol_table(domain_table);
                problem_ctx.set_symbol_table(problem_table);

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
pub fn perform_linking_checks(
    domain: &CheckContext,
    problem: &CheckContext,
    domain_table: &mut SymbolTable,
    problem_table: &mut SymbolTable,
    diagnostic_manager: &mut DiagnosticManager,
) -> Result<bool, LinkingError> {
    let type_hierarchy = domain_table.to_type_hierarchy();
    let type_checker = TypeChecker::new(&type_hierarchy);
    let mut check = true;

    let resolver = SymbolResolver::new(
        problem.syntax_tree(),
        Some(&type_checker),
        Some(domain_table),
    );

    // On résout les symboles du problème par rapport au domaine
    resolver.resolve(problem_table)?;

    // 2. PHASE UNIFIÉE : LE LINKER (Vissage)
    // Cette seule fonction remplace désormais link_undeclared, link_signatures et link_types.
    // Elle parcourt tous les usages et crée les proxies nécessaires.
    check &= perform_linking(
        problem_table,
        domain_table,
        problem, // Ton context pour match_declaration_with_usage
        &type_checker,
        diagnostic_manager,
    )?;

    /*println!(
        "DOMAIN\n{}",
        domain_table.to_string_with_interner(domain.interner())
    );*/
    println!(
        "PROBLEM:\n{}",
        problem_table.to_string_with_interner(problem.interner())
    );

    // 1. Vérification de base : Nom du domaine

    // 1. Vérification de base : Nom du domaine
    linking::checks::check_domain_name(
        domain,
        problem,
        domain_table,
        problem_table,
        diagnostic_manager,
    )?;

    /*let mut check = linking::checks::check_unresolved_usages(
        problem_table,
        domain_table,
        problem,
        diagnostic_manager,
    );*/

    // 2. On vérifie que les types utilisés dans le PROBLÈME existent dans le DOMAINE
    // On réutilise la fonction du domaine !
    check = semantic::checks::check_symbol_types(
        problem,
        problem_table,   // On scanne la table du problème
        &type_hierarchy, // Mais on valide par rapport à la hiérarchie du domaine
        diagnostic_manager,
    )?;

    // 3. Vérifications sémantiques post-linking
    if check {
        // Optionnel : tu peux garder cette vérification si tu veux détecter
        // explicitement des collisions (même nom déclaré dans les deux)
        check &= linking::checks::check_cross_declared_symbols(
            domain,
            problem,
            domain_table,
            problem_table,
            diagnostic_manager,
        )?;

        // Vérification des expressions typées (préconditions, effets, etc.)
        // Maintenant que les liens sont faits, le TypeChecker pourra remonter aux types du domaine.
        semantic::checks::check_typed_expressions(
            problem,
            problem_table,
            &type_checker,
            diagnostic_manager,
        )?;

        // Vérification des contraintes d'ordre et des requirements
        semantic::checks::check_task_ordering(problem, diagnostic_manager)?;
        semantic::checks::check_requirements(problem, diagnostic_manager)?;
    }

    Ok(check)
}
