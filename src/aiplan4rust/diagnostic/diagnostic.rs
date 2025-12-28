//! Diagnostic system for parsing, semantic analysis, and compilation phases.
//!
//! This module provides a structured framework for reporting and managing diagnostics
//! such as errors, warnings, and informational messages that occur during the
//! parsing, validation, and compilation of AI planning files.
//!
//! # Overview
//!
//! The diagnostic system is organized into several components:
//!
//! - [`diagnostic`] defines the common `Diagnostic` type and its conversion from parsing errors.
//! - [`kind`] contains a rich set of `Kind` variants to describe different types of issues.
//! - [`severity`] categorizes diagnostics by severity (e.g., error, warning).
//! - [`diagnostic_manager`] manages a collection of diagnostics and associated source files.
//! - [`provider`] distinguishes the origin of a diagnostic (e.g., domain or problem file).
//! - [`renderer`] contains submodules for formatting diagnostics for display, including:
//!   - [`message`] for message construction,
//!   - [`suggestion`] for auto-fix or guidance,
//!   - [`formatting`] utilities to convert internal data to readable strings,
//!   - [`renderer`] for rendering full diagnostics in user-facing form.
//!
//! # Key Concepts
//!
//! - Diagnostics are created during parsing, semantic validation, or symbol resolution.
//! - Each diagnostic carries metadata such as file origin (`Literal`), source span, and severity.
//! - Formatting modules ensure messages are user-friendly and contextual.
//!
//! # Usage Example
//!
//! ```rust
//! use aiplan4rust::diagnostic::{Diagnostic, DiagnosticKind, Provider};
//! use aiplan4rust::interner::StringInterner;
//!
//! let kind = DiagnosticKind::InvalidToken;
//! let provider = Provider::Parser;
//! let source = interner.intern_literal("domain.pddl");
//! let span = Span::new(0, 5);
//! let diagnostic = Diagnostic::new(kind, provider, source, span);
//! ```
//!
//! This system enables consistent error reporting across the parsing and compilation pipeline.

use crate::aiplan4rust::diagnostic::kind::Kind;
use crate::aiplan4rust::diagnostic::{DiagnosticKind, Provider};
use crate::aiplan4rust::interner::{Ident, Literal};
use crate::aiplan4rust::syntax::lexer::{LexicalError, Token};
use crate::aiplan4rust::syntax::{FastLineTable, Span};

use std::collections::HashMap;
use std::fmt;
use lalrpop_util::ParseError;
use crate::aiplan4rust::lang::{Requirement, Type};
use crate::aiplan4rust::semantic::symbol::{Declaration, Symbol, SymbolKind, Usage};
use crate::aiplan4rust::syntax::ast::AstKind;

/// Represents a diagnostic message generated during parsing, validation, or compilation.
///
/// A `Diagnostic` describes an issue detected in the source code. It includes metadata
/// about the nature of the issue (`kind`), its origin (`provider`), the file in which
/// it occurred (`source`), and the specific location (`span`) for accurate reporting.
#[derive(Clone, Debug, PartialEq)]
pub struct Diagnostic {
    /// The specific type of diagnostic, such as a syntax error, type mismatch, or unused symbol.
    ///
    /// This defines what kind of issue was detected.
    pub kind: Kind,

    /// Identifies the origin of the diagnostic (e.g., from the domain file, problem file, or unknown).
    ///
    /// Helps distinguish between different inputs or compilation units.
    pub provider: Provider,

    /// An interned identifier representing the source file where the issue occurred.
    ///
    /// This is typically obtained via a `StringInterner` and refers to a file path or logical source name.
    pub source: Literal,

    /// The span within the source file that pinpoints the location of the issue.
    ///
    /// Used for error highlighting and precise reporting (line and column numbers).
    pub span: Span,
}

