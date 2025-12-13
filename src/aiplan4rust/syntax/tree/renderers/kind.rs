//! The `render_kind` module defines the `RenderKind` enum, which unifies `AstKind`
//! and `ExprKind` for rendering purposes in syntax trees.
//!
//! `RenderKind` abstracts the way nodes are displayed by different renderers, allowing
//! consistent rendering logic independent of whether a node originates from the AST
//! (`AstKind`) or expressions (`ExprKind`).
//!
//! # Purpose
//! - Provides a uniform "view" of syntax nodes for rendering.
//! - Factors shared display behavior across multiple node types.
//! - Enables multiple renderers (`default`, `syntax`, `tree`) to handle nodes without
//!   duplicating formatting logic.
//!
//! # Key Concepts
//! - Nodes implement the `RenderableNode` trait, giving access to content, children,
//!   and their `RenderKind`.
//! - Each `RenderKind` corresponds to a high-level category of node appearance, rather
//!   than its low-level AST or Expr type.
//! - This separation simplifies renderer implementation and ensures consistent output.
//!

use crate::aiplan4rust::interner::StringInterner;
use crate::aiplan4rust::syntax::lexer::token::{
    ACTION, ALWAYS, ALWAYS_WITHIN, AND, ASSIGN, AT_END, AT_MOST_ONCE, AT_START, CONSTANTS,
    CONSTRAINTS, DERIVED, DOMAIN_DEF, DURATIVE_ACTION, EFFECT, EXISTS, FORALL, FUNCTIONS, GOAL,
    HOLD_AFTER, HOLD_DURING, HTN, IMPLY, INIT, IS_VIOLATED, LENGTH, METHOD, METRIC, NOT, OBJECTS,
    OR, ORDERED_SUBTASKS, ORDERED_TASKS, OVERALL, PARALLEL, PARAMETERS, PRECONDITION,
    PREDICATES, PREFERENCE, PROBLEM, REQUIREMENTS, SERIAL, SOMETIME, SOMETIME_AFTER,
    SOMETIME_BEFORE, SUBTASKS, TASK, TOTAL_TIME, TYPES, WHEN, WITHIN,
};
use crate::aiplan4rust::syntax::{write_indent, SyntaxInternerDisplay};

use serde::{Deserialize, Serialize};
use std::fmt;
use crate::aiplan4rust::lir::expr::ExprKind;
use crate::aiplan4rust::syntax::ast::AstKind;

