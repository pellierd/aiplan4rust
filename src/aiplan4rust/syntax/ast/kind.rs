use crate::aiplan4rust::syntax::elements::ArithmeticOp;
use crate::aiplan4rust::syntax::elements::AssignOp;
use crate::aiplan4rust::syntax::elements::BinaryComp;
use crate::aiplan4rust::syntax::elements::Optimization;
use crate::aiplan4rust::syntax::elements::Requirement;

use crate::aiplan4rust::syntax::lexer::token::ACTION;
use crate::aiplan4rust::syntax::lexer::token::AND;
use crate::aiplan4rust::syntax::lexer::token::CONSTANTS;
use crate::aiplan4rust::syntax::lexer::token::DOMAIN;
use crate::aiplan4rust::syntax::lexer::token::EFFECT;
use crate::aiplan4rust::syntax::lexer::token::EXISTS;
use crate::aiplan4rust::syntax::lexer::token::FORALL;
use crate::aiplan4rust::syntax::lexer::token::FUNCTIONS;
use crate::aiplan4rust::syntax::lexer::token::IMPLY;
use crate::aiplan4rust::syntax::lexer::token::NOT;
use crate::aiplan4rust::syntax::lexer::token::OR;
use crate::aiplan4rust::syntax::lexer::token::PRECONDITION;
use crate::aiplan4rust::syntax::lexer::token::PREDICATES;
use crate::aiplan4rust::syntax::lexer::token::REQUIREMENTS;
use crate::aiplan4rust::syntax::lexer::token::TYPES;
use crate::aiplan4rust::syntax::lexer::token::WHEN;

use crate::aiplan4rust::syntax::SyntaxDisplay;

use ordered_float::OrderedFloat;

use serde::de;
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
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Kind {
    /// Represents a constant value in the planning problem, typically a literal or a fixed value.
    Constant(String),
    /// Represents a variable used in actions or predicates, typically a placeholder for values.
    Variable(String),
    /// Represents a function symbol, typically used in mathematical functions or expressions.
    FunctionSymbol(String),
    /// Represents a basic data type, such as integers or booleans, used in the planning problem.
    PrimitiveType(String),
    /// Represents the name of the domain in a PDDL file.
    DomainName(String),
    /// Represents the name of the problem in a PDDL file.
    ProblemName(String),
    /// Represents a predicate symbol used in logical expressions or actions.
    Predicate(String),
    /// Represents an action symbol used in the problem specification.
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
    TaskOrderingConstraintDef,
    /// Represents a task ordering constraint in HDDL
    TaskOrderingConstraint(BinaryComp),
    /// Represents a collection of logical constraints in HDDL.
    TaskLogicalConstraintDef,
    /// Represents a task network in HDDL.
    TaskNetworkDef,
    /// Represents the initial task network of the HDDL problem.
    InitialTaskNetwork,
}

