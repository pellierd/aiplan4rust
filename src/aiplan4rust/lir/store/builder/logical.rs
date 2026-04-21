// store/builder/logical.rs
use crate::aiplan4rust::lir::store::builder::ExprBuilder;
use crate::aiplan4rust::lir::store::{ExprEntryKind, ExprId};
// OU plus simple si ton mod.rs fait bien le re-export :
// use super::builder::ExprBuilder;

impl<'a> ExprBuilder<'a> {
    /// Crée un nœud logique `AND` avec un ou plusieurs enfants.
    /// Grâce au Hash-Consing du store, les enfants seront triés et dédoublonnés.
    pub fn and(&mut self, children: &[ExprId]) -> ExprId {
        self.nary(ExprEntryKind::And, children)
    }

    /// Crée un nœud `AND` vide, représentant la constante "Vrai".
    pub fn empty_and(&mut self) -> ExprId {
        self.intern(ExprEntryKind::And, &[])
    }

    /// Crée un nœud logique `OR` avec un ou plusieurs enfants.
    pub fn or(&mut self, children: &[ExprId]) -> ExprId {
        self.nary(ExprEntryKind::Or, children)
    }

    /// Crée un nœud `OR` vide, représentant la constante "Faux".
    pub fn empty_or(&mut self) -> ExprId {
        self.intern(ExprEntryKind::Or, &[])
    }

    /// Remplace `nary`.
    /// C'est ici que le tri et le dedup de ton LIR s'activeront pour And/Or.
    /// Pour AND/OR : Simplification au vol (Smart Constructor)
    fn nary(&mut self, kind: ExprEntryKind, children: &[ExprId]) -> ExprId {
        // --- SETUP ---
        let (neutral, absorbing) = match kind {
            ExprEntryKind::And => (self.empty_and(), self.empty_or()),
            ExprEntryKind::Or => (self.empty_or(), self.empty_and()),
            // Pour les autres types (Not, etc.), on interne directement la slice
            _ => return self.intern(kind, children),
        };

        // --- PASS 1 : Tout en un (Flattening + Absorption + Détection When) ---
        // On alloue le Vec de travail ici.
        // Si children est vide, flattened sera vide et on retournera neutral au Pass 3.
        let mut flattened = Vec::with_capacity(children.len());
        let mut has_when = false;

        for &child in children {
            // On déférence l'ID de la slice
            if child == absorbing {
                return absorbing;
            }
            if child == neutral {
                continue;
            }

            if let Some(node) = self.get(child) {
                let child_kind = node.kind();
                if child_kind == &kind {
                    // node.children() retourne déjà une slice, donc extend est très rapide
                    flattened.extend(node.children());
                } else {
                    if kind == ExprEntryKind::And && child_kind == &ExprEntryKind::When {
                        has_when = true;
                    }
                    flattened.push(child);
                }
            }
        }

        // --- PASS 2 : Canonisation ---
        flattened.sort_unstable();
        flattened.dedup();

        // --- PASS 3 : Simplifications de base ---
        if flattened.is_empty() {
            return neutral;
        }
        if flattened.len() == 1 {
            return flattened[0];
        }
        if self.has_complementary_pair(&flattened) {
            return absorbing;
        }

        // --- PASS 4 : Finalisation intelligente ---
        match kind {
            // finalize_and_with_merge et intern devront idéalement
            // accepter soit une slice, soit consommer ce Vec.
            ExprEntryKind::And if has_when => self.finalize_and_with_merge(flattened),
            _ => self.intern(kind, &flattened), // On passe la référence du Vec de travail
        }
    }

