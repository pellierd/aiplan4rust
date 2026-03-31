use crate::aiplan4rust::arena::NodeId;
use crate::aiplan4rust::lang::{SymbolId, Type};
use crate::aiplan4rust::semantic::symbol::Declaration;

pub enum MatchResult {
    /// Match parfait (Subtype)
    Match,
    /// Match avec une réserve (ex: Upcasting Task -> Action)
    /// On stocke les infos nécessaires pour construire le diagnostic plus tard
    UpcastMatch {
        expected: Type<SymbolId>,
        provided: Type<SymbolId>,
        arg_decl: Declaration,
        arg_node_id: NodeId,
    },
    /// Aucun match possible
    NoMatch,
}