impl Diagnostic {
    /// Creates a new `Diagnostic` instance containing information about a detected issue.
    ///
    /// # Arguments
    ///
    /// * `kind` - Describes the type of diagnostic (e.g., syntax error, type mismatch).
    /// * `provider` - Indicates the source or subsystem that generated the diagnostic (e.g., Domain, Problem, Parser).
    /// * `source` - A `Literal` identifying the source file where the issue occurred (via the interner).
    /// * `span` - The precise location in the file where the issue is found.
    ///
    /// # Returns
    ///
    /// A new `Diagnostic` ready to be added to the diagnostic manager or displayed.
    fn new(
        kind: Kind,
        provider: Provider,
        source: Literal,
        span: Span,
    ) -> Self {
        Diagnostic {
            kind,
            provider,
            source,
            span,
        }
    }

    /// Returns a reference to the kind of diagnostic.
    ///
    /// This includes the structured variant describing the nature of the issue
    /// (e.g., `UnexpectedToken`, `TypeMismatch`, etc.).
    pub fn kind(&self) -> &Kind {
        &self.kind
    }

    /// Returns the provider that reported this diagnostic.
    ///
    /// This indicates the origin of the error, such as the domain file, problem file,
    /// or parser infrastructure.
    pub fn provider(&self) -> &Provider {
        &self.provider
    }

    /// Returns the identifier of the source file where this diagnostic occurred.
    ///
    /// The identifier is a `Literal`, typically interned to reduce duplication.
    pub fn source(&self) -> Literal {
        self.source
    }

    /// Returns a reference to the span where the issue was detected.
    ///
    /// The span contains the start and end positions, typically line and column,
    /// allowing precise highlighting or error tracking.
    pub fn span(&self) -> &Span {
        &self.span
    }

    /// Updates the diagnostic kind with a new value.
    ///
    /// This allows changing the classification or message content of the diagnostic.
    pub fn set_kind(&mut self, kind: Kind) {
        self.kind = kind;
    }

    /// Updates the diagnostic's provider (e.g., Domain, Problem).
    pub fn set_provider(&mut self, provider: Provider) {
        self.provider = provider;
    }

    /// Updates the source file associated with this diagnostic.
    ///
    /// The new value must be a valid `Literal` reference to an interned filename.
    pub fn set_source(&mut self, source: Literal) {
        self.source = source;
    }

    /// Updates the span indicating where this diagnostic applies.
    pub fn set_span(&mut self, span: Span) {
        self.span = span;
    }

    /// Remaps all [`Ident`] and [`Literal`] values inside this [`Diagnostic`] using provided mappings.
    ///
    /// This method is typically used after **linking** or **merging** multiple source files
    /// (e.g., domain and problem files) where interner identifiers may differ but refer to the same
    /// logical symbols. It ensures that all identifiers and source references within the diagnostic
    /// are aligned with a unified, global representation.
    ///
    /// # Behavior
    ///
    /// - All [`Ident`] values nested within the diagnostic’s [`Kind`] are updated using `idents`.
    /// - The `source` field (a [`Literal`]) is remapped using `literals`, if a match is found.
    /// - Other fields (e.g., `span`, `provider`) remain unchanged.
    ///
    /// # Arguments
    ///
    /// * `idents` – A mapping from local to global [`Ident`] values.
    /// * `literals` – A mapping from local to global [`Literal`] values (e.g., filenames).
    ///
    /// # Example
    ///
    /// ```rust
    /// let mut diag = Diagnostic::new(kind, provider, source_literal, span);
    ///
    /// let mut id_map = HashMap::new();
    /// id_map.insert(local_id, global_id);
    ///
    /// let mut lit_map = HashMap::new();
    /// lit_map.insert(local_file, global_file);
    ///
    /// diag.remap(&id_map, &lit_map);
    /// ```
    ///
    /// # Notes
    ///
    /// - If an [`Ident`] or [`Literal`] is not present in the mapping, its original value is retained.
    /// - This method is **panic-free** and does not require ownership of the mappings.
    /// - The `source` field is updated in place using [`Literal::remap_literal`].
    ///
    /// [`Diagnostic`]: crate::diagnostics::Diagnostic
    /// [`Kind`]: crate::diagnostics::Kind
    /// [`Ident`]: crate::interner::Ident
    /// [`Literal`]: crate::interner::Literal
    /// [`Literal::remap_literal`]: crate::interner::Literal::remap_literal
    pub fn remap(&mut self, idents: &HashMap<Ident, Ident>, literals: &HashMap<Literal, Literal>) {
        self.kind.remap_idents(idents);
        self.source.remap_literal(literals);
    }

