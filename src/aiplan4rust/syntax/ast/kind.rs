//! The `kind` module: defines the `Kind` enum representing syntax kinds
//! used in the abstract syntax tree (AST) for syntax domain/problem
//! languages like PDDL and HDDL.
//!
//! This includes implementations of the standard `Display` trait and
//! a custom `SyntaxDisplay` trait for language-specific formatted output,
//! including indentation and keywords.
//!

use crate::aiplan4rust::interner::SymbolInterner;
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

/// Represents the different kinds of nodes in an Abstract Syntax Tree (AST).
///
/// This enum models various components found in do©main or problem specifications
/// of syntax problems, typically for domain-specific languages such as PDDL.
///
/// Each variant corresponds to a specific syntactic or semantic element of a syntax
/// problem description. This structured representation facilitates parsing, manipulation,
/// and analysis within syntax frameworks.
///
/// # Usage Examples
///
/// - Representing constants, variables, types, predicates, actions.
/// - Logical constructs like `And`, `Or`, `Not`.
/// - Supporting extensions like the HDDL dialect with tasks, methods, and constraints.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum Kind {
    /// A constant value in the syntax problem (literal or fixed value).
    Object,

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

    /// A predicate symbol used in logical logic or conditions.
    PredicateSymbol,

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

    /// Represents a term in a function logic.
    Function,

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
    Comparison,

    /// Assignment operation.
    Assignment,

    /// Arithmetic or logical operation.
    Arithmetic,

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

    /// Tagged task with an label in HDDL.
    LabeledTask,

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

    /// Task label used to reference subtasks.
    TaskLabel,

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
    /// Returns the syntax string for this `Kind` wrapped in single quotes,
    /// reusing `fmt_syntax_with_interner_and_indent` to avoid duplicating the mapping.
    pub fn to_syntax_string(&self) -> String {
        let s = match self {
            Kind::Object => "",
            Kind::Variable => "",
            Kind::FunctionSymbol => "",
            Kind::PrimitiveType => "",
            Kind::DomainName => "",
            Kind::ProblemName => "",
            Kind::PredicateSymbol => "",
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
            Kind::Function => "",
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
            Kind::Comparison => "",
            Kind::Assignment => ASSIGN,
            Kind::Arithmetic => "",
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
            Kind::LabeledTask => "",
            Kind::MethodDef => METHOD,
            Kind::MethodDefBody => "",
            Kind::MethodSymbol => "",
            Kind::MethodPreconditionDef => PRECONDITION,
            Kind::OrderedSubtaskDef => ORDERED_SUBTASKS,
            Kind::PartiallyOrderedSubtaskDef => SUBTASKS,
            Kind::TaskLabel => "",
            Kind::TaskOrderingConstraintDef => ORDERED_TASKS,
            Kind::TaskOrderingConstraint => TASK,
            Kind::TaskLogicalConstraintDef => CONSTRAINTS,
            Kind::TaskNetworkDef => "",
            Kind::InitialTaskNetwork => HTN,
            Kind::ParametersDef => PARAMETERS,
        };
        s.to_string()
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
            Kind::Object => "Object",
            Kind::Variable => "Variable",
            Kind::FunctionSymbol => "FunctionSymbol",
            Kind::PrimitiveType => "PrimitiveType",
            Kind::DomainName => "DomainName",
            Kind::ProblemName => "ProblemName",
            Kind::PredicateSymbol => "Predicate",
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
            Kind::Function => "FunctionTerm",
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
            Kind::Comparison => "FComp",
            Kind::Assignment => "Assign",
            Kind::Arithmetic => "Operation",
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
            Kind::LabeledTask => "TaggedTask",
            Kind::MethodDef => "MethodDef",
            Kind::MethodDefBody => "MethodDefBody",
            Kind::MethodSymbol => "MethodSymbol",
            Kind::MethodPreconditionDef => "MethodPreconditionDef",
            Kind::OrderedSubtaskDef => "OrderedSubtaskDef",
            Kind::PartiallyOrderedSubtaskDef => "PartiallyOrderedSubtaskDef",
            Kind::TaskLabel => "TaskID",
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
        _interner: &SymbolInterner,
        indent: usize,
    ) -> fmt::Result {
        write_indent(f, indent)?;
        let s = self.to_syntax_string();
        write!(f, "{}", s)
    }
}
