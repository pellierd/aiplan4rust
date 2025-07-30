use std::collections::HashMap;
use std::fmt;
use crate::aiplan4rust::diagnostic::Severity;
use crate::aiplan4rust::lang::{Ident, Type};
use crate::aiplan4rust::lang::Requirement;
use crate::aiplan4rust::semantic::symbol::{Declaration, SymbolKind, Usage};

use crate::aiplan4rust::syntax::ast::AstNode;
use crate::aiplan4rust::syntax::ast::AstKind;
use crate::aiplan4rust::interner::StringInterner;
use crate::aiplan4rust::semantic::symbol::symbol::Symbol;
use crate::aiplan4rust::syntax::SyntaxDisplay;

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
    UnDefinedSymbol {
        symbol: Symbol,
    },
    TypeMismatchInExpression {
        ty1: Type,
        ty2: Type,
    },

    // TO CHECK
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
        scope: AstNode,
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
    DuplicateEitherTypeWarning {
        duplicate_types: Vec<String>,
    },
    ImplicitEitherTypeDeclarationWarning {
        ty: String,
    },
    CyclicTypeDeclarationError {
       cycle: Vec<Declaration>
    },
    CrossConflictSymbolDeclarationError {
        symbol: String,
        problem_kind: SymbolKind,
        domain_kinds: Vec<SymbolKind>,
    },
    DuplicateRequirementWarning {
        duplicate_requirements: Vec<Requirement>,
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
            // WARNING NORMALIZER
            Kind::DuplicateRequirementWarning { ..  } => "W2001".to_string(),
            // ERROR ANALYSER
            Kind::UnDefinedSymbol { .. } => "E1001".to_string(),
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
            Kind::DuplicateEitherTypeWarning { .. } => "W1015".to_string(),
            Kind::ImplicitEitherTypeDeclarationWarning { .. } => "W1016".to_string(),

            // WARNINGS LINKER
            Kind::DomainProblemNameMismatch { .. } => "W2000".to_string(),
            Kind::CrossConflictSymbolDeclarationError { .. } => "E2001".to_string(),


            Kind::CustomError(_) => "E000X".to_string(),
        }
    }

    // Centraliser le message d'erreur directement dans l'enum
    pub fn message(&self, interner: Option<&StringInterner>) -> String {
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
            Kind::UnDefinedSymbol { symbol } => {
                let name = symbol_to_string(symbol, interner);
                match symbol.kind() {
                    SymbolKind::Function => format!("Function '{}' is undefined", name),
                    SymbolKind::Predicate => format!("Predicate '{}' is undefined", name),
                    SymbolKind::Task => format!("Compound task '{}' is undefined", name),
                    SymbolKind::Action => format!("Primitive task '{}' is undefined", name),
                    _ => format!("Symbol '{}' of kind {:?} is undefined", name, symbol.kind()),
                }
            }
            Kind::TypeMismatchInExpression { ty1, ty2 } => {
                let ty1_str = type_to_string(ty1, interner);
                let ty2_str = type_to_string(ty2, interner);
                format!(
                    "Type mismatch between '{}' and '{}'.",
                    ty1_str, ty2_str
                )
            }

            // TO CHECK

            Kind::InvalidTypesInNumericExpression { .. } => {
                "Invalid operand types in numeric expr.".to_string()
            }
            Kind::RequirementViolation { node_kind, .. } => {
                format!("'{}' expr is not allowed in the current context.", node_kind)
            }
            Kind::DuplicatedSymbolDeclarationInScopeError { symbol, .. } => {
                format!("Duplicate declaration of symbol '{}'.", symbol)
            }
            Kind::CyclicTaskOrderingError => {
                "Cyclic task-ordering constraint detected.".to_string()
            }
            Kind::UndeclaredSymbolError { usage} => {
                format!("{} symbol '{}' undeclared.", usage.symbol_ident(), usage.symbol_kind())
            }
            Kind::SymbolDeclaredAsKeywordError {declaration, ..} => {
                format!("Symbol '{}' used as a language keyword", declaration.symbol_ident())
            }
            Kind::SymbolDeclaredAmbiguouslyAsKeywordWarning {declaration, ..} => {
                format!("Symbol '{}' is ambiguous as a language keyword", declaration.symbol_ident())
            }
            Kind::UnusedSymbolWarning { declaration } => {
                format!("{} Symbol '{}' is unused", declaration.symbol_kind(), declaration.symbol_ident())
            }
            Kind::DomainProblemNameMismatch { domain_name, problem_name } => {
                format!("Domain name '{}' does not match problem name '{}'.", domain_name, problem_name)
            }
            Kind::WarningAmbiguousTypePredicateSymbol { symbol, .. } => {
                format!("Ambiguous symbol '{}': declared both as a type_checker and a predicate in the same scope.", symbol)
            }
            Kind::WarningTaskArgumentIsSupertypeOfDeclaration { argument, .. } => {
                format!("Upcasting detected: argument '{}' has broader type_checker(s) than declared.",
                argument)
            }
            Kind::DuplicateEitherTypeWarning { .. } => {
                "Duplicate primitive types found in an 'either' type_checker declaration.".to_string()
            }
            Kind::ImplicitEitherTypeDeclarationWarning { ty, ..} => {
                format!(
                    "Implicit 'either' type_checker declaration for {}.",
                    ty,
                )
            }
            Kind::CyclicTypeDeclarationError { ..} => {
                "Cycle detected in type_checker declarations, causing an invalid hierarchy.".to_string()
            }
            Kind::CrossConflictSymbolDeclarationError { .. } => {
                "Symbol declaration in problem conflicts with domain declaration.".to_string()
            }
            Kind::DuplicateRequirementWarning { .. } => {
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

            // NORMALIZER WARNINGS
            Kind::DuplicateRequirementWarning { .. } => Severity::Warning,

            // ANALYSER ERRORS
            Kind::UnDefinedSymbol { .. } => Severity::Error,
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
            Kind::DuplicateEitherTypeWarning { .. } => Severity::Warning,
            Kind::ImplicitEitherTypeDeclarationWarning { .. } => Severity::Warning,

            // LINKER WARNINGS
            Kind::DomainProblemNameMismatch { .. } => Severity::Warning,
            // LINKER ERROR
            Kind::CrossConflictSymbolDeclarationError {..} => Severity::Error,


        }
    }
    pub fn suggestion(&self, interner: Option<&StringInterner>) -> Option<String> {
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
            Kind::CustomError(_) => None,
            Kind::UnDefinedSymbol { symbol } => {
                let name = symbol_to_string(symbol, interner);

                let msg = match symbol.kind() {
                    SymbolKind::Function => {
                        format!("Make sure that a function with call '{}' is defined in the block ':functions'", name)
                    }
                    SymbolKind::Predicate => {
                        format!("Make sure that a predicate named '{}' is defined in the block ':predicates'", name)
                    }
                    SymbolKind::Task => {
                        format!("Make sure that a compound task '{}' is defined in the block ':tasks'", name)
                    }
                    SymbolKind::Action => {
                        format!("Make sure that an action '{}' is defined un a block ':action'", name)
                    }
                    _ => {
                        format!("Make sure that '{}' is defined", name)
                    }
                };
                Some(msg)
            }
            Kind::TypeMismatchInExpression { ty1, ty2 } => {
                let ty1_str = type_to_string(ty1, interner);
                let ty2_str = type_to_string(ty2, interner);

                Some(format!(
                    "The type '{}' cannot be used with '{}' — make sure the types are compatible according to the type hierarchy.",
                    ty1_str,
                    ty2_str
                ))
            }
            // TO CHECK
            Kind::InvalidTypesInNumericExpression { ty1, ty2 } => {
                Some(format!(
                    "Numeric expr require operands of type_checker 'number', but found types {:?} and {:?}. Ensure both operands are numbers.",
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
                    declaration1.symbol_kind(),
                    declaration2.symbol_kind()
                ))
            }
            Kind::CyclicTaskOrderingError => Some("Check for loops in your task dependencies or ordering constraints.".to_string()),
            Kind::UndeclaredSymbolError { usage} => {
                match usage.symbol_kind() {
                    SymbolKind::Function => Some(format!(
                        "Function '{}' is not declared. Please declare it before use in the ':functions' block.",
                        usage.symbol_ident()
                    )),
                    SymbolKind::Predicate => Some(format!(
                        "Predicate '{}' is not declared. Please declare it before use in ':predicates' block.",
                        usage.symbol_ident()
                    )),
                    SymbolKind::Action => Some(format!(
                        "Action '{}' is not declared. Please define it using the ':action' keyword.",
                        usage.symbol_ident()
                    )),
                    SymbolKind::DASymbol => Some(format!(
                        "Durative action '{}' is not declared. Please define it using the ':durative-action' keyword.",
                        usage.symbol_ident()
                    )),
                    SymbolKind::Method => Some(format!(
                        "Method '{}' is not declared. Please define it in the ':methods' keyword.",
                        usage.symbol_ident()
                    )),
                    SymbolKind::Task => Some(format!(
                        "Task '{}' is not declared. Please ensure it's defined using the ':task' keyword.",
                        usage.symbol_ident()
                    )),
                    SymbolKind::TaskID => Some(format!(
                        "Task identifier '{}' is not declared. Verify it's correctly assigned in your task network.",
                        usage.symbol_ident()
                    )),
                    SymbolKind::Constant => Some(format!(
                        "Constant '{}' is not declared. Declare it in the ':constants' section in domain files of in ':object' in problem files.",
                        usage.symbol_ident()
                    )),
                    SymbolKind::DomainName => Some(format!(
                        "Domain '{}' is not recognized. Make sure the domain name is correctly defined.",
                        usage.symbol_ident()
                    )),
                    SymbolKind::PrimitiveType => Some(format!(
                        "Type '{}' is not declared. Ensure it's defined in the ':types' section.",
                        usage.symbol_ident()
                    )),
                    SymbolKind::ProblemName => Some(format!(
                        "Problem '{}' is not declared. Verify the problem file or declaration.",
                        usage.symbol_ident()
                    )),
                    SymbolKind::Requirement => Some(format!(
                        "Requirement '{}' is not recognized. Check for typos or unsupported features.",
                        usage.symbol_ident()
                    )),
                    SymbolKind::Variable => Some(format!(
                        "Variable '{}' is not declared. Declare it using the correct syntax (e.g., '?x - type_checker').",
                        usage.symbol_ident()
                    )),
                }
            }
            Kind::SymbolDeclaredAsKeywordError { declaration, expected_kind, requirements } => {
                Some(format!(
                    "Symbol '{}' is reserved as '{}' in the language with requirements: {}. '{}' expected. Consider renaming it or using a different symbol.",
                    declaration.symbol_ident(),
                    declaration.symbol_kind(),
                    Self::format_requirements_list(requirements),
                    expected_kind,
                ))
            }

            Kind::SymbolDeclaredAmbiguouslyAsKeywordWarning { declaration, requirements } => {
                Some(format!(
                    "{} symbol '{}' is ambiguous as it is used as a keyword in the language with requirements: {}. Consider renaming it or using a different symbol.",
                    declaration.symbol_kind(),
                    declaration.symbol_ident(),
                    Self::format_requirements_list(requirements),
                ))
            }
            Kind::UnusedSymbolWarning {declaration} => {
                Some(format!(
                    "{} symbol '{}' is declared but not used. Consider removing it to clean up your code.",
                    declaration.symbol_kind(),
                    declaration.symbol_kind()
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
                    "The symbol '{}' is declared both as a type_checker and a predicate in the same scope. This can lead to confusion. Consider renaming one of them.",
                    symbol
                ))
            }
            Kind::WarningTaskArgumentIsSupertypeOfDeclaration {argument, type_declared, type_used} => {
                Some(format!(
                    "The argument '{}' uses type_checker(s) '{}', which are supertypes of the declared type_checker(s) '{}'. \
                        Consider using the exact or a more specific type_checker.",
                    argument,
                    Self::format_types(type_declared),
                    Self::format_types(type_used)
                ))
            }
            Kind::DuplicateEitherTypeWarning { duplicate_types } => {
                let listed_types = if duplicate_types.len() == 1 {
                    format!("type_checker '{}'", duplicate_types[0])
                } else {
                    format!("types '{}'", duplicate_types.join("', '"))
                };
                Some(format!(
                    "Duplicate {} found in an 'either' type_checker declaration; these duplicates have been removed.",
                    listed_types,
                ))
            }
            Kind::CyclicTypeDeclarationError { cycle } => {
                let cycle_symbols: Vec<Ident> = cycle.iter().map(|decl| decl.symbol_ident()).collect();
                Some(format!(
                    "Cycle detected in type_checker hierarchy: {:?}. Remove cyclic inheritance to fix.",
                    cycle_symbols
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
            Kind::ImplicitEitherTypeDeclarationWarning { ty, .. } => {
                Some(format!(
                    "The type_checker `{}` was declared more than once with different parent types. These conflicting declarations were automatically merged into an implicit `(either ...)` type_checker declaration.",
                    ty,
                ))
            }
            Kind::DuplicateRequirementWarning { duplicate_requirements} => {
                Some(format!(
                    "The following requirement(s) are declared multiple times in the domain and have been ignored: {}.",
                    duplicate_requirements.iter().map(|r| r.to_string()).collect::<Vec<_>>().join(", ")
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

    // Ajoutez cette fonction pour formater les kinds en une chaîne séparée par des virgules.
    fn format_symbol_kinds(kinds: &[SymbolKind]) -> String {
        kinds
            .iter()
            .map(|k| format!("{:?}", k))
            .collect::<Vec<_>>()
            .join(", ")
    }


    /// Format a vector of types for display.
    /// - If there is only one type_checker, it returns the type_checker as-is.
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

    /// Remap all `Ident`s in this diagnostic using the provided `map`.
    pub fn remap_idents(&mut self, map: &HashMap<Ident, Ident>) {
        match self {
            Kind::UnDefinedSymbol { symbol } => {
                symbol.remap_idents(map);
            }


            // TO CHECK

            Kind::DuplicatedSymbolDeclarationInScopeError {
                declaration1,
                declaration2,
                ..
            } => {
                declaration1.remap_idents(map);
                declaration2.remap_idents(map);
            }

            Kind::UndeclaredSymbolError { usage } => {
                usage.remap_idents(map);
            }

            Kind::SymbolDeclaredAsKeywordError { declaration, .. }
            | Kind::SymbolDeclaredAmbiguouslyAsKeywordWarning { declaration, .. }
            | Kind::UnusedSymbolWarning { declaration } => {
                declaration.remap_idents(map);
            }

            Kind::CyclicTypeDeclarationError { cycle } => {
                for decl in cycle {
                    decl.remap_idents(map);
                }
            }

            // Variantes qui n'ont pas de `Ident` ou ne nécessitent pas de remap :
            Kind::UnexpectedToken { .. }
            | Kind::UnexpectedEof { .. }
            | Kind::InvalidToken
            | Kind::ExtraToken { .. }
            | Kind::TypeMismatchInExpression { .. }
            | Kind::InvalidTypesInNumericExpression { .. }
            | Kind::RequirementViolation { .. }
            | Kind::CyclicTaskOrderingError
            | Kind::DomainProblemNameMismatch { .. }
            | Kind::WarningAmbiguousTypePredicateSymbol { .. }
            | Kind::WarningTaskArgumentIsSupertypeOfDeclaration { .. }
            | Kind::DuplicateEitherTypeWarning { .. }
            | Kind::ImplicitEitherTypeDeclarationWarning { .. }
            | Kind::CrossConflictSymbolDeclarationError { .. }
            | Kind::DuplicateRequirementWarning { .. }
            | Kind::CustomError(_) => {
                // Pas de remap nécessaire ici
            }
        }
    }
}



impl fmt::Display for Kind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let code = self.code();
        let message = self.message(None);
        let severity = self.severity();
        let suggestion = self.suggestion(None);

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

/// Returns the name of the given symbol as a `String`, optionally resolving it
/// through a string interner by using the `InternerDisplay` trait implementation.
///
/// # Parameters
/// - `symbol`: Reference to the `Symbol` whose name is to be retrieved.
/// - `interner`: Optional reference to a `StringInterner` used to resolve the symbol's identifier.
///
/// # Returns
/// A `String` representing the resolved name of the symbol.
/// - If the `interner` is provided and the identifier is found, returns the resolved string.
/// - If the `interner` is provided but the identifier is not found, returns `"unknown(<ident>)"`.
/// - If the `interner` is not provided, returns the raw identifier as a string.
fn symbol_to_string(symbol: &Symbol, interner: Option<&StringInterner>) -> String {
    if let Some(interner) = interner {
        symbol.to_syntax_string(interner)
    } else {
        symbol.ident().to_string()
    }
}

/// Converts a `Type` to a `String`, optionally resolving identifiers via a `StringInterner`.
///
/// # Parameters
/// - `ty`: The `Type` to convert to a string.
/// - `interner`: An optional reference to a `StringInterner` used to resolve identifiers.
///
/// # Returns
/// A `String` representation of the `Type`. If `interner` is provided, the identifiers
/// inside the `Type` are resolved using it; otherwise, the default string representation is used.
fn type_to_string(ty: &Type, interner: Option<&StringInterner>) -> String {
    if let Some(interner) = interner {
        ty.to_syntax_string(interner)
    } else {
        ty.to_string()
    }
}
