use crate::aiplan4rust::frontend::ParserInternalError;
use crate::aiplan4rust::pddl_display::PDDLDisplay;
use crate::aiplan4rust::syntax::span::Span;
use crate::aiplan4rust::syntax::token::*;
use ordered_float::OrderedFloat;
use serde::de::Visitor;
use serde::ser::Serializer;
use serde::{de, Deserialize, Deserializer, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::fmt::Write;
use std::hash::Hash;

/// # PDDL Requirements Enum
///
/// The `Requirement` enum represents the various features that a PDDL (Planning Domain Definition
/// Language) domain or problem can declare using the `:requirements` keyword. Each variant
/// corresponds to a specific feature that influences the expressiveness of the planning formalism.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Requirement {
    /// Represents the basic STRIPS formalism, which supports only add and delete effects.
    Strips,
    /// Enables the use of typed objects and variables.
    Typing,
    /// Allows the use of negated atoms in preconditions.
    NegativePreconditions,
    /// Supports logical OR (`or`) in preconditions.
    DisjunctivePreconditions,
    /// Enables the use of equality (`=`) in conditions.
    Equality,
    /// Supports existential quantification (`exists`) in preconditions.
    ExistentialPreconditions,
    /// Supports universal quantification (`forall`) in preconditions.
    UniversalPreconditions,
    /// A general requirement covering both existential and universal quantification.
    QuantifiedPreconditions,
    /// Allows conditional (`when`) effects in action definitions.
    ConditionalEffects,
    /// Enables numeric state variables (deprecated in favor of `NumericFluents`).
    Fluents,
    /// Supports numeric state variables and arithmetic expressions.
    NumericFluents,
    /// A general requirement encompassing `NegativePreconditions`, `DisjunctivePreconditions`,
    /// `Equality`, `ExistentialPreconditions`, `UniversalPreconditions`, and `ConditionalEffects`.
    Adl,
    /// Allows the definition of durative (temporal) actions.
    DurativeActions,
    /// Enables constraints on action durations.
    DurationInequalities,
    /// Supports continuous change effects over time.
    ContinuousEffects,
    /// Allows the definition of derived predicates (`:derived`).
    DerivedPredicates,
    /// Supports `at` literals for specifying facts that become true at a specific time.
    TimedInitialLiterals,
    /// Introduces soft constraints using `:preferences`.
    Preferences,
    /// Enables global constraints on plans.
    Constraints,
    /// Supports cost-based planning, where actions have associated costs.
    ActionCosts,
    /// Represents the marker for Hierarchical Task Network (HTN) definitions in HDDL.
    Hierarchy,
    /// Specifies method preconditions in HTN for HDDL.
    MethodPreconditions,
}

impl fmt::Display for Requirement {
    /// Formats the `Requirement` as its corresponding lexeme string.
    ///
    /// This implementation converts each variant of the `Requirement` enum into
    /// its predefined string representation, typically used in PDDL files.
    ///
    /// # Parameters
    /// - `f`: The formatter used to output the formatted string.
    ///
    /// # Returns
    /// - `fmt::Result`: The result of writing the formatted output.
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Requirement::Strips => write!(f, "{}", STRIPS),
            Requirement::Typing => write!(f, "{}", TYPING),
            Requirement::NegativePreconditions => write!(f, "{}", NEGATIVE_PRECONDITION),
            Requirement::DisjunctivePreconditions => write!(f, "{}", DISJUNCTIVE_PRECONDITION),
            Requirement::Equality => write!(f, "{}", EQUALITY),
            Requirement::ExistentialPreconditions => write!(f, "{}", EXISTENTIAL_PRECONDITIONS),
            Requirement::UniversalPreconditions => write!(f, "{}", UNIVERSAL_PRECONDITIONS),
            Requirement::QuantifiedPreconditions => write!(f, "{}", QUANTIFIED_PRECONDITIONS),
            Requirement::ConditionalEffects => write!(f, "{}", CONDITIONAL_EFFECTS),
            Requirement::Fluents => write!(f, "{}", FLUENTS),
            Requirement::NumericFluents => write!(f, "{}", NUMERIC_FLUENTS),
            Requirement::Adl => write!(f, "{}", ADL),
            Requirement::DurativeActions => write!(f, "{}", DURATIVE_ACTIONS),
            Requirement::DurationInequalities => write!(f, "{}", DURATIVE_INEQUALITIES),
            Requirement::ContinuousEffects => write!(f, "{}", CONDITIONAL_EFFECTS),
            Requirement::DerivedPredicates => write!(f, "{}", DERIVED_PREDICATES),
            Requirement::TimedInitialLiterals => write!(f, "{}", TIME_INITIAL_LITERALS),
            Requirement::Preferences => write!(f, "{}", PREFERENCES),
            Requirement::Constraints => write!(f, "{}", CONSTRAINTS),
            Requirement::ActionCosts => write!(f, "{}", ACTION_COSTS),
            Requirement::Hierarchy => write!(f, "{}", HIERARCHY),
            Requirement::MethodPreconditions => write!(f, "{}", METHOD_PRECONDITIONS),
        }
    }
}

impl PDDLDisplay for Requirement {
    /// Converts the `Requirement` into its PDDL-compliant string representation.
    ///
    /// This function leverages the `Display` trait implementation to generate
    /// the corresponding PDDL lexeme for a given `Requirement` variant.
    ///
    /// # Returns
    /// - A `String`
    fn to_pddl_string(&self) -> String {
        format!("{}", self)
    }
}

/// Represents binary comparison operators used in logical and numerical expressions.
///
/// This enumeration defines the common comparison operators that can be used in
/// conditions and constraints, particularly in the context of PDDL or other formal models.
///
/// # Variants
/// - `Greater` (`>`): Represents a "greater than" comparison.
/// - `Less` (`<`): Represents a "less than" comparison.
/// - `Equal` (`=`): Represents an "equal to" comparison.
/// - `GreaterEq` (`>=`): Represents a "greater than or equal to" comparison.
/// - `LessEq` (`<=`): Represents a "less than or equal to" comparison.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BinaryComp {
    /// Represents a "greater than" comparison (`>`).
    Greater,
    /// Represents a "less than" comparison (`<`).
    Less,
    /// Represents an "equal to" comparison (`=`).
    Equal,
    /// Represents a "greater than or equal to" comparison (`>=`).
    GreaterEq,
    /// Represents a "less than or equal to" comparison (`<=`).
    LessEq,
}

