use std::fmt;
use std::fmt::{Display, Formatter};
use logos::Logos;
use crate::aiplan4rust::serialization::header::Header;
use crate::aiplan4rust::source::SourceRole;
use crate::aiplan4rust::syntax::lexer::Token;
use crate::Language;

#[derive(Debug, Clone)]
pub enum SourceInfo {
    Raw(RawInfo),
    Serialized(SerializedInfo),
    Unknown,
}

impl SourceInfo {
    pub fn as_raw(&self) -> Option<&RawInfo> {
        match self {
            SourceInfo::Raw(raw) => Some(raw),
            _ => None,
        }
    }

    pub fn as_serialized(&self) -> Option<&SerializedInfo> {
        match self {
            SourceInfo::Serialized(serialized) => Some(serialized),
            _ => None,
        }
    }

    pub fn is_raw(&self) -> bool {
        matches!(self, SourceInfo::Raw(_))
    }

    pub fn is_serialized(&self) -> bool {
        matches!(self, SourceInfo::Serialized(_))
    }

    pub fn is_unknown(&self) -> bool {
        matches!(self, SourceInfo::Unknown)
    }
}

impl Display for SourceInfo {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            SourceInfo::Raw(raw) => write!(f, "{}", raw),
            SourceInfo::Serialized(serialized) => write!(f, "{}", serialized),
            SourceInfo::Unknown => write!(f, "Unknown source format"),
        }
    }
}



#[derive(Debug, Clone)]
pub struct RawInfo {
    role: SourceRole,
    language: Language,
}

impl RawInfo {
    pub fn new(role: SourceRole, language: Language) -> Self {
        Self { role, language }
    }
    pub fn role(&self) -> SourceRole {
        self.role.clone()
    }

    pub fn language(&self) -> Language {
        self.language.clone()
    }

    /// Tries to detect raw source info (role + language) from input.
    /// Returns `Some(RawInfo)` if a valid role is detected, otherwise `None`.
    pub fn detect_raw_info(input: &str) -> Option<RawInfo> {
        let mut lexer = Token::lexer(input);
        let mut role: Option<SourceRole> = None;
        let mut language = Language::PDDL;

        while let Some(token_res) = lexer.next() {
            let token = match token_res {
                Ok(tok) => tok,
                Err(_) => continue,
            };

            // Detect role
            if role.is_none() && token == Token::LParen {
                if let (Some(Ok(Token::Define)), Some(Ok(Token::LParen)), Some(Ok(next))) =
                    (lexer.next(), lexer.next(), lexer.next())
                {
                    match next {
                        Token::Domain => role = Some(SourceRole::Domain),
                        Token::Problem => role = Some(SourceRole::Problem),
                        _ => {}
                    }
                }
            }

            // Detect HDDL language tokens
            match token {
                Token::Task
                | Token::Method
                | Token::MethodPreconditions
                | Token::Hierarchy
                | Token::Htn => language = Language::HDDL,
                _ => {}
            }
        }

        match role {
            Some(r) => Some(RawInfo::new(r, language)),
            None => None, // No valid role detected => Unknown
        }
    }
}

impl Display for RawInfo {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "RawInfo(role: {}, language: {})", self.role, self.language)
    }
}

#[derive(Debug, Clone)]
pub struct SerializedInfo {
    header: Header,
}

impl SerializedInfo {

    pub fn new(header: Header) -> Self {
        Self { header }
    }
    pub fn header(&self) -> &Header {
        &self.header
    }
}

impl Display for SerializedInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SerializedInfo(header: {})", self.header)
    }
}
