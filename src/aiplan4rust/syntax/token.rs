use logos::Logos;
use std::fmt;
use std::num::ParseFloatError;

// PDDL KEYWORD
/// "define" is used to declare the start of a PDDL domain or problem definition.
pub const DEFINE: &str = "define";

/// "domain" specifies the domain in a PDDL problem definition.
pub const DOMAIN: &str = "domain";

/// "problem" specifies the problem to solve in PDDL.
pub const PROBLEM: &str = "problem";

/// ":requirements" lists the requirements for the PDDL problem or domain.
pub const REQUIREMENTS: &str = ":requirements";

/// "either" represents a logical choice or alternative condition.
pub const EITHER: &str = "either";

/// ":types" defines types in the PDDL domain, such as objects or categories.
pub const TYPES: &str = ":types";

/// ":constants" specifies constant objects or values in the PDDL domain.
pub const CONSTANTS: &str = ":constants";

/// ":predicates" defines predicates or properties in the PDDL domain.
pub const PREDICATES: &str = ":predicates";

/// ":functions" defines functions or operations in the PDDL domain.
pub const FUNCTIONS: &str = ":functions";

/// ":action" specifies an action within the PDDL domain.
pub const ACTION: &str = ":action";

/// ":parameters" defines parameters for a PDDL action or function.
pub const PARAMETERS: &str = ":parameters";

/// ":precondition" specifies the preconditions for an action in PDDL.
pub const PRECONDITION: &str = ":precondition";

/// ":effect" specifies the effects of an action in PDDL.
pub const EFFECT: &str = ":effect";

/// ":derived" indicates derived predicates or functions in PDDL.
pub const DERIVED: &str = ":derived";

/// ":durative-action" specifies actions with durations in PDDL.
pub const DURATIVE_ACTION: &str = ":durative-action";

/// ":duration" specifies the duration of a durative action in PDDL.
pub const DURATION: &str = ":duration";

/// ":condition" specifies conditions in the PDDL domain.
pub const CONDITION: &str = ":condition";

/// ":domain" refers to the domain definition in PDDL.
pub const DOMAIN_DEF: &str = ":domain";

/// ":objects" specifies the objects involved in the PDDL problem.
pub const OBJECTS: &str = ":objects";

/// ":init" defines the initial state of the problem in PDDL.
pub const INIT: &str = ":init";

/// ":goal" defines the goal state to achieve in PDDL.
pub const GOAL: &str = ":goal";

/// ":metric" specifies optimization goals in PDDL.
pub const METRIC: &str = ":metric";

/// ":length" specifies the length of a solution plan in PDDL.
pub const LENGTH: &str = ":length";

/// ":serial" specifies a serial execution strategy in PDDL.
pub const SERIAL: &str = ":serial";

/// ":parallel" specifies a parallel execution strategy in PDDL.
pub const PARALLEL: &str = ":parallel";

// Expression
/// "preference" is used to specify preferences between plans in PDDL.
pub const PREFERENCE: &str = "preference";

/// "and" is a logical operator used to combine conditions that must all be true.
pub const AND: &str = "and";

/// "or" is a logical operator used to combine conditions where at least one must be true.
pub const OR: &str = "or";

/// "not" is a logical operator used to negate a condition.
pub const NOT: &str = "not";

/// "imply" is used to specify an implication, where one condition leads to another.
pub const IMPLY: &str = "imply";

/// "forall" is used to specify that a condition must hold for all instances of a variable.
pub const FORALL: &str = "forall";

/// "exists" is used to specify that at least one instance of a variable satisfies a condition.
pub const EXISTS: &str = "exists";

/// "when" is used to specify that an effect occurs only when a condition holds.
pub const WHEN: &str = "when";

// Time
/// "at start" specifies that an action or condition holds at the beginning of the timeline.
pub const AT_START: &str = "at start";

