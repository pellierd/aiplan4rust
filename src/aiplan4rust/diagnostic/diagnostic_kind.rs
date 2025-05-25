use crate::aiplan4rust::diagnostic::DiagnosticSeverity;
use crate::aiplan4rust::parser::elements::Requirement;
use crate::aiplan4rust::parser::syntax_tree::SyntaxNodeKind;
use crate::aiplan4rust::semantic_analyser::AnnotatedSyntaxNode;
use crate::aiplan4rust::semantic_analyser::symbol::{Declaration, SymbolKind};

use std::fmt;

// Enum pour différents types de diagnostics (erreurs, avertissements, etc.)
#[derive(Clone, Debug, PartialEq)]
pub enum DiagnosticKind {
    UnexpectedToken {
        token: String,
        expected: Vec<String>,
    },
    UnexpectedEof {
        expected: Vec<String>,
    },
    InvalidToken,
    ExtraToken {
        token: String,
    },
    DuplicatedRequirementDeclaration {
        requirement: Requirement
    },
    DuplicatedTypeDeclaration {
        ty: String,
    },
    UnDefinedFunction {
        symbol: String,
    },
    UnDefinedPredicate {
        symbol: String,
    },
    UnDefinedCompoundTask {
        symbol: String,
    },
    UnDefinedPrimitiveTask {
        symbol: String,
    },
    TypeMismatchInExpression {
        ty1: Vec<String>,
        ty2: Vec<String>,
    },
    InvalidTypesInNumericExpression {
        ty1: Vec<String>,
        ty2: Vec<String>,
    },
    RequirementViolation {
        node_kind: SyntaxNodeKind,
        required: Vec<Requirement>,
    },
    DuplicatedDeclarationInScope {
        symbol: String,
        declaration1: Declaration,
        declaration2: Declaration,
        scope: AnnotatedSyntaxNode,
    },
    CyclicOrderingConstraint,
    UndeclaredSymbol {
        symbol: String,
        kind : SymbolKind
    },
    ReservedSymbolUsedAs {
        symbol: String,
        actual_kind: SymbolKind,
        expected_kind: SymbolKind,
        requirements: Vec<Requirement>,
    },
    AmbiguousSymbolUsageWithKeyword {
        symbol: String,
        actual_kind: SymbolKind,
        requirements: Vec<Requirement>,
    },
    UnusedSymbol {
        symbol: String,
        kind: SymbolKind,
    },
    DomainProblemNameMismatch {
        domain_name: String,
        problem_name: String,
    },
    WarningAmbiguousTypePredicateSymbol {
        symbol: String,
    },
    WarningTaskArgumentIsSupertypeOfDeclaration {
        argument: String,
        type_declared: Vec<String>,
        type_used: Vec<String>,
    },
    CustomError(String),
}

impl DiagnosticKind {
    pub fn code(&self) -> String {
        match self {
            // ERROR PARSER
            DiagnosticKind::UnexpectedToken{ .. } => "E0001".to_string(),
            DiagnosticKind::UnexpectedEof{ .. } => "E0002".to_string(),
            DiagnosticKind::InvalidToken => "E0003".to_string(),
            DiagnosticKind::ExtraToken{ .. } => "E0004".to_string(),
            // WARNING PARSER
            DiagnosticKind::DuplicatedRequirementDeclaration { .. } => "W0001".to_string(),
            DiagnosticKind::DuplicatedTypeDeclaration { .. } => "W0002".to_string(),
            // ERROR ANALYSER
            DiagnosticKind::UnDefinedFunction { .. } => "E1001".to_string(),
            DiagnosticKind::UnDefinedPredicate { .. } => "E1002".to_string(),
            DiagnosticKind::UnDefinedCompoundTask { .. } => "E1003".to_string(),
            DiagnosticKind::UnDefinedPrimitiveTask { .. } => "E1004".to_string(),
            DiagnosticKind::TypeMismatchInExpression { .. } => "E1005".to_string(),
            DiagnosticKind::InvalidTypesInNumericExpression { .. } => "E1006".to_string(),
            DiagnosticKind::DuplicatedDeclarationInScope { .. } => "E1007".to_string(),
            DiagnosticKind::CyclicOrderingConstraint { .. } => "E1008".to_string(),
            DiagnosticKind::UndeclaredSymbol { .. } => "E1009".to_string(),
            DiagnosticKind::ReservedSymbolUsedAs { .. } => "E1010".to_string(),
            // WARNINGS ANALYSER
            DiagnosticKind::AmbiguousSymbolUsageWithKeyword { .. } => "W1010".to_string(),
            DiagnosticKind::UnusedSymbol { .. } => "W1011".to_string(),
            DiagnosticKind::RequirementViolation { .. } => "W10012".to_string(),
            DiagnosticKind::WarningAmbiguousTypePredicateSymbol { .. } => "W1013".to_string(),
            DiagnosticKind::WarningTaskArgumentIsSupertypeOfDeclaration { .. } => "W1014".to_string(),
            // WARNINGS LINKER
            DiagnosticKind::DomainProblemNameMismatch { .. } => "W2000".to_string(),


            DiagnosticKind::CustomError(_) => "E000X".to_string(),
        }
    }