impl fmt::Display for BinaryComp {
    /// Formats the binary comparison operator as its standard string representation.
    ///
    /// This implementation ensures that the operator is displayed using its conventional
    /// mathematical notation (`>`, `<`, `=`, `>=`, `<=`).
    ///
    /// # Parameters
    /// - `f`: The formatter used to output the formatted string.
    ///
    /// # Returns
    /// - `fmt::Result`: The result of writing the formatted output.
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            BinaryComp::Greater => write!(f, "{}", GREATER),
            BinaryComp::Less => write!(f, "{}", LESS),
            BinaryComp::Equal => write!(f, "{}", EQUAL),
            BinaryComp::GreaterEq => write!(f, "{}", GREATER_EQ),
            BinaryComp::LessEq => write!(f, "{}", LESS_EQ),
        }
    }
}

impl PDDLDisplay for BinaryComp {}

/// Represents arithmetic operations that can be used in PDDL expressions.
///
/// This enumeration defines the basic arithmetic operators commonly found
/// in PDDL (Planning Domain Definition Language), which are used in numeric
/// expressions for modifying and evaluating numerical state variables.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ArithmeticOp {
    /// Represents the subtraction operation (`-`).
    Sub,
    /// Represents the division operation (`/`).
    Div,
    /// Represents the addition operation (`+`).
    Add,
    /// Represents the multiplication operation (`*`).
    Mul,
}

impl fmt::Display for ArithmeticOp {
    /// Implements the `fmt::Display` trait for the `ArithmeticOp` enum.
    ///
    /// This method allows `ArithmeticOp` to be formatted as a string when printed
    /// using formatting macros such as `println!`. It provides a string representation
    /// of the arithmetic operation, making it easier to display and debug arithmetic expressions.
    ///
    /// # Arguments
    /// - `f`: A mutable reference to the formatter, which is used to build the output string.
    ///
    /// # Returns
    /// - A `fmt::Result` indicating the success or failure of the formatting operation.
    ///
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ArithmeticOp::Sub => write!(f, "{}", SUB),
            ArithmeticOp::Div => write!(f, "{}", DIV),
            ArithmeticOp::Add => write!(f, "{}", ADD),
            ArithmeticOp::Mul => write!(f, "{}", MUL),
        }
    }
}

impl PDDLDisplay for ArithmeticOp {}

/// Represents assignment operations that can be used in planning and mathematical models.
///
/// This enum defines various types of assignment or modification operations that can be applied
/// to variables or parameters in a planning problem or formal model.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AssignOp {
    /// Basic assignment operation (sets a value).
    Assign,
    /// Scales up the value by a factor.
    ScaleUp,
    /// Scales down the value by a factor.
    ScaleDown,
    /// Increases the value by a certain amount.
    Increase,
    /// Decreases the value by a certain amount.
    Decrease,
}

impl fmt::Display for AssignOp {
    /// Formats the `AssignOp` enum as a string representation for display purposes.
    ///
    /// This implementation of the `fmt::Display` trait enables the `AssignOp` enum to be
    /// formatted into a user-friendly string representation for displaying to the user.
    /// Each variant of the `AssignOp` enum is mapped to a corresponding string to provide
    /// a clear and readable output when the enum is printed or logged.
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            AssignOp::Assign => write!(f, "{}", ASSIGN),
            AssignOp::ScaleUp => write!(f, "{}", SCALE_UP),
            AssignOp::ScaleDown => write!(f, "{}", SCALE_DOWN),
            AssignOp::Increase => write!(f, "{}", INCREASE),
            AssignOp::Decrease => write!(f, "{}", DECREASE),
        }
    }
}

impl PDDLDisplay for AssignOp {}

/// This enum represents the two possible types of optimization.
///
/// It derives several traits:
/// - `Clone`: Allows cloning of enum instances.
/// - `Debug`: Enables formatted output for debugging purposes.
/// - `PartialEq` and `Eq`: Allows comparison for equality between enum instances.
/// - `Hash`: Enables the calculation of a hash value for enum instances, useful in data structures
///     like `HashMap` or `HashSet`.
/// - `Serialize` and `Deserialize`: Allow the enum to be serialized and deserialized, facilitating
///     storage or transmission in formats like JSON.
///
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Optimization {
    /// Seeks to minimize the objective function.
    Minimize,
    /// Seeks to maximize the objective function.
    Maximize,
}

impl fmt::Display for Optimization {
    /// This method implements the `fmt::Display` trait for the `Optimization` enum.
    /// It allows an instance of `Optimization` to be formatted as a string for printing or logging
    /// purposes.
    ///
    /// # Arguments
    /// - `f`: A mutable reference to a `fmt::Formatter` that will be used to format the output
    ///     string.
    ///
    /// # Returns
    /// - Returns a `fmt::Result`, which indicates whether the formatting operation was successful.
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Optimization::Minimize => write!(f, "{}", MINIMIZE),
            Optimization::Maximize => write!(f, "{}", MAXIMIZE),
        }
    }
}

impl PDDLDisplay for Optimization {}

