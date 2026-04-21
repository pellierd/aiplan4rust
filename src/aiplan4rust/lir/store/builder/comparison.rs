use crate::aiplan4rust::lang::CompareOp;
use crate::aiplan4rust::lir::store::builder::ExprBuilder;
use crate::aiplan4rust::lir::store::{ExprEntryKind, ExprId};

impl<'a> ExprBuilder<'a> {
    /// Crée une comparaison fonctionnelle (`FComp`) : (op left right)
    /// L'opérateur fait partie du Kind pour une identification structurelle rapide.
    pub fn comparison(&mut self, op: CompareOp, mut left: ExprId, mut right: ExprId) -> ExprId {
        // --- 1. NORMALISATION (Canonicité des opérateurs) ---
        // On transforme systématiquement > en < et >= en <= pour réduire le nombre de cas
        let final_op = match op {
            CompareOp::Greater => {
                std::mem::swap(&mut left, &mut right);
                CompareOp::Less
            }
            CompareOp::GreaterEq => {
                std::mem::swap(&mut left, &mut right);
                CompareOp::LessEq
            }
            _ => op,
        };

        // --- 2. CANONISATION (Ordre des opérandes pour l'égalité) ---
        // Pour '=', on trie les IDs pour que (= A B) et (= B A) soient identiques
        if final_op == CompareOp::Equal && right < left {
            std::mem::swap(&mut left, &mut right);
        }

        // --- 3. IDENTITÉ STRUCTURELLE (x == x) ---
        // Grâce au Hash-Consing, si left == right (IDs identiques), c'est la même expression !
        if left == right {
            return match final_op {
                CompareOp::Equal | CompareOp::LessEq => self.empty_and(), // Toujours vrai
                CompareOp::Less => self.empty_or(),                       // Toujours faux
                _ => unreachable!(), // On a déjà normalisé Greater/GreaterEq
            };
        }

        // --- 4. ÉVALUATION DES CONSTANTES ---
        if let (Some(l_node), Some(r_node)) = (self.get(left), self.get(right)) {
            if let (ExprEntryKind::Number(l_val), ExprEntryKind::Number(r_val)) =
                (l_node.kind(), r_node.kind())
            {
                let l = l_val.into_inner();
                let r = r_val.into_inner();
                let result = match final_op {
                    CompareOp::Equal => l == r,
                    CompareOp::Less => l < r,
                    CompareOp::LessEq => l <= r,
                    _ => unreachable!(),
                };
                return if result {
                    self.empty_and()
                } else {
                    self.empty_or()
                };
            }
        }

        // --- 5. INTERNEMENT ---
        self.intern(ExprEntryKind::Comparison(final_op), &[left, right])
    }

    pub fn equal(&mut self, left: ExprId, right: ExprId) -> ExprId {
        self.comparison(CompareOp::Equal, left, right)
    }

    // --- Helpers de commodité pour les comparaisons ---

    pub fn less(&mut self, left: ExprId, right: ExprId) -> ExprId {
        self.comparison(CompareOp::Less, left, right)
    }

    pub fn less_eq(&mut self, left: ExprId, right: ExprId) -> ExprId {
        self.comparison(CompareOp::LessEq, left, right)
    }

    pub fn greater(&mut self, left: ExprId, right: ExprId) -> ExprId {
        self.comparison(CompareOp::Greater, left, right)
    }

    pub fn greater_eq(&mut self, left: ExprId, right: ExprId) -> ExprId {
        self.comparison(CompareOp::GreaterEq, left, right)
    }
}