    /// Returns a unique diagnostic code string composed of:
    /// - A letter for severity (E, W, I, H)
    /// - A digit for the provider (0-5)
    /// - A two-digit code unique to the kind of diagnostic
    ///
    /// Example: "E01XX" means Error (E), Normalizer (1), code XX for the specific kind.
    pub fn code(&self) -> String {
        // Severity code, e.g. "E"
        let severity_code = self.kind.severity().code();

        // Provider code, e.g. '0'
        let provider_code = self.provider.code();

        // Kind-specific two-character code, e.g. "01"
        // Assume Kind::code() returns &'static str with 2 digits like "01", "05", etc.
        let kind_code = self.kind.code();

        format!("{}{}{}", severity_code, provider_code, kind_code)
    }
}

impl Diagnostic {
    /// Constructs a diagnostic for an unexpected token encountered during parsing.
    ///
    /// # Arguments
    /// - `token`: The unexpected token found.
    /// - `expected`: A list of expected tokens.
    /// - `provider`: The origin of the diagnostic (e.g., Parser, Validator).
    /// - `source`: Interned identifier for the source where the error occurred.
    /// - `span`: The location in the source where the error occurred.
    pub fn error_unexpected_token(
        token: impl Into<String>,
        expected: Vec<impl Into<String>>,
        provider: Provider,
        source: Literal,
        span: Span,
    ) -> Self {
        let token = token.into();
        let expected: Vec<String> = expected.into_iter().map(|s| s.into()).collect();

        Self {
            kind: Kind::UnexpectedToken { token, expected },
            provider,
            source,
            span,
        }
    }

    /// Constructs a diagnostic for an unexpected end-of-file encountered during parsing.
    ///
    /// # Arguments
    /// - `expected`: A list of tokens that were expected before EOF.
    /// - `provider`: The origin of the diagnostic.
    /// - `source`: Interned identifier for the source.
    /// - `span`: The location in the source where the EOF was reached.
    pub fn error_unexpected_eof(
        expected: Vec<impl Into<String>>,
        provider: Provider,
        source: Literal,
        span: Span,
    ) -> Self {
        let expected: Vec<String> = expected.into_iter().map(|s| s.into()).collect();

        Self {
            kind: Kind::UnexpectedEof { expected },
            provider,
            source,
            span,
        }
    }

    /// Constructs a diagnostic for an invalid token.
    ///
    /// # Arguments
    /// - `provider`: The origin of the diagnostic.
    /// - `source`: Interned identifier for the source.
    /// - `span`: The location in the source of the invalid token.
    pub fn error_invalid_token(
        provider: Provider,
        source: Literal,
        span: Span,
    ) -> Self {
        Self {
            kind: Kind::InvalidToken,
            provider,
            source,
            span,
        }
    }

    /// Creates a diagnostic for an unexpected extra token.
    ///
    /// # Arguments
    ///
    /// * `token` - The unexpected token string encountered.
    /// * `provider` - The source of this diagnostic.
    /// * `source` - The interned identifier of the source file.
    /// * `span` - The span where the extra token was found.
    pub fn error_extra_token(token: impl Into<String>, provider: Provider, source: Literal, span: Span) -> Self {
        Self {
            kind: Kind::ExtraToken { token: token.into() },
            provider,
            source,
            span,
        }
    }

