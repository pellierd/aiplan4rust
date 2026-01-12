use std::collections::HashMap;
use thiserror::Error;
use crate::aiplan4rust::interner::Ident;
use crate::aiplan4rust::lang::Type;
use crate::aiplan4rust::lir::expr::ExprError;
use crate::aiplan4rust::syntax::tree::error::SyntaxTreeError;

/// Trait for replacing union types (`Type::Either`) with their corresponding primitive equivalents.
///
/// This trait is intended for **strict/complete flattening**: after calling
/// `flatten_types`, all union types in the object should be replaced by their
/// primitive identifiers according to the provided map. Any union type that
/// cannot be flattened will produce an error.
///
/// # Examples
/// ```ignore
/// let mut ty: Type = Type::Either(vec![...]);
/// let map: HashMap<Type, Ident> = ...;
/// // After this call, ty contains only primitive types.
/// ty.flatten_types(&map)?;
/// ```
pub trait FlattenTypes {
    /// Flattens union types (`Type::Either`) into primitive types according to the provided mapping.
    ///
    /// - Union types must be replaced by their corresponding primitive identifiers.
    /// - Non-union types are left unchanged.
    ///
    /// # Parameters
    /// - `map`: A `HashMap<Type, Ident>` mapping each union type (`Type::Either`) to its corresponding primitive `Ident`.
    ///
    /// # Returns
    /// - `Ok(())` if all types were successfully flattened or already primitive.
    /// - `Err(TypeFlattenError)` if at least one type could not be flattened (strict behavior).
    fn flatten_types(&mut self, map: &HashMap<Type, Ident>) -> Result<(), TypeFlattenError>;
}

#[derive(Debug, Error)]
pub enum TypeFlattenError {
    /// An error originating from the syntax tree system.
    #[error(transparent)]
    SyntaxTree(#[from] SyntaxTreeError),

    /// An error originating from expression processing.
    #[error(transparent)]
    Expr(#[from] ExprError),

    /// The type has no corresponding mapping in the flattening map.
    /// Backtrace is captured when this error is created.
    #[error("Missing flatten mapping for type: {ty:?}")]
    MissingFlattenMapping { ty: Type, },
}

impl TypeFlattenError {
    /// Creates a new `MissingFlattenMapping` error for the given type.
    pub fn missing_flatten_mapping(ty: Type) -> Self {
        TypeFlattenError::MissingFlattenMapping {
            ty,
        }
    }
}
