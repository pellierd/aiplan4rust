use std::fmt;
use crate::aiplan4rust::grounding::iterator::DomainIteratorError;
use crate::aiplan4rust::lang::ObjectId;
use crate::aiplan4rust::grounding::value_domain::ValueDomain;

/// Un itérateur de combinaisons "lazy" conçu pour explorer des domaines de valeurs.
/// Il permet l'élagage (pruning) dynamique de branches entières du produit cartésien.
pub struct DomainIterator<'a> {
    /// Les domaines sources pour chaque paramètre.
    domains: Vec<&'a ValueDomain>,
    /// Indices actuels dans chaque domaine (la position du curseur).
    indices: Vec<usize>,
    /// Buffer interne pour exposer la combinaison actuelle sans allocation.
    current_combo: Vec<ObjectId>,
    /// Indique si toutes les combinaisons ont été parcourues.
    exhausted: bool,
    total_count: usize,
    is_first: bool,
}

impl<'a> DomainIterator<'a> {
    /// Initialise l'itérateur avec une liste de domaines.
    /// Pré-remplit la première combinaison pour permettre l'optimisation par "Partial Update".
    /// Initialise l'itérateur avec une liste de domaines.
    pub fn new(domains: Vec<&'a ValueDomain>) -> Result<Self, DomainIteratorError> {
        let arity = domains.len();

        // 1. Calcul sécurisé du nombre total de combinaisons
        let total_count = match arity {
            0 => 1,
            _ => domains
                .iter()
                .map(|d| d.cardinality())
                .try_fold(1usize, |acc, x| acc.checked_mul(x))
                .ok_or_else(|| DomainIteratorError::combinatorial_explosion(arity))?,
        };

        // 2. Vérification de vacuité
        let is_really_empty = arity > 0 && domains.iter().any(|d| d.is_empty());

        // IMPORTANT : Si total_count est 0, on est épuisé immédiatement.
        let exhausted = total_count == 0 || is_really_empty;

        // 3. Initialisation des buffers
        let indices = vec![0; arity];
        let current_combo = vec![ObjectId::default(); arity];

        // 4. Note : On ne pré-remplit plus current_combo ici !
        // C'est le premier appel à next() qui le fera grâce au flag `first`.

        Ok(Self {
            domains,
            indices,
            current_combo,
            exhausted,
            total_count,
            is_first: true, // On commence à true
        })
    }
    /// Retourne la combinaison actuelle et prépare la suivante.
    /// Version optimisée utilisant des itérateurs pour éviter les bounds checks.
    pub fn next(&mut self) -> Option<&[ObjectId]> {
        if self.exhausted {
            return None;
        }

        if self.is_first {
            // Au premier passage, on ne change pas les indices (ils sont déjà à 0)
            // Mais on doit remplir le buffer initial
            self.is_first = false;
            for i in 0..self.indices.len() {
                self.current_combo[i] = self.domains[i].get_argument(0);
            }
        } else {
            // Aux passages suivants, on avance et on fait l'update partiel
            let changed_idx = self.prepare_next()?;
            for i in changed_idx..self.indices.len() {
                self.current_combo[i] = self.domains[i].get_argument(self.indices[i]);
            }
        }

        Some(&self.current_combo)
    }

    /// Logique interne pour incrémenter les indices (ton ancien `advance`)
    fn prepare_next(&mut self) -> Option<usize> {
        if self.indices.is_empty() {
            self.exhausted = true;
            return None;
        }

        for i in (0..self.indices.len()).rev() {
            if self.indices[i] + 1 < self.domains[i].cardinality() {
                self.indices[i] += 1;
                // On retourne 'i' : tout ce qui est à gauche de 'i' est inchangé.
                return Some(i);
            }
            self.indices[i] = 0;
        }

        self.exhausted = true;
        None
    }

    /// ÉLAGAGE : Saute toutes les combinaisons futures pour les positions à droite de `pos`.
    ///
    /// Appeler `skip_at(1)` signifie : "Passe à la valeur suivante pour l'index 1,
    /// en ignorant toutes les possibilités restantes pour les index 2, 3, etc."
    pub fn skip_at(&mut self, pos: usize) {
        let arity = self.indices.len();
        if pos < arity {
            // 1. On sature les positions à droite pour forcer le saut au prochain tour.
            // On met card - 1 pour que le prochain prepare_next() incrémente l'index à 'pos'.
            for i in (pos + 1)..arity {
                self.indices[i] = self.domains[i].cardinality().saturating_sub(1);
            }

            // Note : On n'a pas besoin de modifier current_combo ici.
            // Le prochain appel à next() appellera prepare_next(), qui renverra Some(pos),
            // et la boucle de mise à jour partielle rafraîchira tout le nécessaire.
        }
    }

    /// Indique si l'itérateur a terminé son parcours.
    pub fn has_next(&self) -> bool {
        !self.exhausted
    }

    pub fn reset(&mut self) {
        // 1. Remise à zéro des indices
        self.indices.fill(0);

        // 2. On redevient "neuf" pour le prochain next()
        self.is_first = true;

        // 3. Calcul de l'état d'épuisement (identique au constructeur)
        let arity = self.indices.len();
        let is_really_empty = arity > 0 && self.domains.iter().any(|d| d.is_empty());
        self.exhausted = self.total_count == 0 || is_really_empty;

        // OPTIONNEL : On peut vider le buffer pour éviter de transporter des vieux IDs,
        // mais next() avec first=true écrasera tout de toute façon.
    }

