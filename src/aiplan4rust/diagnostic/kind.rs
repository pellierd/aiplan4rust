use crate::aiplan4rust::diagnostic::Severity;
use crate::aiplan4rust::syntax::elements::Requirement;
use crate::aiplan4rust::syntax::ast::AstKind;
use crate::aiplan4rust::semantic::hir::HirNode;
use crate::aiplan4rust::semantic::symbol::{Declaration, SymbolKind, Usage};

use std::fmt;

// Enum pour différents types de diagnostics (erreurs, avertissements, etc.)
#[derive(Clone, Debug, PartialEq)]
pub enum Kind {
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
        node_kind: AstKind,
        required: Vec<Requirement>,
    },
    DuplicatedSymbolDeclarationInScopeError {
        symbol: String,
        declaration1: Declaration,
        declaration2: Declaration,
        scope: HirNode,
    },
    CyclicTaskOrderingError,
    UndeclaredSymbolError {
        usage: Usage,
    },
    SymbolDeclaredAsKeywordError {
        declaration: Declaration,
        expected_kind: SymbolKind,
        requirements: Vec<Requirement>,
    },
    SymbolDeclaredAmbiguouslyAsKeywordWarning {
        declaration: Declaration,
        requirements: Vec<Requirement>,
    },
    UnusedSymbolWarning {
        declaration: Declaration,
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
    DuplicateTypesInSymbolDeclarationWarning {
        symbol: String,
        duplicate_types: Vec<String>,
    },
    ImplicitEitherTypeDeclarationWarning {
        ty: String,
        types: Vec<String>,
    },
    CyclicTypeDeclarationError {
       cycle: Vec<Declaration>
    },
    CrossConflictSymbolDeclarationError {
        symbol: String,
        problem_kind: SymbolKind,
        domain_kinds: Vec<SymbolKind>,
    },
    DuplicateRequirementDeclarationWarning {
        requirement: Requirement,
    },
    CustomError(String),
}

impl Kind {
    pub fn code(&self) -> String {
        match self {
            // ERROR PARSER
            Kind::UnexpectedToken{ .. } => "E0001".to_string(),
            Kind::UnexpectedEof{ .. } => "E0002".to_string(),
            Kind::InvalidToken => "E0003".to_string(),
            Kind::ExtraToken{ .. } => "E0004".to_string(),
            // WARNING PARSER
            Kind::DuplicatedRequirementDeclaration { .. } => "W0001".to_string(),
            Kind::DuplicatedTypeDeclaration { .. } => "W0002".to_string(),
            // WARNING NORMALIZER
            Kind::DuplicateRequirementDeclarationWarning { ..  } => "W2001".to_string(),
            // ERROR ANALYSER
            Kind::UnDefinedFunction { .. } => "E1001".to_string(),
            Kind::UnDefinedPredicate { .. } => "E1002".to_string(),
            Kind::UnDefinedCompoundTask { .. } => "E1003".to_string(),
            Kind::UnDefinedPrimitiveTask { .. } => "E1004".to_string(),
            Kind::TypeMismatchInExpression { .. } => "E1005".to_string(),
            Kind::InvalidTypesInNumericExpression { .. } => "E1006".to_string(),
            Kind::DuplicatedSymbolDeclarationInScopeError { .. } => "E1007".to_string(),
            Kind::CyclicTaskOrderingError { .. } => "E1008".to_string(),
            Kind::UndeclaredSymbolError { .. } => "E1009".to_string(),
            Kind::SymbolDeclaredAsKeywordError { .. } => "E1010".to_string(),
            Kind::CyclicTypeDeclarationError { .. } => "E1011".to_string(),
            // WARNINGS ANALYSER
            Kind::SymbolDeclaredAmbiguouslyAsKeywordWarning { .. } => "W1010".to_string(),
            Kind::UnusedSymbolWarning { .. } => "W1011".to_string(),
            Kind::RequirementViolation { .. } => "W10012".to_string(),
            Kind::WarningAmbiguousTypePredicateSymbol { .. } => "W1013".to_string(),
            Kind::WarningTaskArgumentIsSupertypeOfDeclaration { .. } => "W1014".to_string(),
            Kind::DuplicateTypesInSymbolDeclarationWarning { .. } => "W1015".to_string(),
            Kind::ImplicitEitherTypeDeclarationWarning { .. } => "W1016".to_string(),

            // WARNINGS LINKER
            Kind::DomainProblemNameMismatch { .. } => "W2000".to_string(),
            Kind::CrossConflictSymbolDeclarationError { .. } => "W2001".to_string(),


            Kind::CustomError(_) => "E000X".to_string(),
        }
    }

