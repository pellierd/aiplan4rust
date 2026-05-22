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
//! - `None`: Represents no content (debug/empty syntax).
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
//! let mut interner = StringInterner::debug();
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

use crate::aiplan4rust::lang::{
    ArithmeticOp, AssignOp, AtomSkeletonId, CompareOp, FunctionSkeletonId, FunctionSymbolId,
    ObjectId, OptimizationOp, PredicateSymbolId, PreferenceSymbolId, TaskLabelSymbolId,
    TaskSkeletonId, TaskSymbolId, TypeId, TypedList, VariableId,
};
use crate::aiplan4rust::lir::old::expr::error::ExprError;
use crate::aiplan4rust::lir::old::expr::ExprContent;
use crate::aiplan4rust::lir::old::renderers;
use crate::aiplan4rust::serialization::{deserialize_ordered_float, serialize_ordered_float};
use crate::aiplan4rust::tree::SyntaxContent;
use ordered_float::OrderedFloat;
use serde::{Deserialize, Serialize};
use std::fmt;

/// Represents the semantic content attached to an AST syntax node.
///
/// Each variant corresponds to a kind of concrete data associated with the syntax,
/// such as identifiers, literals, or operators.
///
/// This enum is a leaf in the syntax tree — it contains data but no child nodes.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum Content {
    /// No content (empty/debug syntax node).
    #[default]
    None,
    //Ident(StringID),
    Variable(VariableId), // Variables liées (Forall/Exists)
    Object(ObjectId),     // Objets/Constantes du domaine

    // --- Symboles de Définition ---
    PredicateSymbol(PredicateSymbolId),
    FunctionSymbol(FunctionSymbolId),
    TaskSymbol(TaskSymbolId),
    TaskLabelSymbol(TaskLabelSymbolId),
    PreferenceSymbol(PreferenceSymbolId),

    // --- Skeletons (Liaison aux formules atomiques) ---
    /// Référence à ATOMIC_FORMULA_SKELETON_ID
    AtomSkeleton(AtomSkeletonId),
    /// Référence à ATOMIC_FUNCTION_SKELETON_ID
    FunctionSkeleton(FunctionSkeletonId),
    TaskSkeleton(TaskSkeletonId),

    /// Floating-point literal wrapped in [`OrderedFloat`] to ensure total ordering.
    #[serde(
        serialize_with = "serialize_ordered_float",
        deserialize_with = "deserialize_ordered_float"
    )]
    Number(OrderedFloat<f64>),

    /// Binary comparison operator (e.g. `=`, `<`, `>`, etc.).
    Comparison(CompareOp),

    /// Assignment operator (e.g. `assign`, `increase`, etc.).
    Assignment(AssignOp),

    /// Arithmetic operator (e.g. `+`, `-`, `*`, `/`).
    ArithmeticOp(ArithmeticOp),

    /// Optimization directive (e.g. `maximize`, `minimize`).
    OptimizationOp(OptimizationOp),

    /// The bound variables for a quantifier (Forall or Exists) stored as a `TypedList`.
    QuantifierVariables(TypedList<VariableId, TypeId>),
}

impl Content {
    /// Returns the object ID if the content is `Constant`.
    pub fn as_object(&self) -> Option<ObjectId> {
        match self {
            ExprContent::Object(id) => Some(*id),
            _ => None,
        }
    }

    /// Returns the object ID or an error.
    pub fn try_object(&self) -> Result<ObjectId, ExprError> {
        self.as_object().ok_or_else(ExprError::not_constant)
    }

    /// Returns the variable ID if the content is `Variable`.
    pub fn as_variable(&self) -> Option<VariableId> {
        match self {
            ExprContent::Variable(id) => Some(*id),
            _ => None,
        }
    }

    /// Returns the variable ID or an error.
    pub fn try_variable(&self) -> Result<VariableId, ExprError> {
        self.as_variable().ok_or_else(ExprError::not_variable)
    }

    /// Returns the predicate ID if the content is `Predicate`.
    pub fn as_predicate_symbol(&self) -> Option<PredicateSymbolId> {
        match self {
            ExprContent::PredicateSymbol(id) => Some(*id),
            _ => None,
        }
    }

    /// Returns the predicate ID or an error.
    pub fn try_predicate_symbol(&self) -> Result<PredicateSymbolId, ExprError> {
        self.as_predicate_symbol()
            .ok_or_else(ExprError::not_predicate)
    }

    /// Returns the functor ID if the content is `Functor`.
    pub fn as_function_symbol(&self) -> Option<FunctionSymbolId> {
        match self {
            ExprContent::FunctionSymbol(id) => Some(*id),
            _ => None,
        }
    }

    /// Returns the functor ID or an error.
    pub fn try_function_symbol(&self) -> Result<FunctionSymbolId, ExprError> {
        self.as_function_symbol().ok_or_else(ExprError::not_functor)
    }

    /// Returns the task symbol ID if the content is `TaskSymbol`.
    pub fn as_task_symbol(&self) -> Option<TaskSymbolId> {
        match self {
            ExprContent::TaskSymbol(id) => Some(*id),
            _ => None,
        }
    }

