use std::collections::{HashMap, HashSet};
use crate::aiplan4rust::arena::ArenaNode;
use crate::aiplan4rust::grounding::analysis::inertia::registry::InertiaRegistry;
use crate::aiplan4rust::grounding::error::GroundingError;
use crate::aiplan4rust::grounding::iterator::DomainIterator;
use crate::aiplan4rust::grounding::object_fluent::ObjectFluent;
use crate::aiplan4rust::grounding::registry::fluent::FluentRegistry;
use crate::aiplan4rust::grounding::value_domain::ValueDomain;
use crate::aiplan4rust::lang::{ArgumentID, ObjectFluentID, ObjectID, Type, TypeID, TypedSymbol};
use crate::aiplan4rust::lir::expr::{Expr, ExprKind};
use crate::aiplan4rust::lir::problem::LiftedProblem;
use crate::aiplan4rust::tree::NodeId;

pub struct ValueRegistry {
    /// Accès O(1) par index (TypeID). C'est le stockage principal pour le grounding.
    pub type_domains: Vec<ValueDomain>,

    /// Vue par HashMap pour la flexibilité lors de l'analyse ou du debug.
    pub discovered_by_type: HashMap<TypeID, HashSet<ArgumentID>>,
}

impl ValueRegistry {

    pub fn new() -> Self {
        Self {
            type_domains: Vec::new(),
            discovered_by_type: HashMap::new()
        }
    }

    pub fn from_problem(
        problem: &LiftedProblem,
        fluent_reg: &mut FluentRegistry,
    ) -> Result<Self, GroundingError> {
        let n = problem.type_defs().len();

        let tmp_objects = Self::collect_objects(problem);
        let tmp_fluents = Self::collect_initial_fluents(problem, fluent_reg)?;

        let mut type_domains = Vec::with_capacity(n);
        let mut discovered_by_type = HashMap::with_capacity(n);

        for (i, (objs, flus)) in tmp_objects.into_iter().zip(tmp_fluents.into_iter()).enumerate() {
            let ty_id = TypeID::from(i);

            // 1. On remplit le set de découverte (Source de vérité)
            let mut set = HashSet::with_capacity(objs.len() + flus.len());
            set.extend(objs.iter().map(|&id| ArgumentID::Object(id)));
            set.extend(flus.iter().map(|&id| ArgumentID::ObjectFluent(id)));

            discovered_by_type.insert(ty_id, set);

            // 2. On crée le domaine immuable (Trié et Dédupliqué)
            // Note: Ici objs et flus sont consommés par le constructeur.
            type_domains.push(ValueDomain::new(objs, flus));
        }

        Ok(Self { type_domains, discovered_by_type })
    }

    /// Enregistre un argument pour un type complexe (contenant potentiellement plusieurs types primitifs).
    pub fn register_to_type(&mut self, arg: ArgumentID, ty: &Type<TypeID>) -> bool {
        // Un Type<TypeID> possède une liste de membres (IDs de types primitifs après flattening)
        self.register(arg, ty.members())
    }