/// "at end" specifies that an action or condition holds at the end of the timeline.
pub const AT_END: &str = "at end";

/// "over all" specifies that a condition holds throughout the entire timeline.
pub const OVERALL: &str = "over all";

// Constraints
/// "always" indicates a constraint that must hold for the entire duration.
pub const ALWAYS: &str = "always";

/// "sometime" indicates a constraint that must hold at some point during the timeline.
pub const SOMETIME: &str = "sometime";

/// "within" specifies a time window within which a constraint must be satisfied.
pub const WITHIN: &str = "within";

/// "at-most-once" indicates that an event or condition should happen no more than once.
pub const AT_MOST_ONCE: &str = "at-most-once";

/// "sometime-after" specifies that a condition must be satisfied at some point after a certain time.
pub const SOMETIME_AFTER: &str = "sometime-after";

/// "sometime-before" specifies that a condition must be satisfied at some point before a certain time.
pub const SOMETIME_BEFORE: &str = "sometime-before";

/// "always-within" specifies that a condition must always hold within a specific time window.
pub const ALWAYS_WITHIN: &str = "always-within";

/// "hold-during" indicates that a condition must hold continuously during a specified period.
pub const HOLD_DURING: &str = "hold-during";

/// "hold-after" indicates that a condition must hold continuously after a specified time.
pub const HOLD_AFTER: &str = "hold-after";

// Comments
// ";" is used to denote a single-line comment in the code.
pub const LINE_COMMENT: &str = ";";

/// "//" is used to denote a single-line comment in the code.
pub const CLINE_COMMENT: &str = "//";

/// "/*" is used to start a multi-line comment.
pub const CBLOCK_COMMENT: &str = "/*";

// PDDL REQUIREMENTS
/// Represents the ":strips" requirement in PDDL.
pub const STRIPS: &str = ":strips";

/// Represents the ":typing" requirement in PDDL.
pub const TYPING: &str = ":typing";

/// Represents the ":negative-preconditions" requirement in PDDL.
pub const NEGATIVE_PRECONDITION: &str = ":negative-preconditions";

/// Represents the ":disjunctive-preconditions" requirement in PDDL.
pub const DISJUNCTIVE_PRECONDITION: &str = ":disjunctive-preconditions";

/// Represents the ":equality" requirement in PDDL.
pub const EQUALITY: &str = ":equality";

/// Represents the ":existential-preconditions" requirement in PDDL.
pub const EXISTENTIAL_PRECONDITIONS: &str = ":existential-preconditions";

/// Represents the ":universal-preconditions" requirement in PDDL.
pub const UNIVERSAL_PRECONDITIONS: &str = ":universal-preconditions";

/// Represents the ":quantified-preconditions" requirement in PDDL.
pub const QUANTIFIED_PRECONDITIONS: &str = ":quantified-preconditions";

/// Represents the ":conditional-effects" requirement in PDDL.
pub const CONDITIONAL_EFFECTS: &str = ":conditional-effects";

/// Represents the ":fluents" requirement in PDDL.
pub const FLUENTS: &str = ":fluents";

/// Represents the ":numeric-fluents" requirement in PDDL.
pub const NUMERIC_FLUENTS: &str = ":numeric-fluents";

/// Represents the ":adl" requirement in PDDL.
pub const ADL: &str = ":adl";

/// Represents the ":durative-actions" requirement in PDDL.
pub const DURATIVE_ACTIONS: &str = ":durative-actions";

/// Represents the ":duration-inequalities" requirement in PDDL.
pub const DURATIVE_INEQUALITIES: &str = ":duration-inequalities";

/// Represents the ":continuous-effects" requirement in PDDL.
pub const CONTINUS_EFFECTS: &str = ":continuous-effects";

/// Represents the ":derived-predicates" requirement in PDDL.
pub const DERIVED_PREDICATES: &str = ":derived-predicates";