/// Represents the different kinds of nodes in a syntax tree, unifying AST nodes (`AstKind`)
/// and expression nodes (`ExprKind`) for rendering purposes.
///
/// This enum models the various syntactic and semantic components typically found in
/// domain or problem specifications of domain-specific languages (DSLs) such as PDDL,
/// including extensions like HDDL (Hierarchical Task Network Planning).
///
/// Each variant corresponds to a specific element of a syntax problem or logical expression,
/// enabling structured parsing, manipulation, and rendering.
///
/// # Key Points
/// - Combines AST and Expr node kinds for unified rendering logic.
/// - Supports constants, variables, types, predicates, actions, and functions.
/// - Includes logical operators and quantifiers (`And`, `Or`, `Not`, `Forall`, `Exists`, etc.).
/// - Covers temporal constructs (`AtStart`, `AtEnd`, `AlwaysWithin`, `SometimeAfter`, etc.).
/// - Supports HDDL extensions for tasks, methods, subtasks, and constraints.
/// - Designed to allow different renderers (`default`, `syntax`, `tree`) to display nodes
///   consistently without duplicating formatting code.
///
/// # Usage Examples
/// ```rust
/// use crate::aiplan4rust::syntax::render_kind::Kind;
///
/// let node_kind = Kind::ActionDef;
/// match node_kind {
///     Kind::ActionDef => println!("This node defines an action."),
///     Kind::TaskDef => println!("This node defines an HDDL task."),
///     _ => println!("Other node type."),
/// }
/// ```
///
/// # Notes
/// - Variants with empty string representations in syntax formatting (`fmt_syntax_with_indent`)
///   typically represent structural or semantic nodes without direct keywords.
/// - Nodes implementing `RenderableNode` use `Kind` (or `RenderKind`) to determine
///   how they should be displayed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum Kind {
    /// A constant value in the syntax problem (literal or fixed value).
    Constant,

    /// A variable placeholder used in actions or predicates.
    Variable,

    /// A function symbol, typically for mathematical or logical functions.
    FunctionSymbol,

    /// A primitive data type_checker (e.g., integer, boolean).
    PrimitiveType,

    /// The name of the domain in a syntax description.
    DomainName,

    /// The name of the problem in a syntax description.
    ProblemName,

    /// A predicate symbol used in logical expr or conditions.
    Predicate,

    /// An action symbol within the problem domain.
    ActionSymbol,

    /// Symbol representing durative actions.
    DASymbol,

    /// Symbol representing tasks (e.g., in HDDL dialect).
    TaskSymbol,

    /// A preference name used for soft constraints or preferences.
    PrefName,

    /// Definition of requirements (e.g., `(:require ...)` in PDDL).
    RequireDef,

    /// A specific requirement or constraint for the domain/problem.
    Requirement,

    /// A type_checker declaration used to define object types.
    Type,

    /// A list of typed elements (multiple `TypedItem`s).
    TypedList,

    /// A single typed element, pairing names with a type_checker.
    TypedItem,

    /// The untyped element part within a typed item.
    TypedItemElements,

    /// Definition of types in the domain.
    TypesDef,

    /// Definition of constants in the domain/problem.
    ConstantsDef,

    /// Definition of objects available in the domain/problem.
    ObjectsDef,

    /// Represents the entire domain specification.
    Domain,

    /// Represents the entire problem specification.
    Problem,

    /// Definition of predicates available.
    PredicatesDef,

    /// Structure of an atomic formula.
    AtomicFormulaSkeleton,

    /// Definition of functions.
    FunctionsDef,

    /// Represents a term in a function expr.
    FunctionTerm,

    /// Skeleton structure of atomic functions.
    AtomicFunctionSkeleton,

    /// Numeric value representation.
    Number,

    /// Definition of an action.
    ActionDef,

    // Definition of parameters
    ParametersDef,

    /// Definition of a durative action (actions with duration).
    DurativeActionDef,

    /// The body of an action, including preconditions and effects.
    ActionDefBody,

    /// Definition of preconditions for an action.
    PreconditionDef,

    /// Definition of effects for an action.
    EffectDef,

    /// The body of a durative action.
    DADefBody,

    /// Definition of derived predicates or functions.
    DerivedDef,

    /// Represents a simple atomic logical formula.
    AtomicFormula,

    /// Logical AND operator to combine formulas.
    And,

    /// Logical OR operator.
    Or,

    /// Logical NOT operator.
    Not,

    /// Logical implication (if ... then ...).
    Imply,

    /// Universal quantifier (for all).
    Forall,

    /// Existential quantifier (there exists).
    Exists,

    /// Preference condition indicating a preferred solution.
    Preference,

    /// Conditional timing (when) in temporal syntax.
    When,

    /// Binary function comparison (e.g., `<`, `<=`, `=`, `!=`).
    FComp,

    /// Assignment operation.
    Assign,

    /// Arithmetic or logical operation.
    Operation,

    /// Constraints defined on the problem/domain.
    Constraints,

    /// Condition holding at the start of an action.
    AtStart,

    /// Condition holding at the end of an action.
    AtEnd,

    /// Condition holding throughout the action duration.
    Overall,

    /// Condition that must always hold.
    Always,

    /// Condition that must hold sometime during execution.
    Sometime,

    /// Condition holding within a specific time frame.
    Within,

    /// Condition that holds at most once during execution.
    AtMostOnce,

    /// Condition holding sometime after an event.
    SometimeAfter,

    /// Condition holding sometime before an event.
    SometimeBefore,

    /// Condition that must always hold within a certain timeframe.
    AlwaysWithin,

    /// Condition holding during a specified interval.
    HoldDuring,

    /// Condition holding after a specific event/time.
    HoldAfter,

    /// Initial conditions or state of the problem.
    Init,

    /// Timed initial literal (initial condition with timing).
    TimedInitialLiteral,

    /// Goal conditions the planner must achieve.
    Goal,

    /// Metric to optimize (minimize or maximize).
    Metric,

    /// Total time of the problem.
    TotalTime,

    /// Indicates whether a constraint or condition is violated.
    IsViolated,

    /// Length or duration measure.
    Length,

    /// Serial timing/duration (ordered execution).
    Serial,

    /// Parallel timing/duration.
    Parallel,

    #[default]
    /// Represents an error or problem in domain/problem definition.
    Error,


    // HDDL Dialect Extensions

    /// Represents a task in HDDL.
    Task,

    /// Definition of a task in HDDL.
    TaskDef,

    /// Tagged task with an ID in HDDL.
    TaggedTask,

    /// Definition of a method (task decomposition) in HDDL.
    MethodDef,

    /// Symbolic reference to a method in HDDL.
    MethodSymbol,

    /// Definition of method precondition in HDDL.
    MethodPreconditionDef,

    /// Body of a method definition in HDDL.
    MethodDefBody,

    /// List of ordered subtasks (execution order matters).
    OrderedSubtaskDef,

    /// List of partially ordered subtasks (some order constraints).
    PartiallyOrderedSubtaskDef,

    /// Task ID used to reference subtasks.
    TaskID,

    /// Collection of task ordering constraints.
    TaskOrderingConstraintDef,

    /// A single task ordering constraint.
    TaskOrderingConstraint,

    /// Collection of logical constraints on tasks.
    TaskLogicalConstraintDef,

    /// Represents a task network in HDDL.
    TaskNetworkDef,

    /// Initial task network of the HDDL problem.
    InitialTaskNetwork,

}