    /// Accesseur pour obtenir le nombre total de combinaisons calculé au départ.
    pub fn total_count(&self) -> usize {
        self.total_count
    }

    /// Retourne le nombre de combinaisons qu'il reste à parcourir.
    ///
    /// Le calcul est en O(N) où N est l'arité, ce qui est instantané
    /// même pour des millions de combinaisons totales.
    pub fn remaining_count(&self) -> usize {
        if self.exhausted {
            return 0;
        }

        let mut remaining = 0usize;
        let mut multiplier = 1usize;

        for i in (0..self.indices.len()).rev() {
            let card = self.domains[i].cardinality();
            let current_idx = self.indices[i];

            let diff = card.saturating_sub(1).saturating_sub(current_idx);
            remaining = remaining.saturating_add(diff.saturating_mul(multiplier));

            // Sécurité ici : le multiplicateur ne doit pas paniquer
            multiplier = multiplier.saturating_mul(card);
        }

        remaining.saturating_add(1)
    }
}

impl<'a> fmt::Display for DomainIterator<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let current = self.total_count().saturating_sub(self.remaining_count());
        let total = self.total_count();

        // Calcul du pourcentage
        let progress = if total > 0 {
            (current as f64 / total as f64) * 100.0
        } else {
            100.0
        };

        write!(
            f,
            "DomainIterator [Progress: {}/{} ({:.2}%) | Arity: {}]",
            current,
            total,
            progress,
            self.indices.len()
        )
    }
}

#[cfg(test)]
mod tests {
    use crate::aiplan4rust::lang::ObjectFluentId;
    use super::*;

    /// Helper pour créer un domaine de test rapidement
    fn create_test_domain(num_objs: usize, num_fluents: usize) -> ValueDomain {
        let mut objs = Vec::new();
        for i in 0..num_objs {
            objs.push(ConstantId::from(i));
        }
        let mut fluents = Vec::new();
        for i in 0..num_fluents {
            fluents.push(ObjectFluentId::from(i));
        }
        ValueDomain::new(objs, fluents)
    }

    #[test]
    fn test_iterator_basic_product() {
        // D1: 2 objets, D2: 2 objets = 4 combinaisons
        let d1 = create_test_domain(2, 0);
        let d2 = create_test_domain(2, 0);
        let domains = vec![&d1, &d2];

        let mut it = DomainIterator::new(domains).expect("Should not overflow");

        assert_eq!(it.total_count(), 4);

        let mut count = 0;
        while let Some(combo) = it.next() {
            count += 1;
            // Vérification de l'arité du résultat
            assert_eq!(combo.len(), 2);
        }
        assert_eq!(count, 4);
        assert!(!it.has_next());
    }

    #[test]
    fn test_iterator_mixed_types() {
        // D1: 1 objet, 1 fluent = 2 éléments
        let d1 = create_test_domain(1, 1);
        let mut it = DomainIterator::new(vec![&d1]).unwrap();

        // 1er : Object(0)
        let res1 = it.next().unwrap();
        assert!(matches!(res1[0], ObjectId::Constant(_)));

        // 2eme : ObjectFluent(0)
        let res2 = it.next().unwrap();
        assert!(matches!(res2[0], ObjectId::Fluent(_)));

        assert!(it.next().is_none());
    }

    #[test]
    fn test_skip_at_logic() {
        // D1: {Obj0, Obj1}, D2: {Obj0, Obj1}
        let d1 = create_test_domain(2, 0);
        let d2 = create_test_domain(2, 0);
        let mut it = DomainIterator::new(vec![&d1, &d2]).unwrap();

        // On prend le premier : [Obj0, Obj0]
        it.next();

        // On skip à l'index 0.
        // L'itérateur doit passer à l'objet suivant de D1 et ignorer le reste de D2 pour Obj0.
        it.skip_at(0);

        let res = it.next().unwrap();
        // Doit être [Obj1, Obj0]
        if let ObjectId::Constant(id) = res[0] {
            assert_eq!(id, ConstantId::from(1));
        } else {
            panic!("Expected Object");
        }
    }

    #[test]
    fn test_reset_and_consistency() {
        let d = create_test_domain(3, 0);
        let mut it = DomainIterator::new(vec![&d]).unwrap();

        it.next();
        it.next();
        it.reset();

        assert_eq!(it.remaining_count(), 3);
        let res = it.next().unwrap();
        // Après reset, on doit revenir à l'index 0
        if let ObjectId::Constant(id) = res[0] {
            assert_eq!(id, ConstantId::from(0));
        }
    }

    #[test]
    fn test_overflow_protection() {
        // Simulation d'un domaine qui ferait exploser le produit
        // On crée un domaine de taille 1000
        let d = create_test_domain(1000, 0);
        // 1000^10 dépasse largement usize sur 64 bits
        let domains = vec![&d; 10];

        let result = DomainIterator::new(domains);
        assert!(result.is_err());
    }
}
