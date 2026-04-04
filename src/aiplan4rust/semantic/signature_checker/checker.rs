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
//! * [`SignatureChecker`]: The primary orchestrator that holds references to symbol
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
use crate::aiplan4rust::semantic::signature_checker::error::SignatureMatcherError;
use crate::aiplan4rust::semantic::signature_checker::failure::MatchFailure;
use crate::aiplan4rust::semantic::signature_checker::result::MatchResult;
use crate::aiplan4rust::semantic::symbol::{Declaration, Scope, Signature, SymbolKind};
use crate::aiplan4rust::semantic::TypeChecker;
use crate::aiplan4rust::syntax::ast::AstNode;
use crate::aiplan4rust::tree::{NodeId, Tree};
use crate::SymbolTable;

/// A high-level semantic analyzer responsible for validating symbol signatures
/// against their declarations and call sites.
///
/// The `SignatureChecker` acts as the primary gateway for ensuring that symbol
/// usages (e.g., action calls, predicate applications) conform to their defined
/// structural and semantic constraints.
///
/// It supports a two-tier symbol resolution strategy, looking up identifiers
/// in both local (Problem) and global (Domain) contexts.
pub struct SignatureChecker<'a> {
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

impl<'a> SignatureChecker<'a> {
    /// Creates a new `SignatureChecker` instance with the required semantic context.
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
    /// The validation follows three strategic phases:
    /// 1. **Structural Check**: Verifies symbol identity, kind compatibility (e.g., Task vs Action), and arity.
    /// 2. **Argument Resolution**: Locates the semantic declaration for each argument using
    ///    a multi-stage lookup (supporting both direct declarations and bound usages).
    /// 3. **Semantic Validation**: Performs type-checking for each argument, automatically
    ///    switching between strict matching or Bercher-style upcasting based on the symbol kind.
    ///
    /// # Returns
    /// * `Ok(MatchResult::Match)` - All arguments match their expected types exactly.
    /// * `Ok(MatchResult::UpcastMatch)` - At least one argument required a valid type upcast.
    /// * `Ok(MatchResult::NoMatch)` - A structural or semantic mismatch was found (captured in `MatchFailure`).
    ///
    /// # Errors
    /// * Returns [`SignatureMatcherError`] if an argument node cannot be resolved or if the AST is inaccessible.
    pub fn match_signature(
        &self,
        expected: Signature<'a>,
        provided: Signature<'a>,
    ) -> Result<MatchResult, SignatureMatcherError> {
        // Phase 1: Structural Verification (Identity, Kind, and Arity)
        // We first ensure the call site matches the definition's basic signature shape.
        let arguments = match Self::match_structure(expected, provided) {
            Ok(args) => args,
            Err(failure) => {
                return Ok(MatchResult::NoMatch(failure));
            }
        };

        let mut current_global_result = MatchResult::Match;

        // Phase 2: Argument-level Semantic Validation
        // Iterate through each provided argument to verify its specific type and declaration.
        for (index, &arg_id) in arguments.iter().enumerate() {
            let arg_node = self.ast.try_node(arg_id)?;

            // Resolve the argument's declaration using the hybrid lookup strategy.
            // This handles standard variable usages as well as parameter definitions (#163).
            let arg_decl =
                match self.resolve_argument_declaration(arg_node, arg_id, provided.scope())? {
                    Some(decl) => decl,
                    None => {
                        return Err(SignatureMatcherError::unresolved_argument(arg_id));
                    }
                };

            // Wrap the resolved declaration into a Signature for uniform processing.
            let arg_signature = Signature::from_declaration(arg_decl);

            // Phase 3: Logic Selection (Standard vs. Upcasting)
            // Determine if the current context allows implicit upcasting (e.g., matching a Task call
            // against an Action definition).
            let can_upcast =
                allow_implicit_upcast_for_task_matching(expected.kind(), provided.kind());

            let match_res = if can_upcast {
                // Apply stupid Bercher/Holler upcasting logic for HTN task decomposition.
                self.match_argument_with_upcasting(expected, arg_signature, arg_id, index)?
            } else {
                // Apply strict subtyping for standard predicates or action effects.
                self.match_argument(expected, arg_signature, index)?
            };

            // Consolidate the result of the current argument into the global match status.
            match match_res {
                MatchResult::NoMatch(failure) => {
                    // Short-circuit on the first mismatch found.
                    return Ok(MatchResult::NoMatch(failure));
                }
                MatchResult::UpcastMatch { .. } => {
                    // Upgrade the global result to UpcastMatch if it was previously an exact Match.
                    if matches!(current_global_result, MatchResult::Match) {
                        current_global_result = match_res;
                    }
                }
                MatchResult::Match => {
                    // Exact matches do not change the global state.
                }
            }
        }

        // Final validation successful.
        Ok(current_global_result)
    }

