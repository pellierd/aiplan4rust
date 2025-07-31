use std::collections::HashMap;
use std::fmt;

use crate::aiplan4rust::diagnostic::Severity;
use crate::aiplan4rust::lang::{Ident, Type};
use crate::aiplan4rust::lang::Requirement;
use crate::aiplan4rust::semantic::symbol::{Declaration, SymbolKind, Usage};
use crate::aiplan4rust::syntax::ast::AstKind;
use crate::aiplan4rust::interner::StringInterner;
use crate::aiplan4rust::semantic::symbol::symbol::Symbol;
use crate::aiplan4rust::syntax::{Span, SyntaxDisplay};


#[derive(Clone, Debug, PartialEq)]
pub enum Kind {
    /// Errors related to token parsing from the lexer and lalrpop parser.
    /// The string values are preserved because these errors originate from raw tokens.

    /// Unexpected token encountered during parsing.
    /// `token` is the actual token found, `expected` lists the tokens that were expected.
    UnexpectedToken {
        token: String,
        expected: Vec<String>,
    },

    /// Unexpected end of file encountered during parsing.
    /// `expected` lists the tokens that were expected before EOF.
    UnexpectedEof {
        expected: Vec<String>,
    },

    /// Invalid token detected by the lexer or parser.
    InvalidToken,

    /// Extra token found where none was expected.
    /// `token` is the unexpected token encountered.
    ExtraToken {
        token: String,
    },

    /// Error indicating that a symbol is used with a signature that does not match any declaration.
    ///
    /// This occurs when the symbol's usage signature (types of arguments and return type)
    /// is incompatible or undefined compared to its declaration.
    ///
    /// # Note
    /// It would be desirable to enhance this error by identifying the first argument
    /// in the signature that causes the mismatch, to provide more precise diagnostics.
    InvalidSymbolSignature {
        declaration: Declaration,
        usage: Usage,
    },

    /// Represents an error where two types in an expression do not match as expected.
    ///
    /// This error is raised when the expression involves incompatible types that
    /// cannot be reconciled, indicating a type mismatch.
    TypeMismatchInExpression {
        ty1: Type,
        ty2: Type,
    },

    /// Represents an error where two types used in a numeric expression are incompatible.
    ///
    /// This error occurs when an operation expecting numeric types receives types
    /// that are not valid for numeric computations (e.g., mixing incompatible or non-numeric types).
    InvalidTypesInNumericExpression {
        ty1: Type,
        ty2: Type,
    },

    /// Indicates that an expression node uses a feature or construct that violates
    /// the PDDL requirements currently active in the context.
    ///
    /// # Fields
    ///
    /// - `node_kind`: The kind of AST node (expression) that triggered the violation.
    /// - `required`: A list of `Requirement`s that are needed for this node kind to be valid.
    ///
    /// This warning helps identify when a construct is used without the necessary
    /// PDDL requirements enabled, e.g., using numeric fluents without declaring
    /// `:numeric-fluents` in the domain requirements.
    RequirementViolation {
        node_kind: AstKind,
        required: Vec<Requirement>,
    },
    /// Represents an error where a symbol is declared more than once within the same scope.
    ///
    /// # Fields
    ///
    /// - `symbol`: The duplicated symbol causing the conflict.
    /// - `original_declaration`: The first declaration of the symbol.
    /// - `conflicting_declaration`: The second (conflicting) declaration of the symbol.
    /// - `scope`: The AST node kind that defines the scope in which the duplication occurs
    ///   (e.g., predicate, action, forall expression).
    ///
    /// This error indicates that a symbol name has been declared multiple times in the same scope,
    /// which is not allowed and may lead to ambiguous or erroneous behavior.
    DuplicatedSymbolDeclarationInScope {
        symbol: Symbol,
        original_declaration: Declaration,
        conflicting_declaration: Declaration,
        scope: AstKind,
    },

