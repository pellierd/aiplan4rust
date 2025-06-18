use serde::de::Visitor;
use serde::Deserialize;
use serde::Deserializer;
use serde::Serialize;
use serde::Serializer;

use std::fmt;

/// Represents the different types of Abstract Syntax Tree (AST) nodes.
/// This enum is used to represent various components of a domain or problem specification in a
/// planning problem, typically for a domain-specific language used in planning problems (e.g.,
/// PDDL).
///
/// This enum is used as part of a larger planning framework, where each variant represents a
/// distinct component or structure in the domain or problem specification. It allows the
/// representation and manipulation of different parts of a planning problem or domain in a
/// structured and organized way.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum Kind {
    #[default]
    None,
    /// Represents a constant value in the planning problem, typically a literal or a fixed value.
    Constant,
    /// Represents a variable used in actions or predicates, typically a placeholder for values.
    Variable,
    /// Represents a function symbol, typically used in mathematical functions or expressions.
    FunctionSymbol,
    /// Represents a basic data type, such as integers or booleans, used in the planning problem.
    PrimitiveType,
    /// Represents the name of the domain in a PDDL file.
    DomainName,
    /// Represents the name of the problem in a PDDL file.
    ProblemName,
    /// Represents a predicate symbol used in logical expressions or actions.
    Predicate,
    /// Represents an action symbol used in the problem specification.
    ActionSymbol,
    /// Represents a symbol used for defining Durative Actions.
    DASymbol,
    /// Represents a task symbol used in the domain specification.
    /// Add to deal with HDDL dialect
    TaskSymbol,
    /// Represents a preference name used in the planning problem.
    PrefName,
    /// Represents a requirement definition (e.g., `(:require ...)` in PDDL).
    RequireDef,
    /// Represents a requirement, such as specific features or constraints of a problem.
    Requirement,
    /// Represents a type definition, which may be used to define object types.
    Type,
    /// A list of typed elements containing multiple `TypedItem`s.
    TypedList,
    /// A single typed item pairing elements with a type.
    TypedItem,
    /// The untyped elements part of a `TypedItem`.
    TypedItemElements,
    /// Represents the definition of types, often seen in the domain description.
    TypesDef,
    /// Represents the definition of constants in the problem.
    ConstantsDef,
    /// Represents the definition of objects used in the domain or problem.
    ObjectsDef,
    /// Represents the domain in the problem specification.
    Domain,
    /// Represents the problem in the planning task, usually containing a set of goals and initial
    /// states.
    Problem,
    /// Represents the definition of predicates used in the domain and problem.
    PredicatesDef,
    /// Represents the structure of an atomic formula in the logic system.
    AtomicFormulaSkeleton,
    /// Represents the definition of functions in the domain or problem.
    FunctionsDef,
    /// Represents a term used in a function.
    FunctionTerm,
    /// Represents the skeleton structure of an atomic function.
    AtomicFunctionSkeleton,
    /// Represents a number, using `OrderedFloat` for precise float comparison and serialization.
    Number,
    /// Represents the definition of an action in the domain.
    ActionDef,
    /// Represents the definition of a durative action, where actions have durations.
    DurativeActionDef,
    /// Represents the body of an action definition, including preconditions and effects.
    ActionDefBody,
    /// Represents the preconditions of an action.
    PreconditionDef,
    /// Represents the effects of an action.
    EffectDef,
    /// Represents the body of a durative action definition.
    DADefBody,
    /// Represents a derived predicate or function in the domain.
    DerivedDef,
    /// Represents a simple formula in the logical system, typically involving predicates and
    /// constants.
    AtomicFormula,
    /// Represents the logical "and" operator, used in formulas to combine conditions.
    And,
    /// Represents the logical "or" operator, used to combine alternative conditions.
    Or,
    /// Represents the logical "not" operator, negating a condition.
    Not,
    /// Represents the logical implication operator, expressing "if... then..." conditions.
    Imply,
    /// Represents the universal quantifier in logic, indicating that a condition holds for all
    /// instances of a variable.
    Forall,
    /// Represents the existential quantifier in logic, indicating that a condition holds for some
    /// instance of a variable.
    Exists,
    /// Represents a preference condition, indicating that a particular solution or path is
    /// preferred.
    Preference,
    /// Represents the conditional timing of an event or action in a planning problem.
    When,
    /// Represents a binary comparison, used for mathematical or logical comparisons.
    FComp,
    /// Represents an assignment operation in the planning problem.
    Assign,
    /// Represents an arithmetic operation, such as addition or subtraction.
    Operation,
    /// Represents constraints imposed on the problem or domain.
    Constraints,
    /// Represents a condition that must hold at the start of the action.
    AtStart,
    /// Represents a condition that must hold at the end of the action.
    AtEnd,
    /// Represents a condition that must hold throughout the duration of the action.
    Overall,
    /// Represents a condition that must always hold.
    Always,
    /// Represents a condition that holds at some point during the execution.
    Sometime,
    /// Represents a condition that holds within a certain time frame or duration.
    Within,
    /// Represents a condition that can hold at most once during the execution.
    AtMostOnce,
    /// Represents a condition that holds after some event or time.
    SometimeAfter,
    /// Represents a condition that holds before some event or time.
    SometimeBefore,
    /// Represents a condition that must always hold within a specific time frame.
    AlwaysWithin,
    /// Represents a condition that must hold during a specific period of time.
    HoldDuring,
    /// Represents a condition that must hold after a specific event or time.
    HoldAfter,
    /// Represents the initial conditions or state of the problem.
    Init,
    /// Represents an initial literal with a specific timing condition.
    TimedInitialLiteral,
    /// Represents the goal conditions that the planner must achieve.
    Goal,
    /// Represents a metric used to optimize a solution, where the optimization is either to
    /// minimize or maximize a value.
    Metric,
    /// Represents the total time in the problem.
    TotalTime,
    /// Represents a condition that indicates whether something is violated in the problem.
    IsViolated,
    /// Represents the length or duration of something in the planning problem.
    Length,
    /// Represents a serial timing or duration, serialized as an `OrderedFloat` for precision.
    Serial,
    /// Represents parallel timing or duration, serialized as an `OrderedFloat` for precision.
    Parallel,
    /// Represents an error, often used to signal a failure or problem in the domain or problem
    /// definition.
    Error,

    //////////////////////////////////////////////////////////////////////////////////
    // HDDL Dialect
    /// Represents a task in HDDL, which is a basic unit of work that can be executed.
    Task,
    /// Represents the definition of a task in HDDL.
    TaskDef,
    /// Represents a tagged task in HDDL, i.e., a task with its id.
    TaggedTask,
    /// Represents the definition of a method in HDDL, specifying how a compound task can be
    /// decomposed.
    MethodDef,
    /// Represents a symbolic reference to a method in HDDL.
    MethodSymbol,
    /// Represents the definition of the method precondition in HDDL.
    MethodPreconditionDef,
    /// Represents the body of a method definition in HDDL.
    MethodDefBody,
    /// Represents a list of ordered subtasks in a method in HDDL, where the order of execution
    /// matters.
    OrderedSubtaskDef,
    /// Represents a list of partially ordered subtasks in a method in HDDL, where some order
    /// constraints exist but not all.
    PartiallyOrderedSubtaskDef,
    /// Represents a task ID to reference subtask in HDDL.
    TaskID,
    /// Represents a collection of task ordering constraints.
    TaskOrderingConstraintDef,
    /// Represents a task ordering constraint in HDDL
    TaskOrderingConstraint,
    /// Represents a collection of logical constraints in HDDL.
    TaskLogicalConstraintDef,
    /// Represents a task network in HDDL.
    TaskNetworkDef,
    /// Represents the initial task network of the HDDL problem.
    InitialTaskNetwork,
}