    fn finalize_and_with_merge(&mut self, cleaned_children: Vec<ExprId>) -> ExprId {
        // On pré-alloue pour éviter les reallocations
        let mut non_when = Vec::with_capacity(cleaned_children.len());
        let mut groups: Vec<(ExprId, Vec<ExprId>)> = Vec::new();

        // 1. Extraction et groupement
        for &id in &cleaned_children {
            if let Some(node) = self.get(id) {
                if node.kind() == &ExprEntryKind::When {
                    let cond = node.children()[0];
                    let eff = node.children()[1];

                    // Optimisation : recherche du groupe par effet
                    if let Some(group) = groups.iter_mut().find(|g| g.0 == eff) {
                        group.1.push(cond);
                    } else {
                        groups.push((eff, vec![cond]));
                    }
                    continue;
                }
            }
            non_when.push(id);
        }

        let mut fusion_done = false;

        // 2. Reconstruction des 'When' fusionnés
        for (eff, conds) in groups {
            let final_cond = if conds.len() > 1 {
                fusion_done = true;
                // self.or accepte maintenant &[ExprId]
                self.or(&conds)
            } else {
                conds[0]
            };
            non_when.push(self.when(final_cond, eff));
        }

        // 3. Finalisation
        if !fusion_done && non_when.len() == cleaned_children.len() {
            // CAS OPTIMAL : Rien n'a changé.
            // On passe cleaned_children par référence à intern.
            // Comme intern accepte &[ExprId], il n'y a pas de nouvelle copie ici.
            return self.intern(ExprEntryKind::And, &cleaned_children);
        }

        // CAS DE FUSION : On a créé de nouveaux nœuds, on relance une normalisation.
        // On passe la slice du nouveau vecteur de travail.
        self.nary(ExprEntryKind::And, &non_when)
    }

    /// Helper partagé pour détecter (P et ¬P) ou (P ou ¬P)
    fn has_complementary_pair(&self, sorted_ids: &[ExprId]) -> bool {
        // Comme c'est trié, on pourrait optimiser, mais un HashSet
        // ou une recherche linéaire suffit pour les tailles usuelles.
        let mut atoms = std::collections::HashSet::new();
        let mut negated_atoms = std::collections::HashSet::new();

        for &id in sorted_ids {
            if let Some(node) = self.get(id) {
                if node.kind() == &ExprEntryKind::Not {
                    let inner = node.children()[0];
                    if atoms.contains(&inner) {
                        return true;
                    }
                    negated_atoms.insert(inner);
                } else {
                    if negated_atoms.contains(&id) {
                        return true;
                    }
                    atoms.insert(id);
                }
            }
        }
        false
    }

    /// Crée un nœud logique `NOT` (négation).
    /// Intelligence Unaire : Double négation
    pub fn not(&mut self, expr: ExprId) -> ExprId {
        // 1. Constantes logiques (¬True -> False, ¬False -> True)
        // On compare les IDs avec les valeurs canoniques du Store
        if expr == self.empty_and() {
            return self.empty_or();
        }
        if expr == self.empty_or() {
            return self.empty_and();
        }

        // 2. Double négation ( !!A -> A )
        if let Some(node) = self.get(expr) {
            if node.kind() == &ExprEntryKind::Not {
                // Un 'Not' a toujours un seul enfant par construction
                return node.children()[0];
            }
        }

        // 3. Sinon, création/récupération du nœud Not normal
        self.intern(ExprEntryKind::Not, &[expr])
    }

    /// Crée un nœud `Imply` (A → B).
    /// Note : L'implication est binaire et n'est pas triée par le store (non-commutative).
    pub fn imply(&mut self, antecedent: ExprId, consequent: ExprId) -> ExprId {
        // Règle : A => B  est équivalent à  (!A || B)

        // 1. On crée la négation de l'antécédent.
        // Si antecedent est 'True', self.not() renverra 'False' immédiatement.
        let not_a = self.not(antecedent);

        // 2. On crée le OR.
        // Si not_a est 'True' (donc A était 'False'), self.or() renverra 'True'.
        // Si consequent est 'True', self.or() renverra 'True'.
        // Si not_a est 'False' (donc A était 'True'), self.or() renverra simplement 'consequent'.
        self.or(&[not_a, consequent])
    }

    pub fn when(&mut self, cond: ExprId, eff: ExprId) -> ExprId {
        // --- 1. CONDITION TOUJOURS VRAIE (when (and) E) -> E ---
        if cond == self.empty_and() {
            return eff;
        }

        // --- 2. CONDITION TOUJOURS FAUSSE (when (or) E) -> (and) ---
        if cond == self.empty_or() {
            return self.empty_and();
        }

        // --- 3. EFFET VIDE (when C (and)) -> (and) ---
        if eff == self.empty_and() {
            return self.empty_and();
        }

        // --- 4. IDENTITÉ (when E E) -> (and) ---
        if cond == eff {
            return self.empty_and();
        }

        // --- 6. INTERNEMENT ---
        self.intern(ExprEntryKind::When, &[cond, eff])
    }
}
