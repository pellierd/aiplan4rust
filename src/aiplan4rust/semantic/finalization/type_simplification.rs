use crate::aiplan4rust::interner::{InternerDisplay, SymbolInterner};
use crate::aiplan4rust::lang::{SymbolId, Type};
use crate::aiplan4rust::tree::NodeId;
use std::fmt;

/// Represents a planned modification to be applied to both the `SymbolTable` and the `AST`.
///
/// This structure acts as a "diff" or an instruction set generated during semantic analysis
/// to specify how a type union should be simplified and which corresponding AST nodes
/// should be synchronized.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeSimplification {
    /// The unique identifier of the symbol in the `SymbolTable`.
    symbol_id: SymbolId,
    /// The unique identifier of the declaration node in the `AST`.
    node_id: NodeId,
    /// The new optimized semantic definition of the type.
    new_type: Type<SymbolId>,
    /// The indices of the original type members that were kept after simplification.
    kept_indices: Vec<usize>,
}

impl TypeSimplification {
    /// Creates a new type simplification instruction.
    ///
    /// # Arguments
    /// * `symbol_id` - The ID of the symbol to update.
    /// * `node_id` - The AST node ID where the declaration occurs.
    /// * `new_type` - The simplified version of the type.
    /// * `kept_indices` - The list of indices from the original union that remain valid.
    pub fn new(
        symbol_id: SymbolId,
        node_id: NodeId,
        new_type: Type<SymbolId>,
        kept_indices: Vec<usize>,
    ) -> Self {
        Self {
            symbol_id,
            node_id,
            new_type,
            kept_indices,
        }
    }

    // --- Accessors ---

    /// Returns the ID of the symbol associated with this simplification.
    pub fn symbol_id(&self) -> SymbolId {
        self.symbol_id
    }

    /// Returns the ID of the AST node targeted by this simplification.
    pub fn node_id(&self) -> NodeId {
        self.node_id
    }

    /// Returns a reference to the new simplified type.
    pub fn new_type(&self) -> &Type<SymbolId> {
        &self.new_type
    }

    /// Returns a slice of the indices kept from the original type union.
    pub fn kept_indices(&self) -> &[usize] {
        &self.kept_indices
    }
}

// --- Display Implementation ---

impl fmt::Display for TypeSimplification {
    /// Formats the simplification for debugging purposes without name resolution.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "TypeSimplification(symbol: {:?}, node: {:?}, kept_indices: {:?})",
            self.symbol_id, self.node_id, self.kept_indices
        )
    }
}

impl InternerDisplay for TypeSimplification {
    /// Formats the simplification using the provided `SymbolInterner` to resolve symbol names.
    ///
    /// This provides a much more readable output by showing actual type names instead of internal IDs.
    fn fmt_with_interner(
        &self,
        f: &mut fmt::Formatter<'_>,
        interner: &SymbolInterner,
    ) -> fmt::Result {
        let symbol_name = interner.resolve_symbol(self.symbol_id).unwrap_or("unknown");
        write!(
            f,
            "TypeSimplification(symbol: '{}', node: {}, new_type: ",
            symbol_name, self.node_id
        )?;
        self.new_type.fmt_with_interner(f, interner)?;
        write!(f, ", kept_indices: {:?})", self.kept_indices)
    }
}