    /// Enregistre un argument dans une liste de types primitifs.
    /// Retourne `true` si l'argument a été ajouté à au moins un domaine où il n'était pas présent.
    fn register(&mut self, arg: ArgumentID, types: &[TypeID]) -> bool {
        let mut changed = false;

        for &ty_id in types {
            // On insère uniquement dans le set.
            // L'argument ne sera visible dans type_domains qu'après le prochain sync().
            if self.discovered_by_type
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
    pub fn get_domain_of_type(&self, ty: &Type<TypeID>) -> &ValueDomain {
        // On délègue à la méthode primitive en utilisant l'ID du type
        self.get_domain_of_primitive_type(ty.members()[0])
    }

    /// Récupère le domaine pour un TypeID brut.
    /// On l'appelle "primitive" car elle accède directement au stockage indexé.
    /// Récupère le domaine pour un TypeID brut.
    /// Accès ultra-rapide par référence.
    pub fn get_domain_of_primitive_type(&self, type_id: TypeID) -> &ValueDomain {
        // On accède directement au vecteur.
        // On suppose que from_problem a bien dimensionné le vecteur.
        &self.type_domains[type_id.as_usize()]
    }


    /// Synchronise les domaines vectoriels avec les sets de découverte.
    /// À appeler entre chaque round de reachability si `register` a renvoyé `true`.
    pub fn sync(&mut self) {
        for (ty_id, set) in &self.discovered_by_type {
            let idx = ty_id.as_usize();

            // Si le domaine existe déjà, on vérifie s'il y a besoin de reconstruire
            // (Optionnel : on peut comparer set.len() avec domain.len() pour optimiser)

            let mut objs = Vec::new();
            let mut flus = Vec::new();

            for arg in set {
                match arg {
                    ArgumentID::Object(id) => objs.push(*id),
                    ArgumentID::ObjectFluent(id) => flus.push(*id),
                }
            }

            // On utilise le constructeur de ValueDomain qui gère le sort() et le dedup()
            let new_domain = ValueDomain::new(objs, flus);

            if idx < self.type_domains.len() {
                self.type_domains[idx] = new_domain;
            } else {
                // Sécurité pour les types dynamiques
                self.type_domains.push(new_domain);
            }
        }
    }

    /// Collecte tous les objets définis dans le problème et les classe par TypeID.
    /// Retourne un vecteur où l'index correspond au TypeID.as_usize().
    fn collect_objects(problem: &LiftedProblem) -> Vec<Vec<ObjectID>> {
        // On pré-alloue un vecteur de vecteurs pour chaque type existant dans le problème.
        let num_types = problem.type_defs().len();
        let mut tmp_objects = vec![Vec::new(); num_types];

        // On parcourt la liste des définitions d'objets (constantes du problème).
        for typed_object in problem.object_defs() {
            let obj_id = typed_object.symbol();

            // Un objet peut appartenir à plusieurs types primitifs après aplatissement
            // de la hiérarchie. On l'ajoute à chaque "panier" correspondant.
            for &ty_id in typed_object.ty().members() {
                let idx = ty_id.as_usize();

                // Sécurité : on vérifie que l'index est dans les bornes du vecteur.
                if idx < tmp_objects.len() {
                    tmp_objects[idx].push(obj_id);
                }
            }
        }

        tmp_objects
    }

    /// Parcourt l'état initial du problème pour identifier et interner
    /// tous les object-fluents (fonctions) qui y sont définis.
    fn collect_initial_fluents(
        problem: &LiftedProblem,
        fluent_reg: &mut FluentRegistry,
    ) -> Result<Vec<Vec<ObjectFluentID>>, GroundingError> {
        let mut tmp_fluents = vec![Vec::new(); problem.type_defs().len()];
        let init_expr = problem.init();

        for node in init_expr.preorder().values() {
            if let ExprKind::FComp = node.kind() {
                let fluent_node_id = node.try_child(0)?;
                let fluent_node = init_expr.try_node(fluent_node_id)?;

                // On utilise un debug_assert ici : si on est dans un FComp côté gauche,
                // la structure LIR doit garantir qu'il s'agit d'un FunctionTerm.
                debug_assert!(
                    fluent_node.kind() == ExprKind::FunctionTerm,
                    "Structure LIR invalide : le membre gauche d'un FComp doit être un FunctionTerm"
                );

                // Extraction et rangement
                if let Ok(fluent_id) = Self::extract_fluent_from_node(fluent_node_id, init_expr, problem, fluent_reg) {
                    let skeleton_id = fluent_node.try_function_skeleton()?;
                    let function_def = problem.try_get_function(skeleton_id)?;

                    for &ty_id in function_def.ty().members() {
                        let idx = ty_id.as_usize();
                        if idx < tmp_fluents.len() {
                            tmp_fluents[idx].push(fluent_id);
                        }
                    }
                }
            }
        }
        Ok(tmp_fluents)
    }


    fn extract_fluent_from_node(
        node_id: NodeId,
        expr: &Expr,
        problem: &LiftedProblem,
        fluent_reg: &mut FluentRegistry
    ) -> Result<ObjectFluentID, GroundingError> {
        let function_node = expr.try_node(node_id)?;

        // 1. On récupère les enfants du FunctionTerm.
        // Selon ta structure : children[0] = FunctorID, children[1..] = Arguments
        let children = function_node.children();

        // REMPLACEMENT : On utilise debug_assert! pour la performance en release.
        // On part du principe que l'invariant (au moins 1 enfant pour le FunctorID) est garanti.
        debug_assert!(
            !children.is_empty(),
            "Structure d'AST invalide : FunctionTerm doit avoir au moins un enfant (le FunctorID)"
        );

        // 2. Extraction du FunctorID (Premier fils)
        let functor_node = expr.try_node(children[0])?;
        let functor_id = functor_node.try_functor()?; // On suppose que .try_functor() existe sur Node

        // 3. Extraction des arguments (Fils restants)
        let mut args = Vec::with_capacity(children.len() - 1);
        for &child_id in &children[1..] {
            let child_node = expr.try_node(child_id)?;
            let obj_id = child_node.try_constant()?;
            args.push(obj_id);
        }

        // 4. Récupération du type de retour depuis la définition de la fonction
        // On utilise functor_id comme index pour trouver la définition dans le problème
        let function_def = problem.try_get_function(function_node.try_function_skeleton()?)?;
        let return_type_id = function_def.ty().members()[0];

        // 5. Création et Interning
        let fluent = ObjectFluent::new(
            functor_id,
            args,
            return_type_id
        );

        Ok(fluent_reg.intern_object_fluent(fluent))
    }
}

/*pub fn compute_reachability(
    problem: &LiftedProblem,
    registry: &InertiaRegistry,
) -> Result<ValueRegistry, GroundingError> {
    let mut store = ValueRegistry::new();

    // 1. Initialisation avec l'état initial (Objets constants + Fluents de l'init)
    initialize_store(&mut store, problem);

    let mut changed = true;
    while changed {
        changed = false;

        for action in problem.actions() {
            // On prépare les domaines basés UNIQUEMENT sur ce qui est "reachable"
            let param_domains: Vec<ValueDomain> = action.parameters()
                .iter()
                .map(|p| store.get_domain_of_type(p.ty()))
                .collect();

            // On utilise ton Iterator performant
            let mut it = DomainIterator::new(param_domains.iter().collect())?;

            while let Some(combo) = it.next() {
                // Filtre d'inertie : On n'explore que si c'est statiquement possible
                if registry.evaluate_predicate_with_env(action.precond(), combo)? != Some(false) {

                    // On récolte les nouveaux fluents créés par les effets de l'action
                    for (type_id, new_fluent) in action.collect_effects_fluents(combo) {
                        if store.register_to_type(type_id, new_fluent) {
                            changed = true; // Nouvelle découverte !
                        }
                    }
                }
            }
        }
    }
    Ok(store)
}*/