    /// Resolves an argument's declaration using a multi-stage lookup strategy.
    ///
    /// This method ensures that symbol references (usages) are correctly mapped to their
    /// corresponding declarations. It handles both standard calls (where an ID points
    /// to a usage site) and structural definitions (where an ID might point directly
    /// to a declaration, such as in derived predicate headers).
    ///
    /// ### Resolution Strategy:
    /// 1. **Fast-track (Indexed)**: Checks if the `arg_node_id` is already a known `Declaration`
    ///    or a previously bound `Usage` in the local symbol entry.
    /// 2. **Local Lookup (Problem)**: Searches the local table using shadowing rules
    ///    (e.g., prioritizing action parameters over global constants) within the `usage_scope`.
    /// 3. **Global Lookup (Domain)**: Falls back to the domain table for global constants,
    ///    types, or functions.
    ///
    /// # Arguments
    /// * `argument_node` - The AST node representing the argument (variable, object, etc.).
    /// * `arg_node_id` - The unique identifier of the node being resolved (e.g., node #163).
    /// * `usage_scope` - The semantic scope context where the resolution is performed.
    ///
    /// # Returns
    /// * `Ok(Some(&Declaration))` - The resolved semantic declaration of the argument.
    /// * `Ok(None)` - If the symbol is found but no candidate matches the required kind/scope.
    ///
    /// # Errors
    /// * Returns [`SignatureMatcherError::UnresolvedArgument`] if resolution fails.
    /// * Returns [`SignatureMatcherError::InvalidSymbolKind`] if the AST node kind is unsupported.
    fn resolve_argument_declaration(
        &self,
        argument_node: &AstNode,
        arg_node_id: NodeId,
        usage_scope: &Scope,
    ) -> Result<Option<&'a Declaration>, SignatureMatcherError> {
        // Extract the symbol identifier (e.g., "?x" or "p") from the AST node.
        let symbol = argument_node.try_ident()?;

        // --- STEP 0: Fast-track resolution using already indexed data ---
        // Check if the local symbol table already has an entry for this identifier.
        if let Some(entry) = self.local_table.get_symbol(symbol) {
            // --- STEP 0.1: Direct Declaration Check (Case: Derived Predicate Headers) ---
            // If the provided NodeId is already a known Declaration within this symbol entry,
            // return it immediately. This handles cases where parameters are passed as definitions.
            if let Some(decl) = entry.declarations().get(&arg_node_id) {
                return Ok(Some(decl));
            }

            // --- STEP 0.2: Linked Usage Check (Case: Standard Actions/Calls) ---
            // If the NodeId is an Usage that was already bound to a Declaration during
            // the initial resolution phase, retrieve the pinned declaration directly.
            if let Some(usage) = entry.usages().get(&arg_node_id) {
                if let Some(decl_id) = usage.declaration() {
                    if let Some(decl) = entry.declarations().get(&decl_id) {
                        return Ok(Some(decl));
                    }
                }
            }
        }

        // Determine the expected symbol category (Variable, Object, etc.) from the AST node kind.
        let kind = SymbolKind::try_from(argument_node.kind())
            .map_err(|_| SignatureMatcherError::invalid_symbol_kind())?;

        // --- STEP 1: Local Scope Resolution (Problem File) ---
        // Perform a tiered lookup in the local table using shadowing rules.
        // This prioritizes local parameters (e.g., action variables) over global constants.
        if let Some(entry) = self.local_table.get_symbol(symbol) {
            if let Some(decl) = find_shadowing_candidate(entry, kind, usage_scope) {
                return Ok(Some(decl));
            }
        }

        // --- STEP 2: Global Scope Resolution (Domain File) ---
        // If not found locally, fallback to the Domain table.
        // Global symbols are resolved against the root scope as they have universal visibility.
        if let Some(domain) = self.domain_table {
            if let Some(entry) = domain.get_symbol(symbol) {
                if let Some(decl) = find_shadowing_candidate(entry, kind, &domain.root_scope()) {
                    return Ok(Some(decl));
                }
            }
        }

