use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
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