    /// Returns the task symbol ID or an error.
    pub fn try_task_symbol(&self) -> Result<TaskSymbolId, ExprError> {
        self.as_task_symbol().ok_or_else(ExprError::not_task_symbol)
    }

    // Returns the task label ID if the content is `TaskID`.
    pub fn as_task_label_symbol(&self) -> Option<TaskLabelSymbolId> {
        match self {
            ExprContent::TaskLabelSymbol(id) => Some(*id),
            _ => None,
        }
    }

    /// Returns the task label ID or an error.
    pub fn try_task_label_symbol(&self) -> Result<TaskLabelSymbolId, ExprError> {
        self.as_task_label_symbol()
            .ok_or_else(ExprError::not_task_id)
    }

    /// Returns the preference ID if the content is `Preference`.
    pub fn as_preference_symbol(&self) -> Option<PreferenceSymbolId> {
        match self {
            ExprContent::PreferenceSymbol(id) => Some(*id),
            _ => None,
        }
    }

    /// Returns the preference ID or an error.
    pub fn try_preference_symbol(&self) -> Result<PreferenceSymbolId, ExprError> {
        self.as_preference_symbol()
            .ok_or_else(ExprError::not_preference)
    }

    /// Returns the predicate ID if the content is `AtomicSkeleton`.
    pub fn as_atom_skeleton(&self) -> Option<AtomSkeletonId> {
        match self {
            ExprContent::AtomSkeleton(id) => Some(*id),
            _ => None,
        }
    }

    /// Returns the predicate ID or an error.
    pub fn try_atom_skeleton(&self) -> Result<AtomSkeletonId, ExprError> {
        self.as_atom_skeleton()
            .ok_or_else(ExprError::not_atom_skeleton)
    }

    /// Returns the function ID if the content is `FunctionSkeleton`.
    pub fn as_function_skeleton(&self) -> Option<FunctionSkeletonId> {
        match self {
            ExprContent::FunctionSkeleton(id) => Some(*id),
            _ => None,
        }
    }

    /// Returns the function ID or an error.
    ///
    /// # Errors
    /// Returns `ExprError::not_function_skeleton()` if the content is not a function.
    pub fn try_function_skeleton(&self) -> Result<FunctionSkeletonId, ExprError> {
        self.as_function_skeleton()
            .ok_or_else(ExprError::not_function_skeleton)
    }

    /// Returns the task skeleton ID if the content is `TaskSkeleton`.
    pub fn as_task_skeleton(&self) -> Option<TaskSkeletonId> {
        match self {
            ExprContent::TaskSkeleton(id) => Some(*id),
            _ => None,
        }
    }

    /// Returns the task skeleton ID or an error.
    ///
    /// # Errors
    /// Returns `ExprError::not_task_skeleton()` if the content is not a task skeleton.
    pub fn try_task_skeleton(&self) -> Result<TaskSkeletonId, ExprError> {
        self.as_task_skeleton()
            .ok_or_else(ExprError::not_task_skeleton)
    }

    /// Returns a reference to the quantifier’s bound variables if the content is `TypedVariables`.
    ///
    /// # Returns
    /// * `Some(&TypedList)` if the content holds bound variables
    /// * `None` otherwise
    pub fn as_quantifier_vars(&self) -> Option<&TypedList<VariableId, TypeId>> {
        match self {
            ExprContent::QuantifierVariables(list) => Some(list),
            _ => None,
        }
    }

    /// Returns a reference to the quantifier’s bound variables.
    ///
    /// # Errors
    /// Returns `ExprError::unsupported_content` if the content is not `TypedVariables`.
    pub fn try_quantifier_vars(&self) -> Result<&TypedList<VariableId, TypeId>, ExprError> {
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
    pub fn as_quantifier_vars_mut(&mut self) -> Option<&mut TypedList<VariableId, TypeId>> {
        match self {
            ExprContent::QuantifierVariables(list) => Some(list),
            _ => None,
        }
    }

    /// Returns a mutable reference to the quantifier’s bound variables.
    ///
    /// # Errors
    /// Returns `ExprError::not_quantifier_variables()` if the content is not `TypedVariables`.
    pub fn try_quantifier_vars_mut(
        &mut self,
    ) -> Result<&mut TypedList<VariableId, TypeId>, ExprError> {
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
    fn as_number(&self) -> Option<OrderedFloat<f64>> {
        match self {
            Content::Number(f) => Some(*f),
            _ => None,
        }
    }

    fn as_compare_op(&self) -> Option<CompareOp> {
        match self {
            Content::Comparison(bc) => Some(*bc),
            _ => None,
        }
    }

    fn as_assign_op(&self) -> Option<AssignOp> {
        match self {
            Content::Assignment(op) => Some(*op),
            _ => None,
        }
    }

    fn as_arithmetic_op(&self) -> Option<ArithmeticOp> {
        match self {
            Content::ArithmeticOp(op) => Some(*op),
            _ => None,
        }
    }

    fn as_optimization_op(&self) -> Option<OptimizationOp> {
        match self {
            Content::OptimizationOp(opt) => Some(*opt),
            _ => None,
        }
    }
}