    /// Represents an error where task ordering constraints form a cycle,
    /// making the ordering invalid or unsatisfiable.
    ///
    /// This error indicates that there is a loop in the dependency graph
    /// of tasks or ordering constraints, which prevents proper scheduling.
    ///
    /// # Note
    /// It would be beneficial to enhance this error by computing and reporting
    /// the actual cycle detected. Providing the cycle details would help users
    /// to understand and fix the problem more easily.
    CyclicTaskOrdering,

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

    /// Error variant indicating a mismatch between the domain name declared in the domain
    /// definition and the domain name referenced in the problem file.
    ///
    /// This error is triggered when the domain name specified in the problem file
    /// does not match the domain name declared in the corresponding domain definition,
    /// which is a semantic inconsistency.
    ///
    /// # Fields
    /// - `domain_name`: The `Declaration` of the domain name as declared in the domain definition.
    /// - `problem_name`: The `Declaration` of the domain name as referenced in the problem file.
    ///
    /// This mismatch typically means the problem file was intended for a different domain
    /// or there was a typo in the domain name reference.
    DomainProblemNameMismatch {
        domain_name: Declaration,
        problem_name: Declaration,
    },

    /// Warning for ambiguous symbol names that are declared both as a primitive type and a predicate.
    ///
    /// This warning is emitted when the same identifier is used for both a `PrimitiveType` and a
    /// `Predicate` symbol kind, which can lead to confusion or unexpected behavior in semantic analysis.
    ///
    /// # Fields
    /// - `ty`: The declaration of the symbol as a primitive type.
    /// - `predicate`: The declaration of the symbol as a predicate.
    ///
    /// ```
    AmbiguousTypePredicateSymbol {
        ty: Declaration,
        predicate: Declaration,
    },

    /// Warning issued when a task argument uses a type that is a supertype of the one declared.
    ///
    /// This warning highlights a semantic inconsistency where a task uses a more general type
    /// than what is declared by an action or method. According to standard PDDL typing rules,
    /// argument types must match or be more specific (i.e., subtypes), and this form of
    /// upcasting is not permitted.
    ///
    /// **This behavior is considered an aberration and should not be allowed**, as it violates
    /// the intent of typed parameter declarations in PDDL. However, due to compatibility concerns,
    /// notably with the `ultralight_cockpit` domain and the insistence of Holler for more tolerant
    /// behavior, this exception is allowed with a warning.
    ///
    /// # Fields
    ///
    /// - `argument`: The declaration of the argument as defined in the action or method.
    /// - `type_declared`: The declared type of the argument in the action or method.
    /// - `type_used`: The actual type used in the task invocation, which is a supertype of the declared type.
    TaskArgumentIsSupertypeOfDeclaration {
        argument: Declaration,
        type_declared: Type,
        type_used: Type,
    },

    /// Warning indicating the presence of duplicated types within an `Either` construct.
    ///
    /// This warning is emitted during the normalization phase,
    /// before the full symbol table is constructed.
    ///
    /// Therefore, only the identifiers (`Ident`) of the duplicated types
    /// are provided here, without access to their full declarations or supertypes.
    ///
    /// # Field
    ///
    /// - `duplicate_types`: a list of identifiers representing the duplicated types.
    ///
    /// # Note
    ///
    /// For richer diagnostics (including precise declaration locations),
    /// this information should be enhanced later once the symbol table
    /// is available in subsequent analysis phases.
    DuplicateEitherType {
        duplicate_types: Vec<Ident>,
    },

