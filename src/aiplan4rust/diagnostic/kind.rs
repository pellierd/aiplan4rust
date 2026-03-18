//! Diagnostic kinds used to represent errors and warnings detected during
//! parsing, logic, and semantic analysis of PDDL domains and problems.
//!
//! This module defines a comprehensive `Kind` enum that encodes various types of
//! issues such as syntax errors, typing mismatches, undeclared symbols, cyclic definitions,
//! ambiguous declarations, and more.
//!
//! Each variant in the enum corresponds to a specific kind of diagnostic, and many of them
//! carry structured data to support rich, precise error messages and suggestions for users.
//!
//! These diagnostics are used throughout the system to ensure correctness, detect inconsistencies,
//! and provide actionable feedback during different phases of compilation or interpretation.

use std::collections::HashMap;
use std::fmt;

use crate::aiplan4rust::diagnostic::{renderer, DiagnosticKind, Severity};
use crate::aiplan4rust::interner::InternerError;
use crate::aiplan4rust::lang::{RemapSymbol, Requirement};
use crate::aiplan4rust::lang::{SymbolId, Type};
use crate::aiplan4rust::semantic::symbol::symbol::Symbol;
use crate::aiplan4rust::semantic::symbol::{Declaration, SymbolKind, Usage};
use crate::aiplan4rust::syntax::ast::AstKind;
use crate::aiplan4rust::syntax::Span;

