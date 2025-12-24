use std::fmt;
use crate::aiplan4rust::io::RawKind;
use crate::Language;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct RawContent {
    kind: RawKind,
    language: Language,
    inner: String,
}

impl RawContent {
    pub fn new(kind: RawKind, language: Language, inner: impl Into<String>) -> Self {
        Self {
            kind,
            language,
            inner: inner.into(),
        }
    }

    pub fn kind(&self) -> RawKind {
        self.kind
    }

    pub fn language(&self) -> Language {
        self.language
    }

    pub fn inner(&self) -> &str {
        &self.inner
    }
}

impl fmt::Display for RawContent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "RawContent {{ kind: {}, language: {}, length: {} }}",
            self.kind,
            self.language,
            self.inner.len()
        )
    }
}