    /// Represents an error indicating a cycle in the type hierarchy defined in the domain.
    ///
    /// This diagnostic is triggered when user-defined types reference each other
    /// in a circular manner (directly or indirectly), forming a cycle that prevents
    /// correct normalization or analysis of the type system.
    ///
    /// For example, if type `A` extends `B`, and `B` extends `A`, this creates a cycle
    /// that cannot be resolved.
    ///
    /// # Fields
    ///
    /// - `cycle`: A vector of `Declaration` items representing the chain of type
    ///   declarations involved in the cycle. The first and last elements may be equal
    ///   to indicate a closed loop.
    ///
    /// # Context
    ///
    /// This error is typically emitted during the domain normalization phase, when the
    /// hierarchy of type declarations is being validated.
    ///
    /// # Example
    ///
    /// ```text
    /// type A extends B
    /// type B extends A
    /// ```
    ///
    /// This will result in a `CyclicTypeDeclaration` error with a cycle including both `A` and `B`.
    CyclicTypeDeclaration {
        cycle: Vec<Declaration>
    },
    /// Represents an error caused by conflicting symbol declarations across different contexts.
    ///
    /// This error indicates that a symbol declared in the `problem` context conflicts with one or more
    /// declarations of the same symbol found in the `domain` context. Such conflicts typically arise
    /// when symbol kinds or definitions differ, leading to ambiguity or invalid references.
    ///
    /// # Fields
    ///
    /// - `problem_declaration`: The `Declaration` originating from the problem context that conflicts
    ///   with declarations in the domain.
    /// - `conflicting_domain_declarations`: A vector of `Declaration` instances from the domain context
    ///   that are in conflict with the `problem_declaration`.
    ///
    /// # Usage
    ///
    /// This error is used during semantic checking or linking phases to detect and report symbol conflicts
    /// between the problem and domain symbol tables, enabling diagnostics to provide detailed feedback
    /// for resolution.
    CrossConflictSymbolDeclaration {
        problem_declaration: Declaration,
        conflicting_domain_declarations: Vec<Declaration>,
    },

    /// Warning emitted when a type is implicitly declared as an `(either ...)` type due to
    /// multiple conflicting parent type declarations.
    ///
    /// This warning indicates that the type `ty` has been declared with different parent types
    /// listed in `duplicate_types`. The system has automatically merged these into an implicit
    /// `(either ...)` type to resolve ambiguity.
    ///
    /// # Fields
    ///
    /// - `ty`: The identifier of the type being declared.
    /// - `duplicate_types`: A list of conflicting parent type identifiers causing the implicit merge.
    /// - `duplicate_spans`: The source code spans corresponding to each conflicting parent type declaration.
    ///
    /// # Suggestion
    ///
    /// To avoid ambiguity, it is recommended to declare the type explicitly using the `(either ...)`
    /// syntax, listing all parent types.
    ImplicitEitherTypeDeclaration {
        ty: Ident,
        duplicate_types: Vec<Ident>,
        duplicate_spans: Vec<Span>,
    },
    /// A warning emitted when a requirement is declared multiple times.
    ///
    /// This diagnostic is used to indicate that the same requirement appears more than once
    /// in a given context (e.g., in a type, operator, or action definition). Although duplicate
    /// requirements may not cause immediate semantic issues, they are usually unintended and
    /// may clutter the specification.
    ///
    /// # Fields
    /// * `duplicate_requirements` - A list of all duplicated requirements that were found.
    DuplicateRequirementWarning {
        duplicate_requirements: Vec<Requirement>,
    },

    /// Variant representing a custom error with a descriptive message and an optional suggestion.
    ///
    /// This variant stores owned `String`s for both the error message and an optional suggestion,
    /// allowing flexible and dynamic creation of error diagnostics without lifetime constraints.
    ///
    /// # Fields
    ///
    /// - `message`: A detailed description of the error.
    /// - `suggestion`: An optional helpful suggestion or advice on how to resolve or avoid the error.
    ///
    /// # Example
    ///
    /// ```
    /// let error_kind = Kind::CustomError {
    ///     message: "Failed to parse configuration file.".to_string(),
    ///     suggestion: Some("Check if the file path is correct and the file format is valid.".to_string()),
    /// };
    ///
    /// if let Kind::CustomError { message, suggestion } = error_kind {
    ///     println!("Error: {}", message);
    ///     if let Some(sugg) = suggestion {
    ///         println!("Suggestion: {}", sugg);
    ///     }
    /// }
    /// ```
    CustomError {
        message: String,
        suggestion: Option<String>,
    },