/// Represents the different types of Abstract Syntax Tree (AST) nodes.
/// This enum is used to represent various components of a domain or problem specification in a
/// planning problem, typically for a domain-specific language used in planning problems (e.g.,
/// PDDL).
///
/// This enum is used as part of a larger planning framework, where each variant represents a
/// distinct component or structure in the domain or problem specification. It allows the
/// representation and manipulation of different parts of a planning problem or domain in a
/// structured and organized way.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AstKind {
    /// Represents a constant value in the planning problem, typically a literal or a fixed value.
    Constant(String),
    /// Represents a variable used in actions or predicates, typically a placeholder for values.
    Variable(String),
    /// Represents a function symbols, typically used in mathematical functions or expressions.
    FunctionSymbol(String),
    /// Represents a basic data type, such as integers or booleans, used in the planning problem.
    PrimitiveType(String),
    /// Represents the name of the domain in a PDDL file.
    DomainName(String),
    /// Represents the name of the problem in a PDDL file.
    ProblemName(String),
    /// Represents a predicate symbols used in logical expressions or actions.
    Predicate(String),
    /// Represents an action symbols used in the problem specification.
    ActionSymbol(String),
    /// Represents a symbol used for defining Durative Actions.
    DASymbol(String),
    /// Represents a task symbol used in the domain specification.
    /// Add to deal with HDDL dialect
    TaskSymbol(String),
    /// Represents a preference name used in the planning problem.
    PrefName(String),
    /// Represents a requirement definition (e.g., `(:require ...)` in PDDL).
    RequireDef,
    /// Represents a requirement, such as specific features or constraints of a problem.
    Requirement(Requirement),
    /// Represents a type definition, which may be used to define object types.
    Type,
    /// Represents a list of typed elements.
    TypedList,
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
    #[serde(
        serialize_with = "serialize_ordered_float",
        deserialize_with = "deserialize_ordered_float"
    )]
    Number(OrderedFloat<f64>),
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
    FComp(BinaryComp),
    /// Represents an assignment operation in the planning problem.
    Assign(AssignOp),
    /// Represents an arithmetic operation, such as addition or subtraction.
    Operation(ArithmeticOp),
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
    Metric(Optimization),
    /// Represents the total time in the problem.
    TotalTime,
    /// Represents a condition that indicates whether something is violated in the problem.
    IsViolated,
    /// Represents the length or duration of something in the planning problem.
    Length,
    /// Represents a serial timing or duration, serialized as an `OrderedFloat` for precision.
    #[serde(
        serialize_with = "serialize_ordered_float",
        deserialize_with = "deserialize_ordered_float"
    )]
    Serial(OrderedFloat<f64>),
    /// Represents parallel timing or duration, serialized as an `OrderedFloat` for precision.
    #[serde(
        serialize_with = "serialize_ordered_float",
        deserialize_with = "deserialize_ordered_float"
    )]
    Parallel(OrderedFloat<f64>),
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
    MethodSymbol(String),
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
    TaskID(String),
    /// Represents a collection of task ordering constraints.
    TaskNetworkOrderingConstraintDef,
    /// Represents a collection of logical constraints in HDDL.
    TaskNetworkLogicalConstraintDef,
    /// Represents a task network in HDDL.
    TaskNetworkDef,
    /// Represents the initial task network of the HDDL problem.
    InitialTaskNetwork,
}

impl fmt::Display for AstKind {
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
            AstKind::Constant(symbol) => write!(f, "Constant(\"{}\")", symbol),
            AstKind::Variable(symbol) => write!(f, "Variable(\"{}\")", symbol),
            AstKind::FunctionSymbol(symbol) => write!(f, "FunctionSymbol(\"{}\")", symbol),
            AstKind::FunctionTerm => write!(f, "FunctionTerm"),
            AstKind::PrimitiveType(symbol) => write!(f, "PrimitiveType(\"{}\")", symbol),
            AstKind::DomainName(symbol) => write!(f, "DomainName(\"{}\")", symbol),
            AstKind::ProblemName(symbol) => write!(f, "ProblemName(\"{}\")", symbol),
            AstKind::Predicate(symbol) => write!(f, "Predicate(\"{}\")", symbol),
            AstKind::ActionSymbol(symbol) => write!(f, "ActionSymbol(\"{}\")", symbol),
            AstKind::DASymbol(symbol) => write!(f, "DASymbol(\"{}\")", symbol),
            AstKind::PrefName(symbol) => write!(f, "PrefName(\"{}\")", symbol),
            AstKind::Number(value) => write!(f, "Number(\"{}\")", value),
            AstKind::RequireDef => write!(f, "RequireDef"),
            AstKind::Requirement(requirement) => write!(f, "Requirement(\"{}\")", requirement),
            AstKind::Type => write!(f, "Type"),
            AstKind::TypedList => write!(f, "TypedList"),
            AstKind::TypesDef => write!(f, "TypesDef"),
            AstKind::ConstantsDef => write!(f, "ConstantsDef"),
            AstKind::ObjectsDef => write!(f, "ObjectsDef"),
            AstKind::PredicatesDef => write!(f, "PredicatesDef"),
            AstKind::AtomicFormulaSkeleton => write!(f, "AtomicFormulaSkeleton"),
            AstKind::FunctionsDef => write!(f, "FunctionsDef"),
            AstKind::AtomicFunctionSkeleton => write!(f, "AtomicFunctionSkeleton"),
            AstKind::ActionDef => write!(f, "ActionDef"),
            AstKind::PreconditionDef => write!(f, "PreconditionDef"),
            AstKind::EffectDef => write!(f, "EffectDef"),
            AstKind::DurativeActionDef => write!(f, "DurativeActionDef"),
            AstKind::ActionDefBody => write!(f, "ActionDefBody"),
            AstKind::DADefBody => write!(f, "DADefBody"),
            AstKind::DerivedDef => write!(f, "DerivedDef"),
            AstKind::Preference => write!(f, "Preference"),
            AstKind::AtomicFormula => write!(f, "AtomicFormula"),
            AstKind::Not => write!(f, "Not"),
            AstKind::Domain => write!(f, "Domain"),
            AstKind::Problem => write!(f, "Problem"),
            AstKind::Forall => write!(f, "Forall"),
            AstKind::Exists => write!(f, "Exists"),
            AstKind::When => write!(f, "When"),
            AstKind::FComp(comparator) => write!(f, "FComp(\"{}\")", comparator),
            AstKind::Assign(operator) => write!(f, "Assign(\"{}\")", operator),
            AstKind::Operation(operator) => write!(f, "Op(\"{}\")", operator),
            AstKind::And => write!(f, "And"),
            AstKind::Or => write!(f, "Or"),
            AstKind::Imply => write!(f, "Imply"),
            AstKind::AtStart => write!(f, "AtStart"),
            AstKind::AtEnd => write!(f, "AtEnd"),
            AstKind::Overall => write!(f, "Overall"),
            AstKind::Constraints => write!(f, "Constraints"),
            AstKind::Always => write!(f, "Always"),
            AstKind::Sometime => write!(f, "Sometime"),
            AstKind::Within => write!(f, "Within"),
            AstKind::AtMostOnce => write!(f, "AtMostOnce"),
            AstKind::SometimeAfter => write!(f, "SometimeAfter"),
            AstKind::SometimeBefore => write!(f, "SometimeBefore"),
            AstKind::AlwaysWithin => write!(f, "AlwaysWithin"),
            AstKind::HoldDuring => write!(f, "HoldDuring"),
            AstKind::HoldAfter => write!(f, "HoldAfter"),
            AstKind::Init => write!(f, "Init"),
            AstKind::TimedInitialLiteral => write!(f, "TimedInitialLiteral"),
            AstKind::Goal => write!(f, "Goal"),
            AstKind::Metric(operator) => write!(f, "Metric(\"{}\")", operator),
            AstKind::TotalTime => write!(f, "TotalTime"),
            AstKind::IsViolated => write!(f, "IsViolated"),
            AstKind::Length => write!(f, "Length"),
            AstKind::Serial(value) => write!(f, "Serial(\"{}\")", value),
            AstKind::Parallel(value) => write!(f, "Parallel(\"{}\")", value),
            AstKind::Error => write!(f, "Error"),
            // Add for HDDL
            AstKind::Task => write!(f, "Task"),
            AstKind::TaggedTask => write!(f, "TaggedTask"),
            AstKind::TaskDef => write!(f, "TaskDef"),
            AstKind::TaskSymbol(symbol) => write!(f, "TaskSymbol(\"{}\")", symbol),
            AstKind::MethodDef => write!(f, "MethodDef"),
            AstKind::MethodDefBody => write!(f, "MethodDefBody"),
            AstKind::MethodSymbol(symbol) => write!(f, "MethodSymbol(\"{}\")", symbol),
            AstKind::MethodPreconditionDef => write!(f, "MethodPreconditionDef"),
            AstKind::OrderedSubtaskDef => write!(f, "OrderedSubtaskDef"),
            AstKind::PartiallyOrderedSubtaskDef => write!(f, "PartiallyOrderedSubtaskDef"),
            AstKind::TaskID(id) => write!(f, "TaskID(\"{}\")", id),
            AstKind::TaskNetworkOrderingConstraintDef => {
                write!(f, "TaskNetworkOrderingConstraintDef")
            }
            AstKind::TaskNetworkLogicalConstraintDef => {
                write!(f, "TaskNetworkLogicalConstraintDef")
            }
            AstKind::TaskNetworkDef => write!(f, "TaskNetworkDef"),
            AstKind::InitialTaskNetwork => write!(f, "InitialTaskNetwork"),
        }
    }
}

