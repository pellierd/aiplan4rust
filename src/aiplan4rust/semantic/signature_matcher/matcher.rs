//! Signature Matching and Validation Engine
//!
//! This module provides the core logic for verifying that symbol usages in the AST
//! align with their respective declarations. It is a critical part of the semantic
//! analysis phase, ensuring that the planning model is structurally and
//! semantically sound.
//!
//! ## Overview
//! The validation process follows a strict "Gatekeeper" pattern:
//! 1. **Structural Validation**: Ensuring identity (SymbolId), category (SymbolKind),
//!    and arity (argument count) match.
//! 2. **Symbol Resolution**: Locating declarations using a tiered lookup (Local Problem
//!    context first, then Global Domain context).
//! 3. **Semantic Validation**: Performing deep type-checking, including support for
//!    standard subtyping and Bercher-style upcasting.
//!
//! ## Key Components
//! * [`SignatureMatcher`]: The primary orchestrator that holds references to symbol
//!   tables and the type engine.
//! * [`MatchResult`]: An enum capturing the outcome of a match (Exact, Upcast, or Mismatch).
//! * [`MatchFailure`]: Detailed diagnostic information used when a signature
//!   does not conform.
//!
//! ## Safety and Precision
//! This module is designed for "Heavy Duty" use, prioritizing robust error handling
//! over panics. It uses safe extraction methods (`try_*`) to handle potentially
//! incomplete or malformed AST nodes during the resolution process.

use crate::aiplan4rust::semantic::rules::{
    allow_implicit_upcast_for_task_matching, check_kind_compatibility, find_shadowing_candidate,
};
use crate::aiplan4rust::semantic::signature_matcher::error::SignatureMatcherError;
use crate::aiplan4rust::semantic::signature_matcher::failure::MatchFailure;
use crate::aiplan4rust::semantic::signature_matcher::result::MatchResult;
use crate::aiplan4rust::semantic::symbol::{Declaration, Scope, SymbolKind, Usage};
use crate::aiplan4rust::semantic::TypeChecker;
use crate::aiplan4rust::syntax::ast::AstNode;
use crate::aiplan4rust::tree::{NodeId, Tree};
use crate::SymbolTable;

/// A high-level semantic analyzer responsible for validating symbol signatures
/// against their declarations and call sites.
///
/// The `SignatureMatcher` acts as the primary gateway for ensuring that symbol
/// usages (e.g., action calls, predicate applications) conform to their defined
/// structural and semantic constraints.
///
/// It supports a two-tier symbol resolution strategy, looking up identifiers
/// in both local (Problem) and global (Domain) contexts.
pub struct SignatureMatcher<'a> {
    /// Reference to the local symbol table, typically containing objects,
    /// variables, and parameters specific to the current problem or scope.
    local_table: &'a SymbolTable,

    /// The Abstract Syntax Tree (AST) being analyzed, providing the structural
    /// source of truth for the usage sites.
    ast: &'a Tree<AstNode>,

    /// The semantic engine used to perform type compatibility checks and
    /// validate subtyping/upcasting relationships.
    type_checker: &'a TypeChecker<'a>,

    /// An optional reference to the global domain table. If present, it serves
    /// as a fallback for resolving constants, types, and global definitions.
    domain_table: Option<&'a SymbolTable>,
}

impl<'a> SignatureMatcher<'a> {
    /// Creates a new `SignatureMatcher` instance with the required semantic context.
    ///
    /// This constructor initializes the matcher by anchoring it to the problem's
    /// local symbols and the mandatory technical components (AST and TypeChecker).
    ///
    /// # Arguments
    /// * `local_table` - The primary symbol table for the current context (e.g., the Problem file).
    /// * `ast` - The syntax tree used to navigate usage sites and retrieve identifiers.
    /// * `type_checker` - The core engine for validating type hierarchies and compatibility.
    /// * `domain_table` - An optional global reference (the Domain file) for cross-file
    ///   constant and type resolution.
    ///
    /// # Returns
    /// A ready-to-use `SignatureChecker` for performing structural and semantic validation.
    pub fn new(
        local_table: &'a SymbolTable,
        ast: &'a Tree<AstNode>,
        type_checker: &'a TypeChecker<'a>,
        domain_table: Option<&'a SymbolTable>,
    ) -> Self {
        Self {
            local_table,
            ast,
            type_checker,
            domain_table,
        }
    }