/// Represents all possible diagnostic kinds that can be emitted during
/// parsing, logic, or semantic analysis of PDDL structures.
///
/// Each variant of this enum corresponds to a specific class of diagnostic,
/// such as parsing errors, typing mismatches, undeclared symbols, requirement
/// violations, ambiguous or duplicated declarations, and more.
///
/// This enum is the common abstraction used by the diagnostic system to classify
/// and provide structured error or warning messages to the user.
///
/// Most variants carry structured fields that allow downstream components
/// to generate informative diagnostics with suggestions and spans.
///
/// See individual variants for detailed descriptions and usage examples.
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
    UnexpectedEof { expected: Vec<String> },

    /// Invalid token detected by the lexer or parser.
    InvalidToken,

    /// Extra token found where none was expected.
    /// `token` is the unexpected token encountered.
    ExtraToken { token: String },

    /// Error indicating that a numeric literal could not be parsed correctly.
    ///
    /// Typically occurs when a number contains invalid characters or an incorrectly placed decimal point.
    /// The `number` field contains the string slice that failed to parse.
    InvalidNumber {
        number: String,
    },

    /// Error indicating that a definition block appears more than once.
    ///
    /// This happens when a block of the same kind (e.g., `PredicatesDef`, `FunctionsDef`) is defined
    /// multiple times in the same context. The `block` field indicates which `AstKind` was duplicated.
    DuplicateDefinitionBlock {
        block: AstKind,
    },

    /// Error indicating that a definition block appears out of the expected order.
    ///
    /// Certain blocks in the syntax must appear in a specific sequence. If a block is encountered
    /// before blocks that are expected to come earlier, this error is raised.
    /// - `block`: the block that is incorrectly ordered
    /// - `order`: a list of blocks that this block should have appeared after
    InvalidDefinitionBlockOrder {
        block: AstKind,
        order: Vec<AstKind>,
    },

    /// Error indicating that a symbol is used with a signature that does not match any declaration.
    ///
    /// This occurs when the symbol's usage signature (types of arguments and return typing)
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
    /// cannot be reconciled, indicating a typing mismatch.
    TypeMismatchInExpression { ty1: Type<SymbolId>, ty2: Type<SymbolId> },

    /// Represents an error where two types used in a numeric expression are incompatible.
    ///
    /// This error occurs when an operation expecting numeric types receives types
    /// that are not valid for numeric computations (e.g., mixing incompatible or non-numeric types).
    InvalidTypesInNumericExpression { ty1: Type<SymbolId>, ty2: Type<SymbolId> },

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

    /// Represents a warning where a variable is declared more than once within a skeleton scope.
    ///
    /// This is specifically used for `AtomicFormulaSkeleton` or `AtomicFunctionSkeleton`
    /// to support legacy IPC domains (like Logistics) where duplicate parameter names
    /// are used as placeholders.
    ///
    /// # Fields
    ///
    /// - `symbol`: The duplicated variable symbol.
    /// - `original_declaration`: The first declaration of the variable.
    /// - `conflicting_declaration`: The second (conflicting) declaration.
    /// - `scope`: The AST node kind defining the skeleton scope.
    DuplicateVariableSkeletonDeclaration {
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
    UndeclaredSymbol { usage: Usage },

    /// Error raised when a user-defined symbol conflicts with a reserved PDDL keyword,
    /// depending on the active `:requirements`.
    ///
    /// Some identifiers in PDDL (like `object`, `number`, `?duration`, etc.) have a
    /// special meaning when certain features are enabled. Declaring a symbol with such
    /// an identifier leads to this error if the declaration does not match the expected usage.
    ///
    /// For example, declaring a new typing named `object` is invalid when `:typing` is enabled,
    /// since `object` is a built-in primitive typing in that context.
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
    /// with a reserved PDDL language keyword, but its kind matches the expected typing.
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
    UnusedSymbol { declaration: Declaration },

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

    /// Warning for ambiguous symbol names that are declared both as a primitive typing and a predicate.
    ///
    /// This warning is emitted when the same identifier is used for both a `PrimitiveType` and a
    /// `Predicate` symbol kind, which can lead to confusion or unexpected behavior in semantic analysis.
    ///
    /// # Fields
    /// - `types`: The declaration of the symbol as a primitive typing.
    /// - `predicate`: The declaration of the symbol as a predicate.
    ///
    /// ```
    AmbiguousTypePredicateSymbol {
        ty: Declaration,
        predicate: Declaration,
    },

    /// Warning issued when a task argument uses a typing that is a supertype of the one declared.
    ///
    /// This warning highlights a semantic inconsistency where a task uses a more general typing
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
    /// - `type_declared`: The declared typing of the argument in the action or method.
    /// - `type_used`: The actual typing used in the task invocation, which is a supertype of the declared typing.
    TaskArgumentIsSupertypeOfDeclaration {
        argument: Declaration,
        type_declared: Type<SymbolId>,
        type_used: Type<SymbolId>,
    },

    /// Warning indicating the presence of duplicated types within an `Either` construct.
    ///
    /// This warning is emitted during the logic phase,
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
    DuplicateEitherType { duplicate_types: Vec<SymbolId> },

    /// Represents an error indicating a cycle in the typing hierarchy defined in the domain.
    ///
    /// This diagnostic is triggered when user-defined types reference each other
    /// in a circular manner (directly or indirectly), forming a cycle that prevents
    /// correct logic or analysis of the typing system.
    ///
    /// For example, if typing `A` extends `B`, and `B` extends `A`, this creates a cycle
    /// that cannot be resolved.
    ///
    /// # Fields
    ///
    /// - `cycle`: A vector of `Declaration` items representing the chain of typing
    ///   declarations involved in the cycle. The first and last elements may be equal
    ///   to indicate a closed loop.
    ///
    /// # Context
    ///
    /// This error is typically emitted during the domain logic phase, when the
    /// hierarchy of typing declarations is being validated.
    ///
    /// # Example
    ///
    /// ```text
    /// typing A extends B
    /// typing B extends A
    /// ```
    ///
    /// This will result in a `CyclicTypeDeclaration` error with a cycle including both `A` and `B`.
    CyclicTypeDeclaration { cycle: Vec<Declaration> },

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

    /// Warning emitted when an entity (object, type, or constant) is declared multiple times,
    /// leading to an implicit `(either ...)` type construction.
    ///
    /// This warning indicates that the identifier `ty` has been defined in several locations
    /// with different parent types. To maintain consistency, the system merges these into
    /// a single internal representation using an implicit `(either ...)` type.
    ///
    /// # Fields
    ///
    /// - `ty`: The identifier of the entity being redeclared.
    /// - `kind`: The category of the declaration (e.g., `ObjectsDef`, `TypesDef`) to provide context.
    /// - `duplicate_types`: The list of all parent types encountered across the multiple declarations.
    /// - `duplicate_spans`: The source code spans corresponding to each redundant declaration.
    ///
    /// # Suggestion
    ///
    /// For better readability and to avoid ambiguity, consider merging these declarations
    /// manually into a single line using the explicit `(either ...)` syntax.
    DuplicatedDeclaration {
        ty: SymbolId,
        kind: AstKind,
        duplicate_types: Vec<SymbolId>,
        duplicate_spans: Vec<Span>,
    },

    /// Error raised when a symbol is redeclared with an incompatible type signature.
    ///
    /// In PDDL, certain types like `number` are primitive and subject to strict
    /// constraints. This error occurs during the normalization pass when the
    /// merging of two declarations for the same symbol is logically impossible.
    ///
    /// This typically happens when:
    /// - A symbol is declared as a `number` (fluent) in one place and an `object` in another.
    /// - The `number` type is used within an `either` compound type, which is
    ///   disallowed by the PDDL standard.
    ///
    /// ### Fields:
    /// - `symbol`: The identifier of the symbol that has conflicting type declarations.
    /// - `kind`: The [`AstKind`] of the declaration, used to provide a specific
    ///   entity name (e.g., "function", "constant") in the error message.
    /// - `original_span`: The source code location of the first declaration, used as
    ///   the reference point for the conflict.
    /// - `expected_types`: The list of parent types from the original (first)
    ///   declaration.
    /// - `found_types`: The list of parent types from the offending (second)
    ///   declaration that caused the incompatibility.
    ///
    /// ### Example:
    /// ```pddl
    /// (:functions (distance ?a ?b - number))
    /// (:functions (distance ?a ?b - object)) ;; Error: Incompatible with 'number'
    /// ```
    IncompatibleTypeDeclarations {
        symbol: SymbolId,
        kind: AstKind,
        original_span: Span,
        expected_types: Vec<SymbolId>,
        found_types: Vec<SymbolId>,
    },

    /// A warning emitted when a requirement is declared multiple times.
    ///
    /// This diagnostic is used to indicate that the same requirement appears more than once
    /// in a given context (e.g., in a typing, operator, or action definition). Although duplicate
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
    /// Returns a 3-character unique string code for the kind.
    ///
    /// Errors and warnings have separate numbering starting at "000" (errors) and "000" (warnings).
    /// Returns a static string slice.
    pub fn code(&self) -> &'static str {
        match self {
            // ERRORS (000..)
            Kind::CustomError { .. } => "000",
            Kind::UnexpectedToken { .. } => "001",
            Kind::UnexpectedEof { .. } => "002",
            Kind::InvalidToken => "003",
            Kind::ExtraToken { .. } => "004",
            Kind::InvalidNumber { .. } => "005",
            Kind::DuplicateDefinitionBlock { .. } => "006",
            Kind::InvalidDefinitionBlockOrder { .. } => "007",
            Kind::InvalidSymbolSignature { .. } => "008",
            Kind::TypeMismatchInExpression { .. } => "009",
            Kind::InvalidTypesInNumericExpression { .. } => "010",
            Kind::DuplicatedSymbolDeclarationInScope { .. } => "011",
            Kind::CyclicTaskOrdering { .. } => "012",
            Kind::UndeclaredSymbol { .. } => "013",
            Kind::SymbolConflictsWithKeyword { .. } => "014",
            Kind::CyclicTypeDeclaration { .. } => "015",
            Kind::CrossConflictSymbolDeclaration { .. } => "016",

            // WARNINGS (000..)
            Kind::DomainProblemNameMismatch { .. } => "000",
            Kind::CustomWarning { .. } => "001",
            Kind::DuplicateRequirementWarning { .. } => "002",
            Kind::SymbolDeclaredAmbiguouslyAsKeyword { .. } => "003",
            Kind::UnusedSymbol { .. } => "004",
            Kind::RequirementViolation { .. } => "005",
            Kind::AmbiguousTypePredicateSymbol { .. } => "006",
            Kind::TaskArgumentIsSupertypeOfDeclaration { .. } => "007",
            Kind::DuplicateEitherType { .. } => "008",
            Kind::DuplicatedDeclaration { .. } => "009",
            Kind::DuplicateVariableSkeletonDeclaration { .. } => "010",
            Kind::IncompatibleTypeDeclarations { .. } => "011",
        }
    }

    /// Returns the severity level associated with this diagnostic kind.
    ///
    /// The severity indicates the impact of the diagnostic on the correctness or
    /// quality of the code. It is used to distinguish between errors and warnings.
    ///
    /// # Returns
    ///
    /// * `Severity::Error` — Indicates a critical issue that must be fixed for correct
    ///   program behavior or successful compilation.
    ///
    /// * `Severity::Warning` — Indicates a potential problem or code smell that
    ///   should be addressed to improve code quality, but does not prevent compilation.
    ///
    /// # Examples
    ///
    /// ```
    /// let kind = Kind::UnexpectedToken { token: "foo".to_string() };
    /// assert_eq!(kind.severity(), Severity::Error);
    /// ```
    pub fn severity(&self) -> Severity {
        match self {
            // ERRORS
            Kind::UnexpectedToken { .. } => Severity::Error,
            Kind::UnexpectedEof { .. } => Severity::Error,
            Kind::InvalidToken => Severity::Error,
            Kind::ExtraToken { .. } => Severity::Error,
            Kind::InvalidNumber { .. } => Severity::Error,
            Kind::DuplicateDefinitionBlock { .. } => Severity::Error,
            Kind::InvalidDefinitionBlockOrder { .. } => Severity::Error,
            Kind::CustomError { .. } => Severity::Error,
            Kind::CustomWarning { .. } => Severity::Error,
            Kind::DuplicateRequirementWarning { .. } => Severity::Warning,
            Kind::InvalidSymbolSignature { .. } => Severity::Error,
            Kind::TypeMismatchInExpression { .. } => Severity::Error,
            Kind::InvalidTypesInNumericExpression { .. } => Severity::Error,
            Kind::DuplicatedSymbolDeclarationInScope { .. } => Severity::Error,
            Kind::CyclicTaskOrdering => Severity::Error,
            Kind::UndeclaredSymbol { .. } => Severity::Error,
            Kind::SymbolConflictsWithKeyword { .. } => Severity::Error,
            Kind::CyclicTypeDeclaration { .. } => Severity::Error,
            Kind::CrossConflictSymbolDeclaration { .. } => Severity::Error,
            Kind::IncompatibleTypeDeclarations { .. } => Severity::Error,
            // WARNINGS
            Kind::SymbolDeclaredAmbiguouslyAsKeyword { .. } => Severity::Warning,
            Kind::UnusedSymbol { .. } => Severity::Warning,
            Kind::RequirementViolation { .. } => Severity::Warning,
            Kind::AmbiguousTypePredicateSymbol { .. } => Severity::Warning,
            Kind::TaskArgumentIsSupertypeOfDeclaration { .. } => Severity::Warning,
            Kind::DuplicateEitherType { .. } => Severity::Warning,
            Kind::DuplicatedDeclaration { .. } => Severity::Warning,
            Kind::DomainProblemNameMismatch { .. } => Severity::Warning,
            Kind::DuplicateVariableSkeletonDeclaration { .. } => Severity::Warning,
        }
    }
}