impl PDDLDisplay for AstKind {
    /// Converts the `AstKind` variant to its corresponding PDDL string representation.
    ///
    /// This method implements the `PDDLDisplay` trait for the `AstKind` enum and provides
    /// a way to convert different variants of the enum into valid PDDL syntax. Depending
    /// on the variant, the method returns a string that matches the expected format in a PDDL
    /// domain or problem file.
    ///
    /// # Returns
    ///
    /// Returns a `String` containing the PDDL representation of the current `AstKind` variant.
    fn to_pddl_string(&self) -> String {
        match self {
            AstKind::Constant(symbol)
            | AstKind::Variable(symbol)
            | AstKind::FunctionSymbol(symbol)
            | AstKind::PrimitiveType(symbol)
            | AstKind::DomainName(symbol)
            | AstKind::ProblemName(symbol)
            | AstKind::Predicate(symbol)
            | AstKind::ActionSymbol(symbol)
            | AstKind::DASymbol(symbol)
            | AstKind::PrefName(symbol) => symbol.to_string(),

            AstKind::Domain => DOMAIN.to_string(),
            AstKind::RequireDef => REQUIREMENTS.to_string(),
            AstKind::TypesDef => TYPES.to_string(),
            AstKind::ConstantsDef => CONSTANTS.to_string(),
            AstKind::PredicatesDef => PREDICATES.to_string(),
            AstKind::FunctionsDef => FUNCTIONS.to_string(),
            AstKind::ActionDef => ACTION.to_string(),
            AstKind::PreconditionDef => PRECONDITION.to_string(),
            AstKind::EffectDef => EFFECT.to_string(),
            AstKind::Or => OR.to_string(),
            AstKind::And => AND.to_string(),
            AstKind::Not => NOT.to_string(),
            AstKind::Forall => FORALL.to_string(),
            AstKind::Exists => EXISTS.to_string(),
            AstKind::Imply => IMPLY.to_string(),
            AstKind::When => WHEN.to_string(),

            AstKind::TypedList
            | AstKind::AtomicFormulaSkeleton
            | AstKind::AtomicFunctionSkeleton
            | AstKind::AtomicFormula
            | AstKind::FunctionTerm => "".to_string(),

            AstKind::Requirement(req) => req.to_pddl_string(),

            AstKind::FComp(op) => op.to_pddl_string(),
            AstKind::Assign(op) => op.to_pddl_string(),
            AstKind::Operation(op) => op.to_pddl_string(),

            _ => format!("{}", self), // Default fallback
        }
    }
}

/// Represents an Abstract Syntax Tree (AST) used to model elements of a program or PDDL
/// specification.
///
/// This structure derives the `Clone`, `Debug`, and `Deserialize` traits, allowing instances of
/// `Ast` to be cloned, displayed for debugging, serialiszed or deserialized from data formats such
/// as JSON or YAML.
///
/// # Fields (Private)
/// - `kind`: The type of the element represented by this AST node. It is an `AstKind`.
/// - `children`: A list of child `Ast` elements, represented as a `Vec<Box<Ast>>`.
/// - `start`, `end`, `begin_line`, `begin_column`, `end_line`, `end_column`: Positioning data in
/// the original input.
///
/// # Example
/// ```rust
/// use parser::Ast;
/// use parser::AstKind;
///
/// let ast = Ast::new(AstKind::Domain, 0, 10, 1, 1, 1, 10);
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Ast {
    kind: AstKind,
    children: Vec<Box<Ast>>,
    span: Span,
}

impl Ast {
    /// Creates a new AST node with the specified kind, children, start, and end positions.
    ///
    /// This function creates a new AST node with the provided kind, list of children nodes, start
    /// position, and end position.
    ///
    /// # Arguments
    ///
    /// * `kind` - The kind of the AST node.
    /// * `children` - The list of children nodes of the AST node.
    /// * `start` - The start position of the AST node.
    /// * `end` - The end position of the AST node.
    ///
    /// # Returns
    ///
    /// A new AST node initialized with the given parameters.
    ///
    /// # Examples
    ///
    /// ```
    /// let kind = AstKind::PrimitiveType;
    /// let children = vec![]; // Initialize children nodes if any
    /// let start = 0;
    /// let end = 10;
    /// let ast_node = Ast::new(kind, children, start, end);
    /// ```
    pub fn new(kind: AstKind, children: Vec<Box<Ast>>, start: usize, end: usize) -> Ast {
        Ast {
            kind,
            children,
            span: Span::new(start, end),
        }
    }

