//! # Semantic Analyzer Module
//!
//! This module provides the `Analyzer` struct and related functionality to perform semantic analysis
//! on abstract syntax trees (ASTs) generated from parsing AI syntax domain and problem specifications.
//!
//! ## Overview
//!
//! The `Analyzer` is responsible for:
//! - Validating the semantic correctness of the input AST.
//! - Detecting and reporting semantic errors such as undeclared symbols, unused symbols, typing mismatches,
//!   and task ordering violations.
//! - Managing diagnostics (errors, warnings, and informational messages) during analysis.
//!
//! The analysis supports two primary root AST kinds:
//! - `Domain`: checks related to domain specifications (e.g., symbol declarations, typing hierarchies, logic).
//! - `Problem`: checks related to problem instances within a domain (e.g., symbol usage, task ordering).
//!
//! ## Main Types
//!
//! - [`Analyzer`]: The main struct that performs semantic analysis and collects diagnostics.
//! - [`DiagnosticManager`]: Manages diagnostics such as errors and warnings.
//! - [`SemanticContext`]: Represents the annotated semantic information for an AST.
//! - [`Result`]: Contains the outcome of the analysis including semantic context and diagnostics.
//!
//! ## Usage Example
//!
//! ```rust
//! use crate::aiplan4rust::analyzer::Analyzer;
//! use crate::aiplan4rust::syntax::ast::Ast;
//!
//! let mut ast = Ast::parse("...domain and problem source...")?;
//! let mut analyzer = Analyzer::new();
//!
//! match analyzer.analyze(&mut ast) {
//!     Ok(result) => {
//!         if let Some(semantic_ctx) = result.semantic_context() {
//!             println!("Semantic analysis succeeded.");
//!         } else {
//!             println!("Semantic errors were found.");
//!         }
//!         // Diagnostics can be inspected via result.diagnostics()
//!     }
//!     Err(err) => eprintln!("Failed to analyze AST: {}", err),
//! }
//! ```
//!
//! ## Notes
//!
//! - The analyzer may mutate the AST during analysis, consuming internal resources such as string interners
//!   to improve efficiency.
//! - Diagnostic information is collected throughout the analysis and can be retrieved after analysis completes.
//! - Custom diagnostic managers can be injected to collect or customize diagnostic handling.
//!
//! ## Error Handling
//!
//! Semantic errors are returned as variants of [`SemanticError`]. These may include unexpected AST node kinds,
//! typing errors, symbol resolution errors, and other domain-specific semantic validation failures.

use crate::aiplan4rust::diagnostic::{DiagnosticManager, Provider, Severity};
use crate::aiplan4rust::lang::{SymbolId, Type};
use crate::aiplan4rust::normalization::NormalizerResult;
use crate::aiplan4rust::semantic;
use crate::aiplan4rust::semantic::checks::CheckContext;
use crate::aiplan4rust::semantic::symbol::SymbolKind;
use crate::aiplan4rust::semantic::type_checker::TypeCheckError;
use crate::aiplan4rust::semantic::AnalyzerResult;
use crate::aiplan4rust::semantic::{SemanticContext, SemanticError, TypeChecker};
use crate::aiplan4rust::syntax::ast::{Ast, AstKind};
use crate::aiplan4rust::tree::NodeId;
use crate::SymbolTable;

/// The `Analyzer` struct is responsible for performing semantic analysis on a `SyntaxTree`.
///
/// It manages the detection and collection of errors encountered during the analysis process.
/// The `diagnostic_manager` field stores all diagnostics, including errors, warnings,
/// and informational messages that occur during semantic analysis.
///
/// # Example
/// ```rust
/// let mut analyzer = Analyzer::new();
/// let mut ast = ...; // obtain or build the AST
/// let result = analyzer.analyze(&mut ast);
/// match result {
///     Ok(analyzer_result) => {
///         if analyzer_result.is_some() {
///             println!("Semantic analysis succeeded.");
///         } else {
///             println!("Semantic errors were found.");
///         }
///     }
///     Err(err) => eprintln!("Failed to analyze: {}", err),
/// }
/// ```
#[derive(Debug)]
pub struct Analyzer {
    /// Manages and tracks parsing and semantic diagnostics encountered during analysis.
    diagnostic_manager: DiagnosticManager,
}

impl Analyzer {
    /// Creates a new instance of `Analyzer`.
    ///
    /// Initializes a fresh `DiagnosticManager` to collect diagnostics during analysis.
    ///
    /// # Returns
    ///
    /// An `Analyzer` ready to perform semantic checks.
    ///
    /// # Example
    ///
    /// ```rust
    /// let analyzer = Analyzer::new();
    /// ```
    pub fn new() -> Self {
        Self {
            diagnostic_manager: DiagnosticManager::new(),
        }
    }

