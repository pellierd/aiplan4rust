use std::collections::HashMap;
use crate::aiplan4rust::lir::expr::{ops, Expr, ExprKind, ExprNode};
use crate::aiplan4rust::tree::NodeId;
use crate::aiplan4rust::grounding::binding::Bindings;
use crate::aiplan4rust::grounding::binding::BindingError;
use crate::aiplan4rust::grounding::binding::Bindable;
use crate::aiplan4rust::lir::expr::ops::StaticEvaluator;

/// Instancie une nouvelle Expr à partir d'un sous-arbre de la source.
/// Si la source n'a pas de racine, retourne une expression vide sans erreur.

pub fn apply(
    expr: &Expr,
    bindings: &Bindings,
) -> Result<Expr, BindingError> {
    let root_id = match expr.root_id() {
        Some(id) => id,
        None => return Ok(Expr::new()),
    };
    apply_with(expr, root_id, bindings, None)
}


pub fn apply_with(
    expr: &Expr,
    node_id: NodeId,
    bindings: &Bindings,
    evaluator: Option<&dyn StaticEvaluator>,
) -> Result<Expr, BindingError> {
    let mut target = Expr::new();
    let mut stack = vec![(node_id, false)];
    let mut id_map: HashMap<NodeId, NodeId> = HashMap::new();

    while let Some((old_id, processed)) = stack.pop() {
        let node = expr.try_node(old_id)?;
        let kind = node.kind();

        if !processed {
            if is_atomic_block(kind) {
                // 1. Import du sous-arbre de source vers la target locale
                let new_id = target.clone_subtree(old_id)?;

                // 2. Substitution
                target.apply(new_id, bindings)?;

                // 3. Simplification
                ops::simplify_subexpr_with(&mut target, new_id, evaluator)?;


                id_map.insert(old_id, new_id);
            } else {
                stack.push((old_id, true));
                for &child_id in node.children().iter().rev() {
                    stack.push((child_id, false));
                }
            }
        } else {
            // Reconstruction
            let old_children = expr.try_node(old_id)?.children();
            let new_children: Vec<NodeId> = old_children.iter()
                .map(|c| *id_map.get(c).expect("Enfant manquant"))
                .collect();

            let content = expr.try_node(old_id)?.content().clone();

            let new_id = target.alloc_with_children(
                ExprNode::new(kind, content, None),
                new_children
            );
            ops::simplify_subexpr_with(&mut target, new_id, evaluator)?;

            id_map.insert(old_id, new_id);
        }
    }

    // On définit la racine de la nouvelle arène avant de la rendre
    let final_root = *id_map.get(&node_id).unwrap();
    target.set_root_id(final_root)?;

    Ok(target)
}

pub fn apply_in_place(
    expr: &mut Expr,
    root_id: NodeId,
    substitution: &Bindings,
) -> Result<NodeId, BindingError> {
    apply_in_place_with(expr, root_id, substitution, None)
}