/// Represents the ":timed-initial-literals" requirement in PDDL.
pub const TIME_INITIAL_LITERALS: &str = ":timed-initial-literals";

/// Represents the ":preferences" requirement in PDDL.
pub const PREFERENCES: &str = ":preferences";

/// Represents the ":constraints" requirement in PDDL.
pub const CONSTRAINTS: &str = ":constraints";

/// Represents the ":action-costs" requirement in PDDL.
pub const ACTION_COSTS: &str = ":action-costs";

// PDDL SEPARATORS
/// "(" is used to denote the opening parenthesis in expressions or groupings.
pub const LPAREN: &str = "(";

/// ")" is used to denote the closing parenthesis in expressions or groupings.
pub const RPAREN: &str = ")";

// PDDL Arithmetic operators
/// Represents the "-" arithmetic operator in PDDL.
pub const SUB: &str = "-";

/// Represents the "+" arithmetic operator in PDDL.
pub const ADD: &str = "+";

/// Represents the "/" arithmetic operator in PDDL.
pub const DIV: &str = "/";

/// Represents the "*" arithmetic operator in PDDL.
pub const MUL: &str = "*";

// PDDL Comparator operators
/// Represents the ">" comparator operator in PDDL.
pub const GREATER: &str = ">";

/// Represents the "<" comparator operator in PDDL.
pub const LESS: &str = "<";

/// Represents the "=" comparator operator in PDDL.
pub const EQUAL: &str = "=";

/// Represents the ">=" comparator operator in PDDL.
pub const GREATER_EQ: &str = ">=";

/// Represents the "<=" comparator operator in PDDL.
pub const LESS_EQ: &str = "<=";

// PDDL Assign operators
/// Represents the "assign" operator in PDDL.
pub const ASSIGN: &str = "assign";

/// Represents the "scale-up" operator in PDDL.
pub const SCALE_UP: &str = "scale-up";

/// Represents the "scale-down" operator in PDDL.
pub const SCALE_DOWN: &str = "scale-down";

/// Represents the "increase" operator in PDDL.
pub const INCREASE: &str = "increase";

/// Represents the "decrease" operator in PDDL.
pub const DECREASE: &str = "decrease";

/// "minimize" is used to specify that the objective is to minimize the value of an expression or a variable.
pub const MINIMIZE: &str = "minimize";

/// "maximize" is used to specify that the objective is to maximize the value of an expression or a variable.
pub const MAXIMIZE: &str = "maximize";

// Special
/// Represents the "object" type in PDDL. It is used to specify that a variable represents an object
/// type.
pub const OBJECT_TYPE: &str = "object";

/// Represents the "number" type in PDDL. It is used to specify that a variable represents a number
/// type.
pub const NUMBER_TYPE: &str = "number";

/// Represents the "undefined" type in PDDL. It is used to specify that a variable type is not
/// defined.
pub const UNDEFINED: &str = "undefined";

/// Represents the "?duration" special variable in PDDL, which is used to refer to the duration of
/// actions.
pub const DURATION_VARIABLE: &str = "?duration";

/// Represents the "#t" special constant in PDDL, often used to represent the Boolean value true.
pub const SHARP_T: &str = "#t";

/// Represents the "total-time" special constant in PDDL, used to represent the total time of a plan.
pub const TOTAL_TIME: &str = "total-time";

/// Represents the "is-violated" special constant in PDDL, which checks if a constraint is violated.
pub const IS_VIOLATED: &str = "is-violated";

////////////////////////////////////////////////////////////////////////////////////////////////////
// HDDL

/// ":htn" represents the marker for Hierarchical Task Network (HTN) definitions in HDDL.
pub const HIERARCHY: &str = ":hierarchy";

/// ":htn-method-prec" specifies method preconditions in HTN for HDDL.
pub const METHOD_PRECONDITIONS: &str = ":method-preconditions";

/// ":task" defines a task in the HDDL domain.
pub const TASK: &str = ":task";