impl Kind {
    /// Converts an `AstKind` into the corresponding `Kind` (RenderKind).
    pub fn from_ast_kind(ast: AstKind) -> Kind {
        match ast {
            AstKind::FunctionSymbol => Kind::FunctionSymbol,
            AstKind::PrimitiveType => Kind::PrimitiveType,
            AstKind::DomainName => Kind::DomainName,
            AstKind::ProblemName => Kind::ProblemName,
            AstKind::Predicate => Kind::Predicate,
            AstKind::ActionSymbol => Kind::ActionSymbol,
            AstKind::DASymbol => Kind::DASymbol,
            AstKind::TaskSymbol => Kind::TaskSymbol,
            AstKind::PrefName => Kind::PrefName,
            AstKind::RequireDef => Kind::RequireDef,
            AstKind::Requirement => Kind::Requirement,
            AstKind::Type => Kind::Type,
            AstKind::TypedList => Kind::TypedList,
            AstKind::TypedItem => Kind::TypedItem,
            AstKind::TypedItemElements => Kind::TypedItemElements,
            AstKind::TypesDef => Kind::TypesDef,
            AstKind::ConstantsDef => Kind::ConstantsDef,
            AstKind::ObjectsDef => Kind::ObjectsDef,
            AstKind::Domain => Kind::Domain,
            AstKind::Problem => Kind::Problem,
            AstKind::PredicatesDef => Kind::PredicatesDef,
            AstKind::AtomicFormulaSkeleton => Kind::AtomicFormulaSkeleton,
            AstKind::FunctionsDef => Kind::FunctionsDef,
            AstKind::FunctionTerm => Kind::FunctionTerm,
            AstKind::AtomicFunctionSkeleton => Kind::AtomicFunctionSkeleton,
            AstKind::Number => Kind::Number,
            AstKind::ActionDef => Kind::ActionDef,
            AstKind::ParametersDef => Kind::ParametersDef,
            AstKind::DurativeActionDef => Kind::DurativeActionDef,
            AstKind::ActionDefBody => Kind::ActionDefBody,
            AstKind::PreconditionDef => Kind::PreconditionDef,
            AstKind::EffectDef => Kind::EffectDef,
            AstKind::DADefBody => Kind::DADefBody,
            AstKind::DerivedDef => Kind::DerivedDef,
            AstKind::AtomicFormula => Kind::AtomicFormula,
            AstKind::And => Kind::And,
            AstKind::Or => Kind::Or,
            AstKind::Not => Kind::Not,
            AstKind::Imply => Kind::Imply,
            AstKind::Forall => Kind::Forall,
            AstKind::Exists => Kind::Exists,
            AstKind::Preference => Kind::Preference,
            AstKind::When => Kind::When,
            AstKind::FComp => Kind::FComp,
            AstKind::Assign => Kind::Assign,
            AstKind::Operation => Kind::Operation,
            AstKind::Constraints => Kind::Constraints,
            AstKind::AtStart => Kind::AtStart,
            AstKind::AtEnd => Kind::AtEnd,
            AstKind::Overall => Kind::Overall,
            AstKind::Always => Kind::Always,
            AstKind::Sometime => Kind::Sometime,
            AstKind::Within => Kind::Within,
            AstKind::AtMostOnce => Kind::AtMostOnce,
            AstKind::SometimeAfter => Kind::SometimeAfter,
            AstKind::SometimeBefore => Kind::SometimeBefore,
            AstKind::AlwaysWithin => Kind::AlwaysWithin,
            AstKind::HoldDuring => Kind::HoldDuring,
            AstKind::HoldAfter => Kind::HoldAfter,
            AstKind::Init => Kind::Init,
            AstKind::TimedInitialLiteral => Kind::TimedInitialLiteral,
            AstKind::Goal => Kind::Goal,
            AstKind::Metric => Kind::Metric,
            AstKind::TotalTime => Kind::TotalTime,
            AstKind::IsViolated => Kind::IsViolated,
            AstKind::Length => Kind::Length,
            AstKind::Serial => Kind::Serial,
            AstKind::Parallel => Kind::Parallel,
            AstKind::Error => Kind::Error,
            AstKind::Task => Kind::Task,
            AstKind::TaskDef => Kind::TaskDef,
            AstKind::TaggedTask => Kind::TaggedTask,
            AstKind::MethodDef => Kind::MethodDef,
            AstKind::MethodSymbol => Kind::MethodSymbol,
            AstKind::MethodPreconditionDef => Kind::MethodPreconditionDef,
            AstKind::MethodDefBody => Kind::MethodDefBody,
            AstKind::OrderedSubtaskDef => Kind::OrderedSubtaskDef,
            AstKind::PartiallyOrderedSubtaskDef => Kind::PartiallyOrderedSubtaskDef,
            AstKind::TaskID => Kind::TaskID,
            AstKind::TaskOrderingConstraintDef => Kind::TaskOrderingConstraintDef,
            AstKind::TaskOrderingConstraint => Kind::TaskOrderingConstraint,
            AstKind::TaskLogicalConstraintDef => Kind::TaskLogicalConstraintDef,
            AstKind::TaskNetworkDef => Kind::TaskNetworkDef,
            AstKind::InitialTaskNetwork => Kind::InitialTaskNetwork,
            AstKind::Constant => Kind::Constant,
            AstKind::Variable => Kind::Variable,
        }
    }

