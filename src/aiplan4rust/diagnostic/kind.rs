use std::collections::HashMap;
use std::fmt;
use crate::aiplan4rust::diagnostic::Severity;
use crate::aiplan4rust::lang::{Ident, Type};
use crate::aiplan4rust::lang::Requirement;
use crate::aiplan4rust::semantic::symbol::{Declaration, SymbolKind, Usage};

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
    InvalidSymbolSignature {
        declaration: Declaration,
        usage: Usage,
    },
    TypeMismatchInExpression {
        ty1: Type,
        ty2: Type,
    },
    InvalidTypesInNumericExpression {
        ty1: Type,
        ty2: Type,
    },
    RequirementViolation {
        node_kind: AstKind,
        required: Vec<Requirement>,
    },
    DuplicatedSymbolDeclarationInScope {
        symbol: Symbol,
        declaration1: Declaration,
        declaration2: Declaration,
        scope: AstKind,
    },
    CyclicTaskOrdering, // can be more explicit

    /// Error indicating the use of a symbol that has not been declared in the current scope.
    ///
    /// This error occurs when a symbol (such as a function, predicate, variable, or action)
    /// is referenced before it has been introduced or declared. It helps catch typos,
    /// missing declarations, or scoping issues.
    ///
    /// ### Fields:
    /// - `usage`: Contains detailed information about the usage of the undeclared symbol,
    ///    including its identifier, kind (e.g., function, predicate), and the context where it is used.
    ///
    /// ### Example:
    /// ```pddl
    /// (not (unknown_predicate)) ;; Error: 'unknown_predicate' is undeclared
    /// ```
    UndeclaredSymbol {
        usage: Usage,
    },

    /// Error raised when a user-defined symbol conflicts with a reserved PDDL keyword,
    /// depending on the active `:requirements`.
    ///
    /// Some identifiers in PDDL (like `object`, `number`, `?duration`, etc.) have a
    /// special meaning when certain features are enabled. Declaring a symbol with such
    /// an identifier leads to this error if the declaration does not match the expected usage.
    ///
    /// For example, declaring a new type named `object` is invalid when `:typing` is enabled,
    /// since `object` is a built-in primitive type in that context.
    ///
    /// ### Fields:
    /// - `declaration`: The user’s declaration that introduces the conflicting symbol.
    /// - `expected_kind`: The `SymbolKind` that this identifier represents in PDDL when
    ///   used as a keyword (e.g., `PrimitiveType`, `Function`, `Variable`, etc.).
    /// - `requirements`: The list of `Requirement`s (PDDL features) that cause this identifier
    ///   to be reserved. This helps the user understand under which conditions the conflict arises.
    ///
    /// ### Example:
    /// ```pddl
    /// (:types object) ;; Error: 'object' is reserved when :typing or :adl is required
    /// ```
    SymbolConflictsWithKeyword {
        declaration: Declaration,
        expected_kind: SymbolKind,
        requirements: Vec<Requirement>,
    },

    /// Represents a warning when a symbol is declared in a way that ambiguously overlaps
    /// with a reserved PDDL language keyword, but its kind matches the expected type.
    ///
    /// This situation typically occurs when a symbol uses a name reserved by the language,
    /// but the symbol's kind aligns with what is expected for that name, given the current
    /// domain requirements. While not a strict error, this usage may lead to confusion or
    /// unintended behavior, especially if certain domain requirements are not enabled.
    ///
    /// # Fields
    ///
    /// - `declaration`: The `Declaration` of the symbol in question.
    /// - `expected_kind`: The `SymbolKind` that the symbol is expected to have based on
    ///   the reserved keyword semantics and domain requirements.
    /// - `requirements`: A list of `Requirement`s indicating which PDDL features or
    ///   domain requirements must be enabled for this usage to be considered valid.
    ///
    /// # Example
    ///
    /// If the symbol `?duration` is declared as a variable but the `DurativeActions`
    /// requirement is not enabled, this warning might be triggered to indicate
    /// potential ambiguity with the built-in keyword `duration`.
    SymbolDeclaredAmbiguouslyAsKeyword {
        declaration: Declaration,
        expected_kind: SymbolKind,
        requirements: Vec<Requirement>,
    },

    /// Represents a warning or error indicating that a symbol
    /// declared in the code is never used.
    ///
    /// # Fields
    ///
    /// - `declaration`: The declaration of the symbol that has been
    ///   detected as unused. This includes information such as the symbol's
    ///   name, kind (e.g., variable, function), and its location.
    ///
    /// # Purpose
    ///
    /// This is useful for identifying dead code or declarations that
    /// can be removed or need to be reviewed for correctness.
    ///
    /// # Example
    ///
    /// ```rust
    /// // Suppose `declaration` is a symbol declared but never used:
    /// let unused = Kind::UnusedSymbol { declaration };
    /// ```
    UnusedSymbol {
        declaration: Declaration,
    },
    DomainProblemNameMismatch {
        domain_name: String,
        problem_name: String,
    },
    // TO CHECK





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
            Kind::InvalidSymbolSignature { .. } => "E1001".to_string(),
            Kind::TypeMismatchInExpression { .. } => "E1005".to_string(),
            Kind::InvalidTypesInNumericExpression { .. } => "E1006".to_string(),
            Kind::DuplicatedSymbolDeclarationInScope { .. } => "E1007".to_string(),
            Kind::CyclicTaskOrdering { .. } => "E1008".to_string(),
            Kind::UndeclaredSymbol { .. } => "E1009".to_string(),
            Kind::SymbolConflictsWithKeyword { .. } => "E1010".to_string(),
            Kind::CyclicTypeDeclarationError { .. } => "E1011".to_string(),
            // WARNINGS ANALYSER
            Kind::SymbolDeclaredAmbiguouslyAsKeyword { .. } => "W1010".to_string(),
            Kind::UnusedSymbol { .. } => "W1011".to_string(),
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
            Kind::InvalidSymbolSignature { declaration, usage } => {
                let symbol = declaration.symbol();
                let name = symbol_to_string(symbol, interner);
                match symbol.kind() {
                    SymbolKind::Function => format!("Function '{}' does not match any declared signature.", name),
                    SymbolKind::Predicate => format!("Predicate '{}' does not match any declared signature.", name),
                    SymbolKind::Task => format!("Compound task '{}' does not match any declared signature.", name),
                    SymbolKind::Action => format!("Primitive task '{}' does not match any declared signature.", name),
                    _ => format!("Symbol '{}' of kind {:?} does not match any declared signature.", name, symbol.kind()),
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
            Kind::InvalidTypesInNumericExpression { ty1, ty2 } => {
                let ty1_str = type_to_string(ty1, interner);
                let ty2_str = type_to_string(ty2, interner);
                format!(
                    "Invalid operand types for numeric expression: '{}' and '{}'. Operands must be numeric types.",
                    ty1_str, ty2_str
                )
            }
            Kind::RequirementViolation { node_kind, .. } => {
                format!("Expression type '{}' disallowed by current requirements.", node_kind)
            }
            Kind::DuplicatedSymbolDeclarationInScope { symbol, .. } => {
                let name = symbol_to_string(symbol, interner);
                format!("Symbol '{}' is declared multiple times in the same scope.", name)
            }
            Kind::CyclicTaskOrdering => {
                "Cyclic task-ordering constraint detected.".to_string()
            }
            Kind::UndeclaredSymbol { usage } => {
                let name = symbol_to_string(&usage.symbol(), interner);
                format!("{} symbol '{}' is undeclared.", usage.symbol_kind(), name)
            }
            Kind::SymbolConflictsWithKeyword { declaration, .. } => {
                let name = symbol_to_string(&declaration.symbol(), interner);
                format!("Symbol '{}' is used as a language keyword", name)
            }

            Kind::SymbolDeclaredAmbiguouslyAsKeyword { declaration, .. } => {
                let name = symbol_to_string(&declaration.symbol(), interner);
                format!("Symbol '{}' is ambiguous as a language keyword", name)
            }
            Kind::UnusedSymbol { declaration } => {
                let name = symbol_to_string(&declaration.symbol(), interner);
                format!("{} symbol '{}' is unused", declaration.symbol_kind(), name)
            }

            // TO CHECK

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
            Kind::InvalidSymbolSignature { .. } => Severity::Error,
            Kind::TypeMismatchInExpression { .. } => Severity::Error,
            Kind::InvalidTypesInNumericExpression { .. } => Severity::Error,
            Kind::DuplicatedSymbolDeclarationInScope { .. } => Severity::Error,
            Kind::CyclicTaskOrdering => Severity::Error,
            Kind::UndeclaredSymbol { .. } => Severity::Error,
            Kind::SymbolConflictsWithKeyword { .. } => Severity::Error,
            Kind::CyclicTypeDeclarationError { .. } => Severity::Error,
            // ANALYSER WARNINGS
            Kind::SymbolDeclaredAmbiguouslyAsKeyword { .. } => Severity::Warning,
            Kind::UnusedSymbol { .. } => Severity::Warning,
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
            Kind::InvalidSymbolSignature { declaration  , usage } => {
                let symbol = declaration.symbol();
                let name = symbol_to_string(symbol, interner);
                let msg = match symbol.kind() {
                    SymbolKind::Function => {
                        format!("Ensure a function named '{}' with the correct signature is declared in the ':functions' block.", name)
                    }
                    SymbolKind::Predicate => {
                        format!("Ensure a predicate named '{}' with the correct signature is declared in the ':predicates' block.", name)
                    }
                    SymbolKind::Task => {
                        format!("Ensure a compound task '{}' with the correct signature is declared in the ':tasks' block.", name)
                    }
                    SymbolKind::Action => {
                        format!("Ensure an action '{}' with the correct signature is declared in the ':action' block.", name)
                    }
                    _ => {
                        format!("Ensure '{}' with the correct signature is declared properly.", name)
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

            Kind::InvalidTypesInNumericExpression { ty1, ty2 } => {
                let ty1_str = type_to_string(ty1, interner);
                let ty2_str = type_to_string(ty2, interner);

                Some(format!(
                    "Numeric expressions require operands of type 'number', but found '{}' and '{}'. Ensure both operands are numeric types.",
                    ty1_str,
                    ty2_str
                ))
            }
            Kind::RequirementViolation { node_kind, required } => {
                Some(format!(
                    "Expression type '{}' requires one of these requirements: {}.",
                    node_kind, // ou format!("{:?}", node_kind) si besoin
                    format_requirements_list(&required)
                ))
            }
            Kind::DuplicatedSymbolDeclarationInScope { symbol, declaration1, declaration2, scope } => {
                let symbol_name = symbol_to_string(symbol, interner);
                Some(format!(
                    "The symbol '{}' is declared twice in the '{}' scope: once as a '{}' and again as a '{}'. \
                    Consider renaming one of the declarations or ensuring consistent usage.",
                    symbol_name,
                    scope,  // affiche le type de noeud qui définit le scope
                    declaration1.symbol_kind(),
                    declaration2.symbol_kind()
                ))
            }
            Kind::CyclicTaskOrdering => Some("Check for loops in your task dependencies or ordering constraints.".to_string()),
            Kind::UndeclaredSymbol { usage } => {
                let name = symbol_to_string(&usage.symbol(), interner);
                match usage.symbol_kind() {
                    SymbolKind::Function => Some(format!(
                        "Function '{}' is not declared. Declare it in the ':functions' section.",
                        name
                    )),
                    SymbolKind::Predicate => Some(format!(
                        "Predicate '{}' is not declared. Declare it in the ':predicates' section.",
                        name
                    )),
                    SymbolKind::Action => Some(format!(
                        "Action '{}' is not declared. Define it using the ':action' keyword.",
                        name
                    )),
                    SymbolKind::DASymbol => Some(format!(
                        "Durative action '{}' is not declared. Define it using the ':durative-action' keyword.",
                        name
                    )),
                    SymbolKind::Method => Some(format!(
                        "Method '{}' is not declared. Define it in the ':methods' section.",
                        name
                    )),
                    SymbolKind::Task => Some(format!(
                        "Task '{}' is not declared. Define it using the ':task' keyword.",
                        name
                    )),
                    SymbolKind::TaskID => Some(format!(
                        "Task identifier '{}' is not declared. Check the task network for missing definitions.",
                        name
                    )),
                    SymbolKind::Constant => Some(format!(
                        "Constant '{}' is not declared. Declare it in the ':constants' section (domain) or ':objects' section (problem).",
                        name
                    )),
                    SymbolKind::DomainName => Some(format!(
                        "Domain '{}' is not recognized. Make sure it matches the ':domain' declaration.",
                        name
                    )),
                    SymbolKind::PrimitiveType => Some(format!(
                        "Type '{}' is not declared. Declare it in the ':types' section.",
                        name
                    )),
                    SymbolKind::ProblemName => Some(format!(
                        "Problem '{}' is not recognized. Ensure the problem name is correctly defined.",
                        name
                    )),
                    SymbolKind::Requirement => Some(format!(
                        "Requirement '{}' is not recognized. Check for typos or unsupported features.",
                        name
                    )),
                    SymbolKind::Variable => Some(format!(
                        "Variable '{}' is not declared. You likely need to add it to the ':parameters' list of the enclosing definition (e.g., '?x - type').",
                        name
                    )),
                }
            }
            Kind::SymbolConflictsWithKeyword { declaration, expected_kind, requirements } => {
                let name = symbol_to_string(declaration.symbol(), interner);
                let reqs = format_requirements_list(requirements);
                Some(format!(
                    "Symbol '{}' conflicts with a reserved keyword under requirements: {}. It must be declared as a {:?} (e.g., type, function, variable).",
                    name,
                    reqs,
                    expected_kind
                ))
            }

            Kind::SymbolDeclaredAmbiguouslyAsKeyword { declaration, expected_kind, requirements } => {
                let name = symbol_to_string(declaration.symbol(), interner);
                let reqs = format_requirements_list(requirements);
                Some(format!(
                    "Symbol '{}' is ambiguous because it is used as a language keyword with requirements: {}. Consider renaming or using a different symbol.",
                    name,
                    reqs
                ))
            }
            Kind::UnusedSymbol { declaration } => {
                let name = symbol_to_string(declaration.symbol(), interner);
                Some(format!(
                    "{} symbol '{}' is declared but not used. Consider removing it to clean up your code.",
                    declaration.symbol_kind(),
                    name
                ))
            }
            // TO CHECK
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

            Kind::CustomError(_) => None,
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

    /// Remap all `Ident`s in this diagnostic using the provided `map`.
    pub fn remap_idents(&mut self, map: &HashMap<Ident, Ident>) {
        match self {
            Kind::InvalidSymbolSignature { declaration, usage } => {
                declaration.remap_idents(map);
                usage.remap_idents(map);
            }
            Kind::TypeMismatchInExpression { ty1, ty2 }
            | Kind::InvalidTypesInNumericExpression { ty1, ty2 } => {
                ty1.remap_idents(map);
                ty2.remap_idents(map);
            }

            Kind::DuplicatedSymbolDeclarationInScope {
                declaration1,
                declaration2,
                ..
            } => {
                declaration1.remap_idents(map);
                declaration2.remap_idents(map);
            }

            Kind::UndeclaredSymbol { usage } => {
                usage.remap_idents(map);
            }

            Kind::SymbolConflictsWithKeyword { declaration, .. }
            | Kind::SymbolDeclaredAmbiguouslyAsKeyword { declaration, .. } => {
                declaration.remap_idents(map);
            }

            Kind::UnexpectedToken { .. }
            | Kind::UnexpectedEof { .. }
            | Kind::InvalidToken
            | Kind::ExtraToken { .. }
            | Kind::RequirementViolation { .. } => {
                // Pas de remap nécessaire ici
            }

            // TO CHECK



            Kind::UnusedSymbol { declaration } => {
                declaration.remap_idents(map);
            }

            Kind::CyclicTypeDeclarationError { cycle } => {
                for decl in cycle {
                    decl.remap_idents(map);
                }
            }

            // Variantes qui n'ont pas de `Ident` ou ne nécessitent pas de remap :

            Kind::CyclicTaskOrdering
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

/// Formats a slice of `Requirement`s into a comma-separated string,
/// each requirement enclosed in single quotes.
///
/// # Parameters
/// - `requirements`: Slice of `Requirement` items to format.
///
/// # Returns
/// A `String` listing all requirements, each wrapped in single quotes
/// and separated by commas.
///
/// # Example
/// ```
/// let reqs = vec![Requirement::A, Requirement::B];
/// let formatted = format_requirements_list(&reqs);
/// assert_eq!(formatted, "'A', 'B'");
/// ```
fn format_requirements_list(requirements: &[Requirement]) -> String {
    requirements
        .iter()
        .map(|r| format!("'{}'", r))  // Assumes Requirement implements Display
        .collect::<Vec<_>>()
        .join(", ")
}
