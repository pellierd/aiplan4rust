//! Module `ExprContent`
//!
//! This module defines the [`Content`] enum, which represents the semantic content
//! associated with an AST (Abstract Syntax Tree) syntax node.
//!
//! Each variant of [`Content`] corresponds to a specific type_checker of content that can be
//! attached to a syntax node in the arena, such as an identifier, a floating-point literal,
//! or various language-specific operators.
//!
//! [`Content`] serves as a leaf element in the AST: it contains concrete data but does not
//! have child nodes.
//!
//! # Main Variants
//!
//! - `None`: Represents no content (default/empty syntax).
//! - `Ident`: An interned identifier (references a name via [`StringInterner`] for efficient string storage).
//! - `Float`: A floating-point literal (wrapped in [`OrderedFloat`] to guarantee total ordering).
//! - `BinaryComp`: A binary comparison operator (`=`, `<`, `>`, etc.).
//! - `AssignOp`: An assignment operator (`assign`, `increase`, etc.).
//! - `ArithmeticOp`: An arithmetic operator (`+`, `-`, `*`, `/`).
//! - `Optimization`: An optimization directive (`maximize`, `minimize`).
//!
//! # Display and Debugging
//!
//! Identifiers are stored as interned indices, so the method [`Content::display_with_context`]
//! resolves these to human-readable strings using a [`StringInterner`]. This is useful
//! for pretty-printing, debugging, and logging.
//!
//! # Example
//!
//! ```rust
//! use aiplan4rust::syntax::elements::Ident;
//! use aiplan4rust::interner::StringInterner;
//! use aiplan4rust::syntax::Content;
//! use ordered_float::OrderedFloat;
//!
//!
//! let mut interner = StringInterner::default();
//! let id = interner.intern("load");
//! let content = Content::Ident(id);
//!
//! assert_eq!(content.display_with_context(&interner), "Iden(\"load\")");
//!
//! let float_content = Content::Float(OrderedFloat(3.14));
//! println!("{}", float_content); // Prints: 3.14
//! ```
//!
//! # Identifier Remapping
//!
//! The [`Content::remap_idents`] method allows in-place remapping of interned identifiers
//! according to a provided mapping. This is useful during transformations or renaming phases.

use crate::aiplan4rust::lang::{ArithmeticOp, AssignOp, BinaryComp, Optimization, TypedList, VariableID, ObjectID, ParameterID, PredicateID, FunctorID, FunctionSkeletonID, AtomSkeletonID, TaskSkeletonID, TypeID, TaskSymbolID, PreferenceID, TaskLabelID};
use crate::aiplan4rust::lir::expr::error::ExprError;
use crate::aiplan4rust::serialization::{deserialize_ordered_float, serialize_ordered_float};
use crate::aiplan4rust::tree::SyntaxContent;
use ordered_float::OrderedFloat;
use serde::{Deserialize, Serialize};
use std::fmt;
use crate::aiplan4rust::lir::expr::ExprContent;
use crate::aiplan4rust::lir::renderers;

/// Represents the semantic content attached to an AST syntax node.
///
/// Each variant corresponds to a kind of concrete data associated with the syntax,
/// such as identifiers, literals, or operators.
///
/// This enum is a leaf in the syntax tree — it contains data but no child nodes.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum Content {
    /// No content (empty/default syntax node).
    #[default]
    None,
    //Ident(StringID),
    Variable(VariableID),     // Variables liées (Forall/Exists)
    Constant(ObjectID),     // Objets/Constantes du domaine
    Parameter(ParameterID),   // Paramètres d'action

    // --- Symboles de Définition ---
    Predicate(PredicateID),
    Functor(FunctorID),
    TaskSymbol(TaskSymbolID),
    TaskID(TaskLabelID),
    Preference(PreferenceID),

    // --- Skeletons (Liaison aux formules atomiques) ---
    /// Référence à ATOMIC_FORMULA_SKELETON_ID
    AtomSkeleton(AtomSkeletonID),
    /// Référence à ATOMIC_FUNCTION_SKELETON_ID
    FunctionSkeleton(FunctionSkeletonID),
    TaskSkeleton(TaskSkeletonID),



    /// Floating-point literal wrapped in [`OrderedFloat`] to ensure total ordering.
    #[serde(
        serialize_with = "serialize_ordered_float",
        deserialize_with = "deserialize_ordered_float"
    )]
    Float(OrderedFloat<f64>),

    /// Binary comparison operator (e.g. `=`, `<`, `>`, etc.).
    BinaryComp(BinaryComp),

    /// Assignment operator (e.g. `assign`, `increase`, etc.).
    AssignOp(AssignOp),

    /// Arithmetic operator (e.g. `+`, `-`, `*`, `/`).
    ArithmeticOp(ArithmeticOp),

    /// Optimization directive (e.g. `maximize`, `minimize`).
    Optimization(Optimization),

    /// The bound variables for a quantifier (Forall or Exists) stored as a `TypedList`.
    QuantifierVariables(TypedList<VariableID, TypeID>),

}

