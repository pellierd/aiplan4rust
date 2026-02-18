use std::collections::{HashMap, HashSet};
use crate::aiplan4rust::grounding::error::GroundingError;
use crate::aiplan4rust::grounding::value_domain::ValueDomain;
use crate::aiplan4rust::lang::{ObjectId, Type, TypeId};
use crate::aiplan4rust::lir::problem::LiftedProblem;

pub struct ValueRegistry {
    /// Accès O(1) par index (TypeID). Stockage principal pour le grounding.
    pub type_domains: Vec<ValueDomain>,

    /// Vue par HashMap pour la flexibilité (Set de découverte pour éviter les doublons).
    pub discovered_by_type: HashMap<TypeId, HashSet<ObjectId>>,
}

impl ValueRegistry {
    pub fn new() -> Self {
        Self {
            type_domains: Vec::new(),
            discovered_by_type: HashMap::new(),
        }
    }

    /// Construit le registre initial à partir des objets définis dans le problème.
    pub fn from_problem(problem: &LiftedProblem) -> Result<Self, GroundingError> {
        let n = problem.type_defs().len();
        let tmp_objects = Self::collect_objects(problem);

        let mut type_domains = Vec::with_capacity(n);
        let mut discovered_by_type = HashMap::with_capacity(n);

        for (i, objs) in tmp_objects.into_iter().enumerate() {
            let ty_id = TypeId::from(i);

            // 1. On remplit le set de découverte (Source de vérité pour les futurs ajouts)
            let mut set = HashSet::with_capacity(objs.len());
            set.extend(objs.iter().copied());
            discovered_by_type.insert(ty_id, set);

            // 2. On crée le domaine immuable (Trié et Dédupliqué par le constructeur)
            type_domains.push(ValueDomain::new(objs));
        }

        Ok(Self {
            type_domains,
            discovered_by_type,
        })
    }

    /// Enregistre une constante pour un type complexe (multiples types primitifs).
    pub fn register_to_type(&mut self, arg: ObjectId, ty: &Type<TypeId>) -> bool {
        self.register(arg, ty.members())
    }

    /// Enregistre une constante dans une liste de types primitifs.
    /// Retourne `true` si la constante a été ajoutée à au moins un domaine.
    pub(crate) fn register(&mut self, arg: ObjectId, types: &[TypeId]) -> bool {
        let mut changed = false;

        for &ty_id in types {
            if self
                .discovered_by_type
                .entry(ty_id)
                .or_default()
                .insert(arg)
            {
                changed = true;
            }
        }

        changed
    }

    /// Récupère le domaine pour un type riche (ex: un paramètre d'action).
    pub fn get_domain_of_type(&self, ty: &Type<TypeId>) -> &ValueDomain {
        self.get_domain_of_primitive_type(ty.members()[0])
    }

    /// Accès direct au stockage indexé par TypeID.
    pub fn get_domain_of_primitive_type(&self, type_id: TypeId) -> &ValueDomain {
        &self.type_domains[type_id.as_usize()]
    }

    /// Synchronise les domaines vectoriels (`type_domains`) avec les sets (`discovered_by_type`).
    /// À appeler si `register` a renvoyé `true` pour reconstruire les domaines triés.
    pub fn sync(&mut self) {
        for (ty_id, set) in &self.discovered_by_type {
            let idx = ty_id.as_usize();

            // On reconstruit le vecteur à partir du set
            let objs: Vec<ObjectId> = set.iter().copied().collect();
            let new_domain = ValueDomain::new(objs);

            if idx < self.type_domains.len() {
                self.type_domains[idx] = new_domain;
            } else {
                // Cas de types créés dynamiquement (si applicable)
                self.type_domains.push(new_domain);
            }
        }
    }

    /// Collecte tous les objets statiques du problème classés par TypeID.
    fn collect_objects(problem: &LiftedProblem) -> Vec<Vec<ObjectId>> {
        let num_types = problem.type_defs().len();
        let mut tmp_objects = vec![Vec::new(); num_types];

        for typed_object in problem.object_defs() {
            let obj_id = typed_object.symbol();

            for &ty_id in typed_object.ty().members() {
                let idx = ty_id.as_usize();
                if idx < tmp_objects.len() {
                    tmp_objects[idx].push(obj_id);
                }
            }
        }

        tmp_objects
    }
}