/// Grounde une expression en clonant le sous-arbre et en simplifiant au fur et à mesure.
/// Idéal pour instancier des effets ou des préconditions depuis un domaine "lifted".
pub fn apply_in_place_with(
    expr: &mut Expr,
    root_id: NodeId,
    substitution: &Bindings,
    evaluator: Option<&dyn StaticEvaluator>,
) -> Result<NodeId, BindingError> {
    // Pile de travail : (ID du noeud source, est_traité)
    // On utilise un parcours de type Post-Order (reconstruction des parents après les enfants)
    let mut stack = vec![(root_id, false)];
    let mut id_map: HashMap<NodeId, NodeId> = HashMap::new();

    while let Some((old_id, processed)) = stack.pop() {
        let node = expr.try_node(old_id)?;
        let kind = node.kind();

        if !processed {
            // --- 1. BLOCS ATOMIQUES ET FEUILLES ---
            // Si c'est une feuille ou un bloc contenant des arguments à grounder
            if is_atomic_block(kind) {
                // On clone tout le sous-arbre (arguments, etc.)
                let new_id = expr.clone_subtree(old_id)?;

                // On applique la binding via notre Trait (modifie new_id en place)
                expr.apply(new_id, substitution)?;

                // Simplification immédiate (Inertie / Évaluation statique)
                // Utilise le logic_engine interne
                ops::simplify_subexpr_with(expr, new_id, evaluator)?;


                id_map.insert(old_id, new_id);
            } else {
                // --- 2. NOEUDS COMPLEXES (Connecteurs logiques) ---
                // On marque le noeud comme "en attente de ses enfants"
                stack.push((old_id, true));

                // Empiler les enfants pour traitement
                let children = node.children().to_vec();
                for &child_id in children.iter().rev() {
                    stack.push((child_id, false));
                }
            }
        } else {
            // --- 3. RECONSTRUCTION (Remontée) ---
            // Ici, tous les enfants de old_id ont déjà été créés dans target
            let old_children = expr.try_node(old_id)?.children().to_vec();

            let new_children: Vec<NodeId> = old_children.iter()
                .map(|c| *id_map.get(c).expect("L'enfant doit avoir été traité"))
                .collect();

            // On alloue un nouveau noeud identique mais avec les nouveaux enfants
            let content = expr.try_node(old_id)?.content().clone();
            let new_id = expr.alloc_with_children(
                ExprNode::new(kind, content, None),
                new_children
            );

            // Simplification logique finale (ex: AND(True, True) -> True)
            ops::simplify_subexpr_with(expr, new_id, evaluator)?;
            id_map.insert(old_id, new_id);
        }
    }
    Ok(*id_map.get(&root_id).unwrap())
}

/// Définit la "frontière" : ce qui doit être cloné/substitué d'un bloc.
fn is_atomic_block(kind: ExprKind) -> bool {
    matches!(kind,
        ExprKind::AtomicFormula |
        ExprKind::Function  |
        ExprKind::Assignment        |
        ExprKind::Comparison         |
        ExprKind::Arithmetic     |
        ExprKind::Task          |
        ExprKind::Variable      |
        ExprKind::Object      |
        ExprKind::Number
    )
}

#[cfg(test)]
mod tests {
    use crate::aiplan4rust::lang::{VariableId, ObjectId, CompareOp};
    use crate::aiplan4rust::grounding::binding::{apply, Bindings};
    use crate::aiplan4rust::grounding::problem::registry::value::ValueRegistry;
    use crate::aiplan4rust::lir::expr::{ExprKind, ExprContent, ExprBuilder};

    #[test]
    fn test_grounding_engine_basic_substitution() -> Result<(), Box<dyn std::error::Error>> {
        // 1. Préparation de l'expression lifted : (at ?x)
        let mut builder = ExprBuilder::new();
        let var_x = VariableId::from(1);
        let obj_1 = ObjectId::from(100);

        let v_node = builder.variable(var_x);
        // On crée (at ?x). Supposons que le symbole de prédicat est 10.
        let root = builder.atomic_formula(10, vec![v_node]);
        builder.set_root(root)?;
        let mut expr = builder.finish();

        // 2. Préparation du moteur et de la binding
        let mut sub = Bindings::new();
        sub.insert(var_x, obj_1);

        let value_reg = ValueRegistry::new();

        // 3. Exécution du Grounding
        // On récupère explicitement le nouvel ID.
        // L'arène 'expr' contient maintenant l'ancien arbre ET le nouveau.
        let new_root = apply::apply_in_place(&mut expr, root, &sub)?;

        // 4. VALIDATIONS
        // On interroge le noeud retourné par la fonction
        let node = expr.try_node(new_root)?;
        assert_eq!(node.kind(), ExprKind::AtomicFormula);

        // Vérification de la binding de l'argument
        let children = node.children();
        // On récupère le premier argument (index 1 si le predicate_id est en 0, sinon index 0)
        // Adapte l'index selon ton implémentation de builder.atomic_formula
        let arg_id = children[1];
        let arg_node = expr.try_node(arg_id)?;

        if let ExprContent::Object(id) = arg_node.content() {
            assert_eq!(*id, obj_1, "L'ID de l'objet substitué est incorrect");
        } else {
            panic!("L'argument est resté une Variable ou n'est pas une constante. Content: {:?}", arg_node.content());
        }

        // 5. PREUVE DE L'OPTION A
        // La racine "officielle" de l'expression n'a pas dû bouger
        assert_eq!(expr.root_id(), Some(root), "La racine globale ne devrait pas changer avec ground_from");

        // Le nouveau noeud doit être différent de l'ancien (car c'est un clone)
        assert_ne!(new_root, root, "Le grounding devrait créer de nouveaux noeuds");

        Ok(())
    }