    /// Returns the kind of AST node.
    pub fn kind(&self) -> &AstKind {
        &self.kind
    }

    /// Modifies the kind of the AST node.
    ///
    /// This method updates the `kind` field of the `Ast` object to the provided
    /// `new_kind`. It allows changing the type of the AST node after its
    /// creation, enabling dynamic modification of the node's classification
    /// during the AST construction or traversal.
    ///
    /// # Parameters
    /// - `new_kind`: The new `AstKind` to set for the node.
    ///
    /// # Example
    /// ```
    /// let mut node = Ast::new(AstKind::Expression, vec![], 0, 10);
    /// node.set_kind(AstKind::Statement);
    /// ```
    pub fn set_kind(&mut self, new_kind: AstKind) {
        self.kind = new_kind;
    }

    /// Returns a reference to the vector of children of the AST node.
    ///
    /// This method provides access to the vector of children of an AST node.
    /// It returns an immutable reference to the vector, allowing you to read the children
    /// but not modify them directly.
    ///
    /// # Example
    /// ```
    /// let ast = Ast {
    ///     kind: AstKind::SomeKind,
    ///     children: vec![],
    ///     start: 0,
    ///     end: 10,
    /// };
    ///
    /// // Access the children of the AST (read-only)
    /// let children = ast.children();
    /// ```
    ///
    /// # Panics
    /// This method does not panic.
    ///
    /// # Returns
    /// Returns an immutable reference to `self.children` (the vector of `Box<Ast>`).
    pub fn children(&self) -> &Vec<Box<Ast>> {
        &self.children
    }

    /// Returns a reference to the `Span` of this `AstEntry`.
    ///
    /// # Returns
    /// * `&Span` - A reference to the span associated with this `AstEntry`.
    pub fn span(&self) -> &Span {
        &self.span
    }

    /// Sets the children nodes of the current `Ast` node.
    ///
    /// This method replaces the current list of child nodes with a new list provided as an
    /// argument. The new children are specified as a `Vec<Box<Ast>>`, where each element represents
    /// a child node in the Abstract Syntax Tree (AST).
    ///
    /// # Arguments
    ///
    /// * `new_children` - A vector of `Box<Ast>` representing the new set of children for the
    /// current node.
    pub fn set_children(&mut self, new_children: Vec<Box<Ast>>) {
        self.children = new_children;
    }

    /// Returns a mutable reference to the vector of children of the AST node.
    ///
    /// This method allows direct modification of the vector of children of an AST node.
    /// It returns a mutable reference to the vector, enabling addition, removal, or modification
    /// of the elements in the vector.
    ///
    /// # Panics
    /// This method does not panic, but a mutable reference to `self` is required to access it.
    ///
    /// # Returns
    /// Returns a mutable reference to `self.children` (the vector of `Box<Ast>`).
    pub fn children_mut(&mut self) -> &mut Vec<Box<Ast>> {
        &mut self.children
    }

    /// Returns the starting offset of the AST node in the character stream.
    ///
    /// The offset represents the position (in number of characters)
    /// from the beginning of the input file or stream.
    pub fn start_offset(&self) -> usize {
        self.span.start()
    }

    /// Returns the ending offset of the AST node in the character stream.
    ///
    /// The offset represents the position (in number of characters)
    /// from the beginning of the input file or stream.
    pub fn end_offset(&self) -> usize {
        self.span.end()
    }

    /// Returns the starting location (line, column) of the AST node.
    ///
    /// If the location has not been initialized, both `line` and `column` will be set to `usize::MAX`.
    pub fn start_position(&self) -> (usize, usize) {
        self.span.start_position()
    }

    /// Returns the ending location (line, column) of the AST node.
    ///
    /// If the location has not been initialized, both `line` and `column` will be set to `usize::MAX`.
    pub fn end_location(&self) -> (usize, usize) {
        self.span.end_position()
    }

    /// Sets the starting location (line, column) of the AST node.
    ///
    /// # Parameters
    /// - `line`: The line number where the node starts (should be >= 0).
    /// - `column`: The column number where the node starts (should be >= 0).
    ///
    /// Both values are expected to be valid; no specific checks are performed.
    pub fn set_start_position(&mut self, line: usize, column: usize) {
        self.span.set_begin_line(line);
        self.span.set_begin_column(column);
    }

    /// Sets the ending location (line, column) of the AST node.
    ///
    /// # Parameters
    /// - `line`: The line number where the node ends (should be >= 0).
    /// - `column`: The column number where the node ends (should be >= 0).
    ///
    /// Both values are expected to be valid; no specific checks are performed.
    pub fn set_end_position(&mut self, line: usize, column: usize) {
        self.span.set_end_line(line);
        self.span.set_end_column(column);
    }

    /// Retrieves the key for the given AST node based on its type.
    /// This function is used to extract the key for symbols used in the symbol table.
    /// If the node is a constant, variable, or one of the predefined symbols, the key is the
    /// symbol's name. For `FunctionTerm` and `AtomicFormula`, it returns a key formatted as
    /// `name/arity` based on their first child.
    ///
    /// The key is used to store and look up symbols in the symbol table. This function ensures that
    /// each symbol has a unique identifier based on its structure, which is useful for semantics
    /// analysis and symbol resolution.
    ///
    /// # Returns
    /// - Ok(String): The key derived from the node.
    /// - Err(ParserInternalError): An error if the node cannot be processed or if it does not meet
    ///   the expected structure.
    ///
    /// # Errors
    /// - If the node has no children or its first child is not a `FunctionSymbol` or
    ///   `PredicateSymbol`.
    /// - If the node is of an unexpected kind.
    pub fn get_key(&self) -> Result<String, ParserInternalError> {
        match &self.kind {
            // For symbols like constants, variables, action symbols, etc., return the symbol's
            // name directly.
            AstKind::Constant(name)
            | AstKind::Variable(name)
            | AstKind::PrimitiveType(name)
            | AstKind::DomainName(name)
            | AstKind::ProblemName(name)
            | AstKind::ActionSymbol(name)
            | AstKind::DASymbol(name)
            | AstKind::PrefName(name) => Ok(name.to_string()),
            // For `FunctionTerm` and `AtomicFormula`, derive the key from their first child
            AstKind::FunctionTerm | AstKind::AtomicFormula => {
                // Check if the node has children
                if let Some(child) = self.children.first() {
                    // Check the type of the first child (it should be either a FunctionSymbol or
                    // PredicateSymbol)
                    match &child.kind {
                        AstKind::FunctionSymbol(name) => Ok(name.to_string()),
                        AstKind::Predicate(name) => Ok(name.to_string()),
                        _ => {
                            // If the first child is neither a FunctionSymbol nor a PredicateSymbol, return an error
                            Err(ParserInternalError::new(
                                format!(
                                    "First child must be a 'FunctionSymbol' or 'PredicateSymbol', but found: {:?}.",
                                    child.kind
                                )
                            ))
                        }
                    }
                } else {
                    // If there are no children, return an error
                    Err(ParserInternalError::new(
                        "No children found for 'FunctionTerm' or 'AtomicFormula'.".to_string(),
                    ))
                }
            }
            // Handle unexpected AST node kinds
            _ => {
                // Return an error if the AST node kind is not recognized
                Err(ParserInternalError::new(format!(
                    "Unexpected AST kind: {:?}",
                    self.kind
                )))
            }
        }
    }