        // Return None if the symbol cannot be resolved in any available context.
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
        expected_decl: Signature<'_>,
        provided_decl: Signature<'_>,
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
    /// This function implements specialized HTN "Upcasting" logic (based on Bercher/Holler
    /// research). If a standard subtype match fails, it evaluates whether the provided
    /// type can be treated as a supertype of the expected type, which is a requirement
    /// for certain task-to-action decompositions.
    ///
    /// # Arguments
    /// * `expected` - The reference signature (the "contract" to fulfill).
    /// * `provided` - The signature of the actual argument being passed.
    /// * `arg_node_id` - The AST node identifier for the argument at the usage site.
    /// * `index` - The positional index of the argument in the parameter list.
    ///
    /// # Returns
    /// * `Ok(MatchResult::Match)` - On direct type compatibility (subtype).
    /// * `Ok(MatchResult::UpcastMatch)` - If upcasting is semantically valid.
    /// * `Ok(standard_res)` - Returns the original `NoMatch` if upcasting also fails.
    ///
    /// # Errors
    /// * Returns [`SignatureMatcherError`] if type metadata cannot be extracted from signatures.
    pub fn match_argument_with_upcasting(
        &self,
        expected: Signature<'_>,
        provided: Signature<'_>,
        arg_node_id: NodeId,
        index: usize,
    ) -> Result<MatchResult, SignatureMatcherError> {
        // 1. REUSE: Call the base matching function (standard subtyping check)
        let standard_res = self.match_argument(expected, provided, index)?;

        // Short-circuit: if we have an exact match or valid subtype, no further check is needed.
        if matches!(standard_res, MatchResult::Match) {
            return Ok(standard_res);
        }

        // 2. Upcasting Logic (Bercher/Holler Covariance)
        // Extract types using Signature helpers which safely encapsulate technical lookups.
        let ty_expected = expected.try_get_arg_type(index)?;
        let ty_provided = provided.try_type()?;

        // Verify if the provided type is a supertype of the expected one.
        if self
            .type_checker
            .is_any_supertype_of(ty_expected, ty_provided)?
        {
            return Ok(MatchResult::UpcastMatch {
                expected: ty_expected.clone(),
                provided: ty_provided.clone(),
                // Retrieve the actual declaration stored within the Signature.
                arg_decl: provided.declaration().clone(),
                arg_node_id,
            });
        }

        // 3. Fallback: If upcasting fails, return the original standard mismatch (NoMatch).
        Ok(standard_res)
    }

    /// Matches a symbol usage against a declaration based on identity, kind compatibility, and arity.
    ///
    /// This acts as the structural "gatekeeper" of the signature validation process. It ensures
    /// that the basic requirements are met before performing more expensive type-checking
    /// on individual arguments.
    ///
    /// ### Validation Rules:
    /// 1. **Identity**: Ensures both signatures refer to the same logical symbol ([`SymbolId`]).
    /// 2. **Kind Compatibility**: Validates if the call site kind is allowed to match the
    ///    declaration kind (e.g., allowing a Task to be refined by an Action).
    /// 3. **Arity**: Confirms the number of provided arguments exactly matches the
    ///    declaration's parameter count.
    ///
    /// # Arguments
    /// * `expected` - The reference signature from the symbol declaration.
    /// * `observed` - The signature extracted from the actual usage site.
    ///
    /// # Returns
    /// * `Ok(&[NodeId])` - A slice of argument IDs from the usage site if structurally compatible.
    ///
    /// # Errors
    /// * Returns [`MatchFailure`] detailing the specific structural mismatch found.
    pub fn match_structure(
        expected: Signature<'a>,
        observed: Signature<'a>,
    ) -> Result<&'a [NodeId], MatchFailure> {
        // 1. IDENTITY CHECK (Symbol ID)
        // Ensure both signatures refer to the same symbol identifier.
        if expected.id() != observed.id() {
            return Err(MatchFailure::Symbol {
                expected: expected.id(),
                observed: observed.id(),
            });
        }

        // 2. KIND COMPATIBILITY (e.g., Task vs Action)
        // Check if the usage's symbol kind is semantically compatible with the declaration.
        if !check_kind_compatibility(expected.kind(), observed.kind()) {
            return Err(MatchFailure::KindMismatch);
        }

        // 3. STRUCTURAL VERIFICATION (Arity / Argument Count)
        // Compare the argument lists provided by both signatures.
        match (expected.arguments(), observed.arguments()) {
            // Case: Arguments are present and lengths match exactly.
            (Some(e), Some(o)) if e.len() == o.len() => Ok(o),

            // Case: Neither side has arguments (valid empty signature).
            (None, None) => Ok(&[]),

            // Error Cases: Handle various arity mismatch scenarios.
            (Some(e), Some(o)) => Err(MatchFailure::Arity {
                expected: e.len(),
                observed: o.len(),
            }),
            (Some(e), None) => Err(MatchFailure::Arity {
                expected: e.len(),
                observed: 0,
            }),
            (None, Some(o)) => Err(MatchFailure::Arity {
                expected: 0,
                observed: o.len(),
            }),
        }
    }
}