/// ":tasks" represents a collection of tasks in the HDDL domain.
pub const TASKS: &str = ":tasks";

/// ":method" defines a decomposition method in HTN for HDDL.
pub const METHOD: &str = ":method";

/// ":ordered-subtasks" represents a sequence of subtasks with a strict execution order in HDDL.
pub const ORDERED_SUBTASKS: &str = ":ordered-subtasks";

/// ":ordered-tasks" represents a sequence of tasks with a strict execution order in HDDL.
pub const ORDERED_TASKS: &str = ":ordered-tasks";

/// ":subtasks" lists the subtasks of a method in HDDL, allowing flexible ordering.
pub const SUBTASKS: &str = ":subtasks";

/// ":ordering" defines explicit ordering constraints between subtasks in HDDL.
pub const ORDERING: &str = ":ordering";
/// ":order" defines explicit ordering constraints between subtasks in HDDL.
pub const ORDER: &str = ":order";
/// ":htn" defines the initial task network of the problem in HDDL.
pub const HTN: &str = ":htn";

/// Enum representing lexical errors that can occur during token parsing.
///
/// This enum is used to categorize different types of lexical errors encountered
/// during the lexical analysis phase, such as invalid tokens or parsing errors
/// for floating-point values.
///
#[derive(Default, Debug, Clone, PartialEq)]
pub enum LexicalError {
    /// Represents an invalid token encountered during parsing.
    #[default]
    InvalidToken,
    /// Represents a parsing error that occurred while attempting to parse a floating-point value.
    InvalidFloat(ParseFloatError),
}

/// Implementation of the `From<ParseFloatError>` trait for converting
/// a `ParseFloatError` into a `LexicalError`.
///
/// This implementation allows for the conversion of a floating-point parsing
/// error (`ParseFloatError`) into a lexical error of type `LexicalError::InvalidFloat`.
impl From<ParseFloatError> for LexicalError {
    fn from(err: ParseFloatError) -> Self {
        LexicalError::InvalidFloat(err)
    }
}

/// Implements the `fmt::Display` trait for `LexicalError`.
///
/// This trait implementation allows `LexicalError` to be formatted as a human-readable string.
/// It defines how errors of type `LexicalError` should be displayed when printed using the
/// `format!` macro or when the `println!` macro is used. This provides more context for debugging
/// or logging lexical errors.
///
/// The implementation works as follows:
/// - If the error is `InvalidToken`, it will be displayed as `"Invalid Token"`.
/// - If the error is `InvalidFloat`, it will display the message from the underlying `
///   ParseFloatError` like: `"Invalid Float: <error message>"`.
impl fmt::Display for LexicalError {
    /// Formats the `LexicalError` into a user-readable string.
    ///
    /// # Arguments
    ///
    /// * `f` - The formatter that will format the error.
    ///
    /// # Returns
    ///
    /// The result of writing the formatted string to `f`.
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            LexicalError::InvalidToken => write!(f, "Invalid Token"),
            LexicalError::InvalidFloat(err) => write!(f, "Invalid Float: {}", err),
        }
    }
}