    /// Formats the AST node with indentation corresponding to its depth.
    ///
    /// This function formats the AST node with a given depth of indentation. Each level of depth
    /// increases the indentation by two spaces.
    ///
    /// # Arguments
    ///
    /// * `f` - The formatter to write the formatted output to.
    /// * `depth` - The depth of the AST node in the tree hierarchy.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::fmt;
    ///
    /// let ast = Ast::new(/* Initialize AST node */);
    /// let mut formatter = fmt::Formatter::new();
    /// ast.fmt_with_depth(&mut formatter, 0).unwrap();
    /// println!("{}", formatter);
    /// ```
    ///
    fn fmt_with_depth(&self, f: &mut fmt::Formatter, depth: usize) -> fmt::Result {
        let indentation = "  ".repeat(depth); // Indentation par niveau de profondeur

        // Affichage avec l'indentation et le résultat formaté
        write!(f, "{}{} {}", indentation, self.kind, self.span)?;

        // Traitement des enfants s'il y en a
        if !self.children.is_empty() {
            write!(f, "\n")?;
            self.write_children_with_depth(f, depth + 1)?;
        }

        Ok(())
    }

    /// Writes the children of the AST node with indentation corresponding to their depth.
    ///
    /// This function writes the children of the AST node with a given depth of indentation. Each child
    /// is formatted with indentation two spaces greater than the depth of its parent node.
    ///
    /// # Arguments
    ///
    /// * `f` - The formatter to write the formatted output to.
    /// * `depth` - The depth of the AST node in the tree hierarchy.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::fmt;
    ///
    /// let ast = Ast::new(/* Initialize AST node */);
    /// let mut formatter = fmt::Formatter::new();
    /// ast.write_children_with_depth(&mut formatter, 1).unwrap();
    /// println!("{}", formatter);
    /// ```
    fn write_children_with_depth(&self, f: &mut fmt::Formatter, depth: usize) -> fmt::Result {
        let len = self.children.len();
        for (i, child) in self.children.iter().enumerate() {
            // Add newline only between children (not before the first child)
            if i > 0 {
                write!(f, "\n")?;
            }
            child.fmt_with_depth(f, depth)?;
            // Only add "End" if it's the last child
            if i == len - 1 {
                let indentation = "  ".repeat(depth - 1);
                write!(f, "\n{}End {}", indentation, self.kind)?;
            }
        }
        Ok(())
    }

    /// Converts the AST structure into a `HashMap` that maps references to AST nodes to their indices.
    ///
    /// This function generates a hash map where each entry associates a reference to an `Ast` node
    /// with its corresponding index. The indices are assigned recursively to all nodes in the AST
    /// structure, ensuring a unique mapping for each node. This map can be used for efficient
    /// lookups or referencing child nodes.
    ///
    /// # Returns
    /// A `HashMap<&Ast, usize>`, where:
    /// - The keys are references (`&Ast`) to the `Ast` nodes.
    /// - The values are the indices (`usize`) of those nodes in the AST structure.
    ///
    /// # Example
    /// ```rust
    /// let ast = Ast::new(...); // Create or load an Ast structure
    /// let map = ast.to_hash_map();
    /// // Now `map` contains a mapping of AST node references to their indices
    /// ```
    pub fn to_hash_map<'a>(&'a self) -> HashMap<&'a Ast, usize> {
        let mut map = HashMap::new();
        let mut id_counter = 0;

        // Recursively populate the map with unique indices for each AST node
        self.to_recusive_hash_map(&mut map, &mut id_counter);

        map
    }

    /// Recursively explores the AST tree and assigns unique indices to each node.
    ///
    /// This private helper function traverses the AST tree and assigns unique indices to each node.
    /// The indices are based on the size of the `map`, which ensures that each node receives a
    /// unique index as it is visited. The function performs a recursive descent into the children
    /// nodes of the current node.
    ///
    /// # Parameters
    /// - `map`: A mutable reference to a `HashMap<&Ast, usize>` that will store the mapping
    ///     of node references to their respective indices.
    /// - `id_counter`: A mutable reference to a `usize` that tracks the index to assign to the next node.
    ///
    /// # Details
    /// This function is designed to be called internally from `to_hash_map()`. It starts from the
    /// current node, adds an entry for that node in the map, and then recursively processes all of
    /// its children nodes, ensuring every node in the AST structure is indexed.
    ///
    /// # Example
    /// ```rust
    /// // This is a private function, used inside `to_hash_map` to recursively assign indices
    /// let ast = Ast::new(...); // Create or load an Ast structure
    /// let mut map = HashMap::new();
    /// let mut id_counter = 0;
    /// ast.to_recusive_hash_map(&mut map, &mut id_counter);
    /// // `map` now contains mappings of AST node references to unique indices
    /// ```
    fn to_recusive_hash_map<'a>(
        &'a self,
        map: &mut HashMap<&'a Ast, usize>,
        id_counter: &mut usize,
    ) {
        // Insert the current node and assign it an index
        map.insert(self, *id_counter);
        *id_counter += 1;

        // Recursively process each child node
        for child in &self.children {
            child.to_recusive_hash_map(map, id_counter); // Recursive call on each child
        }
    }
}