impl fmt::Display for Kind {
    /// Implements the `fmt::Display` trait for `AstKind` to allow for human-readable string
    /// representations of various `AstKind` variants.
    ///
    /// This function is called when using the `{}` format specifier in macros like `println!`,
    /// `format!`, or `write!`. It matches on the `AstKind` enum and writes an appropriate
    /// string representation for each variant to the provided `fmt::Formatter`.
    ///
    /// # Parameters
    /// - `f`: A mutable reference to a `fmt::Formatter` used to output the formatted result.
    ///
    /// # Returns
    /// - Returns a `fmt::Result`, which indicates whether the formatting operation was successful.
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Kind::None => write!(f, "None"),
            Kind::Constant => write!(f, "Constant"),
            Kind::Variable => write!(f, "Variable"),
            Kind::FunctionSymbol => write!(f, "FunctionSymbol"),
            Kind::FunctionTerm => write!(f, "FunctionTerm"),
            Kind::PrimitiveType => write!(f, "PrimitiveType"),
            Kind::DomainName => write!(f, "DomainName"),
            Kind::ProblemName => write!(f, "ProblemName"),
            Kind::Predicate => write!(f, "Predicate"),
            Kind::ActionSymbol => write!(f, "ActionSymbol"),
            Kind::DASymbol => write!(f, "DASymbol"),
            Kind::PrefName => write!(f, "PrefName"),
            Kind::Number => write!(f, "Number"),
            Kind::RequireDef => write!(f, "RequireDef"),
            Kind::Requirement => write!(f, "Requirement"),
            Kind::Type => write!(f, "Type"),
            Kind::TypedList => write!(f, "TypedList"),
            Kind::TypedItem => write!(f, "TypedItem"),
            Kind::TypedItemElements => write!(f, "TypedItemElements"),
            Kind::TypesDef => write!(f, "TypesDef"),
            Kind::ConstantsDef => write!(f, "ConstantsDef"),
            Kind::ObjectsDef => write!(f, "ObjectsDef"),
            Kind::PredicatesDef => write!(f, "PredicatesDef"),
            Kind::AtomicFormulaSkeleton => write!(f, "AtomicFormulaSkeleton"),
            Kind::FunctionsDef => write!(f, "FunctionsDef"),
            Kind::AtomicFunctionSkeleton => write!(f, "AtomicFunctionSkeleton"),
            Kind::ActionDef => write!(f, "ActionDef"),
            Kind::PreconditionDef => write!(f, "PreconditionDef"),
            Kind::EffectDef => write!(f, "EffectDef"),
            Kind::DurativeActionDef => write!(f, "DurativeActionDef"),
            Kind::ActionDefBody => write!(f, "ActionDefBody"),
            Kind::DADefBody => write!(f, "DADefBody"),
            Kind::DerivedDef => write!(f, "DerivedDef"),
            Kind::Preference => write!(f, "Preference"),
            Kind::AtomicFormula => write!(f, "AtomicFormula"),
            Kind::Not => write!(f, "Not"),
            Kind::Domain => write!(f, "Domain"),
            Kind::Problem => write!(f, "Problem"),
            Kind::Forall => write!(f, "Forall"),
            Kind::Exists => write!(f, "Exists"),
            Kind::When => write!(f, "When"),
            Kind::FComp => write!(f, "FComp"),
            Kind::Assign => write!(f, "Assign"),
            Kind::Operation => write!(f, "Op"),
            Kind::And => write!(f, "And"),
            Kind::Or => write!(f, "Or"),
            Kind::Imply => write!(f, "Imply"),
            Kind::AtStart => write!(f, "AtStart"),
            Kind::AtEnd => write!(f, "AtEnd"),
            Kind::Overall => write!(f, "Overall"),
            Kind::Constraints => write!(f, "Constraints"),
            Kind::Always => write!(f, "Always"),
            Kind::Sometime => write!(f, "Sometime"),
            Kind::Within => write!(f, "Within"),
            Kind::AtMostOnce => write!(f, "AtMostOnce"),
            Kind::SometimeAfter => write!(f, "SometimeAfter"),
            Kind::SometimeBefore => write!(f, "SometimeBefore"),
            Kind::AlwaysWithin => write!(f, "AlwaysWithin"),
            Kind::HoldDuring => write!(f, "HoldDuring"),
            Kind::HoldAfter => write!(f, "HoldAfter"),
            Kind::Init => write!(f, "Init"),
            Kind::TimedInitialLiteral => write!(f, "TimedInitialLiteral"),
            Kind::Goal => write!(f, "Goal"),
            Kind::Metric => write!(f, "Metric"),
            Kind::TotalTime => write!(f, "TotalTime"),
            Kind::IsViolated => write!(f, "IsViolated"),
            Kind::Length => write!(f, "Length"),
            Kind::Serial => write!(f, "Serial"),
            Kind::Parallel => write!(f, "Parallel"),
            Kind::Error => write!(f, "Error"),
            // Add for HDDL
            Kind::Task => write!(f, "Task"),
            Kind::TaggedTask => write!(f, "TaggedTask"),
            Kind::TaskDef => write!(f, "TaskDef"),
            Kind::TaskSymbol => write!(f, "TaskSymbol"),
            Kind::MethodDef => write!(f, "MethodDef"),
            Kind::MethodDefBody => write!(f, "MethodDefBody"),
            Kind::MethodSymbol => write!(f, "MethodSymbol"),
            Kind::MethodPreconditionDef => write!(f, "MethodPreconditionDef"),
            Kind::OrderedSubtaskDef => write!(f, "OrderedSubtaskDef"),
            Kind::PartiallyOrderedSubtaskDef => write!(f, "PartiallyOrderedSubtaskDef"),
            Kind::TaskID => write!(f, "TaskID"),
            Kind::TaskOrderingConstraintDef => write!(f, "TaskOrderingConstraintDef"),
            Kind::TaskOrderingConstraint => write!(f, "TaskOrderingConstraint"),
            Kind::TaskLogicalConstraintDef => write!(f, "TaskLogicalConstraintDef"),
            Kind::TaskNetworkDef => write!(f, "TaskNetworkDef"),
            Kind::InitialTaskNetwork => write!(f, "InitialTaskNetwork"),
        }
    }
}