    /// Orchestrates the full matching process between a symbol declaration and a usage site.
    ///
    /// The process follows three main phases:
    /// 1. **Structural Check**: Verifies identity, kind compatibility, and arity.
    /// 2. **Argument Resolution**: Locates the declaration for each provided argument
    ///    using the tiered lookup (Local then Domain).
    /// 3. **Semantic Validation**: Validates each argument's type, choosing between
    ///    standard matching or Holler upcasting task.
    ///
    /// # Returns
    /// * `Ok(MatchResult::Match)` - If all arguments match exactly.
    /// * `Ok(MatchResult::UpcastMatch)` - If at least one argument required an upcast.
    /// * `Ok(MatchResult::NoMatch)` - If any structural or semantic mismatch is found.
    ///
    /// # Errors
    /// * Returns [`SignatureMatcherError`] if an argument cannot be resolved or
    ///   if the AST is malformed.
    pub fn match_signature(
        &self,
        declaration: &'a Declaration,
        usage: &'a Usage,
    ) -> Result<MatchResult, SignatureMatcherError> {
        // 1. Structural Verification (Names, Kinds, and Arity)
        let arguments = match Self::match_structure(declaration, usage) {
            Ok(args) => args,
            Err(failure) => return Ok(MatchResult::NoMatch(failure)),
        };

        // Initialize the global result at the highest success level (Match)
        let mut current_global_result = MatchResult::Match;

        // 2. Argument Content Validation
        for (index, &arg_id) in arguments.iter().enumerate() {
            let arg_node = self.ast.try_node(arg_id)?;

            // Hierarchical Resolution (Local Problem -> Global Domain)
            let arg_decl = self
                .resolve_argument_declaration(arg_node, usage.scope())?
                .ok_or_else(|| SignatureMatcherError::unresolved_argument(arg_id))?;

            // 3. Binary Branching: Selection of matching logic based on task-matching rules
            let match_res = if allow_implicit_upcast_for_task_matching(
                declaration.symbol().kind(),
                usage.symbol().kind(),
            ) {
                self.match_argument_with_upcasting(declaration, arg_decl, arg_id, index)?
            } else {
                self.match_argument(declaration, arg_decl, index)?
            };

            match match_res {
                MatchResult::NoMatch(failure) => {
                    // Early exit: propagate the specific failure to avoid losing diagnostic info
                    return Ok(MatchResult::NoMatch(failure));
                }
                MatchResult::UpcastMatch { .. } => {
                    // If we were at Match level, downgrade to UpcastMatch
                    if matches!(current_global_result, MatchResult::Match) {
                        current_global_result = match_res;
                    }
                }
                MatchResult::Match => {
                    // Standard match: continue to next argument
                }
            }
        }

        Ok(current_global_result)
    }

    /// Resolves an argument's declaration by searching first in the Local (Problem) table,
    /// then falling back to the Domain (Global) table if necessary.
    ///
    /// This implements a tiered lookup where local definitions (like problem objects or
    /// action parameters) shadow global domain constants.
    ///
    /// # Arguments
    /// * `argument_node` - The AST node representing the argument usage.
    /// * `usage_scope` - The current scope where the argument is being used.
    ///
    /// # Returns
    /// * `Ok(Some(&Declaration))` - If a matching declaration is found in either table.
    /// * `Ok(None)` - If the symbol exists but no declaration matches the kind/scope.
    ///
    /// # Errors
    /// * Returns [`SignatureMatcherError`] if the identifier cannot be retrieved
    ///   or if the `SymbolKind` is incompatible with the context.
    fn resolve_argument_declaration(
        &self,
        argument_node: &AstNode,
        usage_scope: &Scope,
    ) -> Result<Option<&'a Declaration>, SignatureMatcherError> {
        let name = argument_node.try_ident()?;
        let kind = SymbolKind::try_from(argument_node.kind())
            .map_err(|_| SignatureMatcherError::invalid_symbol_kind())?;

