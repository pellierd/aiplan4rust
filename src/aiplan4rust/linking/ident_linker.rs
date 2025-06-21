use bimap::BiMap;
use crate::aiplan4rust::syntax::elements::Ident;
use crate::aiplan4rust::syntax::StringInterner;

#[derive(Debug, Clone)]
pub struct IdentLinker {
    map: BiMap<Ident, Ident>, // left = problem_ident, right = domain_ident
}

impl IdentLinker {
    /// Construct an IdentLinker by matching identifiers with the same string representation
    /// from two different StringInterner instances.
    ///
    /// Maps from problem Ident to domain Ident.
    pub fn new(
        domain_interner: &StringInterner,
        problem_interner: &StringInterner,
    ) -> Self {
        let mut map = BiMap::new();

        // Build a lookup table for problem interner: string -> Ident
        let mut problem_map = std::collections::HashMap::<&str, Ident>::new();
        for problem_ident in problem_interner.keys() {
            if let Some(s) = problem_interner.get_str(problem_ident) {
                problem_map.insert(s, problem_ident);
            }
        }

        // For each domain ident, try to find the matching problem ident by string
        for domain_ident in domain_interner.keys() {
            if let Some(s) = domain_interner.get_str(domain_ident) {
                if let Some(&problem_ident) = problem_map.get(s) {
                    map.insert(problem_ident, domain_ident);
                }
            }
        }

        IdentLinker { map }
    }
    /// Convert an Ident from problem context to domain context
    pub fn to_domain_ident(&self, problem_id: Ident) -> Option<Ident> {
        self.map.get_by_left(&problem_id).copied()
    }

    /// Convert an Ident from domain context to problem context
    pub fn to_problem_ident(&self, domain_id: Ident) -> Option<Ident> {
        self.map.get_by_right(&domain_id).copied()
    }

    /// Iterate over all mapped pairs (problem_ident, domain_ident)
    pub fn iter(&self) -> impl Iterator<Item = (Ident, Ident)> + '_ {
        self.map.iter().map(|(p, d)| (*p, *d))
    }
}