    /// Creates a diagnostic for a user-defined parse error with a custom message.
    ///
    /// # Arguments
    ///
    /// * `message` - A descriptive error message from the parser.
    /// * `provider` - The source of this diagnostic.
    /// * `source` - The interned identifier of the source file.
    /// * `span` - The span related to this error (can be empty if unknown).
    pub fn error_user(message: impl Into<String>, provider: Provider, source: Literal, span: Span) -> Self {
        Self {
            kind: Kind::User { message: message.into() },
            provider,
            source,
            span,
        }
    }

    /// Creates a diagnostic for an invalid symbol signature usage.
    ///
    /// # Arguments
    ///
    /// * `declaration` - The symbol declaration that was mismatched.
    /// * `usage` - The symbol usage signature causing the error.
    /// * `provider` - The source of this diagnostic.
    /// * `source` - The interned identifier of the source file.
    /// * `span` - The span where the signature mismatch was detected.
    pub fn error_invalid_symbol_signature(
        declaration: Declaration,
        usage: Usage,
        provider: Provider,
        source: Literal,
        span: Span,
    ) -> Self {
        Self {
            kind: Kind::InvalidSymbolSignature { declaration, usage },
            provider,
            source,
            span,
        }
    }

    /// Constructs a diagnostic for a type mismatch error in an expression.
    ///
    /// # Arguments
    /// - `ty1`: The first type involved in the mismatch.
    /// - `ty2`: The second type involved in the mismatch.
    /// - `provider`: The origin of the diagnostic.
    /// - `source`: Interned identifier for the source.
    /// - `span`: The location in the source of the mismatch.
    pub fn error_type_mismatch_in_expression(
        ty1: Type,
        ty2: Type,
        provider: Provider,
        source: Literal,
        span: Span,
    ) -> Self {
        Self {
            kind: Kind::TypeMismatchInExpression { ty1, ty2 },
            provider,
            source,
            span,
        }
    }

    /// Constructs a diagnostic for invalid types used in a numeric expression.
    ///
    /// # Arguments
    /// - `ty1`: The first invalid type.
    /// - `ty2`: The second invalid type.
    /// - `provider`: The origin of the diagnostic.
    /// - `source`: Interned identifier for the source.
    /// - `span`: The location in the source of the invalid types.
    pub fn error_invalid_types_in_numeric_expression(
        ty1: Type,
        ty2: Type,
        provider: Provider,
        source: Literal,
        span: Span,
    ) -> Self {
        Self {
            kind: Kind::InvalidTypesInNumericExpression { ty1, ty2 },
            provider,
            source,
            span,
        }
    }

    /// Constructs a warning diagnostic for a PDDL requirement violation.
    ///
    /// # Arguments
    /// - `node_kind`: The kind of AST node triggering the violation.
    /// - `required`: The list of missing requirements needed.
    /// - `provider`: The origin of the diagnostic.
    /// - `source`: Interned identifier for the source.
    /// - `span`: The location in the source where the violation occurs.
    pub fn warning_missing_requirement(
        node_kind: AstKind,
        required: Vec<Requirement>,
        provider: Provider,
        source: Literal,
        span: Span,
    ) -> Self {
        Self {
            kind: Kind::RequirementViolation { node_kind, required },
            provider,
            source,
            span,
        }
    }

    /// Constructs a diagnostic for duplicated symbol declaration within the same scope.
    ///
    /// # Arguments
    /// - `symbol`: The duplicated symbol.
    /// - `original_declaration`: The first declaration of the symbol.
    /// - `conflicting_declaration`: The second conflicting declaration.
    /// - `scope`: The AST node kind defining the scope of duplication.
    /// - `provider`: The origin of the diagnostic.
    /// - `source`: Interned identifier for the source.
    /// - `span`: The location in the source of the duplication error.
    pub fn error_duplicated_symbol_declaration_in_scope(
        symbol: Symbol,
        original_declaration: Declaration,
        conflicting_declaration: Declaration,
        scope: AstKind,
        provider: Provider,
        source: Literal,
        span: Span,
    ) -> Self {
        Self {
            kind: Kind::DuplicatedSymbolDeclarationInScope {
                symbol,
                original_declaration,
                conflicting_declaration,
                scope,
            },
            provider,
            source,
            span,
        }
    }

