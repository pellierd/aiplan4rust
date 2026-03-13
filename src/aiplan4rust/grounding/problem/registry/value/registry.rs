use std::collections::HashMap;
use std::fmt;
use serde::{Deserialize, Serialize};
use crate::aiplan4rust::grounding::error::GroundingError;
use crate::aiplan4rust::lang::{ObjectId, Type, TypeId, TypedList, TypedSymbol, VariableId};

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ValueRegistry {
    /// Stockage contigu de tous les objets du problème, triés par la hiérarchie.
    all_values: Vec<ObjectId>,
    /// Index permettant de retrouver la plage [start, end[ pour chaque TypeId.
    ranges: Vec<TypeRange>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct TypeRange {
    pub start: usize,
    pub end: usize,
}

enum Frame { Enter(TypeId), Exit(TypeId, usize) }

impl ValueRegistry {



    pub fn empty() -> Self {
        Self {
            all_values: Vec::new(),
            ranges: Vec::new(),
        }
    }


    pub fn with_types(num_types: usize) -> Self {
        Self {
            all_values: Vec::new(),
            ranges: vec![TypeRange { start: 0, end: 0 }; num_types],
        }
    }


    fn dfs_flatten(
        u: TypeId,
        adj: &[Vec<TypeId>],
        direct_objects: &[Vec<ObjectId>], // Objets uniquement du type atomique
        all_values: &mut Vec<ObjectId>,
        ranges: &mut [TypeRange],
        current_idx: &mut usize,
    ) {
        let mut stack = vec![Frame::Enter(u)];

        while let Some(frame) = stack.pop() {
            match frame {
                Frame::Enter(u) => {
                    let u_idx = u.as_usize();
                    let start = *current_idx;

                    // 1. Ajouter les objets qui appartiennent DIRECTEMENT à ce type
                    for &obj in &direct_objects[u_idx] {
                        all_values.push(obj);
                        *current_idx += 1;
                    }

                    // 2. Prévoir la sortie pour fermer le range
                    stack.push(Frame::Exit(u, start));

                    // 3. Ajouter les enfants (sous-types)
                    for &v in adj[u_idx].iter().rev() {
                        stack.push(Frame::Enter(v));
                    }
                },
                Frame::Exit(u, start) => {
                    // L'intervalle de u englobe ses objets + ceux de tous ses descendants
                    ranges[u.as_usize()] = TypeRange { start, end: *current_idx };
                }
            }
        }
    }

    /// Prépare les structures intermédiaires pour la construction du registre de valeurs.
    ///
    /// Cette étape extrait la topologie de la hiérarchie des types et distribue les objets
    /// dans leurs types respectifs de manière isolée (sans héritage à ce stade).
    fn prepare_data(
        num_types: usize,
        type_defs: &[TypedSymbol<TypeId, TypeId>],
        object_defs: &[TypedSymbol<ObjectId, TypeId>],
    ) -> (Vec<Vec<TypeId>>, Vec<bool>, Vec<Vec<ObjectId>>) {
        let mut adj = vec![Vec::new(); num_types];
        let mut has_parent = vec![false; num_types];
        let mut direct_objects = vec![Vec::new(); num_types];

        // 1. Construction du graphe d'adjacence (Parents -> Enfants)
        // On parcourt les définitions de types. Chaque 'member' d'un type est ici
        // considéré comme son parent dans la hiérarchie.
        for def in type_defs {
            let child = def.symbol();
            for &parent in def.ty().members() {
                adj[parent.as_usize()].push(child);
                has_parent[child.as_usize()] = true;
            }
        }

        // 2. Distribution des objets
        // Grâce à la passe de flattening, chaque objet possède un type atomique unique.
        for obj_def in object_defs {
            let obj_id = obj_def.symbol();

            // Sécurité : Après flattening, un objet doit avoir au moins (et normalement un seul) type.
            if let Some(&t_id) = obj_def.ty().members().first() {
                direct_objects[t_id.as_usize()].push(obj_id);
            }
        }

        (adj, has_parent, direct_objects)
    }

    pub fn build(
        type_defs: &[TypedSymbol<TypeId, TypeId>],
        object_defs: &[TypedSymbol<ObjectId, TypeId>],
    ) -> Result<Self, GroundingError> {
        let num_types = type_defs.len();
        let (adj, has_parent, direct_objects) = Self::prepare_data(num_types, type_defs, object_defs);

        // 1. Identifier les types racines
        let mut roots: Vec<TypeId> = (0..num_types)
            .filter(|&i| !has_parent[i])
            .map(TypeId::from)
            .collect();
        roots.sort();

        // 2. Lancer la mise à plat
        let mut all_values = Vec::with_capacity(object_defs.len());
        let mut ranges = vec![TypeRange { start: 0, end: 0 }; num_types];
        let mut current_idx = 0;

        for root in roots {
            Self::dfs_flatten(root, &adj, &direct_objects, &mut all_values, &mut ranges, &mut current_idx);
        }

        Ok(Self { all_values, ranges })
    }


    /// Retrieves the domains corresponding to a list of typed variables.
    ///
    /// This is typically used to initialize iterators for quantifier expansion
    /// or action instantiation. Returns a vector of references to the internal
    /// [`ValueDomain`]s.
    /// Retrieves the domains corresponding to a list of typed variables.
    ///
    /// This is typically used to initialize iterators for quantifier expansion
    /// or action instantiation. Returns a vector of references to the internal
    /// [`ValueDomain`]s.
    pub fn get_variable_domains(&self, variables: &TypedList<VariableId, TypeId>) -> Vec<&[ObjectId]> {
        variables
            .iter()
            .map(|var| self.get_type_domain(var.ty()))
            .collect() // OK : Collecte des &[ObjectId] dans un Vec<&[ObjectId]>
    }

    /// Retrieves the value domain for a specific [`Type`].
    ///
    /// # Panics
    /// Panics if the provided type contains no primitive members or if the
    /// internal hierarchy is malformed.
    pub fn get_type_domain(&self, ty: &Type<TypeId>) ->  &[ObjectId] {
        self.get_primitive_type_domain(ty.members()[0])
    }

    /// Direct $O(1)$ access to a type domain via its [`TypeId`].
    ///
    /// # Panics
    /// Panics if the `type_id` is out of bounds for this evaluator.
    pub fn get_primitive_type_domain(&self, type_id: TypeId) -> &[ObjectId] {
        let r = &self.ranges[type_id.as_usize()];
        &self.all_values[r.start..r.end]
    }
}

#[cfg(test)]
impl  ValueRegistry {
    /*pub fn with_typed_list(mut self, objects: TypedList<ObjectId, TypeId>) -> Self {
        let mut grouped: HashMap<TypeId, Vec<ObjectId>> = HashMap::new();

        for ts in objects {
            for tid in ts.ty().members() {
                grouped.entry(*tid).or_default().push(ts.symbol());
            }
        }

        // --- MODIFICATION ICI ---
        // Au lieu de quitter si c'est vide, on regarde l'ID de type le plus élevé
        // que l'on veut supporter, ou on s'assure d'une taille minimale.
        let max_id = grouped.keys()
            .map(|&tid| usize::from(tid))
            .max()
            .unwrap_or(0); // Par défaut 0, donc le vecteur aura au moins une taille de 1

        // On initialise/agrandit le vecteur
        self.type_domains = vec![ValueDomain::new(Vec::new()); max_id + 1];

        for (tid, mut objs) in grouped {
            objs.sort_unstable();
            objs.dedup();
            self.type_domains[usize::from(tid)] = ValueDomain::new(objs);
        }

        self
    }*/


    /// Helper pour les tests unitaires : construit un registre à partir d'une liste d'objets.
    /// Note : Dans cette version de test, on considère que les types sont indépendants
    /// (pas de calcul de hiérarchie récursive, juste le mapping direct).
    pub fn from_objects<I>(objects: I) -> Self
    where
        I: IntoIterator<Item = TypedSymbol<ObjectId, TypeId>>
    {
        use std::collections::HashMap;

        let mut grouped: HashMap<TypeId, Vec<ObjectId>> = HashMap::new();
        let mut max_id = 0;

        // 1. Groupement initial
        for ts in objects {
            let obj_id = ts.symbol();
            for &tid in ts.ty().members() {
                let id_idx = tid.as_usize();
                if id_idx > max_id {
                    max_id = id_idx;
                }
                grouped.entry(tid).or_default().push(obj_id);
            }
        }

        // 2. Construction du stockage contigu
        let mut all_values = Vec::new();
        let mut ranges = vec![TypeRange { start: 0, end: 0 }; max_id + 1];

        // On trie les IDs de types pour que le stockage soit déterministe dans les tests
        let mut sorted_keys: Vec<_> = grouped.keys().cloned().collect();
        sorted_keys.sort();

        for tid in sorted_keys {
            let mut objs = grouped.remove(&tid).unwrap();
            objs.sort_unstable();
            objs.dedup();

            let start = all_values.len();
            all_values.extend(&objs);
            let end = all_values.len();

            ranges[tid.as_usize()] = TypeRange { start, end };
        }

        Self {
            all_values,
            ranges,
        }
    }
}

impl fmt::Display for ValueRegistry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "ValueRegistry ({} objects total):", self.all_values.len())?;

        writeln!(f, "--- Type Domains ---")?;
        for (type_idx, range) in self.ranges.iter().enumerate() {
            // On ne print que les types qui ont des domaines (start != end)
            // ou tous les types si on veut debugger la hiérarchie complète.
            if range.start != range.end {
                let domain = &self.all_values[range.start..range.end];
                writeln!(
                    f,
                    "  Type {:<3} => [start: {:<3}, end: {:<3}] | Values: {:?}",
                    type_idx, range.start, range.end, domain
                )?;
            } else {
                writeln!(f, "  Type {:<3} => Empty", type_idx)?;
            }
        }

        writeln!(f, "--- Raw Contiguous Storage ---")?;
        writeln!(f, "  {:?}", self.all_values)?;

        Ok(())
    }
}
