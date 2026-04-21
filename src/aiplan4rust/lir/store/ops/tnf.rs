use crate::aiplan4rust::lir::store::iter::Scratchpad;
use crate::aiplan4rust::lir::store::ops::error::ExprOpErrorHC;
use crate::aiplan4rust::lir::store::{ExprEntryKind, ExprId};

use crate::aiplan4rust::lir::store::builder::ExprBuilder;
/// Transforme une expression en Forme Normale Temporelle (TNF).
/// Sépare les composants 'at start', 'at end' et 'overall'.
use smallvec::SmallVec;

pub fn to_tnf(
    root: ExprId,
    builder: &mut ExprBuilder,
    scratch: &mut Scratchpad,
) -> Result<ExprId, ExprOpErrorHC> {
    let empty = builder.empty_and();
    scratch.clear();

    // ON RÉSERVE UNE SEULE FOIS AVANT LA BOUCLE
    // Ce buffer servira à extraire les enfants de n'importe quel nœud.
    let mut children_ids: SmallVec<[ExprId; 16]> = SmallVec::new();

    let root_encoded = TimeSpecifier::None.pack(root.as_usize());
    scratch.push(ExprId::from(root_encoded), false);

    while let Some((packed_id_wrapper, processed)) = scratch.pop() {
        let packed_id = packed_id_wrapper.as_usize();
        let (raw_id, ctx) = TimeSpecifier::unpack(packed_id);
        let curr_id = ExprId::from(raw_id);

        // --- PHASE 1 : EXTRACTION ---
        let kind = {
            let entry = builder.fetch(curr_id)?;

            // On réutilise le même espace mémoire à chaque fois
            children_ids.clear();
            children_ids.extend_from_slice(entry.children());

            entry.kind().clone()
        }; // Le borrow de builder s'arrête ici

        if processed {
            // --- RECONSTRUCTION ---
            let res_triplet = match &kind {
                ExprEntryKind::AtStart | ExprEntryKind::AtEnd | ExprEntryKind::Overall => {
                    let next_ctx = match kind {
                        ExprEntryKind::AtStart => TimeSpecifier::AtStart,
                        ExprEntryKind::AtEnd => TimeSpecifier::AtEnd,
                        _ => TimeSpecifier::Overall,
                    };
                    let child_packed = next_ctx.pack(children_ids[0].as_usize());
                    scratch.get_temporal_decomposition(child_packed)
                }

                ExprEntryKind::And
                | ExprEntryKind::Or
                | ExprEntryKind::Not
                | ExprEntryKind::Forall(_)
                | ExprEntryKind::Exists(_) => {
                    scratch.clear_time_specifier_buffers();

                    for &c in &children_ids {
                        let child_packed = ctx.pack(c.as_usize());
                        let (s, e, o) = scratch.get_temporal_decomposition(child_packed);
                        scratch.push_time_specifier(s, e, o, empty);
                    }

                    let s_id = rebuild_safe(builder, &kind, scratch.collected_starts(), empty);
                    let e_id = rebuild_safe(builder, &kind, scratch.collected_ends(), empty);
                    let o_id = rebuild_safe(builder, &kind, scratch.collected_overalls(), empty);
                    (s_id, e_id, o_id)
                }

                _ => {
                    let id = builder.intern(kind, &children_ids);
                    match ctx {
                        TimeSpecifier::AtStart => (id, empty, empty),
                        TimeSpecifier::AtEnd => (empty, id, empty),
                        TimeSpecifier::Overall => (empty, empty, id),
                        TimeSpecifier::None => (id, id, id),
                    }
                }
            };
            scratch.save_temporal_decomposition(packed_id, res_triplet);
        } else {
            // --- DESCENTE ---
            let next_ctx = match kind {
                ExprEntryKind::AtStart => TimeSpecifier::AtStart,
                ExprEntryKind::AtEnd => TimeSpecifier::AtEnd,
                ExprEntryKind::Overall => TimeSpecifier::Overall,
                _ => ctx,
            };

            scratch.push(packed_id_wrapper, true);
            for &child in children_ids.iter().rev() {
                let child_packed = next_ctx.pack(child.as_usize());
                scratch.push(ExprId::from(child_packed), false);
            }
        }
    }

    // Assemblage final...
    let root_key = TimeSpecifier::None.pack(root.as_usize());
    let (s, e, o) = scratch.get_temporal_decomposition(root_key);

    let nodes = [builder.at_start(s), builder.at_end(e), builder.overall(o)];
    Ok(builder.and(&nodes))
}

/// Version optimisée pour la reconstruction utilisant les buffers du scratchpad.
fn rebuild_safe(
    builder: &mut ExprBuilder,
    kind: &ExprEntryKind,
    kids: &[ExprId],
    empty: ExprId,
) -> ExprId {
    if kids.is_empty() {
        empty
    } else if kids.len() == 1 && matches!(kind, ExprEntryKind::And | ExprEntryKind::Or) {
        kids[0]
    } else {
        builder.reconstruct(kind.clone(), kids)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum TimeSpecifier {
    None = 0,
    AtStart = 1,
    AtEnd = 2,
    Overall = 3,
}

impl TimeSpecifier {
    // Constantes internes pour le bit-packing, invisibles à l'extérieur du crate
    pub(crate) const BITS_MASK: usize = 0x3;
    pub(crate) const SHIFT: usize = 2;

    /// Packe n'importe quelle valeur (typiquement un ExprId) avec ce spécificateur.
    /// pub(crate) car c'est une cuisine interne aux algorithmes de réécriture.
    #[inline(always)]
    pub(crate) fn pack(self, id_val: usize) -> usize {
        (id_val << Self::SHIFT) | (self as usize)
    }

    /// Déballe une valeur pour retrouver l'ID brut et le spécificateur.
    #[inline(always)]
    pub(crate) fn unpack(packed_val: usize) -> (usize, Self) {
        let id_val = packed_val >> Self::SHIFT;
        let spec = match packed_val & Self::BITS_MASK {
            1 => Self::AtStart,
            2 => Self::AtEnd,
            3 => Self::Overall,
            _ => Self::None,
        };
        (id_val, spec)
    }
}