    /// Converts an `ExprKind` into the corresponding `Kind` (RenderKind).
    pub fn from_expr_kind(expr: ExprKind) -> Kind {
        match expr {
            ExprKind::Constant => Kind::Constant,
            ExprKind::Variable => Kind::Variable,
            ExprKind::FunctionSymbol => Kind::FunctionSymbol,
            ExprKind::PrimitiveType => Kind::PrimitiveType,
            ExprKind::Predicate => Kind::Predicate,
            ExprKind::TaskSymbol => Kind::TaskSymbol,
            ExprKind::PrefName => Kind::PrefName,
            ExprKind::Type => Kind::Type,
            ExprKind::TypedList => Kind::TypedList,
            ExprKind::TypedSymbol => Kind::TypedItem,
            ExprKind::FunctionTerm => Kind::FunctionTerm,
            ExprKind::Number => Kind::Number,
            ExprKind::AtomicFormula => Kind::AtomicFormula,
            ExprKind::And => Kind::And,
            ExprKind::Or => Kind::Or,
            ExprKind::Not => Kind::Not,
            ExprKind::Imply => Kind::Imply,
            ExprKind::Forall => Kind::Forall,
            ExprKind::Exists => Kind::Exists,
            ExprKind::Preference => Kind::Preference,
            ExprKind::When => Kind::When,
            ExprKind::FComp => Kind::FComp,
            ExprKind::Assign => Kind::Assign,
            ExprKind::Operation => Kind::Operation,
            ExprKind::AtStart => Kind::AtStart,
            ExprKind::AtEnd => Kind::AtEnd,
            ExprKind::Overall => Kind::Overall,
            ExprKind::Always => Kind::Always,
            ExprKind::Sometime => Kind::Sometime,
            ExprKind::Within => Kind::Within,
            ExprKind::AtMostOnce => Kind::AtMostOnce,
            ExprKind::SometimeAfter => Kind::SometimeAfter,
            ExprKind::SometimeBefore => Kind::SometimeBefore,
            ExprKind::AlwaysWithin => Kind::AlwaysWithin,
            ExprKind::HoldDuring => Kind::HoldDuring,
            ExprKind::HoldAfter => Kind::HoldAfter,
            ExprKind::TimedInitialLiteral => Kind::TimedInitialLiteral,
            ExprKind::Metric => Kind::Metric,
            ExprKind::TotalTime => Kind::TotalTime,
            ExprKind::IsViolated => Kind::IsViolated,
            ExprKind::Length => Kind::Length,
            ExprKind::Serial => Kind::Serial,
            ExprKind::Parallel => Kind::Parallel,
            ExprKind::Task => Kind::Task,
            ExprKind::TaskID => Kind::TaskID,
            ExprKind::TaggedTask => Kind::TaggedTask,
            ExprKind::TaskOrderingConstraint => Kind::TaskOrderingConstraint,
        }
    }
}