/// Enum representing the possible tokens in the lexical analysis of a domain specification.
///
/// This enum is used to categorize and parse the different types of tokens that can be encountered
/// in a domain specification, such as keywords, identifiers, numbers, operators, and special
/// symbols. Each variant corresponds to a specific type of token that the lexer can identify, and
/// some variants include regular expressions and logic for parsing token values.
///
/// This enum leverages the `Logos` crate for efficient lexical analysis, utilizing custom regex
/// patterns for token recognition. The associated `lex()` function will match input tokens based on
/// these patterns.
///
/// ## Example:
/// ```rust
/// let input = "define problem";
/// let lexer = Token::lexer(input);
/// for token in lexer {
///     println!("{:?}", token);
/// }
/// ```
#[derive(Logos, Clone, Debug, PartialEq)]
// Skip whitespace characters like spaces, tabs, newlines, and form feeds.
#[logos(skip r"[ \r\t\n\f]+")]
// Lexical errors are handled by the `LexicalError` type.
#[logos(error = LexicalError)]
// Subpatterns for matching specific token types.
#[logos(subpattern letter = r"[a-zA-Z]")]
#[logos(subpattern digit = r"[0-9]")]
#[logos(subpattern decimal = r".(?&digit)+")]
#[logos(subpattern any_char = r"[a-zA-Z0-9-_]")]
#[logos(subpattern name = r"(?&letter)(?&any_char)*")]
pub enum Token {
    // Identifiers: Token for variables and identifiers in the input.
    // Matches an optional '?' followed by a valid name.
    #[regex("[?]?(?&name)", |lex| lex.slice().parse().ok())]
    ID(String),
    //#[regex("(?&name)", |lex| lex.slice().parse().ok())]
    //ID(String),
    //#[regex("[?](?&name)", |lex| lex.slice().parse().ok())]
    //VarID(String),

    // Numbers: Token for numerical values, including optional decimals.
    #[regex("(?&digit)+(?&decimal)?", |lex| lex.slice().parse::<f64>())]
    Number(f64),

    // Keywords: Reserved words that are part of the domain specification syntax.
    #[token("define")]
    Define,
    #[token("domain")]
    Domain,
    #[token("problem")]
    Problem,
    #[token(":requirements")]
    Requirements,
    #[token(":types")]
    Types,
    #[token("either")]
    Either,
    #[token(":constants")]
    Constants,
    #[token(":predicates")]
    Predicates,
    #[token(":functions")]
    Functions,
    #[token(":action")]
    Action,
    #[token(":parameters")]
    Parameters,
    #[token(":precondition")]
    Precondition,
    #[token(":effect")]
    Effect,
    #[token(":derived")]
    Derived,
    #[token(":durative-action")]
    DurativeAction,
    #[token(":duration")]
    Duration,
    #[token(":condition")]
    Condition,
    #[token(":domain")]
    DomainDef,
    #[token(":objects")]
    Objects,
    #[token(":init")]
    Init,
    #[token(":goal")]
    Goal,
    #[token(":metric")]
    Metric,
    #[token(":length")]
    Length,
    #[token(":serial")]
    Serial,
    #[token(":parallel")]
    Parallel,

    // Expressions: Tokens for logical and mathematical expressions.
    #[token("preference")]
    Preference,
    #[token("and")]
    And,
    #[token("or")]
    Or,
    #[token("not")]
    Not,
    #[token("imply")]
    Imply,
    #[token("forall")]
    Forall,
    #[token("exists")]
    Exists,
    #[token("when")]
    When,

    // Requirements: Tokens for various domain requirements and features.
    #[token(":strips")]
    Strips,
    #[token(":typing")]
    Typing,
    #[token(":negative-preconditions")]
    NegativePreconditions,
    #[token(":disjunctive-preconditions")]
    DisjunctivePreconditions,
    #[token(":equality")]
    Equality,
    #[token(":existential-preconditions")]
    ExistentialPreconditions,
    #[token(":universal-preconditions")]
    UniversalPreconditions,
    #[token(":quantified-preconditions")]
    QuantifiedPreconditions,
    #[token(":conditional-effects")]
    ConditionalEffects,
    #[token(":fluents")]
    Fluents,
    #[token(":numeric-fluents")]
    NumericFluents,
    #[token(":adl")]
    Adl,
    #[token(":durative-actions")]
    DurativeActions,
    #[token(":duration-inequalities")]
    DurationInequalities,
    #[token(":continuous-effects")]
    ContinuousEffects,
    #[token(":derived-predicates")]
    DerivedPredicates,
    #[token(":timed-initial-literals")]
    TimedInitialLiterals,
    #[token(":preferences")]
    Preferences,
    #[token(":constraints")]
    Constraints,
    #[token(":action-costs")]
    ActionCosts,