    /// Represents a custom warning with a descriptive message and an optional suggestion.
    ///
    /// This variant owns its warning message and optionally a suggestion string,
    /// allowing for flexible and dynamic creation of warning information without lifetime constraints.
    ///
    /// # Fields
    ///
    /// - `message`: A detailed description of the warning.
    /// - `suggestion`: An optional helpful suggestion or advice on how to address or mitigate the warning.
    ///
    /// # Examples
    ///
    /// ```
    /// let warning = Kind::CustomWarning {
    ///     message: "Deprecated API usage detected.".to_string(),
    ///     suggestion: Some("Consider updating to the new API version.".to_string()),
    /// };
    ///
    /// let warning_without_suggestion = Kind::CustomWarning {
    ///     message: "Minor performance issue found.".to_string(),
    ///     suggestion: None,
    /// };
    ///
    /// println!("Warning: {}", warning.message);
    /// if let Some(sugg) = &warning.suggestion {
    ///     println!("Suggestion: {}", sugg);
    /// }
    /// ```
    CustomWarning {
        message: String,
        suggestion: Option<String>,
    },
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
            Kind::CyclicTypeDeclaration { .. } => "E1011".to_string(),
            // WARNINGS ANALYSER
            Kind::SymbolDeclaredAmbiguouslyAsKeyword { .. } => "W1010".to_string(),
            Kind::UnusedSymbol { .. } => "W1011".to_string(),
            Kind::RequirementViolation { .. } => "W10012".to_string(),
            Kind::AmbiguousTypePredicateSymbol { .. } => "W1013".to_string(),
            Kind::TaskArgumentIsSupertypeOfDeclaration { .. } => "W1014".to_string(),
            Kind::DuplicateEitherType { .. } => "W1015".to_string(),
            Kind::ImplicitEitherTypeDeclaration { .. } => "W1016".to_string(),

            // WARNINGS LINKER
            Kind::DomainProblemNameMismatch { .. } => "W2000".to_string(),
            Kind::CrossConflictSymbolDeclaration { .. } => "E2001".to_string(),


            Kind::CustomError{ .. } => "E000X".to_string(),
            Kind::CustomWarning{ .. } => "W000X".to_string(),
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
            Kind::InvalidSymbolSignature { declaration, ..} => {
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
                let name = symbol_to_string(usage.symbol(), interner);
                format!("{} symbol '{}' is undeclared.", usage.symbol_kind(), name)
            }
            Kind::SymbolConflictsWithKeyword { declaration, .. } => {
                let name = symbol_to_string(declaration.symbol(), interner);
                format!("Symbol '{}' is used as a language keyword", name)
            }