    /// Constructs a diagnostic for a cyclic task ordering error.
    ///
    /// This indicates that task ordering constraints form a cycle,
    /// making the ordering invalid or unsatisfiable.
    ///
    /// # Note
    /// Enhancing this error to include the detected cycle would improve usability.
    ///
    /// # Arguments
    /// - `provider`: The origin of the diagnostic.
    /// - `source`: Interned identifier for the source.
    /// - `span`: The location in the source related to the cycle error.
    pub fn error_cyclic_task_ordering(
        provider: Provider,
        source: Literal,
        span: Span,
    ) -> Self {
        Self {
            kind: Kind::CyclicTaskOrdering,
            provider,
            source,
            span,
        }
    }

    /// Constructs a diagnostic for usage of an undeclared symbol.
    ///
    /// This error occurs when a symbol is referenced without prior declaration.
    ///
    /// # Arguments
    /// - `usage`: Information about the undeclared symbol usage.
    /// - `provider`: The origin of the diagnostic.
    /// - `source`: Interned identifier for the source.
    /// - `span`: The location in the source where the usage occurs.
    pub fn error_undeclared_symbol(
        usage: Usage,
        provider: Provider,
        source: Literal,
        span: Span,
    ) -> Self {
        Self {
            kind: Kind::UndeclaredSymbol { usage },
            provider,
            source,
            span,
        }
    }

    /// Constructs a diagnostic for a symbol conflicting with a reserved PDDL keyword.
    ///
    /// This error arises when a user-defined symbol conflicts with a reserved keyword,
    /// based on active requirements.
    ///
    /// # Arguments
    /// - `declaration`: The conflicting user declaration.
    /// - `expected_kind`: The expected symbol kind that causes the conflict.
    /// - `requirements`: The list of PDDL requirements making this identifier reserved.
    /// - `provider`: The origin of the diagnostic.
    /// - `source`: Interned identifier for the source.
    /// - `span`: The location in the source of the conflict.
    pub fn error_symbol_conflicts_with_keyword(
        declaration: Declaration,
        expected_kind: SymbolKind,
        requirements: Vec<Requirement>,
        provider: Provider,
        source: Literal,
        span: Span,
    ) -> Self {
        Self {
            kind: Kind::SymbolConflictsWithKeyword {
                declaration,
                expected_kind,
                requirements,
            },
            provider,
            source,
            span,
        }
    }

    /// Constructs a warning for a symbol declared ambiguously as a reserved keyword.
    ///
    /// This warning indicates that a symbol overlaps with a reserved PDDL keyword
    /// but matches the expected kind given current requirements.
    ///
    /// # Arguments
    /// - `declaration`: The symbol declaration.
    /// - `expected_kind`: The expected `SymbolKind` for the reserved keyword.
    /// - `requirements`: List of domain requirements related to this usage.
    /// - `provider`: Origin of the diagnostic.
    /// - `source`: Interned source identifier.
    /// - `span`: Location in source related to this declaration.
    pub fn warning_symbol_declared_ambiguously_as_keyword(
        declaration: Declaration,
        expected_kind: SymbolKind,
        requirements: Vec<Requirement>,
        provider: Provider,
        source: Literal,
        span: Span,
    ) -> Self {
        Self {
            kind: Kind::SymbolDeclaredAmbiguouslyAsKeyword {
                declaration,
                expected_kind,
                requirements,
            },
            provider,
            source,
            span,
        }
    }

    /// Constructs a warning or error for an unused symbol declaration.
    ///
    /// Useful to detect dead code or unnecessary declarations.
    ///
    /// # Arguments
    /// - `declaration`: The unused symbol's declaration.
    /// - `provider`: Origin of the diagnostic.
    /// - `source`: Interned source identifier.
    /// - `span`: Location in source related to this declaration.
    pub fn warning_unused_symbol(
        declaration: Declaration,
        provider: Provider,
        source: Literal,
        span: Span,
    ) -> Self {
        Self {
            kind: Kind::UnusedSymbol { declaration },
            provider,
            source,
            span,
        }
    }

