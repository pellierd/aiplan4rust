//! This module defines the `LirBuilder` struct and associated functions to
//! simplification a linked and semantically verified syntax domain and problem
//! into a lifted intermediate representation (`LiftedProblem`).
//!
//! # Overview
//! The `LirBuilder` processes a `LinkedSemanticContext` containing:
//! - The domain and problem abstract syntax trees (ASTs) with all references
//!   resolved and semantic checks passed.
//! - Extracts domain elements such as types, predicates, functions, actions,
//!   and methods.
//! - Extracts problem elements such as objects, initial state, goals, and metrics.
//!
//! The resulting `LiftedProblem`:
//! - Is a structured, reusable, and symbolic representation of the syntax problem.
//! - Remains "lifted", i.e., it uses symbolic references rather than grounded
//!   enumerations of instances.
//!
//! # Purpose
//! This IR is a crucial intermediate step for:
//! - Subsequent compiler logic or transformations.
//! - Planning solvers that instantiate and search for plans.
//! - Frontends for visualization or debugging.
//!
//! # Important Notes
//! - This module does not perform syntax or solving itself.
//! - The input semantic context must already be validated and linked.
//!
//! # Main Components
//! - `IRBuilder`: the main struct performing extraction and building the IR.
//! - `build()`: entry point to produce the `LiftedProblem` from a semantic context.
//! - Helper functions for extracting domain and problem elements.
//!
//! # Example
//! ```ignore
//! let mut encoder = LirEncoder::new();
//! let lifted_problem = encoder.encode(&linked_context)?;
//! ```

use crate::aiplan4rust::core::diagnostic::DiagnosticManager;
use crate::aiplan4rust::linking::LinkedSemanticContext;
use crate::aiplan4rust::lir::encoding::encoding::encode_domain as new_encode_domain;
use crate::aiplan4rust::lir::encoding::encoding::encode_problem as new_encode_problem;
use crate::aiplan4rust::lir::encoding::EncodingRegistry as NewEncodingRegistry;
use crate::aiplan4rust::lir::expr::{ExprBuilder, ExprStore};
use crate::aiplan4rust::lir::problem::LiftedProblem;
use crate::aiplan4rust::lir::{normalization, LirError};
use crate::LirEncoderResult;

/// This module defines the `LirBuilder`, which transforms a parsed and linked
/// syntax domain/problem into a *lifted intermediate representation* (LiftedProblem).
///
/// # What does it do?
/// - It takes as input a `LinkedSemanticContext`, which contains:
///   - The domain and problem ASTs.
///   - All references resolved (names linked to definitions).
///   - Semantic checks already passed (the input is guaranteed to be consistent).
/// - It extracts:
///   - Types, constants, predicates, functions, actions, and methods (from the domain).
///   - Objects, initial state, goals, and metrics (from the problem).
/// - It builds a structured, reusable representation of the problem.
///   - This representation is "lifted", meaning:
///     - It keeps symbolic references (e.g., variable names, types).
///     - It is not grounded yet (no enumeration of all possible substitutions).
///
/// # Why is this useful?
/// - This lifted problem can then be used by:
///   - Other compiler logic or transformations.
///   - A solver to instantiate and search for plans.
///   - A visualization frontend.
///
/// # Note
/// This module **does not** perform any syntax by itself.
/// It only prepares data for later use.
/// The input has already been verified to be semantically correct.
#[derive(Debug, Default)]
pub struct LirEncoder {
    diagnostic_manager: DiagnosticManager,
}

impl LirEncoder {
    /// Creates a new instance of IRBuilder.
    pub fn new() -> Self {
        LirEncoder {
            diagnostic_manager: DiagnosticManager::new(),
        }
    }

    /// Returns an immutable reference to the internal `DiagnosticManager`,
    /// which contains diagnostics collected during the LIR building process.
    pub fn diagnostic_manager(&self) -> &DiagnosticManager {
        &self.diagnostic_manager
    }