        // --- STEP 1: Recherche dans la table LOCALE (Fichier Problem) ---
        // On utilise la logique de shadowing pour prioriser les variables locales (?x)
        // sur les constantes globales du même nom.
        if let Some(entry) = self.local_table.get_symbol(name) {
            if let Some(decl) = find_shadowing_candidate(entry, kind, usage_scope) {
                return Ok(Some(decl));
            }
        }

        // --- STEP 2: Recherche dans la table DOMAINE (Fichier Global) ---
        // Si rien n'est trouvé en local, on regarde dans le domaine.
        // On résout contre le root_scope car tout ce qui est dans le domaine est global.
        if let Some(domain) = self.domain_table {
            if let Some(entry) = domain.get_symbol(name) {
                if let Some(decl) = find_shadowing_candidate(entry, kind, &domain.root_scope()) {
                    return Ok(Some(decl));
                }
            }
        }

        Ok(None)
    }

    /// Matches a specific provided argument against its expected type in a declaration.
    ///
    /// This function verifies that the argument provided at the usage site corresponds to
    /// the expected type defined in the declaration's signature, ensuring type consistency
    /// through subtype validation.
    ///
    /// # Arguments
    /// * `expected_decl` - The reference declaration (e.g., Action or Task definition).
    /// * `provided_decl` - The declaration of the supplied argument (e.g., Variable or Object).
    /// * `index` - The positional index of the argument in the argument list.
    ///
    /// # Returns
    /// * `Ok(MatchResult::Match)` - If the provided type is a valid subtype of the expected type.
    /// * `Ok(MatchResult::NoMatch)` - If a type mismatch occurs, containing failure details.
    ///
    /// # Errors
    /// * Returns [`SignatureMatcherError`] if type extraction fails or internal errors occur.
    fn match_argument(
        &self,
        expected_decl: &Declaration,
        provided_decl: &Declaration,
        index: usize,
    ) -> Result<MatchResult, SignatureMatcherError> {
        // 1. ty_expected: What the definition (action/predicate) requires at this index
        let ty_expected = expected_decl.try_get_arg_type(index)?;

        // 2. ty_provided: What the argument (object/variable) actually possesses
        let ty_provided = provided_decl.try_type()?;

        // 3. Standard subtyping verification via the semantic engine
        if self
            .type_checker
            .is_any_subtype_of(ty_expected, ty_provided)?
        {
            Ok(MatchResult::Match)
        } else {
            // Idiomatic approach: Capture and return the specific cause of the failure
            Ok(MatchResult::NoMatch(MatchFailure::Argument {
                index,
                expected: ty_expected.clone(),
                provided: ty_provided.clone(),
            }))
        }
    }

    /// Matches a provided argument against an expected declaration, supporting type upcasting.
    ///
    /// This function implements the "Holler" upcasting logic: if a standard match fails,
    /// it evaluates whether the provided type can be treated as a supertype of the expected type.
    ///
    /// # Arguments
    /// * `expected_decl` - The reference declaration (signature).
    /// * `provided_decl` - The declaration of the argument being passed.
    /// * `arg_node_id` - The AST node identifier for the argument at the usage site.
    /// * `index` - The positional index of the argument.
    ///
    /// # Returns
    /// * `Ok(MatchResult::Match)` - On direct type compatibility.
    /// * `Ok(MatchResult::UpcastMatch)` - If upcasting is valid and successful.
    /// * `Ok(standard_res)` - Returns the original mismatch if upcasting fails.
    pub fn match_argument_with_upcasting(
        &self,
        expected_decl: &Declaration,
        provided_decl: &Declaration,
        arg_node_id: NodeId,
        index: usize,
    ) -> Result<MatchResult, SignatureMatcherError> {
        // 1. REUSE: Call the base matching function
        let standard_res = self.match_argument(expected_decl, provided_decl, index)?;

        // If it is a direct Match, return immediately
        if matches!(standard_res, MatchResult::Match) {
            return Ok(standard_res);
        }

        // 2. Upcasting (Holler stupidity)
        // Safe because match_argument has already validated the existence of arguments and types
        let ty_expected = expected_decl.arguments().unwrap().get(index).unwrap().ty();
        let ty_provided = provided_decl.ty().unwrap();

        if self
            .type_checker
            .is_any_supertype_of(ty_expected, ty_provided)?
        {
            return Ok(MatchResult::UpcastMatch {
                expected: ty_expected.clone(),
                provided: ty_provided.clone(),
                arg_decl: provided_decl.clone(),
                arg_node_id,
            });
        }

        // 3. If upcasting fails, return the original result (stored in standard_res)
        // This is cleaner than manually reconstructing a MatchFailure
        Ok(standard_res)
    }

    /// Matches a symbol usage against a declaration based on identity, kind compatibility, and arity.
    ///
    /// This is the "fast-path" of signature validation. It ensures that the basic structural
    /// requirements are met before performing more expensive type-checking on individual arguments.
    ///
    /// # Business Rules
    /// 1. **Identity**: The symbols must share the same [`SymbolId`].
    /// 2. **Kind Compatibility**: Uses [`check_kind_compatibility`] to allow cross-kind
    ///    matching (e.g., between a `Task` and an `Action`).
    /// 3. **Arity**: The number of provided arguments must exactly match the number of
    ///    parameters defined in the declaration.
    ///
    /// # Arguments
    /// * `declaration` - The symbol declaration acting as the reference.
    /// * `usage` - The specific occurrence or call site of the symbol.
    ///
    /// # Returns
    /// * `Ok(&[NodeId])` - A slice containing the argument IDs from the usage site if
    ///   the structure is compatible.
    ///
    /// # Errors
    /// * Returns a [`MatchFailure`] if there is a mismatch in identity ([`MatchFailure::Symbol`]),
    ///   kind ([`MatchFailure::KindMismatch`]), or argument count ([`MatchFailure::Arity`]).
    pub fn match_structure(
        declaration: &'a Declaration,
        usage: &'a Usage,
    ) -> Result<&'a [NodeId], MatchFailure> {
        // 1. IDENTITY CHECK (Symbol ID verification)
        if declaration.symbol().id() != usage.symbol().id() {
            return Err(MatchFailure::Symbol {
                expected: declaration.symbol().id(),
                observed: usage.symbol().id(),
            });
        }

        // 2. KIND COMPATIBILITY (Business Rule validation: Task, Action, etc.)
        if !check_kind_compatibility(declaration.symbol().kind(), usage.symbol().kind()) {
            return Err(MatchFailure::KindMismatch);
        }

        // 3. STRUCTURAL VERIFICATION (Arity/Argument count check)
        let decl_args = declaration.argument_sources();
        let usage_args = usage.argument_sources();

        match (decl_args, usage_args) {
            // Match found with arguments: both lists must have identical lengths.
            (Some(d), Some(u)) if d.len() == u.len() => Ok(u),

            // Match found without arguments: both are empty/null; returns an empty slice.
            (None, None) => Ok(&[]),

            // Arity mismatch handling: provides detailed length comparison for error reporting.
            (Some(d), Some(u)) => Err(MatchFailure::Arity {
                expected: d.len(),
                observed: u.len(),
            }),
            (Some(d), None) => Err(MatchFailure::Arity {
                expected: d.len(),
                observed: 0,
            }),
            (None, Some(u)) => Err(MatchFailure::Arity {
                expected: 0,
                observed: u.len(),
            }),
        }
    }
}
