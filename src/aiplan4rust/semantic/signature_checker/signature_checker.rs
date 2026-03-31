use crate::aiplan4rust::semantic::checks::util::{match_signature, resolve_declaration};
use crate::aiplan4rust::semantic::checks::SemanticCheckError;
use crate::aiplan4rust::semantic::signature_checker::match_result::MatchResult;
use crate::aiplan4rust::semantic::symbol::{Declaration, Scope, SymbolKind, Usage};
use crate::aiplan4rust::semantic::{SemanticError, TypeChecker};
use crate::aiplan4rust::syntax::ast::AstNode;
use crate::aiplan4rust::tree::{NodeId, Tree};
use crate::SymbolTable;

pub struct SignatureChecker<'a> {
    local_table: &'a SymbolTable,
    ast: &'a Tree<AstNode>,
    type_checker: &'a TypeChecker<'a>,
    annex_table: Option<&'a SymbolTable>,
}

impl<'a> SignatureChecker<'a> {
    pub fn new(
        local_table: &'a SymbolTable,
        ast: &'a Tree<AstNode>,
        type_checker: &'a TypeChecker,
        annex_table: Option<&'a SymbolTable>,
    ) -> Self {
        Self {
            local_table,
            ast,
            type_checker,
            annex_table,
        }
    }

    pub fn match_declaration_with_usage(
        &self,
        declaration: &Declaration,
        usage: &Usage,
    ) -> Result<MatchResult, SemanticError> {
        // 1. Vérification de la signature (noms et arité)
        let Some(arguments) = match_signature(declaration, usage) else {
            return Ok(MatchResult::NoMatch);
        };

        // On initialise le résultat à Match (le plus haut niveau de succès)
        let mut current_global_result = MatchResult::Match;

        // 2. Validation du contenu des arguments
        for (index, &arg_id) in arguments.iter().enumerate() {
            let arg_node = self.tree.try_node(arg_id)?;

            // Résolution hiérarchique (Local -> Annex)
            let Some(arg_decl) = self.resolve_argument_declaration(arg_node, usage.scope())? else {
                return Ok(MatchResult::NoMatch);
            };

            // 3. Vérification de la compatibilité des types
            let match_res = self.match_argument(declaration, usage, arg_decl, arg_id, index)?;

            match match_res {
                MatchResult::NoMatch => {
                    // Échec critique : cette déclaration ne peut pas être la bonne
                    return Ok(MatchResult::NoMatch);
                }
                MatchResult::UpcastMatch { .. } => {
                    // On dégrade le résultat global en UpcastMatch si on était en Match parfait.
                    // On garde la première occurrence d'Upcast rencontrée.
                    if matches!(current_global_result, MatchResult::Match) {
                        current_global_result = match_res;
                    }
                }
                MatchResult::Match => {
                    // On continue, le résultat global reste inchangé
                }
            }
        }

        Ok(current_global_result)
    }

    /// Résout la déclaration d'un argument en cherchant d'abord dans le Problème (local),
    /// puis dans le Domaine (annexe) si nécessaire.
    fn resolve_argument_declaration(
        &self,
        argument_node: &AstNode,
        usage_scope: &Scope,
    ) -> Result<Option<&'a Declaration>, SemanticError> {
        let name = argument_node.try_ident()?;
        let kind = SymbolKind::try_from(argument_node.kind())?;

        // --- ÉTAPE 1 : Recherche dans la table LOCAL (le Problème) ---
        if let Some(entry) = self.local_table.get_symbol(name) {
            if let Some(decl) = resolve_declaration(entry, kind, usage_scope) {
                return Ok(Some(decl));
            }
        }

