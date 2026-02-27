use std::fmt;
use crate::aiplan4rust::grounding::analysis::reachability::datalog::term::Term;
use crate::aiplan4rust::lang::AtomSkeletonId;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub struct Atom {
    /// L'identifiant du squelette (signature typée : Nom + Types des paramètres).
    /// C'est la clé unique pour retrouver la Relation dans la Database.
    skeleton_id: AtomSkeletonId,
    /// Les arguments réels (Variables ou Constantes) pour cette instance.
    terms: Vec<Term>,
    negated: bool, // Nouveau champ
}

impl Atom {
    /// Constructeur utilisant l'ID du squelette.
    pub fn new(skeleton_id: AtomSkeletonId, terms: Vec<Term>) -> Self {
        Self {
            skeleton_id,
            terms,
            negated: false,
        }
    }

    // --- Getters ---
    /// Retourne une version négative de cet atome.
    /// Utilisé par le flattener lors de la rencontre d'un nœud Not.
    pub fn negated(&mut self) {
        self.negated = true;
    }

    pub fn is_negated(&self) -> bool {
        self.negated
    }

    /// Retourne l'ID du squelette (unique par signature).
    #[inline]
    pub fn skeleton_id(&self) -> AtomSkeletonId {
        self.skeleton_id
    }

    #[inline]
    pub fn terms(&self) -> &[Term] {
        &self.terms
    }

    /// Retourne l'arité (nombre de termes).
    #[inline]
    pub fn arity(&self) -> usize {
        self.terms.len()
    }
}

impl fmt::Display for Atom {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // On affiche "sk" pour "skeleton" afin de clarifier la nature de l'ID
        write!(f, "sk_{}(", self.skeleton_id)?;
        for (i, term) in self.terms.iter().enumerate() {
            if i > 0 {
                write!(f, ", ")?;
            }
            write!(f, "{}", term)?;
        }
        write!(f, ")")
    }
}
