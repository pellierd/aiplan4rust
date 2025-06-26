use std::fmt;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum Kind {
    #[default]
    True,
    False,
    Not,
    And,
    Or,
    Imply,
    Forall,
    Exists,
    Predicate,
    PrimitiveType,
    FunctionSymbol,
    TaskSymbol,
    TaskID,
    TotalTime,
    AtomicFormula,
    Task,
    Variable,
    Constant,
    FunctionTerm,

}

impl fmt::Display for Kind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Kind::True => "true",
            Kind::False => "false",
            Kind::Not => "Not",
            Kind::And => "And",
            Kind::Or => "Or",
            Kind::Imply => "Imply",
            Kind::Forall => "Forall",
            Kind::Exists => "Exists",
            Kind::Predicate => "Predicate",
            Kind::PrimitiveType => "PrimitiveType",
            Kind::FunctionSymbol => "FunctionSymbol",
            Kind::TaskSymbol => "TaskSymbol",
            Kind::TaskID => "TaskID",
            Kind::TotalTime => "TotalTime",
            Kind::AtomicFormula => "AtomicFormula",
            Kind::Task => "Task",
            Kind::Variable => "Variable",
            Kind::Constant => "Constant",
            Kind::FunctionTerm => "FunctionTerm",
        };
        write!(f, "{}", s)
    }
}
