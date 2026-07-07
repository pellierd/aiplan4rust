use crate::aiplan4rust::support::lang::{ActionDefId, AtomSkeletonId, TypeId};
use crate::analysis::reachability::datalog::core::Atom;
use crate::DatalogEngine;

impl<'a> DatalogEngine<'a> {
    #[inline]
    pub fn is_fluent(&self, id: AtomSkeletonId) -> bool {
        // Tout ce qui est avant le début des types est un fluent
        // (cela inclut le bloc positif et le bloc négatif optionnel)
        id.as_usize() < self.type_segment_start
    }

    #[inline]
    pub fn is_negated_fluent(&self, id: AtomSkeletonId) -> bool {
        let val = id.as_usize();
        val >= self.fluence_threshold && val < self.type_segment_start
    }

    #[inline]
    pub fn is_type(&self, id: AtomSkeletonId) -> bool {
        let p = id.as_usize();
        // Le segment des types est coincé entre sa frontière propre et le début des actions
        p >= self.type_segment_start && p < self.type_threshold
    }

    #[inline]
    pub fn atom_id_to_type_id(&self, sk_id: AtomSkeletonId) -> TypeId {
        let id_val = sk_id.as_usize();
        debug_assert!(self.is_type(sk_id));
        // L'offset de soustraction est maintenant dynamique
        TypeId::from(id_val - self.type_segment_start)
    }

    #[inline]
    pub fn negate_id(&self, id: AtomSkeletonId) -> AtomSkeletonId {
        let val = id.as_usize();
        // On part du principe que id est un fluent positif < fluence_threshold
        debug_assert!(val < self.fluence_threshold);
        AtomSkeletonId::from(val + self.fluence_threshold)
    }

    #[inline]
    pub fn pos_id_from_negated(&self, id: AtomSkeletonId) -> AtomSkeletonId {
        let val = id.as_usize();
        // On part du principe que id est un fluent négatif [N..2N[
        debug_assert!(val >= self.fluence_threshold && val < self.type_segment_start);
        AtomSkeletonId::from(val - self.fluence_threshold)
    }

    /// Vérifie si un ID appartient au segment des Actions.
    #[inline]
    pub fn is_action(&self, id: AtomSkeletonId) -> bool {
        let p = id.as_usize();
        p >= self.type_threshold && p < self.action_threshold
    }

    #[inline]
    pub fn atom_id_to_action_def_id(&self, sk_id: AtomSkeletonId) -> ActionDefId {
        let id_val = sk_id.as_usize();
        // On ne touche à rien ici, le calcul est mathématiquement juste pour le vecteur
        ActionDefId::from(id_val - self.type_threshold)
    }

    /// Convertit un ActionDefId (public) en AtomSkeletonId (interne).
    pub fn action_def_id_to_atom_id(&self, def_id: ActionDefId) -> AtomSkeletonId {
        AtomSkeletonId::from(def_id.as_usize() + self.action_base_id)
    }

    /// Convertit un ActionDefId (public) en l'ID interne (AtomSkeletonId)
    /// utilisé par le moteur Datalog.
    pub fn action_id_to_skeleton(&self, action_id: ActionDefId) -> AtomSkeletonId {
        // On utilise le même calcul que ton atom_id_to_action_def_id mais à l'envers
        AtomSkeletonId::from(action_id.as_usize() + self.type_threshold)
    }

    #[inline]
    pub fn is_builtin(&self, id: AtomSkeletonId) -> bool {
        id.as_usize() >= Atom::BUILTIN_ZONE_START
    }

    #[inline]
    pub fn is_auxiliary(&self, id: AtomSkeletonId) -> bool {
        let p = id.as_usize();
        // Un auxiliaire est tout ce qui se trouve entre la fin des actions
        // et le début de la zone réservée aux built-ins (égalité).
        p >= self.action_threshold && p < Atom::BUILTIN_ZONE_START
    }
}