    /// Returns a reference to the internal `DiagnosticManager`.
    ///
    /// This allows inspection of collected diagnostics (errors, warnings, infos) after analysis.
    ///
    /// # Returns
    ///
    /// A reference to the `DiagnosticManager`.
    ///
    /// # Example
    ///
    /// ```rust
    /// let analyzer = Analyzer::new();
    /// let diagnostics = analyzer.diagnostic_manager();
    /// for diagnostic in diagnostics.diagnostics() {
    ///     println!("{}", diagnostic);
    /// }
    /// ```
    pub fn diagnostic_manager(&self) -> &DiagnosticManager {
        &self.diagnostic_manager
    }

    /// Performs semantic analysis on the AST contained within a `NormalizerResult`.
    ///
    /// This method consumes or borrows parts of the `NormalizerResult` (e.g., normalized AST,
    /// diagnostic manager, interner) as needed for efficient semantic analysis.
    ///
    /// # Parameters
    /// - `normalizer_result`: The result of logic containing the AST and diagnostics.
    ///
    /// # Returns
    ///
    /// On success, returns an `AnalyzerResult` encapsulating the semantic context and diagnostics.
    /// On failure, returns a `SemanticError`.
    ///
    /// # Note
    ///
    /// The AST inside the `NormalizerResult` may be mutated during analysis, and internal structures may be consumed.
    pub fn analyze(
        &mut self,
        normalizer_result: &mut NormalizerResult,
    ) -> Result<AnalyzerResult, SemanticError> {
        // Match on the normalized AST to decide how to continue.
        match normalizer_result.take_ast() {
            Some(mut ast) => {
                // Take diagnostics accumulated during logic.
                let diagnostic_manager = normalizer_result.take_diagnostic_manager();
                self.diagnostic_manager
                    .add_diagnostic_from(diagnostic_manager);

                // Analyze the normalized AST.
                let analysis_result = self.perform_analysis(&mut ast)?;

                // Return the final analysis result.
                Ok(analysis_result)
            }
            None => {
                // If no AST, propagate diagnostics and interner to AnalyzerResult.
                let diagnostic_manager = normalizer_result.take_diagnostic_manager();
                let interner = normalizer_result.take_interner();
                Ok(AnalyzerResult::failure(diagnostic_manager, interner))
            }
        }
    }

    /// Internal method to perform semantic analysis on the AST.
    ///
    /// Converts the AST into a `SemanticContext` and dispatches checks depending on
    /// whether the root node represents a `Domain` or a `Problem`.
    ///
    /// # Parameters
    /// - `ast`: Mutable reference to the AST.
    ///
    /// # Returns
    ///
    /// Returns an `AnalyzerResult` with the semantic context if no errors, or none if errors are present.
    ///
    /// # Errors
    ///
    /// Returns a `SemanticError` if the root node kind is not supported.
    fn perform_analysis(&mut self, ast: &mut Ast) -> Result<AnalyzerResult, SemanticError> {
        // Build semantic context from AST
        let mut context = SemanticContext::try_from(ast)?;

        // Determine root kind and run appropriate checks
        let root_ref = context.syntax_tree().try_root_node_ref()?;
        match root_ref.node().kind() {
            AstKind::Domain => {
                Self::check_domain(&mut context, &mut self.diagnostic_manager)?;
            }
            AstKind::Problem => {
                let check_ctx = CheckContext::from(&context);
                Self::check_problem(&check_ctx, &mut self.diagnostic_manager)?;
            }
            found => {
                return Err(SemanticError::unexpected_node_kind(
                    root_ref.id(),
                    vec![AstKind::Domain, AstKind::Problem],
                    found,
                ));
            }
        }

        // Build the AnalyzerResult based on presence of errors
        if !self
            .diagnostic_manager
            .has_diagnostics_of_severity(Severity::Error)
        {
            Ok(AnalyzerResult::success(
                context,
                std::mem::take(&mut self.diagnostic_manager),
            ))
        } else {
            Ok(AnalyzerResult::failure(
                std::mem::take(&mut self.diagnostic_manager),
                context.take_interner(),
            ))
        }
    }