    // Centraliser le message d'erreur directement dans l'enum
    pub fn message(&self) -> String {
        match self {
            DiagnosticKind::UnexpectedToken { token, .. } => {
                format!("Unexpected token '{}'.", token)
            }
            DiagnosticKind::UnexpectedEof { .. } => {
                "Unexpected end of input (EOF).".to_string()
            }
            DiagnosticKind::InvalidToken => {
                "Unrecognized or malformed token.".to_string()
            }
            DiagnosticKind::ExtraToken { token } => {
                format!("Unexpected extra token '{}'.", token)
            }
            DiagnosticKind::DuplicatedRequirementDeclaration { requirement } => {
                format!("Requirement '{}' is declared more than once.", requirement)
            }
            DiagnosticKind::DuplicatedTypeDeclaration { ty } => {
                format!("Type '{}' is declared multiple times.", ty)
            }
            DiagnosticKind::UnDefinedFunction { symbol } => {
                format!("Function '{}' is undefined", symbol)
            }
            DiagnosticKind::UnDefinedPredicate { symbol } => {
                format!("Predicate '{}' is undefined", symbol)
            }
            DiagnosticKind::UnDefinedCompoundTask { symbol } => {
                format!("Compound task '{}' is undefined", symbol)
            }
            DiagnosticKind::UnDefinedPrimitiveTask { symbol } => {
                format!("Primitive task '{}' is undefined", symbol)
            }
            DiagnosticKind::TypeMismatchInExpression { .. } => {
                "Type mismatch in expression.".to_string()
            }
            DiagnosticKind::InvalidTypesInNumericExpression { .. } => {
                "Invalid operand types in numeric expression.".to_string()
            }
            DiagnosticKind::RequirementViolation { node_kind, .. } => {
                format!("'{}' expression is not allowed in the current context.", node_kind)
            }
            DiagnosticKind::DuplicatedDeclarationInScope { symbol, .. } => {
                format!("Duplicate declaration of symbol '{}'.", symbol)
            }
            DiagnosticKind::CyclicOrderingConstraint => {
                "Cyclic task-ordering constraint detected.".to_string()
            }
            DiagnosticKind::UndeclaredSymbol { symbol , kind} => {
                format!("{} symbol '{}' undeclared.", symbol, kind)
            }
            DiagnosticKind::ReservedSymbolUsedAs {symbol, ..} => {
                format!("Symbol '{}' used as a language keyword", symbol)
            }
            DiagnosticKind::AmbiguousSymbolUsageWithKeyword {symbol, ..} => {
                format!("Symbol '{}' is ambiguous as a language keyword", symbol)
            }
            DiagnosticKind::UnusedSymbol {symbol, kind  } => {
                format!("{} Symbol '{}' is unused", kind, symbol)
            }
            DiagnosticKind::DomainProblemNameMismatch { domain_name, problem_name } => {
                format!("Domain name '{}' does not match problem name '{}'.", domain_name, problem_name)
            }
            DiagnosticKind::WarningAmbiguousTypePredicateSymbol { symbol, .. } => {
                format!("Ambiguous symbol '{}': declared both as a type and a predicate in the same scope.", symbol)
            }
            DiagnosticKind::WarningTaskArgumentIsSupertypeOfDeclaration { argument, .. } => {
                format!("Upcasting detected: argument '{}' has broader type(s) than declared.",
                argument)
            }
            DiagnosticKind::CustomError(msg) => msg.to_string(),
        }
    }