    #[test]
    fn test_grounding_engine_fcomp_recursive_substitution() -> Result<(), Box<dyn std::error::Error>> {
        let mut builder = ExprBuilder::new();
        let var_x = VariableId::from(1);
        let obj_truck = ObjectId::from(500);

        // 1. Construction : (= (vitesse ?x) 10.0)
        let var_node = builder.variable(var_x);
        let func_term = builder.function_term(10, vec![var_node]); // vitesse(?x)
        let num_node = builder.number(10.0);
        let root = builder.fcomp(CompareOp::Equal, func_term, num_node);

        // On définit explicitement la racine avant de finir le builder
        builder.set_root(root)?;
        let mut expr = builder.finish();

        // 2. Préparation du nouveau moteur de grounding
        let mut substitution = Bindings::new();
        substitution.insert(var_x, obj_truck);

        let value_reg = ValueRegistry::new();

        // 3. Exécution via ground_from (Option A : On récupère le nouvel ID)
        let new_root = apply::apply_in_place(&mut expr, root, &substitution)?;

        // 4. Validation récursive
        let fcomp_node = expr.try_node(new_root)?;
        assert_eq!(fcomp_node.kind(), ExprKind::Comparison);

        // Le premier enfant de FComp est le FunctionTerm : (vitesse ?x)
        let func_term_id = fcomp_node.children()[0];
        let func_term_node = expr.try_node(func_term_id)?;
        assert_eq!(func_term_node.kind(), ExprKind::Function);

        // Dans FunctionTerm, l'argument est à l'index 1 (si l'index 0 est le symbole de fonction)
        // Vérifie cet index selon ton implémentation de function_term
        let arg_id = func_term_node.children()[1];
        let arg_node = expr.try_node(arg_id)?;

        if let ExprContent::Object(id) = arg_node.content() {
            assert_eq!(*id, obj_truck, "L'objet substitué dans le terme fonctionnel est incorrect");
        } else {
            panic!("La variable ?x n'a pas été substituée par l'objet 500. Contenu actuel : {:?}", arg_node.content());
        }

        // Vérifier également que le deuxième membre de la comparaison (10.0) est toujours là
        let second_arg_id = fcomp_node.children()[1];
        let second_arg_node = expr.try_node(second_arg_id)?;
        if let ExprContent::Number(val) = second_arg_node.content() {
            assert_eq!(*val, 10.0);
        }

        Ok(())
    }