    /// Checks the domain part of the syntax arena with domain-specific semantic validations.
    ///
    /// The checks include verifying symbol declarations, typing hierarchies, atomic formulas,
    /// typed logic, task ordering, and requirement violations.
    ///
    /// # Parameters
    /// - `context`: The `CheckContext` derived from the semantic context.
    /// - `diagnostic_manager`: Mutable reference to the `DiagnosticManager` to collect diagnostics.
    ///
    /// # Returns
    ///
    /// `Ok(true)` if all checks pass without errors, `Ok(false)` if any check fails,
    /// or `Err(SemanticError)` if an internal error occurs.
    fn check_domain(
        context: &mut SemanticContext, // On prend le SemanticContext pour avoir le .symbol_table_mut()
        diagnostic_manager: &mut DiagnosticManager,
    ) -> Result<bool, SemanticError> {
        // --- ÉTAPE 1 : CHECK BASE ---
        // On crée un CheckContext temporaire dans un bloc { }
        // pour qu'il libère son emprunt immuable à la fin du bloc.
        let mut checked = {
            let check_ctx = CheckContext::from(&*context);
            Self::check_domain_base(&check_ctx, diagnostic_manager)?
        };

        if checked {
            // --- ÉTAPE 2 : MODIFICATION (SIMPLIFICATION) ---
            // Ici, check_ctx est détruit, on peut donc accéder au symbol_table_mut()
            let table = context.symbol_table_mut();

            // On crée un Analyzer temporaire ou on appelle la méthode si elle est statique
            // Note: Si simplify_symbol_table_either_type est une méthode de self,
            // il faudra passer &self à check_domain.
            Self::simplify_symbol_table_either_type(table)?;

            // --- ÉTAPE 3 : CHECK ADVANCED ---
            // On recrée un CheckContext tout neuf qui voit la table modifiée
            let check_ctx = CheckContext::from(&*context);
            checked &= Self::check_domain_advanced(&check_ctx, diagnostic_manager)?;
        }

        Ok(checked)
    }

    fn check_domain_base(
        context: &CheckContext,
        diagnostic_manager: &mut DiagnosticManager,
    ) -> Result<bool, SemanticError> {
        let mut checked =
            Self::check_symbols(context, &[], &[SymbolKind::Constant], diagnostic_manager)?;

        // 2. NOUVEAU : On vérifie que TOUS les types utilisés existent
        // (pour les variables, constantes, etc.)
        checked &= semantic::checks::check_symbol_types(context, diagnostic_manager)?;

        checked &= semantic::checks::check_type_hierarchy(
            context,
            Provider::Analyzer,
            diagnostic_manager,
        )?;

        Ok(checked)
    }

    fn check_domain_advanced(
        context: &CheckContext,
        diagnostic_manager: &mut DiagnosticManager,
    ) -> Result<bool, SemanticError> {
        let type_checker = TypeChecker::new(context.symbol_table());
        let mut checked = true;

        checked &=
            semantic::checks::check_symbol_signatures(context, &type_checker, diagnostic_manager)?;
        checked &= semantic::checks::check_typed_expressions(
            context,
            &type_checker,
            Provider::Analyzer,
            diagnostic_manager,
        )?;
        checked &=
            semantic::checks::check_task_ordering(context, Provider::Analyzer, diagnostic_manager)?;

        semantic::checks::check_requirement_violations(
            context,
            Provider::Analyzer,
            diagnostic_manager,
        )?;

        Ok(checked)
    }

    /// Checks the problem part of the syntax arena with problem-specific semantic validations.
    ///
    /// It primarily checks symbols excluding some types and verifies task ordering.
    ///
    /// # Parameters
    /// - `context`: The `CheckContext` derived from the semantic context.
    /// - `diagnostic_manager`: Mutable reference to the `DiagnosticManager` to collect diagnostics.
    ///
    /// # Returns
    ///
    /// `Ok(true)` if checks pass, `Ok(false)` if errors are found,
    /// or `Err(SemanticError)` if internal errors occur.
    fn check_problem(
        context: &CheckContext,
        diagnostic_manager: &mut DiagnosticManager,
    ) -> Result<bool, SemanticError> {
        // Skip these kinds during undeclared symbol check in problems
        let skip_types_undeclared = &[
            SymbolKind::PrimitiveType,
            SymbolKind::Constant,
            SymbolKind::Predicate,
            SymbolKind::Function,
            SymbolKind::Task,
        ];

        let mut checked =
            Self::check_symbols(context, skip_types_undeclared, &[], diagnostic_manager)?;

        checked &=
            semantic::checks::check_task_ordering(context, Provider::Analyzer, diagnostic_manager)?;

        Ok(checked)
    }

