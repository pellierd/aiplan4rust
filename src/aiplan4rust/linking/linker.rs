use crate::aiplan4rust::diagnostic::{DiagnosticManager, Severity, Provider};
use crate::aiplan4rust::AiplanError;
use crate::aiplan4rust::linking::LinkedSemanticContext;
use crate::aiplan4rust::linking::LinkerResult;
use crate::aiplan4rust::semantic::{SemanticContext, SymbolTable, TypeChecker};
use crate::aiplan4rust::{linking, semantic};
use crate::aiplan4rust::interner::{InternerDisplay, InternerMergeResult};
use crate::aiplan4rust::semantic::checks::CheckContext;
use crate::aiplan4rust::semantic::symbol::{Declaration, SymbolOrigin, Usage};
use crate::aiplan4rust::lang::Ident;

use std::collections::HashMap;
use std::mem::take;
use crate::aiplan4rust::arena::ArenaNode;


/// The `Linker` is responsible for performing the linking phase
/// of the AIPlan4Rust compilation pipeline.
///
/// It takes care of:
/// 1. Resolving symbols between the domain and problem definitions,
/// 2. Performing semantic and structural consistency checks,
/// 3. Producing a `LinkerResult` which includes the linked domain/problem
///    pair (`LinkedSemanticContext`) and diagnostic information.
///
/// # Example
/// ```rust
/// let mut linker = Linker::new();
/// let result = linker.link(domain_context, problem_context)?;
/// if let Some(linked) = result.task() {
///     // use the linked planning task...
/// }
/// ```
#[derive(Debug)]
pub struct Linker {
    diagnostic_manager: DiagnosticManager,
}

impl Linker {
    /// Creates a new instance of the `Linker`.
    pub fn new() -> Self {
        Self {
            diagnostic_manager: DiagnosticManager::new(),
        }
    }

    /// Returns an immutable reference to the internal `DiagnosticManager`,
    /// which contains diagnostics collected during the linking process.
    pub fn diagnostic_manager(&self) -> &DiagnosticManager {
        &self.diagnostic_manager
    }

    /// Performs semantic linking between a domain and a problem context.
    ///
    /// This function carries out the following steps:
    ///
    /// 1. Merges string interners from the domain and the problem to create a global interner.
    /// 2. Remaps identifiers in the problem's AST and symbol table to correspond to the global interner.
    /// 3. Resolves external references in the problem against the domain.
    /// 4. Creates a verification context (`CheckContext`) using the problem's AST, symbol table,
    ///    global interner, source name, and requirements.
    /// 5. Performs semantic and structural linking checks on the problem.
    /// 6. Finalizes the linking process and returns the result, while managing diagnostics.
    ///
    /// # Arguments
    ///
    /// * `domain` - The semantic context representing the domain (reference context).
    /// * `problem` - The semantic context of the problem to be linked.
    ///
    /// # Returns
    ///
    /// * `Ok(LinkerResult)` if linking succeeds.
    /// * `Err(ParserInternalError)` if an error occurs during resolution or verification.
    ///
    /// # Notes
    ///
    /// Identifier remapping ensures symbol consistency within the unified identifier space.
    /// The diagnostic manager collects errors or warnings encountered during linking.
    ///
    pub fn link_with_diagnostic_manager(
        &mut self,
        mut domain: SemanticContext,
        mut problem: SemanticContext,
        diagnostic_manager: DiagnosticManager
    ) -> Result<LinkerResult, AiplanError> {
        self.diagnostic_manager = diagnostic_manager;
        self.link(domain, problem)
    }

