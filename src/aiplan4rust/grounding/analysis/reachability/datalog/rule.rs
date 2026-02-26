use std::fmt;
use crate::aiplan4rust::grounding::analysis::reachability::datalog::atom::Atom;

#[derive(Debug, Clone)]
pub struct Rule {
    head: Atom,      // La conclusion
    body: Vec<Atom>, // Les conditions (conjonction)
}

impl Rule {
    /// Crée une nouvelle règle Datalog.
    pub fn new(head: Atom, body: Vec<Atom>) -> Self {
        Self { head, body }
    }

    /// Retourne une référence à la conclusion (tête) de la règle.
    pub fn head(&self) -> &Atom {
        &self.head
    }

    /// Retourne une référence aux conditions (corps) de la règle.
    pub fn body(&self) -> &[Atom] {
        &self.body
    }
}

impl fmt::Display for Rule {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Affiche la tête (Conclusion)
        write!(f, "{}", self.head)?;

        // Si le corps est vide, c'est un fait pur, sinon on affiche le symbole :-
        if !self.body.is_empty() {
            write!(f, " :- ")?;
            for (i, atom) in self.body.iter().enumerate() {
                if i > 0 {
                    write!(f, ", ")?;
                }
                write!(f, "{}", atom)?;
            }
        }

        // Toutes les règles se terminent par un point en Datalog
        write!(f, ".")
    }
}