    /// Constructs an error for mismatched domain and problem names.
    ///
    /// Indicates semantic inconsistency between the domain and problem files.
    ///
    /// # Arguments
    /// - `domain_name`: Declaration of the domain name in the domain file.
    /// - `problem_name`: Declaration of the domain name in the problem file.
    /// - `provider`: Origin of the diagnostic.
    /// - `source`: Interned source identifier.
    /// - `span`: Location in source related to the problem file domain name.
    pub fn warning_domain_problem_name_mismatch(
        domain_name: Declaration,
        problem_name: Declaration,
        provider: Provider,
        source: Literal,
        span: Span,
    ) -> Self {
        Self {
            kind: Kind::DomainProblemNameMismatch {
                domain_name,
                problem_name,
            },
            provider,
            source,
            span,
        }
    }

    /// Constructs a warning for ambiguous symbol declared as both type and predicate.
    ///
    /// Indicates potential semantic confusion when the same name is used for both.
    ///
    /// # Arguments
    /// - `ty`: Declaration of the symbol as a primitive type.
    /// - `predicate`: Declaration of the symbol as a predicate.
    /// - `provider`: Origin of the diagnostic.
    /// - `source`: Interned source identifier.
    /// - `span`: Location in source related to the symbol declarations.
    pub fn warning_ambiguous_type_predicate_symbol(
        ty: Declaration,
        predicate: Declaration,
        provider: Provider,
        source: Literal,
        span: Span,
    ) -> Self {
        Self {
            kind: Kind::AmbiguousTypePredicateSymbol { ty, predicate },
            provider,
            source,
            span,
        }
    }

    /// Constructs a warning for a task argument that uses a supertype of the declared type.
    ///
    /// This warns about an argument type that is more general than the declaration,
    /// which violates PDDL typing rules but is allowed here with a warning for compatibility.
    ///
    /// # Arguments
    /// - `argument`: The argument's declaration in the action or method.
    /// - `type_declared`: The declared type of the argument.
    /// - `type_used`: The actual type used in the task invocation (a supertype).
    /// - `provider`: Origin of the diagnostic.
    /// - `source`: Interned source identifier.
    /// - `span`: Location in source related to the argument usage.
    pub fn warning_task_argument_is_supertype_of_declaration(
        argument: Declaration,
        type_declared: Type,
        type_used: Type,
        provider: Provider,
        source: Literal,
        span: Span,
    ) -> Self {
        Self {
            kind: Kind::TaskArgumentIsSupertypeOfDeclaration {
                argument,
                type_declared,
                type_used,
            },
            provider,
            source,
            span,
        }
    }

    /// Constructs a warning for duplicated types inside an `Either` construct.
    ///
    /// Only the identifiers of the duplicated types are provided, as full declarations
    /// are unavailable during normalization.
    ///
    /// # Arguments
    /// - `duplicate_types`: List of duplicated type identifiers.
    /// - `provider`: Origin of the diagnostic.
    /// - `source`: Interned source identifier.
    /// - `span`: Location in source related to the `Either` construct.
    pub fn warning_duplicate_either_type(
        duplicate_types: Vec<Ident>,
        provider: Provider,
        source: Literal,
        span: Span,
    ) -> Self {
        Self {
            kind: Kind::DuplicateEitherType { duplicate_types },
            provider,
            source,
            span,
        }
    }