    /// Performs semantic linking between a domain and a problem context.
    ///
    /// This function carries out the following steps:
    ///
    /// 1. Merges string interners from the domain and the problem to create a global interner.
    /// 2. Remaps identifiers in the problem's AST and symbol table to correspond to the global interner.
    /// 3. Resolves external references in the problem against the domain.
    /// 4. Creates a verification context (`CheckContext`) using the problem's AST, symbol table,
    ///    global interner, source name, and requirements.
    /// 5. Performs semantic and structural linking checks on the problem.
    /// 6. Finalizes the linking process and returns the result, while managing diagnostics.
    ///
    /// # Arguments
    ///
    /// * `domain` - The semantic context representing the domain (reference context).
    /// * `problem` - The semantic context of the problem to be linked.
    ///
    /// # Returns
    ///
    /// * `Ok(LinkerResult)` if linking succeeds.
    /// * `Err(ParserInternalError)` if an error occurs during resolution or verification.
    ///
    /// # Notes
    ///
    /// Identifier remapping ensures symbol consistency within the unified identifier space.
    /// The diagnostic manager collects errors or warnings encountered during linking.
    ///
    pub fn link(
        &mut self,
        mut domain: SemanticContext,
        mut problem: SemanticContext,
    ) -> Result<LinkerResult, AiplanError> {
//        self.diagnostic_manager.add
        /*println!("DOMAIN ****************************$");
        println!("{}", domain.interner());
        println!("PROBLEM ****************************$");
        println!("{}", problem.interner());*/

        // Step 1: Merge the string interners from domain and problem to form a global interner
        let mut result = InternerMergeResult::from_domain_and_problem(
            domain.interner(),
            problem.interner(),
        );
        let global_interner = result.take_interner();
        /*println!("GLOBAL ****************************$");
        println!("{}", global_interner);*/
        // Step 2: Remap identifiers in the problem's AST and symbol table to the global interner space
        let problem_ident_map = result.take_problem_ident_map();


        remap_problem_idents(&mut problem, &problem_ident_map);

        println!("AVANNT ****************************$");
        println!("{}", problem.symbol_table().to_string_with_interner(&global_interner));

        // Step 3: Resolve external references in the problem with respect to the domain
        resolve_external_references(&domain, &mut problem)?;

        println!("APRES ****************************$");
        println!("{}", problem.symbol_table().to_string_with_interner(&global_interner));

        // Step 4: Create a check context for the problem using the global interner
        // and perform semantic and structural linking checks on the problem
        let problem_ctx = CheckContext::new(
            problem.ast(),
            problem.symbol_table(),
            &global_interner,
            problem.source_name(),
            problem.requirements(),
        );
        perform_linking_checks(&domain, &problem_ctx, &mut self.diagnostic_manager)?;

        // Step 5: If errors, return early with diagnostics only
        if self.diagnostic_manager.has_diagnostics_of_severity(Severity::Error) {
            return Ok(LinkerResult::new(None, take(&mut self.diagnostic_manager)));
        }

        // Step 7: Construct the final linked semantic context
        let semantic_context = LinkedSemanticContext::new(
            domain.take_ast(),
            problem.take_ast(),
            domain.take_symbol_table(),
            problem.take_symbol_table(),
            global_interner,
            domain.source_name().to_string(),
            problem.source_name().to_string(),
        );

        // Step 8: Return the result with the semantic context and diagnostics
        Ok(LinkerResult::new(Some(semantic_context), take(&mut self.diagnostic_manager)))
    }
}

/// Remaps identifiers in the problem's AST and symbol table using the provided mapping.
///
/// This function updates all identifiers in the problem’s AST and symbol table
/// to their corresponding identifiers in the global interner space according to
/// the `problem_ident_map`.
///
/// **Note:** This function does not modify or replace the underlying string interner itself;
/// it only updates the identifier references within the problem to align with the global interner.
///
/// # Arguments
///
/// * `problem` - Mutable reference to the problem semantic context.
/// * `problem_ident_map` - A mapping from the problem's local identifiers to the global identifiers.
fn remap_problem_idents(
    problem: &mut SemanticContext,
    problem_ident_map: &HashMap<Ident, Ident>,
) {
    problem.ast_mut().remap_idents(problem_ident_map);
    problem.symbol_table_mut().remap_idents(problem_ident_map);
}