impl RemapSymbol for DiagnosticKind {
    /// Remaps all `Ident` instances contained within this diagnostic according to the provided map.
    ///
    /// This function traverses the diagnostic's internal data and replaces each `Ident`
    /// using the mapping in `map`. It ensures that all identifiers are updated consistently,
    /// for example after renaming, symbol resolution, or merging operations.
    ///
    /// # Parameters
    ///
    /// - `map`: A reference to a `HashMap` where keys are original `Ident`s and values
    ///   are the corresponding new `Ident`s to substitute.
    ///
    /// # Behavior
    ///
    /// - Only the variants of `DiagnosticKind` that contain `Ident`s or collections of `Ident`s
    ///   are affected.
    /// - Variants without `Ident`s or where remapping is not applicable remain unchanged.
    ///
    /// # Errors
    ///
    /// Returns an [`InternerError`] if:
    /// - A required mapping is missing (`InternerError::MissingIdent`), or
    /// - A remap would cause a conflict (`InternerError::Conflict`).
    ///
    /// # Example
    ///
    /// ```rust
    /// use std::collections::HashMap;
    /// use crate::aiplan4rust::interner::{Ident, InternerError, RemapIdents};
    ///
    /// let mut diagnostic = ...; // some DiagnosticKind instance
    /// let mut map: HashMap<Ident, Ident> = HashMap::new();
    /// map.insert(old_ident, new_ident);
    ///
    /// diagnostic.remap_idents(&map)?;
    /// ```
    fn remap_symbol(&mut self, map: &HashMap<SymbolId, SymbolId>) -> Result<(), InternerError>{
        match self {
            Kind::InvalidSymbolSignature { declaration, usage } => {
                declaration.remap_symbol(map)?;
                usage.remap_symbol(map)?;
            }
            Kind::TypeMismatchInExpression { ty1, ty2 }
            | Kind::InvalidTypesInNumericExpression { ty1, ty2 } => {
                ty1.remap_symbol(map)?;
                ty2.remap_symbol(map)?;
            }

            Kind::DuplicatedSymbolDeclarationInScope {
                original_declaration: declaration1,
                conflicting_declaration: declaration2,
                ..
            } | Kind::DuplicateVariableSkeletonDeclaration {
                original_declaration: declaration1,
                conflicting_declaration: declaration2,
                ..
            } => {
                // Ici, declaration1 et declaration2 sont utilisables car
                // elles sont garanties d'être liées peu importe la variante choisie.
                declaration1.remap_symbol(map)?;
                declaration2.remap_symbol(map)?;
            },

            Kind::UndeclaredSymbol { usage } => {
                usage.remap_symbol(map)?;
            }

            Kind::SymbolConflictsWithKeyword { declaration, .. }
            | Kind::SymbolDeclaredAmbiguouslyAsKeyword { declaration, .. }
            | Kind::UnusedSymbol { declaration } => {
                declaration.remap_symbol(map)?;
            }

            Kind::DomainProblemNameMismatch {
                domain_name,
                problem_name,
            } => {
                domain_name.remap_symbol(map)?;
                problem_name.remap_symbol(map)?;
            }
            Kind::AmbiguousTypePredicateSymbol { ty, predicate } => {
                ty.remap_symbol(map)?;
                predicate.remap_symbol(map)?;
            }
            Kind::TaskArgumentIsSupertypeOfDeclaration {
                argument,
                type_declared,
                type_used,
            } => {
                argument.remap_symbol(map)?;
                type_declared.remap_symbol(map)?;
                type_used.remap_symbol(map)?;
            }
            Kind::DuplicateEitherType { duplicate_types } => {
                for ident in duplicate_types {
                    ident.remap_idents(map)?;
                }
            }
            Kind::CyclicTypeDeclaration { cycle } => {
                for decl in cycle {
                    decl.remap_symbol(map)?;
                }
            }
            Kind::CrossConflictSymbolDeclaration {
                problem_declaration,
                conflicting_domain_declarations,
            } => {
                problem_declaration.remap_symbol(map)?;
                for decl in conflicting_domain_declarations {
                    decl.remap_symbol(map)?;
                }
            }
            Kind::IncompatibleTypeDeclarations {
                symbol,
                expected_types,
                found_types,
                ..
            } => {
                symbol.remap_idents(map)?;
                for ty in expected_types {
                    ty.remap_idents(map)?;
                }
                for ty in found_types {
                    ty.remap_idents(map)?;
                }
            }
            Kind::DuplicatedDeclaration {
                ty,
                duplicate_types,
                ..
            }
            => {
                ty.remap_idents(map)?;
                for ident in duplicate_types {
                    ident.remap_idents(map)?;
                }
            }
            Kind::UnexpectedToken { .. }
            | Kind::UnexpectedEof { .. }
            | Kind::InvalidToken
            | Kind::ExtraToken { .. }
            | Kind::InvalidNumber { .. }
            | Kind::DuplicateDefinitionBlock { .. }
            | Kind::InvalidDefinitionBlockOrder { .. }
            | Kind::RequirementViolation { .. }
            | Kind::CyclicTaskOrdering
            | Kind::DuplicateRequirementWarning { .. }
            | Kind::CustomError { .. }
            | Kind::CustomWarning { .. } => {
                // No remap needed
            }
        }
        Ok(())
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
        let message = renderer::message::format_message_debug(self);
        let severity = self.severity();
        let suggestion = renderer::suggestion::format_suggestion_debug(self);

        match suggestion {
            Some(sugg) => write!(
                f,
                "[{}] ({}) {}. Suggestion: {}",
                code, severity, message, sugg
            ),
            None => write!(f, "[{}] ({}) {}", code, severity, message),
        }
    }
}