impl Content {

    /// Returns the object ID if the content is `Constant`.
    pub fn as_constant(&self) -> Option<ObjectID> {
        match self {
            ExprContent::Constant(id) => Some(*id),
            _ => None,
        }
    }

    /// Returns the object ID or an error.
    pub fn try_constant(&self) -> Result<ObjectID, ExprError> {
        self.as_constant().ok_or_else(ExprError::not_constant)
    }

    /// Returns the variable ID if the content is `Variable`.
    pub fn as_variable(&self) -> Option<VariableID> {
        match self {
            ExprContent::Variable(id) => Some(*id),
            _ => None,
        }
    }

    /// Returns the variable ID or an error.
    pub fn try_variable(&self) -> Result<VariableID, ExprError> {
        self.as_variable().ok_or_else(ExprError::not_variable)
    }


    /// Returns the predicate ID if the content is `Predicate`.
    pub fn as_predicate(&self) -> Option<PredicateID> {
        match self {
            ExprContent::Predicate(id) => Some(*id),
            _ => None,
        }
    }

    /// Returns the predicate ID or an error.
    pub fn try_predicate(&self) -> Result<PredicateID, ExprError> {
        self.as_predicate().ok_or_else(ExprError::not_predicate)
    }

    /// Returns the functor ID if the content is `Functor`.
    pub fn as_functor(&self) -> Option<FunctorID> {
        match self {
            ExprContent::Functor(id) => Some(*id),
            _ => None,
        }
    }

    /// Returns the functor ID or an error.
    pub fn try_functor(&self) -> Result<FunctorID, ExprError> {
        self.as_functor().ok_or_else(ExprError::not_functor)
    }

    /// Returns the task symbol ID if the content is `TaskSymbol`.
    pub fn as_task_symbol(&self) -> Option<TaskSymbolID> {
        match self {
            ExprContent::TaskSymbol(id) => Some(*id),
            _ => None,
        }
    }

    /// Returns the task symbol ID or an error.
    pub fn try_task_symbol(&self) -> Result<TaskSymbolID, ExprError> {
        self.as_task_symbol().ok_or_else(ExprError::not_task_symbol)
    }

    // Returns the task label ID if the content is `TaskID`.
    pub fn as_task_id(&self) -> Option<TaskLabelID> {
        match self {
            ExprContent::TaskID(id) => Some(*id),
            _ => None,
        }
    }

    /// Returns the task label ID or an error.
    pub fn try_task_id(&self) -> Result<TaskLabelID, ExprError> {
        self.as_task_id().ok_or_else(ExprError::not_task_id)
    }

    /// Returns the preference ID if the content is `Preference`.
    pub fn as_preference(&self) -> Option<PreferenceID> {
        match self {
            ExprContent::Preference(id) => Some(*id),
            _ => None,
        }
    }

    /// Returns the preference ID or an error.
    pub fn try_preference(&self) -> Result<PreferenceID, ExprError> {
        self.as_preference().ok_or_else(ExprError::not_preference)
    }

    /// Returns the predicate ID if the content is `AtomicSkeleton`.
    pub fn as_atom_skeleton(&self) -> Option<AtomSkeletonID> {
        match self {
            ExprContent::AtomSkeleton(id) => Some(*id),
            _ => None,
        }
    }

    /// Returns the predicate ID or an error.
    pub fn try_atom_skeleton(&self) -> Result<AtomSkeletonID, ExprError> {
        self.as_atom_skeleton().ok_or_else(ExprError::not_atom_skeleton)
    }

    /// Returns the function ID if the content is `FunctionSkeleton`.
    pub fn as_function_skeleton(&self) -> Option<FunctionSkeletonID> {
        match self {
            ExprContent::FunctionSkeleton(id) => Some(*id),
            _ => None,
        }
    }

