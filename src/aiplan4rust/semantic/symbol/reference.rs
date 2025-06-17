use crate::aiplan4rust::semantic::symbol::SymbolKind;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Reference {
    name: String,
    kind: SymbolKind,
}

impl Reference {
    /// Accepte tout type qui peut se convertir en `String`, ex: `&str`, `String`, `&String`.
    pub fn new<S: Into<String>>(name: S, kind: SymbolKind) -> Self {
        Self {
            name: name.into(),
            kind,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn name_mut(&mut self) -> &mut String {
        &mut self.name
    }

    pub fn set_name<S: Into<String>>(&mut self, name: S) {
        self.name = name.into();
    }

    pub fn kind(&self) -> SymbolKind {
        self.kind
    }

    pub fn set_kind(&mut self, kind: SymbolKind) {
        self.kind = kind;
    }
}

impl fmt::Display for Reference {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} ({:?})", self.name, self.kind)
    }
}