/// Performs semantic and structural linking checks between a domain and a problem.
///
/// This function runs a sequence of verification passes to ensure the compatibility and
/// coherence between a domain and a problem during the linking phase. It emits diagnostics
/// (warnings and errors) via the provided `DiagnosticManager`.
///
/// The checks are performed in two phases:
///
/// 1. **Structural Checks** (always executed):
///     - Domain and problem name consistency (`check_domain_name`)
///     - Duplicate symbol declarations (`check_cross_declared_symbols`)
///     - Undeclared symbol usages (`check_undeclared_symbols`)
///     - Unused symbol declarations (`check_unused_symbols`)
///
/// 2. **Type-Dependent Checks** (executed only if no errors found in phase 1):
///     - Signature validation of declared symbols (`check_declared_symbol_signatures`)
///     - Type correctness of expr (`check_typed_expressions`)
///     - Task ordering consistency (`check_task_ordering`)
///     - Requirement compliance (`check_requirement_violations`)
///
/// # Arguments
///
/// * `domain` - Reference to the domain's semantic context.
/// * `problem` - Reference to the problem's check context.
/// * `diagnostic_manager` - Mutable reference to collect diagnostics.
///
/// # Returns
///
/// Returns `Ok(true)` if all checks passed successfully without critical errors, or `Ok(false)`
/// if some checks failed but no internal error occurred. Returns `Err` if an internal error
/// (e.g., inconsistent state or invalid assumptions) occurs during the process.
///
/// # Errors
///
/// Returns `ParserInternalError` if an internal semantic or resolution error prevents
/// the checks from completing.
///
/// # Example
///
/// ```rust
/// let mut diagnostics = DiagnosticManager::default();
/// let result = perform_linking_checks(&domain_ctx, &problem_ctx, &mut diagnostics)?;
/// if !result {
///     eprintln!("Some linking checks failed");
/// }
/// ```
pub fn perform_linking_checks(
    domain: &SemanticContext,
    problem: &CheckContext,
    diagnostic_manager: &mut DiagnosticManager,
) -> Result<bool, AiplanError> {

    // Check that the domain name matches the problem's declared domain
    linking::checks::check_domain_name(domain, problem, Provider::Linker, diagnostic_manager)?;

    // Check for duplicate symbol declarations across domain and problem
    let mut check = linking::checks::check_cross_declared_symbols(
        domain,
        problem,
        Provider::Linker,
        diagnostic_manager,
    )?;

    // Check for undeclared symbols used in the problem
    check &= semantic::checks::check_undeclared_symbols(
        problem,
        &[],
        Provider::Linker,
        diagnostic_manager,
    )?;

    // Check for symbols declared but never used in the problem
    check &= semantic::checks::check_unused_symbols(
        problem,
        &[],
        Provider::Linker,
        diagnostic_manager,
    )?;

    // If structural checks passed, perform type-dependent semantic checks
    if check {
        // Initialize a type checker with the domain's symbol table
        let type_checker = TypeChecker::new(&domain.symbol_table());

        // Validate signatures of declared symbols
        semantic::checks::check_declared_symbol_signatures(
            problem,
            &type_checker,
            diagnostic_manager,
        )?;

        // Verify the type correctness of expr in the problem
        semantic::checks::check_typed_expressions(
            problem,
            &type_checker,
            Provider::Linker,
            diagnostic_manager,
        )?;

        // Check task ordering constraints in the problem
        semantic::checks::check_task_ordering(problem, Provider::Linker, diagnostic_manager)?;

        // Merge requirements from domain and problem contexts
        let mut requirements = domain.requirements().clone();
        requirements.extend(problem.requirements().clone());

        // Check for any requirement violations
        semantic::checks::check_requirement_violations(
            problem,
            &requirements,
            Provider::Linker,
            diagnostic_manager,
        )?;
    }

    // Return whether all checks passed successfully
    Ok(check)
}