impl fmt::Display for Kind {
    /// Formats the `Kind` variant as a human-readable string.
    ///
    /// Called when formatting with `{}` (e.g., in `println!` or `format!`).
    ///
    /// # Example
    ///
    /// ```
    /// let kind = Kind::Constant;
    /// println!("Kind: {}", kind); // prints "Kind: Constant"
    /// ```
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Kind::Constant => "Constant",
            Kind::Variable => "Variable",
            Kind::FunctionSymbol => "FunctionSymbol",
            Kind::PrimitiveType => "PrimitiveType",
            Kind::DomainName => "DomainName",
            Kind::ProblemName => "ProblemName",
            Kind::Predicate => "Predicate",
            Kind::ActionSymbol => "ActionSymbol",
            Kind::DASymbol => "DASymbol",
            Kind::TaskSymbol => "TaskSymbol",
            Kind::PrefName => "PrefName",
            Kind::RequireDef => "RequireDef",
            Kind::Requirement => "Requirement",
            Kind::Type => "Type",
            Kind::TypedList => "TypedList",
            Kind::TypedItem => "TypedItem",
            Kind::TypedItemElements => "TypedItemElements",
            Kind::TypesDef => "TypesDef",
            Kind::ConstantsDef => "ConstantsDef",
            Kind::ObjectsDef => "ObjectsDef",
            Kind::Domain => "Domain",
            Kind::Problem => "Problem",
            Kind::PredicatesDef => "PredicatesDef",
            Kind::AtomicFormulaSkeleton => "AtomicFormulaSkeleton",
            Kind::FunctionsDef => "FunctionsDef",
            Kind::FunctionTerm => "FunctionTerm",
            Kind::AtomicFunctionSkeleton => "AtomicFunctionSkeleton",
            Kind::Number => "Number",
            Kind::ActionDef => "ActionDef",
            Kind::DurativeActionDef => "DurativeActionDef",
            Kind::ActionDefBody => "ActionDefBody",
            Kind::PreconditionDef => "PreconditionDef",
            Kind::EffectDef => "EffectDef",
            Kind::DADefBody => "DADefBody",
            Kind::DerivedDef => "DerivedDef",
            Kind::AtomicFormula => "AtomicFormula",
            Kind::And => "And",
            Kind::Or => "Or",
            Kind::Not => "Not",
            Kind::Imply => "Imply",
            Kind::Forall => "Forall",
            Kind::Exists => "Exists",
            Kind::Preference => "Preference",
            Kind::When => "When",
            Kind::FComp => "FComp",
            Kind::Assign => "Assign",
            Kind::Operation => "Operation",
            Kind::Constraints => "Constraints",
            Kind::AtStart => "AtStart",
            Kind::AtEnd => "AtEnd",
            Kind::Overall => "Overall",
            Kind::Always => "Always",
            Kind::Sometime => "Sometime",
            Kind::Within => "Within",
            Kind::AtMostOnce => "AtMostOnce",
            Kind::SometimeAfter => "SometimeAfter",
            Kind::SometimeBefore => "SometimeBefore",
            Kind::AlwaysWithin => "AlwaysWithin",
            Kind::HoldDuring => "HoldDuring",
            Kind::HoldAfter => "HoldAfter",
            Kind::Init => "Init",
            Kind::TimedInitialLiteral => "TimedInitialLiteral",
            Kind::Goal => "Goal",
            Kind::Metric => "Metric",
            Kind::TotalTime => "TotalTime",
            Kind::IsViolated => "IsViolated",
            Kind::Length => "Length",
            Kind::Serial => "Serial",
            Kind::Parallel => "Parallel",
            Kind::Error => "Error",
            Kind::Task => "Task",
            Kind::TaskDef => "TaskDef",
            Kind::TaggedTask => "TaggedTask",
            Kind::MethodDef => "MethodDef",
            Kind::MethodDefBody => "MethodDefBody",
            Kind::MethodSymbol => "MethodSymbol",
            Kind::MethodPreconditionDef => "MethodPreconditionDef",
            Kind::OrderedSubtaskDef => "OrderedSubtaskDef",
            Kind::PartiallyOrderedSubtaskDef => "PartiallyOrderedSubtaskDef",
            Kind::TaskID => "TaskID",
            Kind::TaskOrderingConstraintDef => "TaskOrderingConstraintDef",
            Kind::TaskOrderingConstraint => "TaskOrderingConstraint",
            Kind::TaskLogicalConstraintDef => "TaskLogicalConstraintDef",
            Kind::TaskNetworkDef => "TaskNetworkDef",
            Kind::InitialTaskNetwork => "InitialTaskNetwork",
            Kind::ParametersDef => "ParametersDef",
        };
        write!(f, "{}", s)
    }
}