    /// Performs general symbol checks: declared, undeclared, and unused symbols.
    ///
    /// This method verifies that:
    /// - All declared symbols are valid.
    /// - No undeclared symbols are used (except those specified to skip).
    /// - No symbols are unused (except those specified to skip).
    ///
    /// # Parameters
    /// - `context`: The `CheckContext` representing the semantic context.
    /// - `skip_types_undeclared`: Symbol kinds to ignore during undeclared symbol checking.
    /// - `skip_symbols_unused`: Symbol kinds to ignore during unused symbol checking.
    /// - `diagnostic_manager`: Mutable reference to the `DiagnosticManager` for diagnostics.
    ///
    /// # Returns
    ///
    /// `Ok(true)` if all checks succeed without errors,
    /// `Ok(false)` if some checks fail,
    /// or `Err(SemanticError)` if an internal error occurs.
    ///
    /// # Example
    /// ```rust
    /// let mut diagnostic_manager = DiagnosticManager::new();
    /// let result = Analyzer::check_symbols(&check_ctx, &[], &[], &mut diagnostic_manager);
    /// ```
    pub fn check_symbols(
        context: &CheckContext,
        skip_types_undeclared: &[SymbolKind],
        skip_symbols_unused: &[SymbolKind],
        diagnostic_manager: &mut DiagnosticManager,
    ) -> Result<bool, SemanticError> {
        let mut checked = true;

        // Verify declared symbols correctness
        checked &= semantic::checks::check_symbol_declarations(context, diagnostic_manager)?;

        // Check undeclared symbols, skipping specified types
        checked &= semantic::checks::check_undeclared_symbols(
            context,
            skip_types_undeclared,
            Provider::Analyzer,
            diagnostic_manager,
        )?;

        // Check for unused symbols, skipping specified symbols
        checked &= semantic::checks::check_unused_symbols(
            context,
            skip_symbols_unused,
            Provider::Analyzer,
            diagnostic_manager,
        )?;

        Ok(checked)
    }

    pub fn simplify_symbol_table_either_type(
        symbol_table: &mut SymbolTable,
    ) -> Result<(), TypeCheckError> {
        // Sous-fonction 1 : Collecte (Immuable) - Utilise uniquement des IDs
        let changes = Self::collect_type_simplifications(symbol_table)?;

        // Sous-fonction 2 : Application (Mutable) - Utilise take() pour éviter les clones
        if !changes.is_empty() {
            Self::apply_type_simplifications(symbol_table, changes);
        }

        Ok(())
    }

    fn collect_type_simplifications(
        symbol_table: &SymbolTable,
    ) -> Result<Vec<TypeSimplification>, TypeCheckError> {
        let checker = TypeChecker::new(symbol_table);
        let mut changes = Vec::new();

        for (&symbol_id, entry) in symbol_table.iter() {
            for decl in entry.declarations().iter() {
                if let Some(raw_ty) = decl.ty() {
                    if let Some(new_type) = checker.simplify_type(raw_ty)? {
                        // OPTIMISATION : On ne stocke que le NodeId (léger)
                        changes.push(TypeSimplification {
                            symbol_id,
                            node_id: decl.node_id(),
                            new_type,
                        });
                    }
                }
            }
        }
        Ok(changes)
    }

    fn apply_type_simplifications(
        symbol_table: &mut SymbolTable,
        changes: Vec<TypeSimplification>,
    ) {
        for change in changes {
            if let Some(entry) = symbol_table.get_symbol_mut(change.symbol_id) {
                // Étape 1 : Obtenir une clé de recherche sans bloquer le Borrow Checker
                // On clone uniquement le strict nécessaire pour identifier l'objet
                let target_node_id = change.node_id;

                // Étape 2 : Extraction et réinsertion
                // On cherche l'objet complet. On est obligé de passer par une ref temporaire.
                let old_decl_ref = entry
                    .declarations()
                    .iter()
                    .find(|d| d.node_id() == target_node_id);

                if let Some(r) = old_decl_ref {
                    // On clone la structure (très rapide car IDs = Copy)
                    // pour libérer l'emprunt immuable sur 'entry'
                    let key = r.clone();

                    // On extrait l'original, on modifie, on réinsère
                    if let Some(mut original) = entry.declarations_mut().take(&key) {
                        original.set_ty(change.new_type);
                        entry.declarations_mut().insert(original);
                    }
                }
            }
        }
    }
}

/// Représente une modification de type à appliquer sur une déclaration
struct TypeSimplification {
    symbol_id: SymbolId,
    node_id: NodeId,
    new_type: Type<SymbolId>,
}
