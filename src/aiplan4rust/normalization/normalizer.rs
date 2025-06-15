use std::collections::HashSet;
use crate::aiplan4rust::diagnostic::{DiagnosticManager};
use crate::aiplan4rust::frontend::ParserInternalError;
use crate::aiplan4rust::normalization;
use crate::aiplan4rust::normalization::NormalizerResult;
use crate::aiplan4rust::syntax::ast::{Ast, AstNode};


/// Structure de normalisation qui transforme un AST en une forme standardisée.
pub struct Normalizer {
    diagnostic_manager: DiagnosticManager,
}

impl Normalizer {
    /// Crée un nouveau normalizer.
    pub fn new() -> Self {
        Self {
            diagnostic_manager: DiagnosticManager::new(),
        }
    }

    /// Normalise un AST en appliquant des transformations standard.
    ///
    /// # Arguments
    /// * `ast` - AST à normaliser.
    ///
    /// # Retour
    /// * `Ast` normalisé.
    // Prend la possession de l'AST, modifie et retourne NormalizerResult avec l'AST modifié + diagnostics
    // Appelée quand on veut accumuler dans le diagnostic interne du normalizer
    pub fn normalize(&mut self, ast: Ast) -> Result<NormalizerResult, ParserInternalError> {
        self.perform_normalization(ast)
    }

    // Appelée quand on veut fournir un diagnostic externe
    pub fn normalize_with_diagnostic_manager(
        &mut self,
        ast: Ast,
        diagnostic_manager: DiagnosticManager,
    ) -> Result<NormalizerResult, ParserInternalError> {
        self.diagnostic_manager = diagnostic_manager;
        self.perform_normalization(ast)
    }

    // Fonction interne privée partagée
    fn perform_normalization(
        &mut self,
        mut ast: Ast,
    ) -> Result<NormalizerResult, ParserInternalError> {
        normalization::normalize_typed_list(&mut ast)?;
        normalization::normalize_either_type(&mut ast, &mut self.diagnostic_manager)?;
        normalization::normalize_require_def(&mut ast, &mut self.diagnostic_manager)?;
        normalization::normalize_type_def(&mut ast, &mut self.diagnostic_manager)?;
        ast.assign_unique_ids(0);
        Ok(NormalizerResult::new(Some(ast), std::mem::take(&mut self.diagnostic_manager)))
    }
    /// Accès aux diagnostics produits lors de la normalisation.
    pub fn diagnostic_manager(&self) -> &DiagnosticManager {
        &self.diagnostic_manager
    }
}