    /// Returns the function ID or an error.
    ///
    /// # Errors
    /// Returns `ExprError::not_function_skeleton()` if the content is not a function.
    pub fn try_function_skeleton(&self) -> Result<FunctionSkeletonID, ExprError> {
        self.as_function_skeleton()
            .ok_or_else(ExprError::not_function_skeleton)
    }

    /// Returns the task skeleton ID if the content is `TaskSkeleton`.
    pub fn as_task_skeleton(&self) -> Option<TaskSkeletonID> {
        match self {
            ExprContent::TaskSkeleton(id) => Some(*id),
            _ => None,
        }
    }

    /// Returns the task skeleton ID or an error.
    ///
    /// # Errors
    /// Returns `ExprError::not_task_skeleton()` if the content is not a task skeleton.
    pub fn try_task_skeleton(&self) -> Result<TaskSkeletonID, ExprError> {
        self.as_task_skeleton()
            .ok_or_else(ExprError::not_task_skeleton)
    }

    /// Returns a reference to the quantifier’s bound variables if the content is `TypedVariables`.
    ///
    /// # Returns
    /// * `Some(&TypedList)` if the content holds bound variables
    /// * `None` otherwise
    pub fn as_quantifier_vars(&self) -> Option<&TypedList<VariableID, TypeID>> {
        match self {
            ExprContent::QuantifierVariables(list) => Some(list),
            _ => None,
        }
    }

    /// Returns a reference to the quantifier’s bound variables.
    ///
    /// # Errors
    /// Returns `ExprError::unsupported_content` if the content is not `TypedVariables`.
    pub fn try_quantifier_vars(&self) -> Result<&TypedList<VariableID, TypeID>, ExprError> {
        match self {
            ExprContent::QuantifierVariables(list) => Ok(list),
            _ => Err(ExprError::not_quantifier_variables()),
        }
    }


    /// Returns a mutable reference to the quantifier’s bound variables if the content is `TypedVariables`.
    ///
    /// # Returns
    /// * `Some(&mut TypedList)` if the content holds bound variables
    /// * `None` otherwise
    pub fn as_quantifier_vars_mut(&mut self) -> Option<&mut TypedList<VariableID, TypeID>> {
        match self {
            ExprContent::QuantifierVariables(list) => Some(list),
            _ => None,
        }
    }

    /// Returns a mutable reference to the quantifier’s bound variables.
    ///
    /// # Errors
    /// Returns `ExprError::not_quantifier_variables()` if the content is not `TypedVariables`.
    pub fn try_quantifier_vars_mut(&mut self) -> Result<&mut TypedList<VariableID, TypeID>, ExprError> {
        match self {
            ExprContent::QuantifierVariables(list) => Ok(list),
            _ => Err(ExprError::not_quantifier_variables()),
        }
    }
}

impl fmt::Display for Content {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        renderers::default::render_node_expr_content(f, self)
    }
}


impl SyntaxContent for Content {

    fn as_float(&self) -> Option<OrderedFloat<f64>> {
        match self {
            Content::Float(f) => Some(*f),
            _ => None,
        }
    }

    fn as_binary_comp(&self) -> Option<BinaryComp> {
        match self {
            Content::BinaryComp(bc) => Some(*bc),
            _ => None,
        }
    }

    fn as_assign_op(&self) -> Option<AssignOp> {
        match self {
            Content::AssignOp(op) => Some(*op),
            _ => None,
        }
    }

    fn as_arithmetic_op(&self) -> Option<ArithmeticOp> {
        match self {
            Content::ArithmeticOp(op) => Some(*op),
            _ => None,
        }
    }

    fn as_optimization(&self) -> Option<Optimization> {
        match self {
            Content::Optimization(opt) => Some(*opt),
            _ => None,
        }
    }
}

/*impl RemapIdents for Content {
    /// Remaps the identifier inside this content, if it is an `Ident` and exists in the mapping.
    ///
    /// # Parameters
    /// - `map`: A mapping from old `Ident`s to new `Ident`s.
    ///
    /// # Behavior
    /// - If the content is an `Ident` and a corresponding mapping exists, it is replaced.
    /// - If the content is not an `Ident` or no mapping exists, it remains unchanged.
    ///
    /// # Notes
    /// - The operation is performed in place and is panic-free.
    fn remap_idents(&mut self, map: &HashMap<StringID, StringID>) -> Result<(), InternerError>{
        if let Content::Ident(id) = self {
            if let Some(new_id) = map.get(id) {
                *id = *new_id;
            }
        }
        Ok(())
    }
}*/
