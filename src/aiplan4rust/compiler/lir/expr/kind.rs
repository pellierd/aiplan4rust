use crate::aiplan4rust::cli::io::serialization::deserialize_ordered_float;
use crate::aiplan4rust::cli::io::serialization::serialize_ordered_float;
use crate::aiplan4rust::support::lang::{
    ArithmeticOp, AssignOp, AtomSkeletonId, CompareOp, FunctionSkeletonId, FunctionSymbolId,
    ObjectId, OptimizationOp, PredicateSymbolId, PreferenceSymbolId, TaskLabelSymbolId,
    TaskSkeletonId, TaskSymbolId, TypedListId, VariableId,
};
use ordered_float::OrderedFloat;
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Default, Hash, Serialize, Deserialize)]
pub enum ExprKind {
    Object(ObjectId),
    Variable(VariableId),
    FunctionSymbol(FunctionSymbolId),
    PredicateSymbol(PredicateSymbolId),
    TaskSymbol(TaskSymbolId),
    PrefName(PreferenceSymbolId),
    Function(FunctionSkeletonId),
    #[serde(
        serialize_with = "serialize_ordered_float",
        deserialize_with = "deserialize_ordered_float"
    )]
    Number(OrderedFloat<f64>),
    AtomicFormula(AtomSkeletonId),
    And,
    #[default]
    Or,
    Not,
    Imply,
    Forall(TypedListId),
    Exists(TypedListId),
    Preference,
    When,
    Comparison(CompareOp),
    Assignment(AssignOp),
    Arithmetic(ArithmeticOp),
    AtStart,
    AtEnd,
    Overall,
    Always,
    Sometime,
    Within,
    AtMostOnce,
    SometimeAfter,
    SometimeBefore,
    AlwaysWithin,
    HoldDuring,
    HoldAfter,
    TimedInitialLiteral,
    Metric(OptimizationOp),
    TotalTime,
    TotalCost,
    IsViolated,
    Length,
    Serial,
    Parallel,
    Task(TaskSkeletonId),
    TaskLabel(TaskLabelSymbolId),
    LabeledTask,                       // check
    TaskOrderingConstraint(CompareOp), // check
    TypedList(TypedListId),
}

impl ExprKind {
    pub fn to_pddl_keyword(&self) -> &'static str {
        match self {
            // Leaves and terminals (content is handled by the Content module)
            ExprKind::Object(_)
            | ExprKind::Variable(_)
            | ExprKind::FunctionSymbol(_)
            | ExprKind::PredicateSymbol(_)
            | ExprKind::TaskSymbol(_)
            | ExprKind::PrefName(_)
            | ExprKind::Function(_)
            | ExprKind::Number(_)
            | ExprKind::AtomicFormula(_)
            | ExprKind::Task(_)
            | ExprKind::TaskLabel(_)
            | ExprKind::TypedList(_)
            | ExprKind::LabeledTask => "",

            // Logical Connectives
            ExprKind::And => "and",
            ExprKind::Or => "or",
            ExprKind::Not => "not",
            ExprKind::Imply => "imply",
            ExprKind::Forall(_) => "forall",
            ExprKind::Exists(_) => "exists",
            ExprKind::When => "when",

            // Quantifiers and Preferences
            ExprKind::Preference => "preference",
            ExprKind::IsViolated => "is-violated",

            // Numerical Comparisons and Operations
            // Note: Usually handled by Content (e.g., <, >, +, -)
            ExprKind::Comparison(_) | ExprKind::Arithmetic(_) => "",
            ExprKind::Assignment(_) => "",

            // Temporal (PDDL 2.1+)
            ExprKind::AtStart => "at start",
            ExprKind::AtEnd => "at end",
            ExprKind::Overall => "overall",

            // Modal Constraints / Trajectories (PDDL 3.0)
            ExprKind::Always => "always",
            ExprKind::Sometime => "sometime",
            ExprKind::Within => "within",
            ExprKind::AtMostOnce => "at-most-once",
            ExprKind::SometimeAfter => "sometime-after",
            ExprKind::SometimeBefore => "sometime-before",
            ExprKind::AlwaysWithin => "always-within",
            ExprKind::HoldDuring => "hold-during",
            ExprKind::HoldAfter => "hold-after",

            // Temporal Planning and Metrics
            ExprKind::TimedInitialLiteral => "at",
            ExprKind::Metric(_) => "metric",
            ExprKind::TotalTime => "total-time",
            ExprKind::TotalCost => "total-cost",

            // HTN and specific extensions
            ExprKind::TaskOrderingConstraint(_) => "ordering",
            ExprKind::Serial => "serial",
            ExprKind::Parallel => "parallel",
            ExprKind::Length => "length",
        }
    }
}