impl Kind {
    /// Returns the name associated with this `SyntaxNodeKind` if it represents a symbolic node.
    ///
    /// Symbolic nodes are those that carry a meaningful identifier such as constants, variables,
    /// action symbols, function symbols, etc. This method extracts and returns that identifier
    /// as a `String`.
    ///
    /// # Returns
    /// - `Some(String)` containing the symbol's name, if the node kind represents a symbol.
    /// - `None` if the node kind does not carry a name.
    ///
    /// # Examples
    /// ```
    /// let kind = SyntaxNodeKind::Constant("speed".to_string());
    /// assert_eq!(kind.get_symbol(), Some("speed".to_string()));
    ///
    /// let kind = SyntaxNodeKind::And; // assuming it's a logical operator
    /// assert_eq!(kind.get_symbol(), None);
    /// ```
    pub fn get_symbol(&self) -> Option<&String> {
        match self {
            // Symbolic node kinds: return their name
            Kind::Constant(name)
            | Kind::Variable(name)
            | Kind::PrimitiveType(name)
            | Kind::DomainName(name)
            | Kind::ProblemName(name)
            | Kind::ActionSymbol(name)
            | Kind::DASymbol(name)
            | Kind::PrefName(name)
            | Kind::FunctionSymbol(name)
            | Kind::Predicate(name) => Some(name),

            // Other kinds of nodes don't have a symbolic name
            _ => None,
        }
    }
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
            Kind::Constant(symbol) => write!(f, "Constant(\"{}\")", symbol),
            Kind::Variable(symbol) => write!(f, "Variable(\"{}\")", symbol),
            Kind::FunctionSymbol(symbol) => write!(f, "FunctionSymbol(\"{}\")", symbol),
            Kind::FunctionTerm => write!(f, "FunctionTerm"),
            Kind::PrimitiveType(symbol) => write!(f, "PrimitiveType(\"{}\")", symbol),
            Kind::DomainName(symbol) => write!(f, "DomainName(\"{}\")", symbol),
            Kind::ProblemName(symbol) => write!(f, "ProblemName(\"{}\")", symbol),
            Kind::Predicate(symbol) => write!(f, "Predicate(\"{}\")", symbol),
            Kind::ActionSymbol(symbol) => write!(f, "ActionSymbol(\"{}\")", symbol),
            Kind::DASymbol(symbol) => write!(f, "DASymbol(\"{}\")", symbol),
            Kind::PrefName(symbol) => write!(f, "PrefName(\"{}\")", symbol),
            Kind::Number(value) => write!(f, "Number(\"{}\")", value),
            Kind::RequireDef => write!(f, "RequireDef"),
            Kind::Requirement(requirement) => {
                write!(f, "Requirement(\"{}\")", requirement)
            }
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
            Kind::FComp(comparator) => write!(f, "FComp(\"{}\")", comparator),
            Kind::Assign(operator) => write!(f, "Assign(\"{}\")", operator),
            Kind::Operation(operator) => write!(f, "Op(\"{}\")", operator),
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
            Kind::Metric(operator) => write!(f, "Metric(\"{}\")", operator),
            Kind::TotalTime => write!(f, "TotalTime"),
            Kind::IsViolated => write!(f, "IsViolated"),
            Kind::Length => write!(f, "Length"),
            Kind::Serial(value) => write!(f, "Serial(\"{}\")", value),
            Kind::Parallel(value) => write!(f, "Parallel(\"{}\")", value),
            Kind::Error => write!(f, "Error"),
            // Add for HDDL
            Kind::Task => write!(f, "Task"),
            Kind::TaggedTask => write!(f, "TaggedTask"),
            Kind::TaskDef => write!(f, "TaskDef"),
            Kind::TaskSymbol(symbol) => write!(f, "TaskSymbol(\"{}\")", symbol),
            Kind::MethodDef => write!(f, "MethodDef"),
            Kind::MethodDefBody => write!(f, "MethodDefBody"),
            Kind::MethodSymbol(symbol) => write!(f, "MethodSymbol(\"{}\")", symbol),
            Kind::MethodPreconditionDef => write!(f, "MethodPreconditionDef"),
            Kind::OrderedSubtaskDef => write!(f, "OrderedSubtaskDef"),
            Kind::PartiallyOrderedSubtaskDef => write!(f, "PartiallyOrderedSubtaskDef"),
            Kind::TaskID(id) => write!(f, "TaskID(\"{}\")", id),
            Kind::TaskOrderingConstraintDef => {
                write!(f, "TaskOrderingConstraintDef")
            }
            Kind::TaskOrderingConstraint(comparator) => {
                write!(f, "TaskOrderingConstraint(\"{}\")", comparator)
            }
            Kind::TaskLogicalConstraintDef => {
                write!(f, "TaskLogicalConstraintDef")
            }
            Kind::TaskNetworkDef => write!(f, "TaskNetworkDef"),
            Kind::InitialTaskNetwork => write!(f, "InitialTaskNetwork"),
        }
    }
}

impl SyntaxDisplay for Kind {
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
    fn to_syntax_string(&self) -> String {
        match self {
            Kind::Constant(symbol)
            | Kind::Variable(symbol)
            | Kind::FunctionSymbol(symbol)
            | Kind::PrimitiveType(symbol)
            | Kind::DomainName(symbol)
            | Kind::ProblemName(symbol)
            | Kind::Predicate(symbol)
            | Kind::ActionSymbol(symbol)
            | Kind::DASymbol(symbol)
            | Kind::PrefName(symbol) => symbol.to_string(),

            Kind::Domain => DOMAIN.to_string(),
            Kind::RequireDef => REQUIREMENTS.to_string(),
            Kind::TypesDef => TYPES.to_string(),
            Kind::ConstantsDef => CONSTANTS.to_string(),
            Kind::PredicatesDef => PREDICATES.to_string(),
            Kind::FunctionsDef => FUNCTIONS.to_string(),
            Kind::ActionDef => ACTION.to_string(),
            Kind::PreconditionDef => PRECONDITION.to_string(),
            Kind::EffectDef => EFFECT.to_string(),
            Kind::Or => OR.to_string(),
            Kind::And => AND.to_string(),
            Kind::Not => NOT.to_string(),
            Kind::Forall => FORALL.to_string(),
            Kind::Exists => EXISTS.to_string(),
            Kind::Imply => IMPLY.to_string(),
            Kind::When => WHEN.to_string(),

            Kind::TypedList
            | Kind::TypedItem
            | Kind::TypedItemElements
            | Kind::AtomicFormulaSkeleton
            | Kind::AtomicFunctionSkeleton
            | Kind::AtomicFormula
            | Kind::FunctionTerm => "".to_string(),

            Kind::Requirement(req) => req.to_syntax_string(),

            Kind::FComp(op) => op.to_syntax_string(),
            Kind::Assign(op) => op.to_syntax_string(),
            Kind::Operation(op) => op.to_syntax_string(),

            _ => format!("{}", self), // Default fallback
        }
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
fn serialize_ordered_float<S>(x: &OrderedFloat<f64>, serializer: S) -> Result<S::Ok, S::Error>
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
fn deserialize_ordered_float<'de, D>(deserializer: D) -> Result<OrderedFloat<f64>, D::Error>
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