    /// Constructs an error for cycles detected in the type declaration hierarchy.
    ///
    /// Indicates a circular type inheritance or extension preventing normalization.
    ///
    /// # Arguments
    /// - `cycle`: Vector of declarations forming the cycle.
    /// - `provider`: Origin of the diagnostic.
    /// - `source`: Interned source identifier.
    /// - `span`: Location in source related to the cycle detection.
    pub fn error_cyclic_type_declaration(
        cycle: Vec<Declaration>,
        provider: Provider,
        source: Literal,
        span: Span,
    ) -> Self {
        Self {
            kind: Kind::CyclicTypeDeclaration { cycle },
            provider,
            source,
            span,
        }
    }

    /// Constructs an error for conflicting symbol declarations between problem and domain.
    ///
    /// Used to report when symbols declared in the problem conflict with domain declarations.
    ///
    /// # Arguments
    /// - `problem_declaration`: Declaration from the problem context.
    /// - `conflicting_domain_declarations`: Conflicting declarations from the domain.
    /// - `provider`: Origin of the diagnostic.
    /// - `source`: Interned source identifier.
    /// - `span`: Location in source related to the conflict.
    pub fn error_cross_conflict_symbol_declaration(
        problem_declaration: Declaration,
        conflicting_domain_declarations: Vec<Declaration>,
        provider: Provider,
        source: Literal,
        span: Span,
    ) -> Self {
        Self {
            kind: Kind::CrossConflictSymbolDeclaration {
                problem_declaration,
                conflicting_domain_declarations,
            },
            provider,
            source,
            span,
        }
    }

    /// Constructs a warning for an implicit `(either ...)` type declaration caused by multiple conflicting parent types.
    ///
    /// # Arguments
    /// - `ty`: Identifier of the type being declared.
    /// - `duplicate_types`: Conflicting parent type identifiers merged implicitly.
    /// - `duplicate_spans`: Source code spans of each conflicting parent type declaration.
    /// - `provider`: Origin of the diagnostic.
    /// - `source`: Interned source identifier.
    /// - `span`: Location in source related to the implicit either type declaration.
    pub fn warning_implicit_either_type_declaration(
        ty: Ident,
        duplicate_types: Vec<Ident>,
        duplicate_spans: Vec<Span>,
        provider: Provider,
        source: Literal,
        span: Span,
    ) -> Self {
        Self {
            kind: Kind::ImplicitEitherTypeDeclaration {
                ty,
                duplicate_types,
                duplicate_spans,
            },
            provider,
            source,
            span,
        }
    }

    /// Constructs a warning for duplicated requirements.
    ///
    /// # Arguments
    /// - `duplicate_requirements`: List of duplicated requirements found.
    /// - `provider`: Origin of the diagnostic.
    /// - `source`: Interned source identifier.
    /// - `span`: Location in source related to the duplicate requirements.
    pub fn warning_duplicate_requirement(
        duplicate_requirements: Vec<Requirement>,
        provider: Provider,
        source: Literal,
        span: Span,
    ) -> Self {
        Self {
            kind: Kind::DuplicateRequirementWarning { duplicate_requirements },
            provider,
            source,
            span,
        }
    }

    /// Constructs a custom error with a message and optional suggestion.
    ///
    /// # Arguments
    /// - `message`: Description of the error.
    /// - `suggestion`: Optional suggestion to resolve or avoid the error.
    /// - `provider`: Origin of the diagnostic.
    /// - `source`: Interned source identifier.
    /// - `span`: Location in source related to the error.
    pub fn error_custom(
        message: String,
        suggestion: Option<String>,
        provider: Provider,
        source: Literal,
        span: Span,
    ) -> Self {
        Self {
            kind: Kind::CustomError { message, suggestion },
            provider,
            source,
            span,
        }
    }

    /// Constructs a custom warning with a message and optional suggestion.
    ///
    /// # Arguments
    /// - `message`: Description of the warning.
    /// - `suggestion`: Optional advice to mitigate or address the warning.
    /// - `provider`: Origin of the diagnostic.
    /// - `source`: Interned source identifier.
    /// - `span`: Location in source related to the warning.
    pub fn warning_custom(
        message: String,
        suggestion: Option<String>,
        provider: Provider,
        source: Literal,
        span: Span,
    ) -> Self {
        Self {
            kind: Kind::CustomWarning { message, suggestion },
            provider,
            source,
            span,
        }
    }
}

