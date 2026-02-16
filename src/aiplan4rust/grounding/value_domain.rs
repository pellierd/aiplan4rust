use serde::{Deserialize, Serialize};
use crate::aiplan4rust::lang::ids::{ObjectFluentID, ObjectID, ArgumentID};

/// Domaine de valeurs pour un type donné
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ValueDomain {
    objects: Vec<ObjectID>,             // objets constants
    object_fluents: Vec<ObjectFluentID>, // object-fluents générés a priori
}

impl ValueDomain {
    pub fn new(mut objects: Vec<ObjectID>, mut fluents: Vec<ObjectFluentID>) -> Self {
        objects.sort_unstable();
        objects.dedup();

        fluents.sort_unstable();
        fluents.dedup();

        Self { objects, object_fluents: fluents }
    }

    pub fn cardinality(&self) -> usize {
        self.objects.len() + self.object_fluents.len()
    }
    pub fn is_empty(&self) -> bool {
        self.objects.is_empty() && self.object_fluents.is_empty()
    }
    /*pub fn empty() -> Self {
        Self::new(Vec::new(), Vec::new())
    }*/
    /// Retourne une référence au vecteur d'objets constants
    pub fn objects(&self) ->  &[ObjectID] {
        &self.objects
    }

    /// Retourne une référence au vecteur d'object-fluents
    pub fn object_fluents(&self) -> &[ObjectFluentID] {
        &self.object_fluents
    }
    
    /// Accès direct par index pour l'itérateur performant
    pub fn get_argument(&self, index: usize) -> ArgumentID {
        let obj_len = self.objects.len();
        if index < obj_len {
            ArgumentID::Object(self.objects[index])
        } else {
            // On suppose que l'index est valide par rapport à total_cardinality()
            ArgumentID::ObjectFluent(self.object_fluents[index - obj_len])
        }
    }

    /// Iterateur simple sur tous les ParameterID
    pub fn iter(&self) -> impl Iterator<Item =ArgumentID> + '_ {
        let objects_iter = self.objects.iter().copied().map(ArgumentID::Object);
        let object_fluents_iter = self.object_fluents.iter().copied().map(ArgumentID::ObjectFluent);
        objects_iter.chain(object_fluents_iter)
    }
}

use std::fmt;

impl fmt::Display for ValueDomain {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Concatène objets et object-fluents en une seule liste
        let all_ids = self
            .objects
            .iter()
            .map(|id| id.to_string())
            .chain(self.object_fluents.iter().map(|id| id.to_string()))
            .collect::<Vec<_>>()
            .join(", ");

        write!(f, "{}", if all_ids.is_empty() { "<None>" } else { &all_ids })?;

        Ok(())
    }
}


/*/// Génération a priori des object-fluents avec filtrage simple
pub fn generate_object_fluents_map(
    lifted_functions: &Vec<Function>,
    domains: &HashMap<Ident, Vec<ObjectID>>,
    type_symbole_table: SymbolTable<TypeID>
) -> HashMap<ObjectFluentID, ObjectFluent> {
    let mut object_fluents_map = HashMap::new();

    for f in lifted_functions {
        // Pour chaque paramètre, récupère le domaine correspondant
        let param_domains: Vec<Vec<ObjectID>> = f.parameters().iter()
            .map(|ts| {
                let types = ts.types().members()[0]; // type types
                domains.get(&types)
                    .expect("ValueDomain manquant")
                    .clone() // copie des objets du type
            })
            .collect();

        // Produit cartésien de toutes les combinaisons de paramètres
        for combination in param_domains.into_iter().multi_cartesian_product() {
            // Ici tu peux filtrer si besoin, ex:
            // if !is_valid_combination(&combination) { continue; }

            // Crée ObjectFluent avec id unique
            let of_id = ObjectFluentID(object_fluents_map.len());
            let ty_ident = f.return_type().members()[0];
            object_fluents_map.insert(of_id, ObjectFluent::new(f.symbol(), combination, type_symbole_table.get_index(&ty_ident);
        }
    }

    object_fluents_map
}
*/