    pub fn severity(&self) -> DiagnosticSeverity {
        match self {
            // PARSER ERRORS
            DiagnosticKind::UnexpectedToken { .. } => DiagnosticSeverity::Error,
            DiagnosticKind::UnexpectedEof { .. } => DiagnosticSeverity::Error,
            DiagnosticKind::InvalidToken => DiagnosticSeverity::Error,
            DiagnosticKind::ExtraToken { .. } => DiagnosticSeverity::Error,
            DiagnosticKind::CustomError(_) => DiagnosticSeverity::Error,
            // PARSER WARNINGS
            DiagnosticKind::DuplicatedRequirementDeclaration { .. } => DiagnosticSeverity::Warning,
            DiagnosticKind::DuplicatedTypeDeclaration { .. } => DiagnosticSeverity::Warning,
            // ANALYSER ERRORS
            DiagnosticKind::UnDefinedFunction { .. } => DiagnosticSeverity::Error,
            DiagnosticKind::UnDefinedPredicate { .. } => DiagnosticSeverity::Error,
            DiagnosticKind::UnDefinedCompoundTask { .. } => DiagnosticSeverity::Error,
            DiagnosticKind::UnDefinedPrimitiveTask { .. } => DiagnosticSeverity::Error,
            DiagnosticKind::TypeMismatchInExpression { .. } => DiagnosticSeverity::Error,
            DiagnosticKind::InvalidTypesInNumericExpression { .. } => DiagnosticSeverity::Error,
            DiagnosticKind::DuplicatedDeclarationInScope { .. } => DiagnosticSeverity::Error,
            DiagnosticKind::CyclicOrderingConstraint => DiagnosticSeverity::Error,
            DiagnosticKind::UndeclaredSymbol { .. } => DiagnosticSeverity::Error,
            DiagnosticKind::ReservedSymbolUsedAs { .. } => DiagnosticSeverity::Error,
            // ANALYSER WARNINGS
            DiagnosticKind::AmbiguousSymbolUsageWithKeyword { .. } => DiagnosticSeverity::Warning,
            DiagnosticKind::UnusedSymbol { .. } => DiagnosticSeverity::Warning,
            DiagnosticKind::RequirementViolation { .. } => DiagnosticSeverity::Warning,
            DiagnosticKind::WarningAmbiguousTypePredicateSymbol { .. } => DiagnosticSeverity::Warning,
            DiagnosticKind::WarningTaskArgumentIsSupertypeOfDeclaration { .. } => DiagnosticSeverity::Warning,

            // LINKER WARNINGS
            DiagnosticKind::DomainProblemNameMismatch { .. } => DiagnosticSeverity::Warning,

        }
    }
    pub fn suggestion(&self) -> Option<String> {
        match self {
            DiagnosticKind::UnexpectedToken { expected, .. }
            | DiagnosticKind::UnexpectedEof { expected } => {
                Self::format_expected_message(expected)
            }
            DiagnosticKind::ExtraToken { .. } => {
                Some("Extra token detected. Check for unnecessary symbols or misplaced characters.".to_string())
            }
            DiagnosticKind::InvalidToken => {
                Some("Make sure there are no typos or invalid characters.".to_string())
            }
            DiagnosticKind::DuplicatedRequirementDeclaration { requirement} => {
                Some(format!("Requirement '{}' is already declared. You can safely remove the duplicate.", requirement))
            }
            DiagnosticKind::DuplicatedTypeDeclaration { .. } => {
                Some("This type is already declared. Consider removing the duplicate.".to_string())
            }
            DiagnosticKind::CustomError(_) => None,
            DiagnosticKind::UnDefinedFunction { symbol } => {
                Some(format!("Make sure that a function with call '{}' is defined in the block ':functions'", symbol))
            }
            DiagnosticKind::UnDefinedPredicate { symbol } => {
                Some(format!("Make sure that a predicate named '{}' is defined in the block ':predicates'", symbol))
            }
            DiagnosticKind::UnDefinedCompoundTask { symbol } => {
                Some(format!("Make sure that a compound task '{}' is defined in the block ':tasks'", symbol))
            }
            DiagnosticKind::UnDefinedPrimitiveTask { symbol } => {
                Some(format!("Make sure that an action '{}' is defined", symbol))
            }
            DiagnosticKind::TypeMismatchInExpression { ty1, ty2 } => {
                Some(format!(
                    "Incompatible types: {:?} is not related to {:?} by the type hierarchy.",
                    Self::format_types(ty1),
                    Self::format_types(ty2)
                ))
            }
            DiagnosticKind::InvalidTypesInNumericExpression { ty1, ty2 } => {
                Some(format!(
                    "Numeric expressions require operands of type 'number', but found types {:?} and {:?}. Ensure both operands are numbers.",
                    Self::format_types(ty1), Self::format_types(ty2)
                ))
            }
            DiagnosticKind::RequirementViolation { node_kind, required } => {
                Some(format!(
                    "The use of '{}' requires one of the following requirements: {}.",
                    node_kind,  // ou juste format!("{:?}", node_kind) si pas encore défini
                    Self::format_requirements_list(&required)
                ))
            }
            DiagnosticKind::DuplicatedDeclarationInScope { symbol, declaration1, declaration2, .. } => {
                Some(format!(
                    "The symbol '{}' is declared once as a '{}' and again as a '{}'. \
                        Consider renaming one of the declarations or ensuring consistent usage.",
                    symbol,
                    declaration1.kind(),
                    declaration2.kind()
                ))
            }
            DiagnosticKind::CyclicOrderingConstraint => Some("Check for loops in your task dependencies or ordering constraints.".to_string()),
            DiagnosticKind::UndeclaredSymbol { symbol, kind } => {
                match kind {
                    SymbolKind::Function => Some(format!(
                        "Function '{}' is not declared. Please declare it before use in the ':functions' block.",
                        symbol
                    )),
                    SymbolKind::Predicate => Some(format!(
                        "Predicate '{}' is not declared. Please declare it before use in ':predicates' block.",
                        symbol
                    )),
                    SymbolKind::Action => Some(format!(
                        "Action '{}' is not declared. Please define it using the ':action' keyword.",
                        symbol
                    )),
                    SymbolKind::DASymbol => Some(format!(
                        "Durative action '{}' is not declared. Please define it using the ':durative-action' keyword.",
                        symbol
                    )),
                    SymbolKind::Method => Some(format!(
                        "Method '{}' is not declared. Please define it in the ':methods' keyword.",
                        symbol
                    )),
                    SymbolKind::Task => Some(format!(
                        "Task '{}' is not declared. Please ensure it's defined using the ':task' keyword.",
                        symbol
                    )),
                    SymbolKind::TaskID => Some(format!(
                        "Task identifier '{}' is not declared. Verify it's correctly assigned in your task network.",
                        symbol
                    )),
                    SymbolKind::Constant => Some(format!(
                        "Constant '{}' is not declared. Declare it in the ':constants' section in domain files of in ':object' in problem files.",
                        symbol
                    )),
                    SymbolKind::DomainName => Some(format!(
                        "Domain '{}' is not recognized. Make sure the domain name is correctly defined.",
                        symbol
                    )),
                    SymbolKind::PrimitiveType => Some(format!(
                        "Type '{}' is not declared. Ensure it's defined in the ':types' section.",
                        symbol
                    )),
                    SymbolKind::ProblemName => Some(format!(
                        "Problem '{}' is not declared. Verify the problem file or declaration.",
                        symbol
                    )),
                    SymbolKind::Requirement => Some(format!(
                        "Requirement '{}' is not recognized. Check for typos or unsupported features.",
                        symbol
                    )),
                    SymbolKind::Variable => Some(format!(
                        "Variable '{}' is not declared. Declare it using the correct syntax (e.g., '?x - type').",
                        symbol
                    )),
                }
            }
            DiagnosticKind::ReservedSymbolUsedAs { symbol, actual_kind, expected_kind, requirements } => {
                Some(format!(
                    "Symbol '{}' is reserved as '{}' in the language with requirements: {}. '{}' expected. Consider renaming it or using a different symbol.",
                    symbol,
                    actual_kind,
                    Self::format_requirements_list(requirements),
                    expected_kind,
                ))
            }

            DiagnosticKind::AmbiguousSymbolUsageWithKeyword { symbol, actual_kind, requirements } => {
                Some(format!(
                    "{} symbol '{}' is ambiguous as it is used as a keyword in the language with requirements: {}. Consider renaming it or using a different symbol.",
                    actual_kind,
                    symbol,
                    Self::format_requirements_list(requirements),
                ))
            }
            DiagnosticKind::UnusedSymbol {symbol, kind} => {
                Some(format!(
                    "{} symbol '{}' is declared but not used. Consider removing it to clean up your code.",
                    kind,
                    symbol
                ))
            }
            DiagnosticKind::DomainProblemNameMismatch { domain_name, .. } => {
                Some(format!(
                    "Ensure that the domain name in the problem file matches the domain definition: expected '{}'.",
                    domain_name
                ))
            }
            DiagnosticKind::WarningAmbiguousTypePredicateSymbol { symbol} => {
                Some(format!(
                    "The symbol '{}' is declared both as a type and a predicate in the same scope. This can lead to confusion. Consider renaming one of them.",
                    symbol
                ))
            }
            DiagnosticKind::WarningTaskArgumentIsSupertypeOfDeclaration {argument, type_declared, type_used} => {
                Some(format!(
                    "The argument '{}' uses type(s) '{}', which are supertypes of the declared type(s) '{}'. \
                        Consider using the exact or a more specific type.",
                    argument,
                    Self::format_types(type_declared),
                    Self::format_types(type_used)
                ))
            }
        }
    }

