use crate::aiplan4rust::lang::{Type, TypeId, TypedList, TypedSymbol, VariableId};
use crate::aiplan4rust::lir::store::builder::ExprBuilder;
use crate::aiplan4rust::lir::store::{ExprEntryKind, ExprId};

impl<'a> ExprBuilder<'a> {
    /// Crée un nœud `Forall` avec simplifications intégrées :
    /// 1. Élimination des corps triviaux (True/False).
    /// 2. Fusion récursive des quantificateurs imbriqués.
    /// 3. Élimination si la liste de variables est vide.
    /// 4. Canonisation (tri/dedup) des variables pour le Hash-Consing.
    pub fn forall(&mut self, mut vars: TypedList<VariableId, TypeId>, body: ExprId) -> ExprId {
        // --- 1. CORPS TRIVIAL (forall (x) true) -> true ---
        // Si le corps est (and) vide ou (or) vide, le résultat est la constante elle-même.
        if body == self.empty_and() || body == self.empty_or() {
            return body;
        }

        // --- 2. FUSION (forall (x) (forall (y) B)) -> (forall (x, y) B) ---
        if let Some(body_node) = self.get(body) {
            // On vérifie si le corps est lui-même un Forall
            if let ExprEntryKind::Forall(inner_vars) = body_node.kind() {
                let mut combined_vars = vars;
                combined_vars.extend(inner_vars.clone());

                // On récupère le corps du Forall interne (son unique enfant)
                let final_body = body_node.children()[0];

                // APPEL RÉCURSIF : Cela gère (forall (x) (forall (y) (forall (z) ...)))
                return self.forall(combined_vars, final_body);
            }
        }

        // --- 3. CANONISATION DES VARIABLES ---
        // Indispensable pour que (forall (x y) B) == (forall (y x) B) dans le Store
        vars.sort_by_symbol();
        vars.dedup_by_symbol();

        // --- 4. QUANTIFICATEUR VIDE (forall () B) -> B ---
        if vars.is_empty() {
            return body;
        }

        // --- 5. INTERNEMENT ---
        // Ici, le Hash-Consing garantit l'unicité structurelle
        self.intern(ExprEntryKind::Forall(vars), &[body])
    }

    /// Crée un nœud `Exists` avec simplifications intégrées :
    /// 1. Élimination des corps triviaux (True/False).
    /// 2. Fusion récursive des `exists` imbriqués.
    /// 3. Suppression du nœud si la liste de variables est vide.
    /// 4. Canonisation (tri/dedup) des variables pour le Hash-Consing.
    pub fn exists(&mut self, mut vars: TypedList<VariableId, TypeId>, body: ExprId) -> ExprId {
        // --- 1. CORPS TRIVIAL (exists (x) false) -> false ---
        // Si le corps est une constante (and vide ou or vide), on remonte la constante.
        if body == self.empty_and() || body == self.empty_or() {
            return body;
        }

        // --- 2. FUSION (exists (x) (exists (y) B)) -> (exists (x, y) B) ---
        if let Some(body_node) = self.get(body) {
            if let ExprEntryKind::Exists(inner_vars) = body_node.kind() {
                let mut combined_vars = vars;
                combined_vars.extend(inner_vars.clone());

                // On récupère le corps du Exists interne
                let final_body = body_node.children()[0];

                // APPEL RÉCURSIF pour aplatir n-niveaux de exists
                return self.exists(combined_vars, final_body);
            }
        }

        // --- 3. CANONISATION ---
        // Pour que (exists (?a ?b) P) == (exists (?b ?a) P)
        vars.sort_by_symbol();
        vars.dedup_by_symbol();

        // --- 4. NETTOYAGE SI VIDE (exists () B) -> B ---
        if vars.is_empty() {
            return body;
        }

        // --- 5. INTERNEMENT ---
        self.intern(ExprEntryKind::Exists(vars), &[body])
    }
    // --- Helpers de construction de types (inchangés car ils ne touchent pas au Store) ---

    pub fn typed_variable_list(
        &mut self,
        vars: Vec<TypedSymbol<VariableId, TypeId>>,
    ) -> TypedList<VariableId, TypeId> {
        let mut list = TypedList::new();
        for typed_var in vars {
            list.push(typed_var);
        }
        list
    }

    pub fn typed_variable(
        &mut self,
        id: usize,
        type_ids: &[usize],
    ) -> TypedSymbol<VariableId, TypeId> {
        let ty = self.ty(type_ids);
        TypedSymbol::new(VariableId::from(id), ty)
    }

    pub fn ty(&mut self, ids: &[usize]) -> Type<TypeId> {
        Type::either(ids.iter().map(|&id| TypeId::from(id)).collect())
    }
}