        // --- ÉTAPE 2 : Recherche dans la table ANNEXE (le Domaine) ---
        // On n'y va que si on a une table annexe ET que le local n'a rien donné
        if let Some(annex) = self.annex_table {
            if let Some(entry) = annex.get_symbol(name) {
                // IMPORTANT : Dans le domaine, on cherche dans le root_scope (portée globale)
                // car les constantes/types du domaine sont globaux pour le problème.
                if let Some(decl) = resolve_declaration(entry, kind, &annex.root_scope()) {
                    return Ok(Some(decl));
                }
            }
        }

        Ok(None)
    }

    /// Matches a specific argument in the declaration to its expected type_checker.
    ///
    /// This function verifies that the argument in the usage corresponds to the declaration,
    /// ensuring that types match correctly and the argument is within valid bounds.
    ///
    /// # Arguments
    ///
    /// * `declaration` - The declaration of the symbol.
    /// * `usage` - The usage of the symbol.
    /// * `symbol_declaration` - The already resolved declaration of the argument.
    /// * `context` - The check context.
    /// * `argument` - The AST node of the argument.
    /// * `index` - The index of the argument in the argument list.
    /// * `type_checker` - A type_checker checker to validate type_checker consistency.
    ///
    /// # Returns
    ///
    /// `Result<bool, ParserInternalError>`: Returns `Ok(true)` if the argument matches the expected
    /// declaration, or `Err` with a `ParserInternalError` if any validation error occurs.

    fn match_argument_base(
        &self,
        atom_decl: &Declaration, // La définition (ex: l'Action "marcher")
        arg_decl: &Declaration,  // L'argument réel (ex: la variable "V1")
        usage_scope: &Scope,
        index: usize,
    ) -> Result<bool, SemanticCheckError> {
        // 1. ty_expected (Type attendu)
        // On va chercher dans la LISTE des arguments de l'atome à la POSITION index
        let ty_expected = atom_decl
            .arguments()
            .and_then(|args| args.get(index))
            .map(|arg| arg.ty())
            .ok_or_else(|| {
                SemanticCheckError::argument_index_out_of_bounds(index, atom_decl.scope().clone())
            })?;

        // 2. ty_provided (Type fourni)
        // On prend le type DIRECT de l'argument (pas d'index ici !)
        let ty_provided = arg_decl.ty().ok_or_else(|| {
            SemanticCheckError::missing_symbol_types(arg_decl.symbol().id(), usage_scope.clone())
        })?;

        // 3. Vérification : est-ce que "V1" est bien du type attendu par "marcher" ?
        Ok(self
            .type_checker
            .is_any_subtype_of(ty_expected, ty_provided)?)
    }

    fn match_argument(
        &self, // Plus besoin de mutabilité ici
        atom_decl: &Declaration,
        usage: &Usage,
        arg_decl: &Declaration,
        arg_node_id: NodeId,
        index: usize,
    ) -> Result<MatchResult, SemanticCheckError> {
        // 1. On tente le match standard (Subtype)
        let is_subtype = self.match_argument_base(atom_decl, arg_decl, usage.scope(), index)?;

        if is_subtype {
            return Ok(MatchResult::Match);
        }

        // 2. Vérification de l'exception d'upcasting (uniquement pour les Tasks appelant des Actions)
        if usage.symbol().kind() == SymbolKind::Task
            && matches!(atom_decl.symbol().kind(), SymbolKind::Action)
        {
            // On récupère les types pour vérifier le super-type
            // Note: .unwrap() est sûr ici car match_argument_base a déjà validé l'existence
            let ty_expected = atom_decl.arguments().unwrap().get(index).unwrap().ty();
            let ty_provided = arg_decl.ty().unwrap();

            // Si l'attendu est un super-type du fourni (Upcasting)
            if self
                .type_checker
                .is_any_supertype_of(ty_expected, ty_provided)?
            {
                return Ok(MatchResult::UpcastMatch {
                    expected: ty_expected.clone(),
                    provided: ty_provided.clone(),
                    arg_decl: arg_decl.clone(),
                    arg_node_id,
                });
            }
        }

        // 3. Aucun des deux ne match
        Ok(MatchResult::NoMatch)
    }
}