    /// Entry point for generating a `LiftedProblem` IR from a `LinkedSemanticContext`.
    ///
    /// This function performs extraction of both the domain and problem components
    /// from the linked semantic context and builds the corresponding intermediate representation (IR).
    ///
    /// # Arguments
    ///
    /// - `context`: A mutable reference to the `LinkedSemanticContext` which contains
    ///   the parsed and linked semantic trees for domain and problem.
    ///
    /// # Returns
    ///
    /// - `Ok(LirBuilderResult)`: Contains the constructed `LiftedProblem` IR wrapped in
    ///   `Some` along with any diagnostics collected during the build process, if successful.
    /// - `Err(LirError)`: An error indicating failure during the extraction or building phase.
    ///
    /// # Errors
    ///
    /// This function propagates errors encountered during domain or problem extraction.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// let mut builder = IRBuilder::new();
    /// let mut linked_context = /* obtain linked semantic context */;
    /// match builder.build(&mut linked_context) {
    ///     Ok(result) => {
    ///         if let Some(ir) = result.lifted_problem() {
    ///             // Use the IR here
    ///         }
    ///     }
    ///     Err(e) => {
    ///         eprintln!("Error during IR building: {:?}", e);
    ///     }
    /// }
    /// ```
    pub fn encode(&mut self, context: LinkedSemanticContext) -> Result<LirEncoderResult, LirError> {
        // 1. Create a LiftedProblem from the linked semantic context
        let lifted_problem = encode_lifted_problem(context)?;

        // 2. Return the successful result containing the constructed LiftedProblem
        //    and the diagnostics collected during the build process
        Ok(LirEncoderResult::success(
            lifted_problem,
            std::mem::take(&mut self.diagnostic_manager),
        ))
    }

    pub fn build_with_diagnostic_manager(
        &mut self,
        context: LinkedSemanticContext,
        diagnostic_manager: DiagnosticManager,
    ) -> Result<LirEncoderResult, LirError> {
        self.diagnostic_manager = diagnostic_manager;
        self.encode(context)
    }
}

/// Encode a LiftedProblem from a LinkedSemanticContext.
/// This is the core transformation that now integrates the ExprStore.
fn encode_lifted_problem(mut context: LinkedSemanticContext) -> Result<LiftedProblem, LirError> {
    // 1. Consommation de l'interner et des requirements
    let interner = context.take_interner();
    let requirements = context.take_required_requirements();

    // 2. Création du LiftedProblem
    let mut problem = LiftedProblem::new(requirements);
    problem.set_interner(interner);

    // --- ARCHITECTURE STORE ---
    // 3. Initialisation du Builder d'expressions.
    // C'est lui qui va posséder le Store pendant toute la phase d'encodage.
    let mut expr_store = ExprStore::new();
    let mut builder = ExprBuilder::new(&mut expr_store);

    // 4. Encodage des éléments du DOMAINE
    let domain_symbol_table = context.take_domain_table();
    let domain_syntax_tree = context.take_domain_syntax_tree();

    // Le registre commence avec la table des symboles du domaine
    let mut registry = NewEncodingRegistry::new(domain_symbol_table);

    // On utilise ton nouveau module d'orchestration pour le domaine
    // Note: On passe le builder pour que les actions/méthodes soient stockées
    new_encode_domain(
        &domain_syntax_tree,
        &mut registry,
        &mut problem,
        &mut builder,
    )
    .map_err(|e| LirError::from(e))?;

    // 5. Encodage des éléments du PROBLÈME
    let problem_symbol_table = context.take_problem_table();
    let problem_syntax_tree = context.take_problem_syntax_tree();

    // On met à jour le registre avec la table des symboles du problème (objets, etc.)
    registry.set_symbol_table(problem_symbol_table);

    // On utilise ton nouveau module d'orchestration pour le problème
    // Note: Le builder continue de remplir le même Store
    new_encode_problem(
        &problem_syntax_tree,
        &mut registry,
        &mut problem,
        &mut builder,
    )?;

    // --- FINALISATION ---

    // 6. Transfert du Store vers le LiftedProblem
    // Une fois l'encodage fini, on extrait le old du builder pour le donner au problème.
    problem.set_store(expr_store);

    println!("{} ", problem.domain_view());

    println!("{} ", problem.problem_view());

    // 7. Normalisation (si tes passes sont à jour pour le nouveau Store)
    normalization::normalize(&mut problem)?;

    // 8. Retour du problème entièrement construit
    Ok(problem)
}
