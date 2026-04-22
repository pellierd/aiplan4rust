use crate::aiplan4rust::lang::{
    ArithmeticOp, AssignOp, AtomSkeletonId, CompareOp, FunctionSkeletonId, FunctionSymbolId,
    ObjectId, OptimizationOp, PredicateSymbolId, PreferenceSymbolId, TaskLabelSymbolId,
    TaskSkeletonId, TaskSymbolId, TypeId, TypedList, VariableId,
};
use crate::aiplan4rust::serialization::deserialize_ordered_float;
use crate::aiplan4rust::serialization::serialize_ordered_float;
use ordered_float::OrderedFloat;
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Clone, Debug, PartialEq, Eq, Default, Hash, Serialize, Deserialize)]
pub enum ExprEntryKind {
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
    Forall(TypedList<VariableId, TypeId>),
    Exists(TypedList<VariableId, TypeId>),
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
}

impl ExprEntryKind {
    pub fn to_pddl_keyword(&self) -> &'static str {
        match self {
            // Leaves and terminals (content is handled by the Content module)
            ExprEntryKind::Object(_)
            | ExprEntryKind::Variable(_)
            | ExprEntryKind::FunctionSymbol(_)
            | ExprEntryKind::PredicateSymbol(_)
            | ExprEntryKind::TaskSymbol(_)
            | ExprEntryKind::PrefName(_)
            | ExprEntryKind::Function(_)
            | ExprEntryKind::Number(_)
            | ExprEntryKind::AtomicFormula(_)
            | ExprEntryKind::Task(_)
            | ExprEntryKind::TaskLabel(_)
            | ExprEntryKind::LabeledTask => "",

            // Logical Connectives
            ExprEntryKind::And => "and",
            ExprEntryKind::Or => "or",
            ExprEntryKind::Not => "not",
            ExprEntryKind::Imply => "imply",
            ExprEntryKind::Forall(_) => "forall",
            ExprEntryKind::Exists(_) => "exists",
            ExprEntryKind::When => "when",

            // Quantifiers and Preferences
            ExprEntryKind::Preference => "preference",
            ExprEntryKind::IsViolated => "is-violated",

            // Numerical Comparisons and Operations
            // Note: Usually handled by Content (e.g., <, >, +, -)
            ExprEntryKind::Comparison(_) | ExprEntryKind::Arithmetic(_) => "",
            ExprEntryKind::Assignment(_) => "",

            // Temporal (PDDL 2.1+)
            ExprEntryKind::AtStart => "at start",
            ExprEntryKind::AtEnd => "at end",
            ExprEntryKind::Overall => "overall",

            // Modal Constraints / Trajectories (PDDL 3.0)
            ExprEntryKind::Always => "always",
            ExprEntryKind::Sometime => "sometime",
            ExprEntryKind::Within => "within",
            ExprEntryKind::AtMostOnce => "at-most-once",
            ExprEntryKind::SometimeAfter => "sometime-after",
            ExprEntryKind::SometimeBefore => "sometime-before",
            ExprEntryKind::AlwaysWithin => "always-within",
            ExprEntryKind::HoldDuring => "hold-during",
            ExprEntryKind::HoldAfter => "hold-after",

            // Temporal Planning and Metrics
            ExprEntryKind::TimedInitialLiteral => "at",
            ExprEntryKind::Metric(_) => "metric",
            ExprEntryKind::TotalTime => "total-time",
            ExprEntryKind::TotalCost => "total-cost",

            // HTN and specific extensions
            ExprEntryKind::TaskOrderingConstraint(_) => "ordering",
            ExprEntryKind::Serial => "serial",
            ExprEntryKind::Parallel => "parallel",
            ExprEntryKind::Length => "length",
        }
    }
}

impl fmt::Display for ExprEntryKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            // --- Terminaux avec valeurs (Utilise leur propre Display) ---
            ExprEntryKind::Object(id) => write!(f, "Object({})", id),
            ExprEntryKind::Variable(id) => write!(f, "Variable({})", id),
            ExprEntryKind::Number(n) => write!(f, "Number({})", n),

            // --- Symboles et Squelettes ---
            ExprEntryKind::PredicateSymbol(id) => write!(f, "Predicate({})", id),
            ExprEntryKind::FunctionSymbol(id) => write!(f, "Functor({})", id),
            ExprEntryKind::TaskSymbol(id) => write!(f, "TaskSymbol({})", id),
            ExprEntryKind::PrefName(id) => write!(f, "PrefName({})", id),
            ExprEntryKind::TaskLabel(id) => write!(f, "TaskLabel({})", id),

            ExprEntryKind::AtomicFormula(id) => write!(f, "Atome({})", id),
            ExprEntryKind::Function(id) => write!(f, "Function({})", id),
            ExprEntryKind::Task(id) => write!(f, "Task({})", id),

            // --- Opérateurs (Utilise leur propre Display) ---
            ExprEntryKind::Comparison(op) => write!(f, "Comparison({})", op),
            ExprEntryKind::Assignment(op) => write!(f, "Assign({})", op),
            ExprEntryKind::Arithmetic(op) => write!(f, "Op({})", op),

            // --- Quantificateurs (Affiche le nombre de variables) ---
            ExprEntryKind::Forall(vars) => write!(f, "Forall({})", vars.len()),
            ExprEntryKind::Exists(vars) => write!(f, "Exists({})", vars.len()),

            // --- Connecteurs simples (Juste le nom) ---
            ExprEntryKind::And => write!(f, "And"),
            ExprEntryKind::Or => write!(f, "Or"),
            ExprEntryKind::Not => write!(f, "Not"),
            ExprEntryKind::Imply => write!(f, "Imply"),
            ExprEntryKind::When => write!(f, "When"),
            ExprEntryKind::Preference => write!(f, "Preference"),

            // --- Temporel et Modalités ---
            ExprEntryKind::AtStart => write!(f, "AtStart"),
            ExprEntryKind::AtEnd => write!(f, "AtEnd"),
            ExprEntryKind::Overall => write!(f, "Overall"),
            ExprEntryKind::Always => write!(f, "Always"),
            ExprEntryKind::Sometime => write!(f, "Sometime"),
            ExprEntryKind::Within => write!(f, "Within"),
            ExprEntryKind::AtMostOnce => write!(f, "AtMostOnce"),
            ExprEntryKind::SometimeAfter => write!(f, "SometimeAfter"),
            ExprEntryKind::SometimeBefore => write!(f, "SometimeBefore"),
            ExprEntryKind::AlwaysWithin => write!(f, "AlwaysWithin"),
            ExprEntryKind::HoldDuring => write!(f, "HoldDuring"),
            ExprEntryKind::HoldAfter => write!(f, "HoldAfter"),

            // --- Metrics et HTN ---
            ExprEntryKind::TimedInitialLiteral => write!(f, "TimedInitialLiteral"),
            ExprEntryKind::Metric(_) => write!(f, "Metric"),
            ExprEntryKind::TotalTime => write!(f, "TotalTime"),
            ExprEntryKind::TotalCost => write!(f, "TotalCost"),
            ExprEntryKind::IsViolated => write!(f, "IsViolated"),
            ExprEntryKind::Length => write!(f, "Length"),
            ExprEntryKind::Serial => write!(f, "Serial"),
            ExprEntryKind::Parallel => write!(f, "Parallel"),
            ExprEntryKind::LabeledTask => write!(f, "LabeledTask"),
            ExprEntryKind::TaskOrderingConstraint(_) => write!(f, "Ordering"),
        }
    }
}