    #[test]
    // =========================================================================
    // DESCRIPTION DU TEST
    // -------------------------------------------------------------------------
    // OBJECTIF : Vérifier que la simplification logique se propage vers le haut
    //            pendant la phase de reconstruction (remontée de la pile).
    //
    // INPUT : Un connecteur AND avec deux enfants :
    //         1. (at ?x) -> Sera substitué par (at obj:100)
    //         2. (= 1.0 2.0) -> Comparaison mathématique statiquement FAUSSE.
    //
    // OUTPUT ATTENDU :
    //         Puisque AND(..., faux) est FAUX, le moteur de simplification
    //         doit réduire l'ID racine à un noeud représentant FALSE (EmptyOr).
    // =========================================================================
    fn test_grounding_engine_and_propagation_false_via_math() -> Result<(), Box<dyn std::error::Error>> {
        let mut builder = ExprBuilder::new();

        // 1. Construction : (And (at ?x) (= 1.0 2.0))
        let var_x = VariableId::from(1);
        let var_node = builder.variable(var_x);
        let at_node = builder.atomic_formula(1, vec![var_node]);

        // Une comparaison qui est toujours FAUSSE (1.0 == 2.0)
        let n1 = builder.number(1.0);
        let n2 = builder.number(2.0);
        let false_comp = builder.fcomp(CompareOp::Equal, n1, n2);

        let root = builder.and(vec![at_node, false_comp]);
        builder.set_root(root)?; // On fixe la racine
        let mut expr = builder.finish();

        // 2. Préparation du nouveau moteur
        let mut sub = Bindings::new();
        sub.insert(var_x, ObjectId::from(100));

        let value_reg = ValueRegistry::new();

        // 3. Appel du grounding
        // Rappel : ground_from parcourt les enfants, simplifie le (= 1 2) en False,
        // puis reconstruit le AND et le simplifie immédiatement.
        let new_root = apply::apply_in_place(&mut expr, root, &sub)?;

        // 4. Validation :
        let final_node = expr.try_node(new_root)?;

        // On vérifie que le résultat final est bien FALSE.
        // Dans ton arène, cela correspond à ExprKind::EmptyOr.
        assert!(
            final_node.is_empty_or(),
            "Le AND(at, 1==2) devrait être simplifié en FALSE (EmptyOr), mais c'est un {:?}",
            final_node.kind()
        );

        // Vérification bonus : si c'est simplifié en EmptyOr, il ne doit pas avoir d'enfants
        assert_eq!(final_node.children().len(), 0);

        Ok(())
    }

    #[test]
    fn test_grounding_engine_nested_logic_structure() -> Result<(), Box<dyn std::error::Error>> {
        // =========================================================================
        // DESCRIPTION DU TEST
        // -------------------------------------------------------------------------
        // OBJECTIF : Vérifier la reconstruction d'un arbre profond et la
        //            binding multiple à différents niveaux.
        //
        // INPUT : (Not (And (at_10 ?x) (at_11 ?y)))
        //         Substitution : { ?x -> 100, ?y -> 200 }
        //
        // OUTPUT ATTENDU : (Not (And (at_10 100) (at_11 200)))
        //         L'ordre des enfants du AND doit être préservé.
        // =========================================================================

        let mut builder = ExprBuilder::new();

        // 1. Construction de l'arbre original
        let x_id = VariableId::from(1);
        let y_id = VariableId::from(2);

        let var_x = builder.variable(x_id);
        let p_x = builder.atomic_formula(10, vec![var_x]); // Atome 1 (index 0 du AND)

        let var_y = builder.variable(y_id);
        let q_y = builder.atomic_formula(11, vec![var_y]); // Atome 2 (index 1 du AND)

        let and_node = builder.and(vec![p_x, q_y]);
        let root = builder.not(and_node);

        builder.set_root(root)?; // On définit la racine
        let mut expr = builder.finish();

        // 2. Préparation de la binding via ton nouveau type
        let mut sub = Bindings::new();
        sub.insert(x_id, ObjectId::from(100));
        sub.insert(y_id, ObjectId::from(200));

        // 3. Initialisation du moteur
        let value_reg = ValueRegistry::new();

        // 4. Exécution du grounding (Option A : on récupère le nouveau NodeId)
        let new_root = apply::apply_in_place(&mut expr, root, &sub)?;

        // 5. Validation de la structure

        // --- Niveau 0 : NOT ---
        let not_node = expr.try_node(new_root)?;
        assert_eq!(not_node.kind(), ExprKind::Not);
        assert_ne!(new_root, root, "Le root ID doit être différent (clone)");

        // --- Niveau 1 : AND ---
        let child_and_id = not_node.children()[0];
        let and_node = expr.try_node(child_and_id)?;
        assert_eq!(and_node.kind(), ExprKind::And);
        assert_eq!(and_node.children().len(), 2, "Le AND doit avoir exactement 2 enfants");

        // --- Niveau 2 : Atomes substitués ---
        let new_p_x_id = and_node.children()[0];
        let new_q_y_id = and_node.children()[1];

        // Vérification de P(?x -> 100)
        let p_node = expr.try_node(new_p_x_id)?;
        assert_eq!(p_node.kind(), ExprKind::AtomicFormula);

        // On vérifie l'argument (index 1 si predicate_id est à l'index 0)
        let p_args = p_node.children();
        let p_val_node = expr.try_node(p_args[1])?;

        if let ExprContent::Object(id) = p_val_node.content() {
            assert_eq!(*id, ObjectId::from(100), "L'atome P devrait avoir l'objet 100");
        } else {
            panic!("Le premier enfant du AND n'a pas été substitué en Constant");
        }

        // Vérification de Q(?y -> 200)
        let q_node = expr.try_node(new_q_y_id)?;
        assert_eq!(q_node.kind(), ExprKind::AtomicFormula);

        let q_args = q_node.children();
        let q_val_node = expr.try_node(q_args[1])?;

        if let ExprContent::Object(id) = q_val_node.content() {
            assert_eq!(*id, ObjectId::from(200), "L'atome Q devrait avoir l'objet 200");
        } else {
            panic!("Le second enfant du AND n'a pas été substitué en Constant");
        }

        Ok(())
    }