    // Centraliser le message d'erreur directement dans l'enum
    pub fn message(&self) -> String {
        match self {
            Kind::UnexpectedToken { token, .. } => {
                format!("Unexpected token '{}'.", token)
            }
            Kind::UnexpectedEof { .. } => {
                "Unexpected end of input (EOF).".to_string()
            }
            Kind::InvalidToken => {
                "Unrecognized or malformed token.".to_string()
            }
            Kind::ExtraToken { token } => {
                format!("Unexpected extra token '{}'.", token)
            }
            Kind::DuplicatedRequirementDeclaration { requirement } => {
                format!("Requirement '{}' is declared more than once.", requirement)
            }
            Kind::DuplicatedTypeDeclaration { ty } => {
                format!("Type '{}' is declared multiple times.", ty)
            }
            Kind::UnDefinedFunction { symbol } => {
                format!("Function '{}' is undefined", symbol)
            }
            Kind::UnDefinedPredicate { symbol } => {
                format!("Predicate '{}' is undefined", symbol)
            }
            Kind::UnDefinedCompoundTask { symbol } => {
                format!("Compound task '{}' is undefined", symbol)
            }
            Kind::UnDefinedPrimitiveTask { symbol } => {
                format!("Primitive task '{}' is undefined", symbol)
            }
            Kind::TypeMismatchInExpression { .. } => {
                "Type mismatch in expression.".to_string()
            }
            Kind::InvalidTypesInNumericExpression { .. } => {
                "Invalid operand types in numeric expression.".to_string()
            }
            Kind::RequirementViolation { node_kind, .. } => {
                format!("'{}' expression is not allowed in the current context.", node_kind)
            }
            Kind::DuplicatedSymbolDeclarationInScopeError { symbol, .. } => {
                format!("Duplicate declaration of symbol '{}'.", symbol)
            }
            Kind::CyclicTaskOrderingError => {
                "Cyclic task-ordering constraint detected.".to_string()
            }
            Kind::UndeclaredSymbolError { usage} => {
                format!("{} symbol '{}' undeclared.", usage.symbol(), usage.kind())
            }
            Kind::SymbolDeclaredAsKeywordError {declaration, ..} => {
                format!("Symbol '{}' used as a language keyword", declaration.symbol())
            }
            Kind::SymbolDeclaredAmbiguouslyAsKeywordWarning {declaration, ..} => {
                format!("Symbol '{}' is ambiguous as a language keyword", declaration.symbol())
            }
            Kind::UnusedSymbolWarning { declaration } => {
                format!("{} Symbol '{}' is unused", declaration.kind(), declaration.symbol())
            }
            Kind::DomainProblemNameMismatch { domain_name, problem_name } => {
                format!("Domain name '{}' does not match problem name '{}'.", domain_name, problem_name)
            }
            Kind::WarningAmbiguousTypePredicateSymbol { symbol, .. } => {
                format!("Ambiguous symbol '{}': declared both as a type and a predicate in the same scope.", symbol)
            }
            Kind::WarningTaskArgumentIsSupertypeOfDeclaration { argument, .. } => {
                format!("Upcasting detected: argument '{}' has broader type(s) than declared.",
                argument)
            }
            Kind::DuplicateTypesInSymbolDeclarationWarning { symbol, .. } => {
                format!("Symbol '{}' has duplicated types in its declaration.", symbol)
            }
            Kind::ImplicitEitherTypeDeclarationWarning { ty, ..} => {
                format!(
                    "Implicit disjunctive type declaration for {}.",
                    ty,
                )
            }
            Kind::CyclicTypeDeclarationError { ..} => {
                "Cycle detected in type declarations, causing an invalid hierarchy.".to_string()
            }
            Kind::CrossConflictSymbolDeclarationError { .. } => {
                "Symbol declaration in problem conflicts with domain declaration.".to_string()
            }
            Kind::DuplicateRequirementDeclarationWarning { .. } => {
                "Redundant requirement declaration detected.".to_string()
            }
            Kind::CustomError(msg) => msg.to_string(),
        }
    }