    fn format_expected_message(expected: &[String]) -> Option<String> {
        match expected.len() {
            0 => Some("Unexpected input. Please verify the syntax near this token.".to_string()),
            1 => Some(format!("Expected token: `{}`.", expected[0])),
            _ => Some(format!(
                "Expected one of the following tokens: {}.",
                Self::join_expected_tokens(expected)
            )),
        }
    }

    fn join_expected_tokens(expected: &[String]) -> String {
        expected
            .iter()
            .map(|t| format!("'{}'", t))
            .collect::<Vec<_>>()
            .join(", ")
    }

    /// Format a vector of types for display.
    /// - If there is only one type, it returns the type as-is.
    /// - If there are multiple types, it returns them in the form `(either t1 t2 ...)`.
    fn format_types(types: &[String]) -> String {
        match types.len() {
            0 => "unknown".to_string(),
            1 => types[0].clone(),
            _ => format!("(either {})", types.join(" ")),
        }
    }

    fn format_requirements_list(requirements: &[Requirement]) -> String {
        requirements
            .iter()
            .map(|r| format!("'{}'", r)) // ou r.to_string() si implémenté
            .collect::<Vec<_>>()
            .join(", ")
    }
}

impl fmt::Display for DiagnosticKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let code = self.code();
        let message = self.message();
        let severity = self.severity();
        let suggestion = self.suggestion();

        match suggestion {
            Some(sugg) => write!(
                f,
                "[{}] ({}) {}. Suggestion: {}",
                code,
                severity,
                message,
                sugg
            ),
            None => write!(f, "[{}] ({}) {}", code, severity, message),
        }
    }
}
