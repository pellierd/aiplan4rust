pub mod pnf;
pub mod qnf;
/// --- PIPELINE DE GROUNDING (Ordre de dépendance strict) ---
///
/// 1. `type_inference`:
///    Identifie les types de tous les paramètres et objets.
///    C'est la fondation indispensable pour la résolution des symboles.
///
/// 2. `typing`:
///    Aplatit la hiérarchie des types (ex: Robot < Mobile devient une liste directe).
///    Permet d'itérer sur les objets sans recalculer l'héritage à chaque fois.
///
/// 3. `object_fluent_flattening`:
///    Convertit les fonctions PDDL `(f ?x) = ?y` en prédicats `f(?x, ?y)`.
///    Le moteur Datalog ne manipule que des relations logiques pures.
///
/// 4. `constant_predicate_detection`:
///    Identifie les prédicats qui ne sont jamais modifiés par une action (Inertia).
///    Ces faits deviennent des constantes pour l'élagage précoce.
///
/// 5. `predicate_arity_reduction`:
///    Supprime les variables d'actions qui ne servent qu'à filtrer via des prédicats constants.
///    Réduit drastiquement la combinatoire du grounding.
///
/// 6. `quantifier_expansion`:
///    Déroule les `forall` et `exists` en conjonctions/disjonctions.
///    Transforme la logique du premier ordre en squelettes de règles Datalog.
///
/// 7. `action_pruning`:
///    Supprime les actions dont les préconditions sont évaluées à `FALSE` après expansion.
///    Évite de charger l'encodeur avec des branches mortes.
///
/// 8. `positive_form_normalization` (PNF):
///    Élimine les négations sur les fluents (ex: `(not at ?x)` -> `at_neg(?x)`).
///    Garantit un programme Datalog purement positif, évitant la stratification complexe.
pub use qnf::problem::expand;
pub use qnf::problem::expand_with;


pub use pnf::problem::to_pnf;
pub use pnf::problem::to_pnf_with;
