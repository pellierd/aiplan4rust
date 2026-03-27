use crate::aiplan4rust::lang::SymbolId;
use crate::aiplan4rust::semantic::symbol::{Declaration, Filterable, SymbolKind};
use crate::aiplan4rust::semantic::symbol_table::table::Table;
use crate::aiplan4rust::semantic::symbol_table::SymbolTableError;
use crate::aiplan4rust::tree::NodeId;
use indexmap::IndexMap;

impl Table {
    /// Selects a valid declaration among candidates for a given symbol usage.
    ///
    /// Applies kind-specific rules to determine which declaration is valid and unambiguous.
    /// Used internally by `resolve_declaration`.
    ///
    /// # Parameters
    /// - `symbol_name`: The name of the symbol.
    /// - `usage_kind`: The kind of usage that triggered the lookup.
    /// - `declarations`: All candidate declarations matching the symbol, kind, and scope.
    ///
    /// # Returns
    /// - `Ok(Some(&Declaration))`: If one valid declaration is found.
    /// - `Ok(None)`: If no declaration is acceptable.
    /// - `Err`: If multiple valid declarations cause ambiguity.
    pub(super) fn apply_semantic_matching(
        symbol_name: &SymbolId,
        usage_kind: &SymbolKind,
        all_decls: &IndexMap<NodeId, Declaration>,
        indices: &[usize],
    ) -> Result<Option<usize>, SymbolTableError> {
        match usage_kind {
            SymbolKind::PrimitiveType | SymbolKind::Predicate => {
                Self::type_predicate_matching_rule(symbol_name, usage_kind, all_decls, indices)
            }
            SymbolKind::Task => Self::task_matching_rule(symbol_name, all_decls, indices),
            _ => match indices.len() {
                0 => Ok(None),
                1 => Ok(Some(indices[0])),
                _ => Err(SymbolTableError::duplicate_declaration(
                    *symbol_name,
                    *usage_kind,
                    indices.len(),
                )),
            },
        }
    }

    /// Validates declarations for types and predicates, allowing limited overlap.
    ///
    /// Allows exactly one matching declaration and optionally one compatible declaration (e.g.,
    /// Predicate + PrimitiveType).
    ///
    /// # Rules
    /// - Only one declaration with the correct kind is allowed.
    /// - One other compatible kind may exist, but not more.
    ///
    /// # Returns
    /// - `Ok(Some(&Declaration))`: If validation logic.
    /// - `Ok(None)`: If no matching declaration exists.
    /// - `Err`: If validation fails due to ambiguity or incompatible kinds.
    pub(super) fn type_predicate_matching_rule(
        symbol_name: &SymbolId,
        usage_kind: &SymbolKind,
        all_decls: &IndexMap<NodeId, Declaration>,
        indices: &[usize],
    ) -> Result<Option<usize>, SymbolTableError> {
        // 1. On compte sans allouer de Vec
        let matching_count = indices
            .iter()
            .filter(|&&idx| {
                all_decls
                    .get_index(idx)
                    .map(|(_, d)| d.kind() == *usage_kind)
                    .unwrap_or(false)
            })
            .count();

        match matching_count {
            0 => Ok(None),
            1 => {
                // On récupère l'index unique
                let matching_idx = *indices
                    .iter()
                    .find(|&&idx| {
                        all_decls
                            .get_index(idx)
                            .map(|(_, d)| d.kind() == *usage_kind)
                            .unwrap_or(false)
                    })
                    .unwrap();

                match indices.len() {
                    1 => Ok(Some(matching_idx)),
                    2 => {
                        // On cherche l'autre index pour vérifier la compatibilité
                        let other_idx = indices.iter().find(|&&i| i != matching_idx).unwrap();
                        let other_kind = all_decls.get_index(*other_idx).unwrap().1.kind();

                        //if Self::is_declaration_kind_compatible(usage_kind, &other_kind) {
                        if usage_kind.can_share_name_space_with(&other_kind) {
                            Ok(Some(matching_idx))
                        } else {
                            Err(SymbolTableError::duplicate_declaration(
                                *symbol_name,
                                *usage_kind,
                                2,
                            ))
                        }
                    }
                    _ => Err(SymbolTableError::duplicate_declaration(
                        *symbol_name,
                        *usage_kind,
                        indices.len(),
                    )),
                }
            }
            _ => Err(SymbolTableError::duplicate_declaration(
                *symbol_name,
                *usage_kind,
                matching_count,
            )),
        }
    }

    /// Validates task declarations, allowing exactly one declaration of kind `Task` or `Action`.
    ///
    /// If the declaration kind is incompatible, it will be ignored.
    ///
    /// # Parameters
    /// - `symbol_name`: The identifier of the symbol being validated.
    /// - `declarations`: A slice of references to declarations associated with the symbol.
    ///
    /// # Returns
    /// - `Ok(Some(&Declaration))` if exactly one valid declaration (`Task` or `Action`) is found.
    /// - `Ok(None)` if no valid declarations are found or if declarations are incompatible.
    /// - `Err(SymbolTableError)` if multiple declarations cause ambiguity.
    ///
    /// # Errors
    /// Returns an error if more than one declaration exists, indicating ambiguous task declarations.
    pub(super) fn task_matching_rule(
        symbol_name: &SymbolId,
        all_decls: &IndexMap<NodeId, Declaration>,
        indices: &[usize],
    ) -> Result<Option<usize>, SymbolTableError> {
        // Si nous avons plus d'une déclaration candidate, c'est une ambiguïté
        if indices.len() > 1 {
            return Err(SymbolTableError::duplicate_declaration(
                *symbol_name,
                SymbolKind::Task,
                indices.len(),
            ));
        }

        // On vérifie le premier (et seul) index s'il existe
        match indices.first() {
            Some(&idx) => {
                // On récupère la déclaration via son index
                let decl = all_decls.get_index(idx).map(|(_, d)| d).unwrap();

                match decl.symbol_kind() {
                    // Pour une Task, une déclaration de type Task ou Action est valide
                    SymbolKind::Task | SymbolKind::Action => Ok(Some(idx)),
                    // Sinon, ce n'est pas une déclaration valide pour cet usage
                    _ => Ok(None),
                }
            }
            // Aucun candidat trouvé
            None => Ok(None),
        }
    }
}