    // Time-related Tokens: Tokens for time-related actions.
    #[token("at start")]
    AtStart,
    #[token("at end")]
    AtEnd,
    #[token("over all")]
    OverAll,

    // Constraints: Tokens related to constraints in the domain specification.
    #[token("always")]
    Always,
    #[token("sometime")]
    Sometime,
    #[token("within")]
    Within,
    #[token("at-most-once")]
    AtMostOnce,
    #[token("sometime-after")]
    SometimeAfter,
    #[token("sometime-before")]
    SometimeBefore,
    #[token("always-within")]
    AlwaysWithin,
    #[token("hold-during")]
    HoldDuring,
    #[token("hold-after")]
    HoldAfter,

    // Arithmetic Operators: Tokens representing arithmetic operations.
    #[token("-")]
    Sub,
    #[token("+")]
    Add,
    #[token("/")]
    Div,
    #[token("*")]
    Mul,

    // Comparison Operators: Tokens representing comparison operations.
    #[token(">")]
    Greater,
    #[token("<")]
    Less,
    #[token("=")]
    Equal,
    #[token(">=")]
    GreaterEq,
    #[token("<=")]
    LessEq,

    // Assignment Operators: Tokens for assigning values or modifying variables.
    #[token("assign")]
    Assign,
    #[token("scale-up")]
    ScaleUp,
    #[token("scale-down")]
    ScaleDown,
    #[token("increase")]
    Increase,
    #[token("decrease")]
    Decrease,

    // Optimization: Tokens for optimization operations in the domain.
    #[token("minimize")]
    Minimize,
    #[token("maximize")]
    Maximize,

    // Separators: Tokens for separating elements in the domain specification.
    #[token("(")]
    LParen,
    #[token(")")]
    RParen,

    // Special Tokens: Special-purpose tokens for specific actions or types.
    #[token("object")]
    ObjectType,
    #[token("number")]
    NumberType,
    #[token("undefined")]
    Undefined,
    #[token("?duration")]
    DurationVariable,
    #[token("#t")]
    SharpT,
    #[token("total-time")]
    TotalTime,
    #[token("is-violated")]
    IsViolated,

    // Comment Tokens: Tokens representing comments in the input as PDDL
    #[regex(r";.*\n", logos::skip)]
    LineComment,

    // Comment Tokens: Tokens representing comments in the input as C language
    #[regex(r"//.*\n", logos::skip)]
    CLineComment,
    #[regex(r"/\*([^*]|\**[^*/])*\*+/", logos::skip)]
    CBlockComment,

    // Error Token: Token to handle unclosed comments or other errors.
    #[regex(r"/\*([^*]|\*+[^*/])*\*?", |lex| lex.slice().parse().ok())]
    Error(String),

    // Add for HDDL
    #[token(":hierarchy")]
    Hierarchy,
    #[token(":method-preconditions")]
    MethodPreconditions,
    #[token(":task")]
    Task,
    #[token(":tasks")]
    Tasks,
    #[token(":method")]
    Method,
    #[token(":ordered-subtasks")]
    OrderedSubtasks,
    #[token(":ordered-tasks")]
    OrderedTasks,
    #[token(":subtasks")]
    Subtasks,
    #[token(":ordering")]
    Ordering,
    #[token(":order")]
    Order,
    #[token(":htn")]
    Htn,
}