impl fmt::Display for Diagnostic {
    /// Provides a human-readable display of the diagnostic.
    ///
    /// Example output:
    /// `[Error] Domain at file.pddl:12:5`
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(
            f,
            "[{:?}] {} at {}:{}:{}",
            self.kind,
            self.provider,
            self.source,
            self.span.begin_line(),
            self.span.begin_column()
        )
    }
}

impl<'a> From<(&'a ParseError<usize, Token, LexicalError>, Literal, &'a FastLineTable)> for Diagnostic {
    /// Converts a LALRPOP `ParseError` into a structured `Diagnostic`, enriched with
    /// source span and interner-based file context.
    ///
    /// This implementation maps a `ParseError` (produced by the parser), along with
    /// a file identifier (`Literal`) and a `FastLineTable`, into a `Diagnostic`
    /// value suitable for reporting. It resolves parser-specific errors
    /// into diagnostic kinds and calculates source spans for accurate positioning.
    ///
    /// # Arguments
    ///
    /// The input is a tuple consisting of:
    /// - `&ParseError<usize, Token, LexicalError>`: the parse error to convert.
    /// - `Literal`: the interned identifier of the source file where the error occurred.
    /// - `&FastLineTable`: used to convert byte offsets into source code spans (line/column).
    ///
    /// # Returns
    ///
    /// A `Diagnostic` instance representing the error, with its kind, source location,
    /// and source file identifier.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let diagnostic = Diagnostic::from((&parse_error, file_id, &line_table));
    /// eprintln!("{}", diagnostic);
    /// ```
    ///
    /// # Notes
    ///
    /// - The `Literal` is not a file path but an interned handle to the file identifier.
    /// - For `User`-defined errors, a fallback empty span is used (position 0).
    /// - Expected token names are cleaned before being included in the message.
    fn from(
        value: (&'a ParseError<usize, Token, LexicalError>, Literal, &'a FastLineTable),
    ) -> Self {
        let (error, source, fast_line_table) = value;

        match error {
            // Handles unexpected tokens by including the token and expected set
            ParseError::UnrecognizedToken {
                token: (start, t, end),
                expected,
            } => {
                // Clean expected tokens by removing quotes for better message display
                Diagnostic::new(
                    DiagnosticKind::UnexpectedToken {
                        token: t.to_string(),
                        expected: expected.clone(),
                    },
                    Provider::Parser,
                    source,
                    // Calculate the span using FastLineTable for accurate error location
                    fast_line_table.get_span(*start, *end),
                )
            }
            // Handles invalid token errors at a specific location
            ParseError::InvalidToken { location } => {
                Diagnostic::new(
                    DiagnosticKind::InvalidToken,
                    Provider::Parser,
                    source,
                    fast_line_table.get_span(*location, *location),
                )
            }
            // Handles user-defined errors with arbitrary messages
            ParseError::User { error } => {
                let content = error.to_string();
                Diagnostic::new(
                    DiagnosticKind::User {
                        message: content,
                    },
                    Provider::Parser,
                    source,
                    // No span information available, use empty span (0,0)
                    fast_line_table.get_span(0, 0),
                )
            }
            // Handles unexpected EOF errors and lists expected tokens
            ParseError::UnrecognizedEof { location, expected } => {
                Diagnostic::new(
                    DiagnosticKind::UnexpectedEof { expected: expected.clone() },
                    Provider::Parser,
                    source,
                    fast_line_table.get_span(*location, *location),
                )
            }
            // Handles extra token errors, providing the token string
            ParseError::ExtraToken {
                token: (start, t, end),
            } => {
                Diagnostic::new(
                    DiagnosticKind::ExtraToken {
                        token: t.to_string(),
                    },
                    Provider::Parser,
                    source,
                    fast_line_table.get_span(*start, *end),
                )
            }
        }
    }
}