/// Resolves external references in the problem by injecting missing declarations from the domain.
///
/// This function updates the problem's symbol table by adding declarations found in the domain's
/// symbol table for symbols that are used but not declared in the problem. It ensures that the
/// problem's symbols have all necessary declarations available for subsequent semantic analysis.
///
/// The process involves:
/// - Collecting declared and undeclared symbols in the problem relative to the domain.
/// - For each declared symbol missing declarations, cloning and injecting the corresponding
///   declarations from the domain symbol table into the problem's symbol table.
///
/// # Arguments
///
/// * `domain` - Reference to the domain's semantic context.
/// * `problem` - Mutable reference to the problem's semantic context, to be updated.
///
/// # Returns
///
/// Returns `Ok(())` if external references are successfully resolved.
/// Returns `Err(ParserInternalError)` if any internal semantic error occurs during resolution.
///
/// # Example
///
/// ```ignore
/// resolve_external_references(&domain_context, &mut problem_context)?;
/// ```
pub fn resolve_external_references(
    domain: &SemanticContext,
    problem: &mut SemanticContext,
) -> Result<(), AiplanError> {
    // Collect declared and undeclared symbols in the problem relative to the domain symbol table
    let mut declared = Vec::new();
    let mut undeclared = Vec::new();

    collect_declared_and_undeclared_symbols(
        problem,
        domain.symbol_table(),
        &mut declared,
        &mut undeclared,
    )?;

    let problem_symbol_table = problem.symbol_table_mut();

    // For each declared symbol, inject the corresponding declaration into the problem symbol table
    for (symbol_name, declaration) in declared {
        if let Some(symbol) = problem_symbol_table.get_symbol_mut(symbol_name) {
            symbol.add_declaration(declaration);
        }
    }

    Ok(())
}

/// Collects symbol declarations from the domain symbol table for problem symbols that lack declarations,
/// and gathers symbols that remain undeclared.
///
/// This function does **not** mutate any symbol tables directly.
/// Instead, it fills the provided vectors:
/// - `declared`: collects `(symbol_name, declaration)` pairs to later add to the problem's symbol table.
/// - `undeclared`: collects `(symbol_name, usage)` pairs representing unresolved symbols.
///
/// The function iterates over all symbols in the problem's symbol table that currently have no declarations.
/// For each usage of such a symbol, it attempts to resolve a matching declaration in the domain's symbol table.
/// If found, it clones the declaration, marks it as originating from the domain,
/// and adds it to `declared`. Otherwise, it records the symbol and usage as `undeclared`.
///
/// # Arguments
///
/// * `problem` - Reference to the problem's semantic context.
/// * `domain_symbol_table` - Reference to the domain's symbol table for resolving declarations.
/// * `declared` - Mutable vector to collect declarations to add to problem symbols.
/// * `undeclared` - Mutable vector to collect unresolved symbols and their usages.
///
/// # Returns
///
/// Returns `Ok(true)` if all problem symbols were successfully resolved from the domain.
/// Returns `Ok(false)` if some symbols remain undeclared.
/// Returns `Err(ParserInternalError)` if any error occurs during symbol resolution.
///
/// # Example
///
/// ```ignore
/// let mut declared = Vec::new();
/// let mut undeclared = Vec::new();
/// let all_resolved = collect_declared_and_undeclared_symbols(
///     &problem_context,
///     &domain_symbol_table,
///     &mut declared,
///     &mut undeclared,
/// )?;
/// if !all_resolved {
///     // Handle diagnostics for undeclared symbols here
/// }
/// ```
fn collect_declared_and_undeclared_symbols<'a>(
    problem: &'a SemanticContext,
    domain_symbol_table: &'a SymbolTable,
    declared: &mut Vec<(Ident, Declaration)>,
    undeclared: &mut Vec<(Ident, &'a Usage)>,
) -> Result<bool, AiplanError> {
    let problem_symbol_table = problem.symbol_table();
    let mut all_resolved = true;

    for symbol in problem_symbol_table.values() {
        if symbol.declarations().is_empty() {
            for usage in symbol.usages() {
                let domain_declaration_option = domain_symbol_table.resolve_declaration(
                    &symbol.name(),
                    &usage.symbol_kind(),
                    &domain_symbol_table.root_scope(),
                )?;

                if let Some(domain_declaration) = domain_declaration_option {
                    let mut domain_declaration = domain_declaration.clone();
                    domain_declaration.set_origin(SymbolOrigin::Domain);
                    domain_declaration.set_imported_scope(Some(domain_declaration.scope().clone()));
                    domain_declaration.set_scope(problem.symbol_table().root_scope().clone());
                    declared.push((symbol.name(), domain_declaration));
                } else {
                    undeclared.push((symbol.name(), usage));
                    all_resolved = false;
                }
            }
        }
    }

    Ok(all_resolved)
}