    pub fn severity(&self) -> Severity {
        match self {
            // PARSER ERRORS
            Kind::UnexpectedToken { .. } => Severity::Error,
            Kind::UnexpectedEof { .. } => Severity::Error,
            Kind::InvalidToken => Severity::Error,
            Kind::ExtraToken { .. } => Severity::Error,
            Kind::CustomError(_) => Severity::Error,
            // PARSER WARNINGS
            Kind::DuplicatedRequirementDeclaration { .. } => Severity::Warning,
            Kind::DuplicatedTypeDeclaration { .. } => Severity::Warning,

            // NORMALIZER WARNINGS
            Kind::DuplicateRequirementDeclarationWarning { .. } => Severity::Warning,

            // ANALYSER ERRORS
            Kind::UnDefinedFunction { .. } => Severity::Error,
            Kind::UnDefinedPredicate { .. } => Severity::Error,
            Kind::UnDefinedCompoundTask { .. } => Severity::Error,
            Kind::UnDefinedPrimitiveTask { .. } => Severity::Error,
            Kind::TypeMismatchInExpression { .. } => Severity::Error,
            Kind::InvalidTypesInNumericExpression { .. } => Severity::Error,
            Kind::DuplicatedSymbolDeclarationInScopeError { .. } => Severity::Error,
            Kind::CyclicTaskOrderingError => Severity::Error,
            Kind::UndeclaredSymbolError { .. } => Severity::Error,
            Kind::SymbolDeclaredAsKeywordError { .. } => Severity::Error,
            Kind::CyclicTypeDeclarationError { .. } => Severity::Error,
            // ANALYSER WARNINGS
            Kind::SymbolDeclaredAmbiguouslyAsKeywordWarning { .. } => Severity::Warning,
            Kind::UnusedSymbolWarning { .. } => Severity::Warning,
            Kind::RequirementViolation { .. } => Severity::Warning,
            Kind::WarningAmbiguousTypePredicateSymbol { .. } => Severity::Warning,
            Kind::WarningTaskArgumentIsSupertypeOfDeclaration { .. } => Severity::Warning,
            Kind::DuplicateTypesInSymbolDeclarationWarning { .. } => Severity::Warning,
            Kind::ImplicitEitherTypeDeclarationWarning { .. } => Severity::Warning,

            // LINKER WARNINGS
            Kind::DomainProblemNameMismatch { .. } => Severity::Warning,
            // LINKER ERROR
            Kind::CrossConflictSymbolDeclarationError {..} => Severity::Error,


        }
    }
    pub fn suggestion(&self) -> Option<String> {
        match self {
            Kind::UnexpectedToken { expected, .. }
            | Kind::UnexpectedEof { expected } => {
                Self::format_expected_message(expected)
            }
            Kind::ExtraToken { .. } => {
                Some("Extra token detected. Check for unnecessary symbols or misplaced characters.".to_string())
            }
            Kind::InvalidToken => {
                Some("Make sure there are no typos or invalid characters.".to_string())
            }
            Kind::DuplicatedRequirementDeclaration { requirement} => {
                Some(format!("Requirement '{}' is already declared. You can safely remove the duplicate.", requirement))
            }
            Kind::DuplicatedTypeDeclaration { .. } => {
                Some("This type is already declared. Consider removing the duplicate.".to_string())
            }
            Kind::CustomError(_) => None,
            Kind::UnDefinedFunction { symbol } => {
                Some(format!("Make sure that a function with call '{}' is defined in the block ':functions'", symbol))
            }
            Kind::UnDefinedPredicate { symbol } => {
                Some(format!("Make sure that a predicate named '{}' is defined in the block ':predicates'", symbol))
            }
            Kind::UnDefinedCompoundTask { symbol } => {
                Some(format!("Make sure that a compound task '{}' is defined in the block ':tasks'", symbol))
            }
            Kind::UnDefinedPrimitiveTask { symbol } => {
                Some(format!("Make sure that an action '{}' is defined", symbol))
            }
            Kind::TypeMismatchInExpression { ty1, ty2 } => {
                Some(format!(
                    "Incompatible types: {:?} is not related to {:?} by the type hierarchy.",
                    Self::format_types(ty1),
                    Self::format_types(ty2)
                ))
            }
            Kind::InvalidTypesInNumericExpression { ty1, ty2 } => {
                Some(format!(
                    "Numeric expressions require operands of type 'number', but found types {:?} and {:?}. Ensure both operands are numbers.",
                    Self::format_types(ty1), Self::format_types(ty2)
                ))
            }
            Kind::RequirementViolation { node_kind, required } => {
                Some(format!(
                    "The use of '{}' requires one of the following requirements: {}.",
                    node_kind,  // ou juste format!("{:?}", node_kind) si pas encore défini
                    Self::format_requirements_list(&required)
                ))
            }
            Kind::DuplicatedSymbolDeclarationInScopeError { symbol, declaration1, declaration2, .. } => {
                Some(format!(
                    "The symbol '{}' is declared once as a '{}' and again as a '{}'. \
                        Consider renaming one of the declarations or ensuring consistent usage.",
                    symbol,
                    declaration1.kind(),
                    declaration2.kind()
                ))
            }
            Kind::CyclicTaskOrderingError => Some("Check for loops in your task dependencies or ordering constraints.".to_string()),
            Kind::UndeclaredSymbolError { usage} => {
                match usage.kind() {
                    SymbolKind::Function => Some(format!(
                        "Function '{}' is not declared. Please declare it before use in the ':functions' block.",
                        usage.symbol()
                    )),
                    SymbolKind::Predicate => Some(format!(
                        "Predicate '{}' is not declared. Please declare it before use in ':predicates' block.",
                        usage.symbol()
                    )),
                    SymbolKind::Action => Some(format!(
                        "Action '{}' is not declared. Please define it using the ':action' keyword.",
                        usage.symbol()
                    )),
                    SymbolKind::DASymbol => Some(format!(
                        "Durative action '{}' is not declared. Please define it using the ':durative-action' keyword.",
                        usage.symbol()
                    )),
                    SymbolKind::Method => Some(format!(
                        "Method '{}' is not declared. Please define it in the ':methods' keyword.",
                        usage.symbol()
                    )),
                    SymbolKind::Task => Some(format!(
                        "Task '{}' is not declared. Please ensure it's defined using the ':task' keyword.",
                        usage.symbol()
                    )),
                    SymbolKind::TaskID => Some(format!(
                        "Task identifier '{}' is not declared. Verify it's correctly assigned in your task network.",
                        usage.symbol()
                    )),
                    SymbolKind::Constant => Some(format!(
                        "Constant '{}' is not declared. Declare it in the ':constants' section in domain files of in ':object' in problem files.",
                        usage.symbol()
                    )),
                    SymbolKind::DomainName => Some(format!(
                        "Domain '{}' is not recognized. Make sure the domain name is correctly defined.",
                        usage.symbol()
                    )),
                    SymbolKind::PrimitiveType => Some(format!(
                        "Type '{}' is not declared. Ensure it's defined in the ':types' section.",
                        usage.symbol()
                    )),
                    SymbolKind::ProblemName => Some(format!(
                        "Problem '{}' is not declared. Verify the problem file or declaration.",
                        usage.symbol()
                    )),
                    SymbolKind::Requirement => Some(format!(
                        "Requirement '{}' is not recognized. Check for typos or unsupported features.",
                        usage.symbol()
                    )),
                    SymbolKind::Variable => Some(format!(
                        "Variable '{}' is not declared. Declare it using the correct syntax (e.g., '?x - type').",
                        usage.symbol()
                    )),
                }
            }
            Kind::SymbolDeclaredAsKeywordError { declaration, expected_kind, requirements } => {
                Some(format!(
                    "Symbol '{}' is reserved as '{}' in the language with requirements: {}. '{}' expected. Consider renaming it or using a different symbol.",
                    declaration.symbol(),
                    declaration.kind(),
                    Self::format_requirements_list(requirements),
                    expected_kind,
                ))
            }

            Kind::SymbolDeclaredAmbiguouslyAsKeywordWarning { declaration, requirements } => {
                Some(format!(
                    "{} symbol '{}' is ambiguous as it is used as a keyword in the language with requirements: {}. Consider renaming it or using a different symbol.",
                    declaration.kind(),
                    declaration.symbol(),
                    Self::format_requirements_list(requirements),
                ))
            }
            Kind::UnusedSymbolWarning {declaration} => {
                Some(format!(
                    "{} symbol '{}' is declared but not used. Consider removing it to clean up your code.",
                    declaration.kind(),
                    declaration.kind()
                ))
            }
            Kind::DomainProblemNameMismatch { domain_name, .. } => {
                Some(format!(
                    "Ensure that the domain name in the problem file matches the domain definition: expected '{}'.",
                    domain_name
                ))
            }
            Kind::WarningAmbiguousTypePredicateSymbol { symbol} => {
                Some(format!(
                    "The symbol '{}' is declared both as a type and a predicate in the same scope. This can lead to confusion. Consider renaming one of them.",
                    symbol
                ))
            }
            Kind::WarningTaskArgumentIsSupertypeOfDeclaration {argument, type_declared, type_used} => {
                Some(format!(
                    "The argument '{}' uses type(s) '{}', which are supertypes of the declared type(s) '{}'. \
                        Consider using the exact or a more specific type.",
                    argument,
                    Self::format_types(type_declared),
                    Self::format_types(type_used)
                ))
            }
            Kind::DuplicateTypesInSymbolDeclarationWarning { symbol, duplicate_types } => {
                let listed_types = if duplicate_types.len() == 1 {
                    format!("type '{}'", duplicate_types[0])
                } else {
                    format!("types '{}'", duplicate_types.join("', '"))
                };
                Some(format!(
                    "Duplicate {} found in the type declarations of symbol '{}'; these duplicates have been removed.",
                    listed_types, symbol
                ))
            }
            Kind::CyclicTypeDeclarationError { cycle } => {
                let cycle_symbols: Vec<&str> = cycle.iter().map(|decl| decl.symbol().as_str()).collect();
                Some(format!(
                    "Cycle detected in type hierarchy: {}. Remove cyclic inheritance to fix.",
                    cycle_symbols.join(" -> ")
                ))
            }
            Kind::CrossConflictSymbolDeclarationError { symbol, problem_kind, domain_kinds } => {
                Some(format!(
                    "Symbol `{}` declared as `{}` in the problem, but in the domain it is declared as: {}. Ensure the symbol’s kind matches in both.",
                    symbol,
                    problem_kind,
                    Self::format_symbol_kinds(&domain_kinds),
                ))
            }
            Kind::ImplicitEitherTypeDeclarationWarning { ty, types } => {
                Some(format!(
                    "Multiple type declarations for '{}': interpreted as {}. \
                    To make this explicit and avoid ambiguity, declare the type as '{} - {}'.",
                    ty,
                    Self::format_types(types),
                    ty,
                    Self::format_types(types),
                ))
            }
            Kind::DuplicateRequirementDeclarationWarning { requirement } => {
                Some(format!("The requirement `{}` was duplicated and has been ignored.", requirement))
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

    // Ajoutez cette fonction pour formater les kinds en une chaîne séparée par des virgules.
    fn format_symbol_kinds(kinds: &[SymbolKind]) -> String {
        kinds
            .iter()
            .map(|k| format!("{:?}", k))
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

impl fmt::Display for Kind {
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
