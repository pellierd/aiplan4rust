
use crate::aiplan4rust::arena::ArenaNode;
use crate::aiplan4rust::grounding::engine::{GroundingEngine, Substitution};
use crate::aiplan4rust::grounding::error::GroundingError;
use crate::aiplan4rust::grounding::iterator::DomainIterator;
use crate::aiplan4rust::lang::{TypeId, TypedList, VariableId};
use crate::aiplan4rust::lir::expr::{Expr, ExprContent, ExprKind};
use crate::aiplan4rust::tree::NodeId;


pub fn expand(
    expr: &mut Expr,
    engine: &GroundingEngine,
) -> Result<(), GroundingError> {
    let mut global_change = false;

    // L'astuce : On ne collecte que les IDs des nœuds Forall et Exists
    // Mais attention : en post-ordre pour respecter l'imbrication !
    // 1. Collecte des IDs en post-ordre pour traiter les imbrications de l'intérieur vers l'extérieur
    let quantifier_ids: Vec<NodeId> = expr.postorder()
        .ids()
        .filter(|(_, node)| is_quantifier(node.kind()))
        .map(|(id, _)| id)
        .collect();

    if quantifier_ids.is_empty() {
        return Ok(());
    }

    for node_id in quantifier_ids {
        // 1. On récupère le nœud (sécurité arène)
        let node = match expr.try_node(node_id) {
            Ok(n) => n,
            Err(_) => continue,
        };

        // 2. On vérifie qu'il est TOUJOURS un quantificateur par securité
        // S'il a été simplifié par un enfant (en post-order), on le saute
        // Normalement pas absolument nécessaire
        if !is_quantifier(node.kind()) {
            continue;
        }

        if expand_quantified_expr(expr, node_id, engine)? {
            engine.logic_engine().simplify_from(expr, node_id)?;
            global_change = true;
        }
}

    if global_change {
        engine.logic_engine().simplify(expr)?;
    }

    Ok(())
}

/// Helper pour identifier les quantificateurs.
fn is_quantifier(kind: ExprKind) -> bool {
    matches!(kind, ExprKind::Forall | ExprKind::Exists)
}

pub fn expand_quantified_expr(
    expr: &mut Expr,
    node_id: NodeId,
    engine: &GroundingEngine,
) -> Result<bool, GroundingError> {
    // --- 1. EXTRACTION ---
    let (variables, body_id, is_forall) = {
        let node = expr.try_node(node_id)?;
        let vars = node.content().try_quantifier_vars()?.clone();
        let body = node.try_child(0)?;
        let forall = node.kind() == ExprKind::Forall;
        (vars, body, forall)
    };

    // --- 2. RÉCUPÉRATION DES DOMAINES ---
    let var_domains = engine.registry().get_variable_domains(&variables);
    let mut iterator = DomainIterator::new(var_domains)?;

    if handle_empty_domains(expr, node_id, &variables, is_forall, iterator.has_next())? {
        return Ok(true);
    }

    // --- 3. GÉNÉRATION ---
    let mut instances = Vec::new();
    let mut substitution = Substitution::with_capacity(variables.len());

    while let Some(combo) = iterator.next() {
        substitution.clear();
        for (i, &obj_id) in combo.iter().enumerate() {
            substitution.insert(variables[i].symbol(), obj_id);
        }

        // Appel de la fonction de clonage qui renvoie un NodeId (simplifié)
        let result_id = engine.instantiate_in_place(expr, body_id, &substitution)?;
        let body_node = expr.try_node(result_id)?;
        // --- DÉTECTION DES CONSTANTES VIA L'ARÈNE ---
        if body_node.is_empty_or() { // Représente FALSE
            if is_forall {
                // FORALL + un seul FALSE = FALSE GLOBAL
                expr.set_to_bool(node_id, false)?;
                return Ok(true);
            }
            // Si c'est un Exists, on ignore juste cette branche False
            continue;
        }

        if body_node.is_empty_and() { // Représente TRUE
            if !is_forall {
                // EXISTS + un seul TRUE = TRUE GLOBAL
                expr.set_to_bool(node_id, true)?;
                return Ok(true);
            }
            // Si c'est un Forall, on ignore cette branche True (neutre)
            continue;
        }

        // Si ce n'est pas une constante vide, c'est une instance dynamique
        instances.push(result_id);
    }

    // --- 4. FINALISATION ---
    if instances.is_empty() {
        // Si tout a été filtré (ex: Forall où tout est True), le résultat est la valeur neutre
        expr.set_to_bool(node_id, is_forall)?;
    } else {
        let new_kind = if is_forall { ExprKind::And } else { ExprKind::Or };
        let node_mut = expr.try_node_mut(node_id)?;
        node_mut.set_kind(new_kind);
        node_mut.set_content(ExprContent::None);
        *node_mut.children_mut() = instances;

        // Simplification finale du parent
        engine.logic_engine().simplify_from(expr, node_id)?;
    }

    Ok(true)
}



