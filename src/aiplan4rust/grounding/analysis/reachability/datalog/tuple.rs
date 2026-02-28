use std::fmt;
use crate::aiplan4rust::lang::ObjectId;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Tuple<ID> {
    /// L'identifiant de la définition (ex: ActionDefId, AtomSkeletonId)
    pub symbol: ID,
    /// Les arguments concrets (les objets instanciés)
    pub args: Vec<ObjectId>,
}

impl<ID> Tuple<ID> {
    /// Constructeur simple
    pub fn new(symbol: ID, args: Vec<ObjectId>) -> Self {
        Self { symbol, args }
    }

    /// Accesseur pour le symbole (souvent un ID copiable comme u32/u64)
    pub fn symbol(&self) -> ID
    where
        ID: Copy
    {
        self.symbol
    }

    /// Accesseur pour les arguments
    pub fn args(&self) -> &[ObjectId] {
        &self.args
    }

    /// Utile pour l'arité (nombre d'arguments)
    pub fn arity(&self) -> usize {
        self.args.len()
    }
}

impl<ID: fmt::Display> fmt::Display for Tuple<ID> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}(", self.symbol)?;
        for (i, arg) in self.args.iter().enumerate() {
            if i > 0 {
                write!(f, ", ")?;
            }
            write!(f, "{}", arg)?;
        }
        write!(f, ")")
    }
}