impl fmt::Display for Ast {
    /// Formats the `Ast` with indentation based on its depth.
    ///
    /// This method implements the `Display` trait for the `Ast` struct, allowing it to be
    /// formatted as a string for user-friendly printing. The formatting is done with an
    /// indentation that reflects the depth of the node in the AST (Abstract Syntax Tree).
    ///
    /// # Parameters
    /// - `f`: A mutable reference to the `fmt::Formatter` used to format the output.
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.fmt_with_depth(f, 0)
    }
}

impl PDDLDisplay for Ast {
    /// Converts the current AST node into a PDDL string representation, starting from depth 0.
    ///
    /// This function is a convenience method that delegates the actual conversion to the
    /// `to_pddl_string_with_depth` function with an initial depth of 0. It is intended to be used
    /// when the user doesn't need to control the depth of the string representation.
    ///
    /// # Returns
    /// A `String` that represents the AST node in PDDL format starting at depth 0.
    fn to_pddl_string(&self) -> String {
        self.to_pddl_string_with_depth(0)
    }

    /// Converts the current AST node into a PDDL string representation with the specified depth.
    ///
    /// This function recursively generates the PDDL string representation of the AST node and its
    /// children, considering the depth of the node in the tree. The `depth` parameter helps control
    /// the indentation or the level of nesting for the PDDL string representation. It is intended
    /// for more advanced use cases where control over the depth is required (e.g., formatting the
    /// output).
    ///
    /// # Parameters
    /// - `depth`: A `usize` that represents the depth of the current node in the AST tree. This can
    /// be used to adjust the indentation or nesting of the resulting PDDL string.
    ///
    /// # Returns
    /// A `String` that represents the AST node and its children in PDDL format, taking into account
    /// the depth.
    fn to_pddl_string_with_depth(&self, depth: usize) -> String {
        let offset = " ".repeat(depth * 2);
        let mut pddl = String::new();

        match &self.kind() {
            AstKind::Domain => {
                write!(pddl, "{}({}", offset, self.kind.to_pddl_string()).unwrap();
                for child in self.children() {
                    write!(
                        pddl,
                        "\n{}{}",
                        offset,
                        child.to_pddl_string_with_depth(depth + 1)
                    )
                    .unwrap();
                }
                write!(pddl, "\n{})", offset).unwrap();
            }
            AstKind::DomainName(_) => {
                write!(pddl, "{}({})", offset, self.kind.to_pddl_string(),).unwrap();
            }
            AstKind::RequireDef => {
                write!(pddl, "{}{}", offset, self.kind.to_pddl_string()).unwrap();
                for child in self.children() {
                    write!(pddl, " {}", child.to_pddl_string()).unwrap();
                }
                write!(pddl, ")").unwrap();
            }
            AstKind::Requirement(requirement) => {
                write!(pddl, "{}", requirement.to_pddl_string()).unwrap();
            }
            AstKind::TypesDef => {
                write!(pddl, "{}({}", offset, self.kind.to_pddl_string()).unwrap();
                for child in self.children() {
                    write!(pddl, "\n{}", child.to_pddl_string_with_depth(depth + 1)).unwrap();
                }
                write!(pddl, "\n{})", offset).unwrap();
            }
            AstKind::TypedList => {
                write!(pddl, "{}", offset).unwrap();
                for (i, child) in self.children().iter().enumerate() {
                    if i > 0 && !(child.kind == AstKind::TypedList && child.children().is_empty()) {
                        write!(pddl, " ").unwrap();
                    }
                    if matches!(child.kind(), AstKind::Type) {
                        write!(pddl, "- ").unwrap();
                    }
                    write!(pddl, "{}", child.to_pddl_string()).unwrap();
                }
            }
            AstKind::PrimitiveType(symbol) => {
                write!(pddl, "{}{}", offset, symbol).unwrap();
            }
            AstKind::Type => {
                write!(pddl, "{}", offset).unwrap();
                match self.children().as_slice() {
                    [single_child] => {
                        write!(pddl, "{}", single_child.to_pddl_string()).unwrap();
                    }
                    multiple_children if multiple_children.len() > 1 => {
                        write!(pddl, "(either").unwrap();
                        for child in multiple_children {
                            write!(pddl, " {}", child.to_pddl_string()).unwrap();
                        }
                        write!(pddl, ")").unwrap();
                    }
                    _ => unreachable!("AstKind::Type with with no child encountered"),
                }
            }
            AstKind::ConstantsDef => {
                write!(pddl, "{}({}", offset, self.kind.to_pddl_string()).unwrap();
                for child in self.children() {
                    write!(pddl, "\n{}", child.to_pddl_string_with_depth(depth + 1)).unwrap();
                }
                write!(pddl, "\n{})", offset).unwrap();
            }
            AstKind::Constant(symbol) => {
                write!(pddl, "{}{}", offset, symbol).unwrap();
            }
            AstKind::PredicatesDef => {
                write!(pddl, "{}({}", offset, self.kind.to_pddl_string()).unwrap();
                for child in self.children() {
                    write!(pddl, "\n{}", child.to_pddl_string_with_depth(depth + 1)).unwrap();
                }
                write!(pddl, "\n{})", offset).unwrap();
            }
            AstKind::AtomicFormulaSkeleton
            | AstKind::AtomicFunctionSkeleton
            | AstKind::AtomicFormula
            | AstKind::FunctionTerm => {
                write!(pddl, "{}(", offset).unwrap();
                for (i, child) in self.children().iter().enumerate() {
                    if i > 0 {
                        write!(pddl, " ").unwrap();
                    }
                    write!(pddl, "{}", child.to_pddl_string()).unwrap();
                }
                write!(pddl, ")").unwrap();
            }
            AstKind::Predicate(symbol) => {
                write!(pddl, "{}{}", offset, symbol).unwrap();
            }
            AstKind::FunctionsDef => {
                write!(pddl, "{}({}", offset, self.kind.to_pddl_string()).unwrap();
                for child in self.children() {
                    write!(pddl, "\n{}", child.to_pddl_string_with_depth(depth + 1)).unwrap();
                }
                write!(pddl, "\n{})", offset).unwrap();
            }
            AstKind::FunctionSymbol(symbol) => {
                write!(pddl, "{}{}", offset, symbol).unwrap();
            }
            AstKind::ActionDef => {
                let children = self.children();
                write!(
                    pddl,
                    "{}({} {} ",
                    offset,
                    self.kind.to_pddl_string(),
                    children[0].to_pddl_string()
                )
                .unwrap();
                write!(
                    pddl,
                    "\n{}",
                    children[1].to_pddl_string_with_depth(depth + 1)
                )
                .unwrap();
                write!(pddl, "{}", children[2].to_pddl_string_with_depth(depth + 1)).unwrap();
                write!(pddl, "\n{})", offset).unwrap();
            }
            AstKind::ActionSymbol(symbol) => {
                write!(pddl, "{}", symbol).unwrap();
            }
            AstKind::ActionDefBody => {
                for child in self.children() {
                    write!(pddl, "\n{}", child.to_pddl_string_with_depth(depth)).unwrap();
                }
            }
            AstKind::PreconditionDef => {
                write!(
                    pddl,
                    "{}{}\n{}",
                    offset,
                    self.kind.to_pddl_string(),
                    self.children()[0].to_pddl_string_with_depth(depth + 1)
                )
                .unwrap();
            }
            AstKind::EffectDef => {
                write!(
                    pddl,
                    "{}{}\n{}",
                    offset,
                    self.kind.to_pddl_string(),
                    self.children()[0].to_pddl_string_with_depth(depth + 1)
                )
                .unwrap();
            }
            AstKind::Or => {
                write!(pddl, "{}({}", offset, self.kind.to_pddl_string()).unwrap();
                for child in self.children() {
                    write!(pddl, " {}", child.to_pddl_string()).unwrap();
                }
                write!(pddl, ")").unwrap();
            }
            AstKind::And => {
                write!(pddl, "{}({}", offset, self.kind.to_pddl_string()).unwrap();
                for child in self.children() {
                    write!(pddl, " {}", child.to_pddl_string()).unwrap();
                }
                write!(pddl, ")").unwrap();
            }
            AstKind::Not => {
                write!(pddl, "{}({}", offset, self.kind.to_pddl_string()).unwrap();
                for child in self.children() {
                    write!(pddl, " {}", child.to_pddl_string()).unwrap();
                }
                write!(pddl, ")").unwrap();
            }
            AstKind::FComp(op) => {
                write!(pddl, "{}({}", offset, op.to_pddl_string()).unwrap();
                for child in self.children() {
                    write!(pddl, " {}", child.to_pddl_string()).unwrap();
                }
                write!(pddl, ")").unwrap();
            }
            AstKind::Assign(op) => {
                write!(pddl, "{}({}", offset, op.to_pddl_string()).unwrap();
                for child in self.children() {
                    write!(pddl, " {}", child.to_pddl_string()).unwrap();
                }
                write!(pddl, ")").unwrap();
            }
            AstKind::Operation(op) => {
                write!(pddl, "{}({}", offset, op.to_pddl_string()).unwrap();
                for child in self.children() {
                    write!(pddl, " {}", child.to_pddl_string()).unwrap();
                }
                write!(pddl, ")").unwrap();
            }
            AstKind::Forall => {
                write!(
                    pddl,
                    "{}({} ({}) {})",
                    offset,
                    self.kind.to_pddl_string(),
                    self.children[0].to_pddl_string(),
                    self.children[1].to_pddl_string()
                )
                .unwrap();
            }
            AstKind::Exists => {
                write!(
                    pddl,
                    "{}({} ({}) {})",
                    offset,
                    self.kind.to_pddl_string(),
                    self.children[0].to_pddl_string(),
                    self.children[1].to_pddl_string()
                )
                .unwrap();
            }
            AstKind::Imply => {
                write!(
                    pddl,
                    "{}({} {} {})",
                    offset,
                    self.kind.to_pddl_string(),
                    self.children[0].to_pddl_string(),
                    self.children[1].to_pddl_string()
                )
                .unwrap();
            }
            AstKind::When => {
                write!(
                    pddl,
                    "{}({} {} {})",
                    offset,
                    self.kind.to_pddl_string(),
                    self.children[0].to_pddl_string(),
                    self.children[1].to_pddl_string()
                )
                .unwrap();
            }
            _ => pddl = self.kind.to_pddl_string(),
        }
        pddl
    }
}