            Kind::SymbolDeclaredAmbiguouslyAsKeyword { declaration, .. } => {
                let name = symbol_to_string(declaration.symbol(), interner);
                format!("Symbol '{}' is ambiguous as a language keyword", name)
            }
            Kind::UnusedSymbol { declaration } => {
                let name = symbol_to_string(&declaration.symbol(), interner);
                format!("{} symbol '{}' is unused", declaration.symbol_kind(), name)
            }
            Kind::DomainProblemNameMismatch { domain_name, problem_name } => {
                let domain_str = symbol_to_string(domain_name.symbol(), interner);
                let problem_str = symbol_to_string(problem_name.symbol(), interner);
                format!("Domain '{}' and problem '{}' names do not match.", domain_str, problem_str)
            }
            Kind::AmbiguousTypePredicateSymbol { ty, ..} => {
                let ty_name = symbol_to_string(ty.symbol(), interner);
                format!(
                    "Ambiguous symbol '{}': declared both as a type and a predicate.",
                    ty_name
                )
            }
            Kind::TaskArgumentIsSupertypeOfDeclaration { argument, .. } => {
                format!(
                    "Type mismatch: argument '{}' uses a broader type than declared (upcasting is discouraged).",
                    symbol_to_string(argument.symbol(), interner)
                )
            }
            Kind::DuplicateEitherType { duplicate_types } => {
                let names = format_ident_list(duplicate_types, interner);
                format!("Duplicate primitive types in 'either' type: {}.", names)
            }
            Kind::CyclicTypeDeclaration { .. } => {
                "Type declarations form a cycle; this creates an invalid type hierarchy.".to_string()
            }
            Kind::CrossConflictSymbolDeclaration { problem_declaration, .. } => {
                format!(
                    "Conflicting declaration for symbol '{}' found between problem and domain.",
                    symbol_to_string(problem_declaration.symbol(), interner)
                )
            }
            Kind::ImplicitEitherTypeDeclaration { ty, .. } => {
                format!(
                    "Type `{}` was declared multiple times and was implicitly interpreted as an `(either ...)` type.",
                    ident_to_string(*ty, interner),
                )
            }
            Kind::DuplicateRequirementWarning { .. } => {
                "Requirements definition contains duplicated declarations which have been ignored."
                    .to_string()
            }
            Kind::CustomError {message, .. } => message.to_string(),
            Kind::CustomWarning {message, .. } => message.to_string(),
        }
    }

    pub fn severity(&self) -> Severity {
        match self {
            // PARSER ERRORS
            Kind::UnexpectedToken { .. } => Severity::Error,
            Kind::UnexpectedEof { .. } => Severity::Error,
            Kind::InvalidToken => Severity::Error,
            Kind::ExtraToken { .. } => Severity::Error,
            Kind::CustomError { .. } => Severity::Error,
            Kind::CustomWarning { .. } => Severity::Error,

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
            Kind::CyclicTypeDeclaration { .. } => Severity::Error,
            // ANALYSER WARNINGS
            Kind::SymbolDeclaredAmbiguouslyAsKeyword { .. } => Severity::Warning,
            Kind::UnusedSymbol { .. } => Severity::Warning,
            Kind::RequirementViolation { .. } => Severity::Warning,
            Kind::AmbiguousTypePredicateSymbol { .. } => Severity::Warning,
            Kind::TaskArgumentIsSupertypeOfDeclaration { .. } => Severity::Warning,
            Kind::DuplicateEitherType { .. } => Severity::Warning,
            Kind::ImplicitEitherTypeDeclaration { .. } => Severity::Warning,

            // LINKER WARNINGS
            Kind::DomainProblemNameMismatch { .. } => Severity::Warning,
            // LINKER ERROR
            Kind::CrossConflictSymbolDeclaration {..} => Severity::Error,


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
            Kind::InvalidSymbolSignature { declaration, .. } => {
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
                    "Numeric expressions require operands of type 'number', but found '{}' and '{}'. \
                    Ensure both operands are numeric types.",
                    ty1_str,
                    ty2_str
                ))
            }
            Kind::RequirementViolation { node_kind, required } => {
                Some(format!(
                    "Expression type '{}' requires one of these requirements: {}.",
                    node_kind, // ou format!("{:?}", node_kind) si besoin
                    format_requirement_list(&required)
                ))
            }
            Kind::DuplicatedSymbolDeclarationInScope { symbol, original_declaration: declaration1, conflicting_declaration: declaration2, scope } => {
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
                        "Constant '{}' is not declared. Declare it in the ':constants' \
                        section (domain) or ':objects' section (problem).",
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
                        "Variable '{}' is not declared.\
                         You likely need to add it to the ':parameters' list of the enclosing \
                         definition (e.g., '?x - type').",
                        name
                    )),
                }
            }
            Kind::SymbolConflictsWithKeyword { declaration, expected_kind, requirements } => {
                let name = symbol_to_string(declaration.symbol(), interner);
                let reqs = format_requirement_list(requirements);
                Some(format!(
                    "Symbol '{}' conflicts with a reserved keyword under requirements: {}. \
                    It must be declared as a {:?} (e.g., type, function, variable).",
                    name,
                    reqs,
                    expected_kind
                ))
            }
            Kind::SymbolDeclaredAmbiguouslyAsKeyword { declaration, requirements, .. } => {
                let name = symbol_to_string(declaration.symbol(), interner);
                let reqs = format_requirement_list(requirements);
                Some(format!(
                    "Symbol '{}' is ambiguous because it is used as a language keyword with \
                    requirements: {}. Consider renaming or using a different symbol.",
                    name,
                    reqs
                ))
            }
            Kind::UnusedSymbol { declaration } => {
                let name = symbol_to_string(declaration.symbol(), interner);
                Some(format!(
                    "{} symbol '{}' is declared but not used. \
                    Consider removing it to clean up your code.",
                    declaration.symbol_kind(),
                    name
                ))
            }
            Kind::DomainProblemNameMismatch { domain_name, .. } => {
                let domain_str = symbol_to_string(domain_name.symbol(), interner);
                Some(format!(
                    "Check that the problem's domain name matches the domain definition: \
                    expected '{}'.",
                    domain_str
                ))
            }
            Kind::AmbiguousTypePredicateSymbol { ty, .. } => {
                let symbol = symbol_to_string(ty.symbol(), interner);
                Some(format!(
                    "The symbol '{}' is declared both as a type and a predicate.  \
                    Consider renaming one of them to avoid ambiguity.",
                    symbol
                ))
            }
            Kind::TaskArgumentIsSupertypeOfDeclaration {
                argument,
                type_declared,
                type_used,
            } => {
                Some(format!(
                    "The argument '{}' uses type '{}' which is a supertype of the declared type '{}'. \
                    Argument types should match exactly. \
                    Prefer defining a new method with matching types instead.",
                    symbol_to_string(argument.symbol(), interner),
                    type_to_string(type_used, interner),
                    type_to_string(type_declared, interner)
                ))
            }
            Kind::DuplicateEitherType { duplicate_types } => {
                let listed_types = if duplicate_types.len() == 1 {
                    format!("type '{}'", format_ident_list(&duplicate_types, interner))
                } else {
                    format!("types '{}'", format_ident_list(&duplicate_types, interner))
                };
                Some(format!(
                    "Duplicate {} found in an 'either' type declaration; \
                    these duplicates are ignored but consider removing them to clean up your code.",
                    listed_types,
                ))
            }
            Kind::CyclicTypeDeclaration { cycle } => {
                Some(format!(
                    "Cycle detected in type hierarchy involving types: {}. \
                    Remove the cyclic inheritance to resolve the issue.",
                    format_declaration_list(cycle, interner)
                ))
            }
            Kind::CrossConflictSymbolDeclaration { problem_declaration, conflicting_domain_declarations } => {
                // Collect domain declaration spans (line numbers or code ranges)
                let domain_spans: Vec<String> = conflicting_domain_declarations
                    .iter()
                    .map(|decl| span_to_string(&decl.span()))
                    .collect();

                let formatted_lines = match domain_spans.len() {
                    0 => String::from("an unknown location"),
                    1 => domain_spans[0].clone(),
                    2 => format!("{} and {}", domain_spans[0], domain_spans[1]),
                    _ => {
                        let (all_but_last, last) = domain_spans.split_at(domain_spans.len() - 1);
                        format!("{} and {}", all_but_last.join(", "), last[0])
                    }
                };
                Some(format!(
                    "Symbol `{}` declared as `{}` in the problem conflicts with domain declarations at lines: {}. \
                    Please resolve these conflicts by ensuring consistent declarations \
                    or consider renaming the symbol in the problem.",
                    symbol_to_string(problem_declaration.symbol(), interner),
                    problem_declaration.symbol().kind(),
                    formatted_lines,
                ))
            }
            Kind::ImplicitEitherTypeDeclaration { ty, duplicate_spans, .. } => {
                // Convert spans to readable location strings
                let duplicate_locations: Vec<String> = duplicate_spans
                    .iter()
                    .map(|span| span_to_string(span))
                    .collect();

                // Format locations nicely for display
                let formatted_locations = match duplicate_locations.len() {
                    0 => String::from("an unknown location"),
                    1 => duplicate_locations[0].clone(),
                    2 => format!("{} and {}", duplicate_locations[0], duplicate_locations[1]),
                    _ => {
                        let (all_but_last, last) = duplicate_locations.split_at(duplicate_locations.len() - 1);
                        format!("{} and {}", all_but_last.join(", "), last[0])
                    }
                };

                Some(format!(
                    "The type `{}` was declared multiple times at locations: {}. \
                    These declarations were implicitly merged into an `(either ...)` type declaration. \
                    To avoid ambiguity, consider explicitly declaring the type using `(either ...)`.",
                    ident_to_string(*ty, interner),
                    formatted_locations,
                ))
            }
            Kind::DuplicateRequirementWarning { duplicate_requirements } => {
                Some(format!(
                    "The following requirement(s) are declared multiple times in the domain and have been ignored. \
                    Consider removing them to prevent redundancy: {}.",
                    format_requirement_list(duplicate_requirements),
                ))
            }
            Kind::CustomError {suggestion, .. } => suggestion.clone(),
            Kind::CustomWarning {suggestion, .. } => suggestion.clone(),
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
                original_declaration: declaration1,
                conflicting_declaration: declaration2,
                ..
            } => {
                declaration1.remap_idents(map);
                declaration2.remap_idents(map);
            }

            Kind::UndeclaredSymbol { usage } => {
                usage.remap_idents(map);
            }

            Kind::SymbolConflictsWithKeyword { declaration, .. }
            | Kind::SymbolDeclaredAmbiguouslyAsKeyword { declaration, .. }
            |Kind::UnusedSymbol { declaration } => {
                declaration.remap_idents(map);
            }

            | Kind::DomainProblemNameMismatch { domain_name, problem_name} => {
                domain_name.remap_idents(map);
                problem_name.remap_idents(map);
            }
            | Kind::AmbiguousTypePredicateSymbol { ty, predicate } => {
                ty.remap_idents(map);
                predicate.remap_idents(map);
            }
            | Kind::TaskArgumentIsSupertypeOfDeclaration {
                argument, type_declared, type_used,
            } => {
                argument.remap_idents(map);
                type_declared.remap_idents(map);
                type_used.remap_idents(map);
            }
            | Kind::DuplicateEitherType { duplicate_types } => {
                for ident in duplicate_types {
                   ident.remap_idents(map);
                }
            }
            Kind::CyclicTypeDeclaration { cycle } => {
                for decl in cycle {
                    decl.remap_idents(map);
                }
            }
            | Kind::CrossConflictSymbolDeclaration {
                problem_declaration,
                conflicting_domain_declarations,
            } => {
                problem_declaration.remap_idents(map);
                for decl in conflicting_domain_declarations {
                    decl.remap_idents(map);
                }
            },
            | Kind::ImplicitEitherTypeDeclaration {
                ty,
                duplicate_types,
                ..
            } => {
                ty.remap_idents(map);
                for ident in duplicate_types {
                    ident.remap_idents(map);
                }
            },
            Kind::UnexpectedToken { .. }
            | Kind::UnexpectedEof { .. }
            | Kind::InvalidToken
            | Kind::ExtraToken { .. }
            | Kind::RequirementViolation { .. }
            | Kind::CyclicTaskOrdering
            | Kind::DuplicateRequirementWarning { .. }
            | Kind::CustomError { .. }
            | Kind::CustomWarning { .. }=> {
                // Pas de remap nécessaire ici
            }
        }
    }
}