impl SyntaxInternerDisplay for Kind {
    /// Formats the `Kind` enum as a syntax keyword string with indentation.
    ///
    /// This method writes an appropriate keyword corresponding to the variant of
    /// `Kind` to the given formatter `f`. The output includes indentation based on
    /// the `indent` parameter (number of spaces).
    ///
    /// The `_interner` parameter is currently unused but may be used for
    /// future implementations involving string interning or symbol resolution.
    ///
    /// # Parameters
    /// - `f`: The formatter to write the output string.
    /// - `_interner`: A reference to the string interner (unused).
    /// - `indent`: The indentation level (number of spaces) to prefix the keyword.
    ///
    /// # Returns
    /// A `fmt::Result` indicating success or failure during formatting.
    ///
    /// # Examples
    /// ```rust
    /// use std::fmt::Write;
    /// let kind = Kind::ActionDef;
    /// let mut output = String::new();
    /// kind.fmt_syntax_with_indent(&mut output, &interner, 4).unwrap();
    /// assert_eq!(output, "    action");
    /// ```
    fn fmt_syntax_with_interner_and_indent(
        &self,
        f: &mut fmt::Formatter<'_>,
        _interner: &StringInterner,
        indent: usize,
    ) -> fmt::Result {
        write_indent(f, indent)?;
        let s = match self {
            Kind::Constant => "",
            Kind::Variable => "",
            Kind::FunctionSymbol => "",
            Kind::PrimitiveType => "",
            Kind::DomainName => "",
            Kind::ProblemName => "",
            Kind::Predicate => "",
            Kind::ActionSymbol => "",
            Kind::DASymbol => "",
            Kind::TaskSymbol => "",
            Kind::PrefName => "",
            Kind::RequireDef => REQUIREMENTS,
            Kind::Requirement => "",
            Kind::Type => "",
            Kind::TypedList => "",
            Kind::TypedItem => "",
            Kind::TypedItemElements => "",
            Kind::TypesDef => TYPES,
            Kind::ConstantsDef => CONSTANTS,
            Kind::ObjectsDef => OBJECTS,
            Kind::Domain => DOMAIN_DEF,
            Kind::Problem => PROBLEM,
            Kind::PredicatesDef => PREDICATES,
            Kind::AtomicFormulaSkeleton => "",
            Kind::FunctionsDef => FUNCTIONS,
            Kind::FunctionTerm => "",
            Kind::AtomicFunctionSkeleton => "",
            Kind::Number => "",
            Kind::ActionDef => ACTION,
            Kind::DurativeActionDef => DURATIVE_ACTION,
            Kind::ActionDefBody => "",
            Kind::PreconditionDef => PRECONDITION,
            Kind::EffectDef => EFFECT,
            Kind::DADefBody => "",
            Kind::DerivedDef => DERIVED,
            Kind::AtomicFormula => "",
            Kind::And => AND,
            Kind::Or => OR,
            Kind::Not => NOT,
            Kind::Imply => IMPLY,
            Kind::Forall => FORALL,
            Kind::Exists => EXISTS,
            Kind::Preference => PREFERENCE,
            Kind::When => WHEN,
            Kind::FComp => "",
            Kind::Assign => ASSIGN,
            Kind::Operation => "",
            Kind::Constraints => CONSTRAINTS,
            Kind::AtStart => AT_START,
            Kind::AtEnd => AT_END,
            Kind::Overall => OVERALL,
            Kind::Always => ALWAYS,
            Kind::Sometime => SOMETIME,
            Kind::Within => WITHIN,
            Kind::AtMostOnce => AT_MOST_ONCE,
            Kind::SometimeAfter => SOMETIME_AFTER,
            Kind::SometimeBefore => SOMETIME_BEFORE,
            Kind::AlwaysWithin => ALWAYS_WITHIN,
            Kind::HoldDuring => HOLD_DURING,
            Kind::HoldAfter => HOLD_AFTER,
            Kind::Init => INIT,
            Kind::TimedInitialLiteral => "",
            Kind::Goal => GOAL,
            Kind::Metric => METRIC,
            Kind::TotalTime => TOTAL_TIME,
            Kind::IsViolated => IS_VIOLATED,
            Kind::Length => LENGTH,
            Kind::Serial => SERIAL,
            Kind::Parallel => PARALLEL,
            Kind::Error => "Error",
            Kind::Task => "",
            Kind::TaskDef => TASK,
            Kind::TaggedTask => "",
            Kind::MethodDef => METHOD,
            Kind::MethodDefBody => "",
            Kind::MethodSymbol => "",
            Kind::MethodPreconditionDef => PRECONDITION,
            Kind::OrderedSubtaskDef => ORDERED_SUBTASKS,
            Kind::PartiallyOrderedSubtaskDef => SUBTASKS,
            Kind::TaskID => "",
            Kind::TaskOrderingConstraintDef => ORDERED_TASKS,
            Kind::TaskOrderingConstraint => TASK,
            Kind::TaskLogicalConstraintDef => CONSTRAINTS,
            Kind::TaskNetworkDef => "",
            Kind::InitialTaskNetwork => HTN,
            Kind::ParametersDef => PARAMETERS,
        };
        write!(f, "{}", s)
    }
}