impl fmt::Display for ExprKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            // --- Terminaux avec valeurs (Utilise leur propre Display) ---
            ExprKind::Object(id) => write!(f, "Object({})", id),
            ExprKind::Variable(id) => write!(f, "Variable({})", id),
            ExprKind::Number(n) => write!(f, "Number({})", n),
            ExprKind::TypedList(id) => write!(f, "TypedList(ArenaIdx: {})", id),

            // --- Symboles et Squelettes ---
            ExprKind::PredicateSymbol(id) => write!(f, "Predicate({})", id),
            ExprKind::FunctionSymbol(id) => write!(f, "Functor({})", id),
            ExprKind::TaskSymbol(id) => write!(f, "TaskSymbol({})", id),
            ExprKind::PrefName(id) => write!(f, "PrefName({})", id),
            ExprKind::TaskLabel(id) => write!(f, "TaskLabel({})", id),

            ExprKind::AtomicFormula(id) => write!(f, "Atome({})", id),
            ExprKind::Function(id) => write!(f, "Function({})", id),
            ExprKind::Task(id) => write!(f, "Task({})", id),

            // --- Opérateurs (Utilise leur propre Display) ---
            ExprKind::Comparison(op) => write!(f, "Comparison({})", op),
            ExprKind::Assignment(op) => write!(f, "Assign({})", op),
            ExprKind::Arithmetic(op) => write!(f, "Op({})", op),

            // --- Quantificateurs (Affiche le nombre de variables) ---
            ExprKind::Forall(id) => write!(f, "Forall({})", id),
            ExprKind::Exists(id) => write!(f, "Exists({})", id),

            // --- Connecteurs simples (Juste le nom) ---
            ExprKind::And => write!(f, "And"),
            ExprKind::Or => write!(f, "Or"),
            ExprKind::Not => write!(f, "Not"),
            ExprKind::Imply => write!(f, "Imply"),
            ExprKind::When => write!(f, "When"),
            ExprKind::Preference => write!(f, "Preference"),

            // --- Temporel et Modalités ---
            ExprKind::AtStart => write!(f, "AtStart"),
            ExprKind::AtEnd => write!(f, "AtEnd"),
            ExprKind::Overall => write!(f, "Overall"),
            ExprKind::Always => write!(f, "Always"),
            ExprKind::Sometime => write!(f, "Sometime"),
            ExprKind::Within => write!(f, "Within"),
            ExprKind::AtMostOnce => write!(f, "AtMostOnce"),
            ExprKind::SometimeAfter => write!(f, "SometimeAfter"),
            ExprKind::SometimeBefore => write!(f, "SometimeBefore"),
            ExprKind::AlwaysWithin => write!(f, "AlwaysWithin"),
            ExprKind::HoldDuring => write!(f, "HoldDuring"),
            ExprKind::HoldAfter => write!(f, "HoldAfter"),

            // --- Metrics et HTN ---
            ExprKind::TimedInitialLiteral => write!(f, "TimedInitialLiteral"),
            ExprKind::Metric(_) => write!(f, "Metric"),
            ExprKind::TotalTime => write!(f, "TotalTime"),
            ExprKind::TotalCost => write!(f, "TotalCost"),
            ExprKind::IsViolated => write!(f, "IsViolated"),
            ExprKind::Length => write!(f, "Length"),
            ExprKind::Serial => write!(f, "Serial"),
            ExprKind::Parallel => write!(f, "Parallel"),
            ExprKind::LabeledTask => write!(f, "LabeledTask"),
            ExprKind::TaskOrderingConstraint(_) => write!(f, "Ordering"),
        }
    }
}
