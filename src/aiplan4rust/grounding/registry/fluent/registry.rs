use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use crate::aiplan4rust::lang::ids::{FluentId, NumericFluentId};
use crate::aiplan4rust::grounding::problem::{Fluent, NumericFluent};
use crate::aiplan4rust::grounding::registry::fluent::error::FluentRegistryError;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FluentRegistry {
    // Fluents (Prédicats Ground)
    fluent_lookup: HashMap<Fluent, FluentId>,
    fluents: Vec<Fluent>,

    // Numeric Fluents (Fonctions numériques Ground)
    numeric_fluents_lookup: HashMap<NumericFluent, NumericFluentId>,
    numeric_fluents: Vec<NumericFluent>,
}

impl FluentRegistry {
    /// Crée un nouveau registre vide.
    pub fn new() -> Self {
        Self::default()
    }

    /// Interne un Fluent (Prédicat Ground) et retourne son ID unique.
    pub fn intern_fluent(&mut self, fluent: Fluent) -> FluentId {
        if let Some(&id) = self.fluent_lookup.get(&fluent) {
            return id;
        }

        let id = FluentId::from(self.fluents.len());
        self.fluent_lookup.insert(fluent.clone(), id);
        self.fluents.push(fluent);
        id
    }

    /// Interne un NumericFluent (Fonction numérique Ground) et retourne son ID unique.
    pub fn intern_numeric_fluent(&mut self, numeric_fluent: NumericFluent) -> NumericFluentId {
        if let Some(&id) = self.numeric_fluents_lookup.get(&numeric_fluent) {
            return id;
        }

        let id = NumericFluentId::from(self.numeric_fluents.len());
        self.numeric_fluents_lookup.insert(numeric_fluent.clone(), id);
        self.numeric_fluents.push(numeric_fluent);
        id
    }

    // --- Résolution des Fluents (Prédicats Ground) ---

    /// Résout un `FluentID` pour obtenir sa définition groundée.
    pub fn resolve_fluent(&self, id: FluentId) -> Option<&Fluent> {
        self.fluents.get(id.as_usize())
    }

    /// Tente de résoudre un `FluentID` ou retourne une erreur `FluentRegistryError`.
    pub fn try_resolve_fluent(&self, id: FluentId) -> Result<&Fluent, FluentRegistryError> {
        self.resolve_fluent(id)
            .ok_or_else(|| FluentRegistryError::invalid_fluent_id(id, self.fluents.len()))
    }

    // --- Résolution des Numeric Fluents ---

    /// Résout un `NumericFluentID` pour obtenir sa définition groundée.
    pub fn resolve_numeric_fluent(&self, id: NumericFluentId) -> Option<&NumericFluent> {
        self.numeric_fluents.get(id.as_usize())
    }

    /// Tente de résoudre un `NumericFluentID` ou retourne une erreur `FluentRegistryError`.
    pub fn try_resolve_numeric_fluent(&self, id: NumericFluentId) -> Result<&NumericFluent, FluentRegistryError> {
        self.resolve_numeric_fluent(id)
            .ok_or_else(|| FluentRegistryError::invalid_numeric_fluent_id(id, self.numeric_fluents.len()))
    }

    /// Consomme le registre pour ne retourner que les vecteurs de données.
    /// Utile pour libérer la mémoire des HashMaps après la phase de grounding.
    pub fn freeze(self) -> (Vec<Fluent>, Vec<NumericFluent>) {
        (self.fluents, self.numeric_fluents)
    }

    /// Retourne le nombre total de fluents enregistrés (tous types confondus).
    pub fn total_count(&self) -> usize {
        self.fluents.len() + self.numeric_fluents.len()
    }
}