impl fmt::Display for Kind {
    /// Formats the diagnostic kind as a user-readable string, primarily for debugging purposes.
    ///
    /// This method outputs the diagnostic code, severity level, main message, and if available,
    /// a suggestion message.
    ///
    /// **Note:** Identifiers (`Ident`) within the diagnostic are **not** replaced by their
    /// string representations here; the output may contain raw identifier references.
    ///
    /// # Arguments
    ///
    /// * `f` - The formatter to write the formatted string into.
    ///
    /// # Returns
    ///
    /// Returns `fmt::Result` indicating success or failure of the write operation.
    ///
    /// # Example
    ///
    /// ```
    /// let kind = Kind::SomeDiagnosticKind { ... };
    /// println!("{}", kind); // Prints formatted diagnostic info
    /// ```
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

/// Returns the name of the given identifier as a `String`, optionally resolving it
/// through a string interner.
///
/// # Parameters
/// - `ident`: The identifier to convert to a string.
/// - `interner`: Optional reference to a `StringInterner` used to resolve the identifier.
///
/// # Returns
/// A `String` representing the resolved name of the identifier.
/// - If the `interner` is provided and the identifier is found, returns the resolved string.
/// - If the `interner` is provided but the identifier is not found, returns `"unknown(<ident>)"`.
/// - If the `interner` is not provided, returns the raw identifier as a string.
fn ident_to_string(ident: Ident, interner: Option<&StringInterner>) -> String {
    if let Some(interner) = interner {
        interner
            .resolve_ident(ident)
            .map(|s| s.to_string())
            .unwrap_or_else(|| format!("unknown({})", ident))
    } else {
        ident.to_string()
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
    ident_to_string(symbol.ident(), interner)
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

/// Formats a list of `Ident` values into a comma-separated string, resolving each ident using
/// an optional `StringInterner`.
///
/// # Parameters
/// - `idents`: Slice of `Ident` to format.
/// - `interner`: Optional reference to a `StringInterner` used to resolve identifiers.
///
/// # Returns
/// A string of comma-separated identifiers, each converted to string via `ident_to_string`.
fn format_ident_list(idents: &[Ident], interner: Option<&StringInterner>) -> String {
    idents
        .iter()
        .map(|&ident| ident_to_string(ident, interner))
        .collect::<Vec<_>>()
        .join(", ")
}

/// Converts a `Span` into a user-friendly string representation of its line range.
///
/// This function is designed to simplify span information for end-user messages
/// by focusing only on line numbers. If the span covers a single line,
/// it returns `"line N"`. If it spans multiple lines, it returns `"lines N–M"`.
///
/// # Arguments
///
/// * `span` - A reference to the `Span` to be formatted.
///
/// # Returns
///
/// A `String` describing the line(s) the span covers.
///
/// # Examples
///
/// ```rust
/// let span = Span::new(0, 10, 12, 1, 12, 5); // example: same line
/// assert_eq!(span_to_string(&span), "line 12");
///
/// let span = Span::new(0, 20, 14, 1, 16, 10); // example: multi-line
/// assert_eq!(span_to_string(&span), "lines 14–16");
/// ```
fn span_to_string(span: &Span) -> String {
    if span.begin_line() == span.end_line() {
        format!("line {}", span.begin_line())
    } else {
        format!("lines {}–{}", span.begin_line(), span.end_line())
    }
}

/// Formats a list of `Declaration`s into a comma-separated string by extracting their symbol identifiers
/// and resolving them using an optional `StringInterner`.
///
/// # Parameters
/// - `declarations`: Slice of `Declaration` to format.
/// - `interner`: Optional reference to a `StringInterner` used to resolve identifiers.
///
/// # Returns
/// A string of comma-separated symbols (idents) of the declarations, resolved via `format_ident_list`.
fn format_declaration_list(
    declarations: &[Declaration],
    interner: Option<&StringInterner>
) -> String {
    let idents: Vec<Ident> = declarations.iter().map(|decl| decl.symbol_ident()).collect();
    format_ident_list(&idents, interner)
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
fn format_requirement_list(requirements: &[Requirement]) -> String {
    requirements
        .iter()
        .map(|r| format!("'{}'", r))  // Assumes Requirement implements Display
        .collect::<Vec<_>>()
        .join(", ")
}