impl Token {
    /// Returns a string representation of a token.
    ///
    /// The `symbols` function takes a `Token` as input and returns a string
    /// that represents its textual form. This allows converting different
    /// variants of a `Token` into a string that can be used for display, logging,
    /// or further processing in parsing or code generation. Each token type is
    /// mapped to a specific string, whether it's an identifier, keyword, operator, or error.
    ///
    /// # Arguments
    ///
    /// * `self` - The token to be converted to a string.
    ///
    /// # Returns
    ///
    /// * A `String` representing the token in its textual form.
    pub fn symbol(&self) -> String {
        match self {
            // Identifiers
            Token::ID(symbol) => symbol.to_string(),
            //Token::VarID(symbols) => symbols.to_string(),
            Token::Number(value) => value.to_string(),

            // Keywords
            Token::Define => DEFINE.to_string(),
            Token::Domain => DOMAIN.to_string(),
            Token::Problem => PROBLEM.to_string(),
            Token::Requirements => REQUIREMENTS.to_string(),
            Token::Types => TYPES.to_string(),
            Token::Constants => CONSTANTS.to_string(),
            Token::Either => EITHER.to_string(),
            Token::Predicates => PREDICATES.to_string(),
            Token::Functions => FUNCTIONS.to_string(),
            Token::Action => ACTION.to_string(),
            Token::Parameters => PARAMETERS.to_string(),
            Token::Precondition => PRECONDITION.to_string(),
            Token::Effect => EFFECT.to_string(),
            Token::Derived => DERIVED.to_string(),
            Token::DurativeAction => DURATIVE_ACTION.to_string(),
            Token::Duration => DURATION.to_string(),
            Token::Condition => CONDITION.to_string(),
            Token::DomainDef => DOMAIN_DEF.to_string(),
            Token::Objects => OBJECTS.to_string(),
            Token::Init => INIT.to_string(),
            Token::Goal => GOAL.to_string(),
            Token::Metric => METRIC.to_string(),

            // Deprecated since PDDL 2.1
            Token::Length => LENGTH.to_string(),
            Token::Serial => SERIAL.to_string(),
            Token::Parallel => PARALLEL.to_string(),

            // Expression
            Token::Preference => PREFERENCE.to_string(),
            Token::And => AND.to_string(),
            Token::Or => OR.to_string(),
            Token::Not => NOT.to_string(),
            Token::Imply => IMPLY.to_string(),
            Token::Forall => FORALL.to_string(),
            Token::Exists => EXISTS.to_string(),
            Token::When => WHEN.to_string(),

            // Requirements
            Token::Strips => STRIPS.to_string(),
            Token::Typing => TYPING.to_string(),
            Token::NegativePreconditions => NEGATIVE_PRECONDITION.to_string(),
            Token::DisjunctivePreconditions => DISJUNCTIVE_PRECONDITION.to_string(),
            Token::Equality => EQUALITY.to_string(),
            Token::ExistentialPreconditions => EXISTENTIAL_PRECONDITIONS.to_string(),
            Token::UniversalPreconditions => UNIVERSAL_PRECONDITIONS.to_string(),
            Token::QuantifiedPreconditions => QUANTIFIED_PRECONDITIONS.to_string(),
            Token::ConditionalEffects => CONDITIONAL_EFFECTS.to_string(),
            Token::Fluents => FLUENTS.to_string(),
            Token::NumericFluents => NUMERIC_FLUENTS.to_string(),
            Token::Adl => ADL.to_string(),
            Token::DurativeActions => DURATIVE_ACTIONS.to_string(),
            Token::DurationInequalities => DURATIVE_INEQUALITIES.to_string(),
            Token::ContinuousEffects => CONTINUS_EFFECTS.to_string(),
            Token::DerivedPredicates => DERIVED_PREDICATES.to_string(),
            Token::TimedInitialLiterals => TIME_INITIAL_LITERALS.to_string(),
            Token::Preferences => PREFERENCES.to_string(),
            Token::Constraints => CONSTRAINTS.to_string(),
            Token::ActionCosts => ACTION_COSTS.to_string(),

            // Separators
            Token::LParen => LPAREN.to_string(),
            Token::RParen => RPAREN.to_string(),

            // Arithmetic operators
            Token::Sub => SUB.to_string(),
            Token::Add => ADD.to_string(),
            Token::Div => DIV.to_string(),
            Token::Mul => MUL.to_string(),

            // Comparison operators
            Token::Greater => GREATER.to_string(),
            Token::Less => LESS.to_string(),
            Token::Equal => EQUAL.to_string(),
            Token::GreaterEq => GREATER_EQ.to_string(),
            Token::LessEq => LESS_EQ.to_string(),

            // Assign operators
            Token::Assign => ASSIGN.to_string(),
            Token::ScaleUp => SCALE_UP.to_string(),
            Token::ScaleDown => SCALE_DOWN.to_string(),
            Token::Increase => INCREASE.to_string(),
            Token::Decrease => DECREASE.to_string(),

            // Time
            Token::AtStart => AT_START.to_string(),
            Token::AtEnd => AT_END.to_string(),
            Token::OverAll => OVERALL.to_string(),

            // Constraints
            Token::Always => ALWAYS.to_string(),
            Token::Sometime => SOMETIME.to_string(),
            Token::Within => WITHIN.to_string(),
            Token::AtMostOnce => AT_MOST_ONCE.to_string(),
            Token::SometimeAfter => SOMETIME_AFTER.to_string(),
            Token::SometimeBefore => SOMETIME_BEFORE.to_string(),
            Token::AlwaysWithin => ALWAYS_WITHIN.to_string(),
            Token::HoldDuring => HOLD_DURING.to_string(),
            Token::HoldAfter => HOLD_AFTER.to_string(),

            // Optimization
            Token::Minimize => MINIMIZE.to_string(),
            Token::Maximize => MAXIMIZE.to_string(),

            // Special
            Token::ObjectType => OBJECT_TYPE.to_string(),
            Token::NumberType => NUMBER_TYPE.to_string(),
            Token::Undefined => UNDEFINED.to_string(),
            Token::DurationVariable => DURATION_VARIABLE.to_string(),
            Token::SharpT => SHARP_T.to_string(),
            Token::TotalTime => TOTAL_TIME.to_string(),
            Token::IsViolated => IS_VIOLATED.to_string(),

            // Comments
            Token::LineComment => LINE_COMMENT.to_string(),
            Token::CLineComment => CLINE_COMMENT.to_string(),
            Token::CBlockComment => CBLOCK_COMMENT.to_string(),

            // Error
            Token::Error(symbol) => symbol.to_string(),
            // _ => "unexpected_lexeme".to_string()

            // HDDL
            Token::Task => TASK.to_string(),
            Token::Hierarchy => HIERARCHY.to_string(),
            Token::MethodPreconditions => METHOD_PRECONDITIONS.to_string(),
            Token::Tasks => TASKS.to_string(),
            Token::Method => METHOD.to_string(),
            Token::OrderedSubtasks => ORDERED_SUBTASKS.to_string(),
            Token::OrderedTasks => ORDERED_TASKS.to_string(),
            Token::Subtasks => SUBTASKS.to_string(),
            Token::Ordering => ORDERING.to_string(),
            Token::Order => ORDER.to_string(),
            Token::Htn => HTN.to_string(),
        }
    }
}

/// Implements the `fmt::Display` trait for `Token`.
///
/// This trait implementation allows `Token` to be formatted as a human-readable string.
/// It defines how different variants of the `Token` enum should be displayed when printed using
/// the `format!` macro or when the `println!` macro is used. This helps make the `Token`
/// more understandable when debugging or logging.
///
/// The implementation works as follows:
/// - Each variant of `Token` is converted to its corresponding string representation using the
///   `symbols()` method. This provides a meaningful and readable format for all possible tokens,
///   including identifiers, keywords, operators, and more.
impl fmt::Display for Token {
    /// Formats the `Token` into a user-readable string.
    ///
    /// # Arguments
    ///
    /// * `f` - The formatter that will format the token.
    ///
    /// # Returns
    ///
    /// The result of writing the formatted string to `f`.
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.symbol())
    }
}