/// Gère les cas limites des domaines vides pour les quantificateurs.
///
/// Selon la logique du premier ordre :
/// - ∀x ∈ ∅, P(x) est TRUE (Vacuous Truth)
/// - ∃x ∈ ∅, P(x) est FALSE
fn handle_empty_domains(
    expr: &mut Expr,
    node_id: NodeId,
    variables: &TypedList<VariableId, TypeId>,
    is_forall: bool,
    has_next: bool,
) -> Result<bool, GroundingError> {
    // Si l'itérateur n'a pas de combinaisons alors que des variables sont définies,
    // c'est qu'au moins un des domaines est vide.
    if !has_next && !variables.is_empty() {
        // Pour Forall -> True, pour Exists -> False
        expr.set_to_bool(node_id, is_forall)?;
        return Ok(true); // Indique qu'un changement (élagage) a été fait
    }
    Ok(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aiplan4rust::lang::{VariableId, TypeId, ObjectId, TypedList, Type, TypedSymbol, PredicateSymbolId};
    use crate::aiplan4rust::grounding::engine::GroundingEngine;
    use crate::aiplan4rust::grounding::registry::value::ValueRegistry;
    use crate::aiplan4rust::lir::expr::{ExprBuilder, ExprKind};
    use crate::aiplan4rust::lir::logic::LogicEngine;

    #[test]
    fn test_expand_forall_quantifier() -> Result<(), Box<dyn std::error::Error>> {
        let mut builder = ExprBuilder::new();

        // 1. Setup identifiers
        let type_robot = TypeId::from(0);
        let var_x = VariableId::from(1); // ?x
        let obj_r1 = ObjectId::from(10);
        let obj_r2 = ObjectId::from(20);

        // 2. Setup the ValueRegistry with two objects for the 'robot' type
        let mut object_list = TypedList::new();
        object_list.push(TypedSymbol::new(obj_r1, Type::primitive(type_robot)));
        object_list.push(TypedSymbol::new(obj_r2, Type::primitive(type_robot)));

        let registry = ValueRegistry::new().with_typed_list(object_list);
        let logic_engine = LogicEngine::new();
        let engine = GroundingEngine::new(&registry, &logic_engine);

        // 3. Construct: (forall (?x - robot) (predicate_1 ?x))
        let mut typed_vars = TypedList::new();
        typed_vars.push(TypedSymbol::new(var_x, Type::primitive(type_robot)));

        let v_x = builder.variable(var_x);
        let atom = builder.atomic_formula(PredicateSymbolId::from(1), vec![v_x]);
        let forall_node = builder.forall(typed_vars, atom);

        builder.set_root(forall_node)?;
        let mut expr = builder.finish();

        // 4. Execute the expansion logic
        // This calls your `expand` function which iterates over quantifiers
        expand(&mut expr, &engine)?;

        // 5. Verification
        // The root was the Forall; it should now be an AND node
        let root_id = expr.try_root_id()?;
        let node = expr.try_node(root_id)?;

        // Ensure the Forall was converted to an AND
        assert_eq!(node.kind(), ExprKind::And, "Forall should be expanded to And");

        // Ensure we have 2 children (one for each robot)
        let children = node.children();
        assert_eq!(children.len(), 2, "Should have 2 grounded instances");

        // Verify the content of the instances (optional but recommended)
        // Each child should be an AtomicFormula with the correct ObjectId
        for &child_id in children {
            let child_node = expr.try_node(child_id)?;
            assert_eq!(child_node.kind(), ExprKind::AtomicFormula);
            // In a real test, you'd also check that the arguments are [10] and [20]
        }

        Ok(())
    }

    #[test]
    fn test_expand_forall_with_empty_domain_expansion() -> Result<(), Box<dyn std::error::Error>> {
        let mut builder = ExprBuilder::new();

        let type_vide = TypeId::from(0);
        let type_autre = TypeId::from(1); // Type auxiliaire pour forcer l'allocation
        let var_x = VariableId::from(1);

        let mut registry = ValueRegistry::new();

        // On crée une liste qui contient un objet dans 'type_autre'
        // mais rien dans 'type_vide'. Cela force `with_typed_list` à
        // allouer un vecteur de taille 2 (indices 0 et 1).
        let mut objects = TypedList::new();
        objects.push(TypedSymbol::new(ObjectId::from(999), Type::primitive(type_autre)));

        // Maintenant grouped n'est plus vide, max_id sera 1,
        // et type_domains sera [ValueDomain(vide), ValueDomain(999)]
        registry = registry.with_typed_list(objects);

        let logic_engine = LogicEngine::new();
        let engine = GroundingEngine::new(&registry, &logic_engine);

        let mut variables_typees = TypedList::new();
        variables_typees.push(TypedSymbol::new(var_x, Type::primitive(type_vide)));

        let v_x = builder.variable(var_x);
        let atome = builder.atomic_formula(PredicateSymbolId::from(1), vec![v_x]);
        let noeud_forall = builder.forall(variables_typees, atome);

        builder.set_root(noeud_forall)?;
        let mut expr = builder.finish();

        // L'exécution ne plantera plus car l'index 0 existe (même s'il est vide)
        expand(&mut expr, &engine)?;

        let root_id = expr.try_root_id()?;
        let node = expr.try_node(root_id)?;

        // Forall sur ensemble vide est toujours Vrai (And vide)
        assert!(node.is_empty_and(), "Un Forall vide devrait se simplifier en TRUE (And vide)");

        Ok(())
    }

    #[test]
    fn test_expand_nested_forall_with_binary_predicate() -> Result<(), Box<dyn std::error::Error>> {
        let mut builder = ExprBuilder::new();
        let mut registry = ValueRegistry::new();

        // 1. Définition des types et des domaines
        let type_u = TypeId::from(0);
        let type_other = TypeId::from(1);

        let obj_1 = ObjectId::from(1);
        let obj_2 = ObjectId::from(2);
        let obj_3 = ObjectId::from(3);

        let mut objects = TypedList::new();
        objects.push(TypedSymbol::new(obj_1, Type::primitive(type_u)));
        objects.push(TypedSymbol::new(obj_2, Type::primitive(type_u)));
        objects.push(TypedSymbol::new(obj_3, Type::primitive(type_other)));

        registry = registry.with_typed_list(objects);

        // 2. Variables et Prédicats
        let var_x = VariableId::from(10);
        let var_y = VariableId::from(11);
        let pred_p = PredicateSymbolId::from(50);

        let v_x = builder.variable(var_x);
        let v_y = builder.variable(var_y);
        let c_3 = builder.constant(obj_3);

        let atome_p = builder.atomic_formula(pred_p, vec![v_x, v_y]);
        let eq_x_3 = builder.equal(v_x, c_3);
        let and_body = builder.and(vec![atome_p, eq_x_3]);

        let mut vars_y = TypedList::new();
        vars_y.push(TypedSymbol::new(var_y, Type::primitive(type_u)));
        let forall_y = builder.forall(vars_y, and_body);

        let mut vars_x = TypedList::new();
        vars_x.push(TypedSymbol::new(var_x, Type::primitive(type_u)));
        let forall_x = builder.forall(vars_x, forall_y);

        builder.set_root(forall_x)?;
        let mut expr = builder.finish();

        // 3. Exécution du Grounding
        let logic_engine = LogicEngine::new();
        let engine = GroundingEngine::new(&registry, &logic_engine);
        expand(&mut expr, &engine)?;

        // 4. Analyse du résultat simplifié
        let root_id = expr.try_root_id()?;
        let node = expr.try_node(root_id)?;
        assert!(
            node.is_empty_or(),
            "L'expression devrait s'être simplifiée en un OR vide (FALSE). Actuellement : {:?}",
            node.kind()
        );

        Ok(())
    }

    #[test]
    fn test_expand_exists_simplification_to_true() -> Result<(), Box<dyn std::error::Error>> {
        let mut builder = ExprBuilder::new();

        // Initialisation directe
        let mut typed_objects = TypedList::new();
        typed_objects.push(TypedSymbol::new(ObjectId::from(1), Type::primitive(TypeId::from(0))));
        typed_objects.push(TypedSymbol::new(ObjectId::from(2), Type::primitive(TypeId::from(0))));

        let registry = ValueRegistry::new().with_typed_list(typed_objects);
        let logic_engine = LogicEngine::new();
        let engine = GroundingEngine::new(&registry, &logic_engine);

        let var_x = VariableId::from(1);
        let type_0 = TypeId::from(0);

        let v_x = builder.variable(var_x);
        let c_1 = builder.constant(ObjectId::from(1));
        let body = builder.equal(v_x, c_1); // x == 1

        let mut vars = TypedList::new();
        vars.push(TypedSymbol::new(var_x, Type::primitive(type_0)));
        let exists_node = builder.exists(vars, body);

        builder.set_root(exists_node)?;
        let mut expr = builder.finish();

        expand(&mut expr, &engine)?;

        let root = expr.try_node(expr.try_root_id()?)?;
        assert!(root.is_empty_and(), "L'existence d'une instance vraie doit rendre le EXISTS vrai");
        Ok(())
    }

    #[test]
    fn test_expand_forall_vacuous_truth() -> Result<(), Box<dyn std::error::Error>> {
        let mut builder = ExprBuilder::new();

        let type_vide = TypeId::from(0);
        let type_autre = TypeId::from(1); // Type auxiliaire pour forcer l'allocation
        let var_x = VariableId::from(1);

        // Initialisation avec un objet dans le type 1 pour que l'index 0 soit alloué
        let mut objects = TypedList::new();
        objects.push(TypedSymbol::new(ObjectId::from(999), Type::primitive(type_autre)));

        let registry = ValueRegistry::new().with_typed_list(objects);
        let logic_engine = LogicEngine::new();
        let engine = GroundingEngine::new(&registry, &logic_engine);

        let v_x = builder.variable(var_x);
        let atom = builder.atomic_formula(PredicateSymbolId::from(10), vec![v_x]);

        let mut vars = TypedList::new();
        // On itère sur le type_vide (index 0), qui est maintenant alloué mais vide
        vars.push(TypedSymbol::new(var_x, Type::primitive(type_vide)));
        let forall_node = builder.forall(vars, atom);

        builder.set_root(forall_node)?;
        let mut expr = builder.finish();

        expand(&mut expr, &engine)?;

        let root = expr.try_node(expr.try_root_id()?)?;
        assert!(root.is_empty_and(), "∀x ∈ ∅ est toujours vrai (And vide)");
        Ok(())
    }

    #[test]
    fn test_expand_exists_empty_domain() -> Result<(), Box<dyn std::error::Error>> {
        let mut builder = ExprBuilder::new();
        let type_vide = TypeId::from(0);
        let type_autre = TypeId::from(1);
        let var_x = VariableId::from(1);

        let mut objects = TypedList::new();
        objects.push(TypedSymbol::new(ObjectId::from(999), Type::primitive(type_autre)));

        let registry = ValueRegistry::new().with_typed_list(objects);
        let logic_engine = LogicEngine::new();
        let engine = GroundingEngine::new(&registry, &logic_engine);

        let mut variables_typees = TypedList::new();
        variables_typees.push(TypedSymbol::new(var_x, Type::primitive(type_vide)));

        let x = builder.variable(var_x);
        let atom = builder.atomic_formula(PredicateSymbolId::from(1), vec![x]);
        let exists_node = builder.exists(variables_typees, atom);
        builder.set_root(exists_node)?;
        let mut expr = builder.finish();

        expand(&mut expr, &engine)?;
        assert!(expr.try_node(expr.try_root_id()?)?.is_empty_or(), "∃x ∈ ∅ doit être FALSE");
        Ok(())
    }

    #[test]
    fn test_expand_nested_mixed_quantifiers() -> Result<(), Box<dyn std::error::Error>> {
        let mut builder = ExprBuilder::new();

        let mut typed_objects = TypedList::new();
        typed_objects.push(TypedSymbol::new(ObjectId::from(1), Type::primitive(TypeId::from(0))));
        typed_objects.push(TypedSymbol::new(ObjectId::from(2), Type::primitive(TypeId::from(0))));

        let registry = ValueRegistry::new().with_typed_list(typed_objects);
        let logic_engine = LogicEngine::new();
        let engine = GroundingEngine::new(&registry, &logic_engine);

        let var_x = VariableId::from(1);
        let var_y = VariableId::from(2);
        let type_0 = TypeId::from(0);

        let v_x = builder.variable(var_x);
        let v_y = builder.variable(var_y);
        let atom = builder.atomic_formula(PredicateSymbolId::from(50), vec![v_x, v_y]);

        let mut vars_y = TypedList::new();
        vars_y.push(TypedSymbol::new(var_y, Type::primitive(type_0)));
        let exists_y = builder.exists(vars_y, atom);

        let mut vars_x = TypedList::new();
        vars_x.push(TypedSymbol::new(var_x, Type::primitive(type_0)));
        let forall_x = builder.forall(vars_x, exists_y);

        builder.set_root(forall_x)?;
        let mut expr = builder.finish();

        expand(&mut expr, &engine)?;

        let root_node = expr.try_node(expr.try_root_id()?)?;
        assert_eq!(root_node.kind(), ExprKind::And);
        assert_eq!(root_node.children().len(), 2);

        for &or_id in root_node.children() {
            let or_node = expr.try_node(or_id)?;
            assert_eq!(or_node.kind(), ExprKind::Or);
            assert_eq!(or_node.children().len(), 2);
        }
        Ok(())
    }

    #[test]
    fn test_expand_forall_short_circuit() -> Result<(), Box<dyn std::error::Error>> {
        let mut builder = ExprBuilder::new();

        let mut typed_objects = TypedList::new();
        typed_objects.push(TypedSymbol::new(ObjectId::from(1), Type::primitive(TypeId::from(0))));
        typed_objects.push(TypedSymbol::new(ObjectId::from(2), Type::primitive(TypeId::from(0))));

        let registry = ValueRegistry::new().with_typed_list(typed_objects);
        let logic_engine = LogicEngine::new();
        let engine = GroundingEngine::new(&registry, &logic_engine);

        let var_x = VariableId::from(1);
        let type_0 = TypeId::from(0);

        let v_x = builder.variable(var_x);
        let p_x = builder.atomic_formula(PredicateSymbolId::from(1), vec![v_x]);
        let c1 = builder.constant(ObjectId::from(1));
        let c2 = builder.constant(ObjectId::from(2));
        let f_node = builder.equal(c1, c2); // 1 == 2 (Faux)
        let body = builder.and(vec![p_x, f_node]);

        let mut vars = TypedList::new();
        vars.push(TypedSymbol::new(var_x, Type::primitive(type_0)));
        let forall_node = builder.forall(vars, body);

        builder.set_root(forall_node)?;
        let mut expr = builder.finish();

        expand(&mut expr, &engine)?;

        let root = expr.try_node(expr.try_root_id()?)?;
        assert!(root.is_empty_or(), "Un Forall avec une instance Fausse doit être Faux");
        Ok(())
    }

    #[test]
    fn test_expand_multi_variable_quantifier() -> Result<(), Box<dyn std::error::Error>> {
        let mut builder = ExprBuilder::new();
        let type_u = TypeId::from(0);
        let mut objects = TypedList::new();
        objects.push(TypedSymbol::new(ObjectId::from(1), Type::primitive(type_u)));
        objects.push(TypedSymbol::new(ObjectId::from(2), Type::primitive(type_u)));

        let registry = ValueRegistry::new().with_typed_list(objects);
        let logic_engine = LogicEngine::new();
        let engine = GroundingEngine::new(&registry, &logic_engine);

        // Construct: (exists (?x ?y - type_u) (P ?x ?y))
        let mut vars = TypedList::new();
        vars.push(TypedSymbol::new(VariableId::from(1), Type::primitive(type_u)));
        vars.push(TypedSymbol::new(VariableId::from(2), Type::primitive(type_u)));

        let x = builder.variable(VariableId::from(1));
        let y =builder.variable(VariableId::from(2));
        let atom = builder.atomic_formula(PredicateSymbolId::from(1), vec![x, y]);

        let exists = builder.exists(vars, atom);
        builder.set_root(exists)?;
        let mut expr = builder.finish();

        expand(&mut expr, &engine)?;

        let root = expr.try_node(expr.try_root_id()?)?;
        assert_eq!(root.kind(), ExprKind::Or);
        // 2 objets, 2 variables -> 2^2 = 4 instances attendues
        assert_eq!(root.children().len(), 4, "Le produit cartésien des domaines n'est pas respecté");
        Ok(())
    }

    #[test]
    fn test_expand_nested_simplification_upward() -> Result<(), Box<dyn std::error::Error>> {
        let mut builder = ExprBuilder::new();
        let type_u = TypeId::from(0);

        let mut objects = TypedList::new();
        objects.push(TypedSymbol::new(ObjectId::from(1), Type::primitive(type_u)));

        let registry = ValueRegistry::new().with_typed_list(objects);
        let logic_engine = LogicEngine::new();
        let engine = GroundingEngine::new(&registry, &logic_engine);

        // Structure: (forall (?x) (exists (?y) (x == y)))
        // Pour x=1, il existe y=1 tel que 1==1 (True).
        // L'Exists devient True, donc le Forall devient True.
        let var_x = VariableId::from(1);
        let var_y = VariableId::from(2);

        let v_x = builder.variable(var_x);
        let v_y = builder.variable(var_y);
        let body = builder.equal(v_x, v_y);

        let mut vars_y = TypedList::new();
        vars_y.push(TypedSymbol::new(var_y, Type::primitive(type_u)));
        let exists_node = builder.exists(vars_y, body);

        let mut vars_x = TypedList::new();
        vars_x.push(TypedSymbol::new(var_x, Type::primitive(type_u)));
        let forall_node = builder.forall(vars_x, exists_node);

        builder.set_root(forall_node)?;
        let mut expr = builder.finish();

        expand(&mut expr, &engine)?;

        let root = expr.try_node(expr.try_root_id()?)?;
        // L'imbrication doit résulter en une simplification totale vers TRUE (And vide)
        assert!(root.is_empty_and(), "L'imbrication aurait dû se simplifier en TRUE (And vide)");
        Ok(())
    }

    #[test]
    fn test_expand_quantifier_already_simplified_by_child() -> Result<(), Box<dyn std::error::Error>> {
        let mut builder = ExprBuilder::new();

        let type_u = TypeId::from(0);
        let type_vide = TypeId::from(1);
        let type_autre = TypeId::from(2); // Pour forcer l'allocation jusqu'à l'index 2

        let mut objects = TypedList::new();
        objects.push(TypedSymbol::new(ObjectId::from(1), Type::primitive(type_u)));
        objects.push(TypedSymbol::new(ObjectId::from(999), Type::primitive(type_autre)));

        let registry = ValueRegistry::new().with_typed_list(objects);
        let logic_engine = LogicEngine::new();
        let engine = GroundingEngine::new(&registry, &logic_engine);

        // Structure: (forall (?x - type_u) (exists (?y - type_vide) P(x,y)))
        // L'Exists sur un domaine vide devient False.
        // Le Forall voit un enfant False et doit devenir False (court-circuit).

        let var_x = VariableId::from(1);
        let var_y = VariableId::from(2);

        let x = builder.variable(var_x);
        let y = builder.variable(var_y);

        let atom = builder.atomic_formula(PredicateSymbolId::from(10), vec![x, y]);

        let mut vars_y = TypedList::new();
        vars_y.push(TypedSymbol::new(var_y, Type::primitive(type_vide)));
        let exists_y = builder.exists(vars_y, atom);

        let mut vars_x = TypedList::new();
        vars_x.push(TypedSymbol::new(var_x, Type::primitive(type_u)));
        let forall_x = builder.forall(vars_x, exists_y);

        builder.set_root(forall_x)?;
        let mut expr = builder.finish();

        expand(&mut expr, &engine)?;

        let root = expr.try_node(expr.try_root_id()?)?;
        // Un Forall dont l'instance est False (car l'exists était vide) doit être False (Or vide)
        assert!(root.is_empty_or(), "Le Forall aurait dû être court-circuité en FALSE (Or vide)");
        Ok(())
    }
}