    #[test]
    fn test_grounding_engine_simplification() -> Result<(), Box<dyn std::error::Error>> {
        let mut builder = ExprBuilder::new();

        // 1. Construction : (Not (And (at ?x) (= 1.0 2.0)))
        let var_x = VariableId::from(1);
        let var_node = builder.variable(var_x);
        let at_node = builder.atomic_formula(10, vec![var_node]);

        let n1 = builder.number(1.0);
        let n2 = builder.number(2.0);
        let false_comp = builder.fcomp(CompareOp::Equal, n1, n2);

        let and_node = builder.and(vec![at_node, false_comp]);
        let root = builder.not(and_node);

        builder.set_root(root)?;
        let mut expr = builder.finish();

        // 2. Préparation du moteur et binding
        let mut sub = Bindings::new();
        sub.insert(var_x, ObjectId::from(100));

        let value_reg = ValueRegistry::new();

        // 3. Exécution
        // Processus attendu :
        // a) (= 1 2) -> False
        // b) And(at, False) -> False
        // c) Not(False) -> True (EmptyAnd)
        let new_root = apply::apply_in_place(&mut expr, root, &sub)?;

        // 4. Validation
        let final_node = expr.try_node(new_root)?;

        assert!(
            final_node.is_empty_and(),
            "L'expression (Not (And ... False)) aurait dû être simplifiée en TRUE (EmptyAnd), trouvé : {:?}",
            final_node.kind()
        );

        Ok(())
    }

