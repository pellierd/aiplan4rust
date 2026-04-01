use crate::aiplan4rust::error::Traceable;
use crate::aiplan4rust::lang::SymbolId;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum DeclarationError {
    #[error("Argument index {index} is out of bounds for the declaration")]
    ArgumentIndexOutOfBounds { index: usize },

    #[error("Missing type definition for symbol {symbol_id}")]
    MissingSymbolTypes { symbol_id: SymbolId },
}

impl DeclarationError {
    pub fn argument_index_out_of_bounds(index: usize) -> Self {
        Self::ArgumentIndexOutOfBounds { index }.trace()
    }

    pub fn missing_symbol_types(symbol_id: SymbolId) -> Self {
        Self::MissingSymbolTypes { symbol_id }.trace()
    }
}

impl Traceable for DeclarationError {}