/// Serialization implementation for `OrderedFloat<f64>`.
///
/// This function implements custom serialization for the `OrderedFloat<f64>` type, which
/// wraps a `f64` value while preserving the order of floating-point numbers, handling edge cases
/// like NaN values.
///
/// # Parameters
/// - `x`: A reference to the `OrderedFloat<f64>` value that needs to be serialized.
/// - `serializer`: The serializer that will be used to convert the `OrderedFloat<f64>` to a
///   suitable format (e.g., JSON).
///
/// # Type Parameters
/// - `S`: The type of the serializer that implements the `Serializer` trait.
///
/// # Return Value
/// - This function returns the result of calling the `serialize_f64` method on the serializer,
///   which will serialize the inner `f64` value of the `OrderedFloat`.
pub fn serialize_ordered_float<S>(x: &OrderedFloat<f64>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_f64(x.into_inner())
}

/// Deserialization implementation for `OrderedFloat<f64>`.
///
/// This function implements custom deserialization for the `OrderedFloat<f64>` type, which wraps a
/// `f64` value. It allows deserializing a floating-point number and wrapping it into an
/// `OrderedFloat<f64>`.
///
/// # Parameters
/// - `deserializer`: The deserializer that will be used to convert the serialized data back into an
///   `OrderedFloat<f64>`.
///
/// # Type Parameters
/// - `'de`: The lifetime of the deserialization data.
/// - `D`: The type of the deserializer, which implements the `Deserializer` trait.
///
/// # Return Value
/// - This function returns the result of deserializing the floating-point number into an
///   `OrderedFloat<f64>` wrapped value.
pub fn deserialize_ordered_float<'de, D>(deserializer: D) -> Result<OrderedFloat<f64>, D::Error>
where
    D: Deserializer<'de>,
{
    struct OrderedFloatVisitor;

    impl<'de> Visitor<'de> for OrderedFloatVisitor {
        type Value = OrderedFloat<f64>;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("a floating point number")
        }

        fn visit_f64<E>(self, value: f64) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(OrderedFloat(value))
        }
    }

    deserializer.deserialize_f64(OrderedFloatVisitor)
}