    #[test]
    fn test_grounding_engine_orphan_variable_root() -> Result<(), Box<dyn std::error::Error>> {
        // =========================================================================
        // OBJECTIF : Vérifier que la fonction gère correctement une racine qui est
        //            directement une Variable.
        // =========================================================================

        let mut builder = ExprBuilder::new();
        let x_id = VariableId::from(1);

        // 1. Construction : La racine est juste la variable ?x
        let root = builder.variable(x_id);
        builder.set_root(root)?;
        let mut expr = builder.finish();

        // 2. Substitution : ?x -> 100
        let mut sub = Bindings::new();
        sub.insert(x_id, ObjectId::from(100));

        // 3. Moteur
        let value_reg = ValueRegistry::new();

        // 4. Appel
        let new_root = apply::apply_in_place(&mut expr, root, &sub)?;

        // 5. Validation
        let node = expr.try_node(new_root)?;

        // On vérifie que la variable a été clonée PUIS substituée en constante
        assert_ne!(new_root, root, "Le root ID doit être différent (clone)");

        if let ExprContent::Object(id) = node.content() {
            assert_eq!(*id, ObjectId::from(100), "La variable racine aurait dû devenir l'objet 100");
        } else {
            panic!("La racine devrait être une Constant après binding, mais c'est un {:?}", node.kind());
        }

        Ok(())
    }
    #[test]
    fn test_grounding_engine_partial_substitution() -> Result<(), Box<dyn std::error::Error>> {
        // =========================================================================
        // OBJECTIF : Vérifier qu'un atome avec plusieurs variables est
        //            partiellement substitué si seule une variable est dans la map.
        // =========================================================================

        let mut builder = ExprBuilder::new();
        let x_id = VariableId::from(1);
        let z_id = VariableId::from(99);

        // 1. Construction : (at ?x ?z)
        let var_x = builder.variable(x_id);
        let var_z = builder.variable(z_id);
        let root = builder.atomic_formula(10, vec![var_x, var_z]);
        builder.set_root(root)?;
        let mut expr = builder.finish();

        // 2. Substitution : Uniquement ?x
        let mut sub = Bindings::new();
        sub.insert(x_id, ObjectId::from(100));

        // 3. Moteur
        let value_reg = ValueRegistry::new();

        // 4. Appel
        let new_root = apply::apply_in_place(&mut expr, root, &sub)?;

        // 5. Validation
        let node = expr.try_node(new_root)?;
        let children = node.children();

        // L'argument 1 (?x) doit être devenu la Constant 100
        // Note: On utilise l'index 1 car l'index 0 est souvent réservé au symbole
        let arg_x_id = children[1];
        let arg_x_node = expr.try_node(arg_x_id)?;
        if let ExprContent::Object(id) = arg_x_node.content() {
            assert_eq!(*id, ObjectId::from(100), "La variable ?x aurait dû être remplacée par 100");
        } else {
            panic!("Le premier argument devrait être une Constant");
        }

        // L'argument 2 (?z) doit être RESTÉ une Variable 99
        let arg_z_id = children[2];
        let arg_z_node = expr.try_node(arg_z_id)?;
        if let ExprContent::Variable(id) = arg_z_node.content() {
            assert_eq!(*id, z_id, "La variable ?z ne devrait pas avoir bougé");
        } else {
            panic!("Le second argument devrait toujours être la Variable(99)");
        }

        assert_ne!(new_root, root, "Le root ID doit être différent car c'est un clone");

        Ok(())
    }

    #[test]
    fn test_grounding_engine_structural_independence() -> Result<(), Box<dyn std::error::Error>> {
        // =========================================================================
        // OBJECTIF : Vérifier l'indépendance structurelle totale entre l'original
        //            et le clone (pas de partage de NodeId).
        // =========================================================================

        let mut builder = ExprBuilder::new();

        // 1. Construction de l'original : (at ?x)
        let x_id = VariableId::from(1);
        let var_x = builder.variable(x_id);
        let root = builder.atomic_formula(10, vec![var_x]);
        builder.set_root(root)?;
        let mut expr = builder.finish();

        // Récupération de l'ID de l'enfant original (la variable)
        let old_child_id = expr.try_node(root)?.children()[1];

        // 2. Paramètres neutres (Substitution vide)
        let sub = Bindings::new();
        let value_reg = ValueRegistry::new();

        // 3. Appel de la fonction
        let new_root = apply::apply_in_place(&mut expr, root, &sub)?;

        // 4. Validation de l'indépendance structurelle
        assert_ne!(new_root, root, "La racine doit être un nouvel ID");

        let new_node = expr.try_node(new_root)?;
        let new_child_id = new_node.children()[1];

        // CRUCIAL : Même si le contenu est le même (?x), l'ID doit être différent
        assert_ne!(
            old_child_id,
            new_child_id,
            "L'enfant (Variable) doit avoir été cloné : il doit posséder un NodeId unique"
        );

        // Vérification que le contenu est identique malgré l'ID différent
        let old_child_node = expr.try_node(old_child_id)?;
        let new_child_node = expr.try_node(new_child_id)?;
        assert_eq!(
            old_child_node.content(),
            new_child_node.content(),
            "Le contenu du clone doit être identique à l'original (VariableId(1))"
        );

        Ok(())
    }
}
