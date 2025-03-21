use crate::aiplan4rust::error::error_manager::ErrorManager;
use crate::aiplan4rust::error::parsing_error::ParserErrorKind::ParseError;
use crate::aiplan4rust::error::parsing_error::{ParserErrorKind, ParsingError};
use crate::aiplan4rust::frontend::ParserInternalError;
use crate::aiplan4rust::semantics::analyser_result::AnalyzerResult;
use crate::aiplan4rust::semantics::annotated_syntax_tree::AnnotatedSyntaxTree;
use crate::aiplan4rust::semantics::ast_table::AstTable;
use crate::aiplan4rust::semantics::atomic_expression_checker::AtomicExpressionChecker;
use crate::aiplan4rust::semantics::scope::Scope;
use crate::aiplan4rust::semantics::symbol::{Declaration, Symbol, SymbolKind};
use crate::aiplan4rust::semantics::symbol_table::SymbolTable;
use crate::aiplan4rust::syntax::ast::AstKind;
use crate::aiplan4rust::syntax::ast::Requirement::{Adl, DurativeActions, NumericFluents, Typing};
use crate::aiplan4rust::syntax::syntax_tree::SyntaxTree;
use crate::aiplan4rust::syntax::token::{DURATION_VARIABLE, NUMBER_TYPE, OBJECT_TYPE, TOTAL_TIME};
use std::collections::HashSet;
use std::mem;
use std::option::Option;

//const PDDL_BUILTIN_SYMBOLS: [&str; 4] = [OBJECT_TYPE, NUMBER_TYPE, TOTAL_TIME, DURATION_VARIABLE];

#[derive(Debug)]
pub struct Analyzer {
    error_manager: ErrorManager,
    file_path: Option<String>,
}

impl Analyzer {
    pub fn new() -> Self {
        Self {
            error_manager: ErrorManager::new(),
            file_path: None,
        }
    }

    pub fn errors_manager(&self) -> &ErrorManager {
        &self.error_manager
    }

    pub fn analyze(
        &mut self,
        syntax_tree: &SyntaxTree,
    ) -> Result<AnalyzerResult, ParserInternalError> {
        // Vérification si l'AST existe dans le syntax_tree
        let ast = syntax_tree.ast();
        let filename: Option<&String> = syntax_tree.filename();
        self.file_path = filename.cloned();

        // Convertir l'AST en hash map
        let mut ast_table = AstTable::from(&ast)?;
        println!("{}", ast_table);

        // Créer le SymbolTable avec l'AST et le Bimap
        let mut symbol_table = SymbolTable::new();
        symbol_table.initialize_from_ast(0, &ast_table)?;

        // Print the symbol table for debugging
        println!("{}", symbol_table);

        match ast.kind() {
            AstKind::Domain => {
                // Vérification des symboles dupliqués
                let duplicated_symbol_check =
                    self.check_duplicated_symbol_declaration(&symbol_table, &ast_table);

                // Vérification des symboles non déclarés
                let undeclared_symbol_check =
                    self.check_undeclared_symbol(&symbol_table, &ast_table, &[]);

                // Si les vérifications passées sont correctes, procéder à la vérification des formules atomiques
                if matches!(duplicated_symbol_check, Ok(true))
                    && matches!(undeclared_symbol_check, Ok(true))
                {
                    self.check_atomic_formula_usages(&symbol_table, &ast_table)?;
                }

                // Vérification des symboles inutilisés, indépendamment des précédentes vérifications
                // Vérification des symboles non déclarés
                let skip_symbols = &[SymbolKind::Constant];
                self.check_symbol_usage(&symbol_table, &ast_table, skip_symbols)?;
            }
            AstKind::Problem => {
                // Vérification des symboles dupliqués
                self.check_duplicated_symbol_declaration(&symbol_table, &ast_table)?;

                // Vérification des symboles non déclarés
                let skip_symbols = &[
                    SymbolKind::PrimitiveType,
                    SymbolKind::Constant,
                    SymbolKind::Predicate,
                    SymbolKind::Function,
                    SymbolKind::Task, // Add for HDDL
                ];
                self.check_undeclared_symbol(&symbol_table, &ast_table, skip_symbols)?;

                // Vérification des symboles inutilisés, indépendamment des précédentes vérifications
                self.check_symbol_usage(&symbol_table, &ast_table, &[])?;
            }
            _ => {
                return Err(ParserInternalError::new(format!(
                    "Unexpected AST node kind found: {}",
                    ast.kind()
                )));
            }
        }

        if !self.error_manager.has_errors_of_kind(ParseError) {
            let annotated_syntax_tree = AnnotatedSyntaxTree::new(
                mem::take(&mut ast_table),
                mem::take(&mut symbol_table),
                syntax_tree.filename().unwrap().clone(),
                std::time::SystemTime::now(),
            );
            Ok(AnalyzerResult::new(
                Some(annotated_syntax_tree),
                mem::take(&mut self.error_manager),
            ))
        } else {
            Ok(AnalyzerResult::new(
                None,
                mem::take(&mut self.error_manager),
            ))
        }
    }

    /// Checks for duplicate symbol declarations in the symbol table and logs errors
    /// to the error manager if duplicates are found.
    ///
    /// Duplicate checking is skipped for symbols of kind `DomainName` and
    /// `ProblemName` because these kinds are often used as symbols for types or
    /// predicates, where duplicates may be allowed.
    ///
    /// # Parameters
    /// - `symbol_table`: A reference to the symbol table to check for duplicates.
    ///
    /// # Returns
    /// - `Ok(())` if the check completes (errors are logged via the error manager).
    /// - `Err(ParserInternalError)` if an error occurs during processing.
    fn check_duplicated_symbol_declaration(
        &mut self,
        symbol_table: &SymbolTable,
        ast_table: &AstTable,
    ) -> Result<bool, ParserInternalError> {
        let mut checked = true;
        // Iterate over each symbol in the symbol table.
        for symbol in symbol_table.values() {
            let mut seen_scopes = HashSet::new();
            let symbol_name = symbol.name(); // Avoid multiple borrows of `symbol`

            // Iterate over each declaration for the symbol.
            for declaration in symbol.declarations() {
                // Skip duplicate checks for symbols of kind DomainName or ProblemName.
                // These can be used as symbols for types or predicates, so duplicates may be allowed.
                if Analyzer::skip_duplicated_declaration(declaration)? {
                    /*println!(
                        "SKIP DUPLICATED SYMBOL: {} {}",
                        declaration.kind(),
                        symbol.name()
                    );*/
                    continue;
                }

                let ast_entry = ast_table.get_entry(declaration.ast()).unwrap();
                if seen_scopes
                    .iter()
                    .any(|s: &&Scope| declaration.scope().starts_with(s))
                {
                    checked = false;
                    let (line, column) = ast_entry.span().start_position();
                    let content = format!(
                        "Duplicate declaration of symbol '{}' in a related scope at line {} column {}.",
                        symbol_name, line, column
                    );
                    let error = ParsingError::new(
                        ParserErrorKind::ParseError,
                        self.file_path.clone(),
                        line,
                        column,
                        content,
                    );
                    self.error_manager.add_error(error);
                } else {
                    // Ajouter à seen_scopes si aucun élément existant ne commence par declaration.scope()
                    seen_scopes.insert(declaration.scope());
                }
            }
        }

        Ok(checked)
    }

    /// Checks for symbols used in a scope without being declared in the same or a parent scope.
    /// This function reports errors for any undeclared symbols found.
    ///
    /// # Parameters
    /// - `symbol_table`: A reference to the symbol table containing symbols to be checked.
    /// - `skip_symbols`: A list of `SymbolKind` values representing symbols that should be ignored during checking.
    ///
    /// # Returns
    /// - `Ok(())` if the check completes successfully. Errors are logged through the error manager.
    /// - `Err(ParserInternalError)` if an error occurs during processing.
    ///
    /// # Note
    /// - The built-in PDDL symbols `"object"` and `"number"` are ignored, as they are always valid.
    /// - Symbols whose kind appears in `skip_symbols` are not checked.
    fn check_undeclared_symbol(
        &mut self,
        symbol_table: &SymbolTable,
        ast_table: &AstTable,
        skip_symbols: &[SymbolKind],
    ) -> Result<bool, ParserInternalError> {
        let mut no_error = true;

        // Iterate over each symbol in the symbol table.
        for symbol in symbol_table.values() {
            // Iterate over all usages of the symbol.
            for usage in symbol.usages() {
                if Analyzer::skip_undeclared_symbol_declaration(symbol, ast_table)? {
                    //println!("SKIP UNDECLARED SYMBOL: {} {}", usage.kind(), symbol.name());
                    continue;
                }
                if skip_symbols.contains(usage.kind()) {
                    //println!("SKIP UNDECLARED SYMBOL: {} {}", usage.kind(), symbol.name());
                    continue;
                }

                let usage_scope = usage.scope();

                let declaration_found = symbol.declarations().iter().any(|declaration| {
                    usage_scope.starts_with(&declaration.scope())
                        && (declaration.kind() == usage.kind()
                            || (*declaration.kind() == SymbolKind::Action
                                && *usage.kind() == SymbolKind::Task))
                });

                // If no declaration is found, report an error for the undeclared symbol.
                if !declaration_found {
                    no_error = false;
                    let entry = ast_table.get_entry(usage.ast()).unwrap();
                    let (line, column) = entry.span().start_position();
                    let content = format!(
                        "{} '{}' used but not declared at line {} column {}.",
                        usage.kind(),
                        symbol.name(),
                        line,
                        column
                    );
                    let error = ParsingError::new(
                        ParserErrorKind::ParseError,
                        self.file_path.clone(),
                        line,
                        column,
                        content,
                    );
                    self.error_manager.add_error(error);
                }
            }
        }

        Ok(no_error)
    }

    /// Checks for symbols that are declared but never used in the same or a parent scope.
    /// This function reports warnings for any unused symbols found.
    ///
    /// # Parameters
    /// - `symbol_table`: A reference to the symbol table containing symbols to be checked.
    /// - `skip_symbols`: A list of `SymbolKind` values representing symbols that should be ignored during checking.
    ///
    /// # Returns
    /// - `Ok(())` if the check completes successfully. Warnings are logged through the error manager.
    /// - `Err(ParserInternalError)` if an error occurs during processing.
    ///
    /// # Note
    /// - The built-in PDDL symbols `"object"` and `"number"` are ignored, as they are always valid.
    /// - Symbols whose kind appears in `skip_symbols` are not checked.
    fn check_symbol_usage(
        &mut self,
        symbol_table: &SymbolTable,
        ast_table: &AstTable,
        skip_symbols: &[SymbolKind],
    ) -> Result<bool, ParserInternalError> {
        let mut no_error = true;

        // Iterate over each symbol in the symbol table.
        for symbol in symbol_table.values() {
            for declaration in symbol.declarations() {
                if Analyzer::skip_unused_symbol_declaration(symbol, declaration, ast_table)?
                    || skip_symbols.contains(declaration.kind())
                {
                    /*println!(
                        "SKIP UNUSED SYMBOL: {} {}",
                        declaration.kind(),
                        symbol.name()
                    );*/
                    continue;
                }
                self.check_pddl_builtin_symbol_declaration(symbol, declaration, ast_table)?;

                let declaration_scope = declaration.scope();
                let declaration_kind = declaration.kind();

                // Trouver une utilisation du symbole dans le même ou un sous-scope
                let usage_opt = symbol
                    .usages()
                    .iter()
                    .find(|usage| usage.scope().starts_with(&declaration_scope));

                match usage_opt {
                    None => {
                        // Aucun usage trouvé : générer un avertissement
                        no_error = false;
                        let entry = ast_table.get_entry(declaration.ast()).unwrap();
                        let (line, column) = entry.span().start_position();
                        let content = format!(
                            "Symbol '{}' declared at line {} column {} but never used.",
                            symbol.name(),
                            line,
                            column
                        );
                        let warning = ParsingError::new(
                            ParserErrorKind::ParseWarning,
                            self.file_path.clone(),
                            line,
                            column,
                            content,
                        );
                        self.error_manager.add_error(warning);
                    }
                    Some(usage) => {
                        // Vérifier la cohérence du type entre la déclaration et l'utilisation
                        if usage.kind() != declaration_kind {
                            no_error = false;
                            let entry = ast_table.get_entry(usage.ast()).unwrap();
                            let (line, column) = entry.span().start_position();
                            let content = format!(
                                "Symbol '{}' declared as {:?} but used as {:?} at line {} column {}.",
                                symbol.name(),
                                declaration_kind,
                                usage.kind(),
                                line,
                                column
                            );
                            let error = ParsingError::new(
                                ParserErrorKind::ParseError, // Erreur de type
                                self.file_path.clone(),
                                line,
                                column,
                                content,
                            );
                            self.error_manager.add_error(error);
                        }
                    }
                }
                //println!("USAGE OPT: {} {:?}", symbol, usage_opt);
            }
        }

        Ok(no_error)
    }

    /// Determines whether a declaration should be skipped during duplicate checking.
    ///
    /// This function returns `true` if the declaration's kind indicates that it is not
    /// subject to duplicate checks. In particular, it skips declarations of symbols of kind
    /// `Requirement`, `Action`, or `DASymbol`, as well as variables declared within the scope
    /// of atomic formula or atomic function skeletons. Such symbols are typically declared
    /// in the domain and are not intended to be checked for duplicates in problem files.
    ///
    /// # Parameters
    /// - `declaration`: A reference to the `Declaration` to check.
    ///
    /// # Returns
    /// - `true` if the declaration should be skipped,
    /// - `false` otherwise.
    fn skip_unused_symbol_declaration(
        symbol: &Symbol,
        declaration: &Declaration,
        ast_table: &AstTable,
    ) -> Result<bool, ParserInternalError> {
        // Skip if the declaration is of a built-in kind: Requirement, Action, DASymbol or Method
        if matches!(
            declaration.kind(),
            SymbolKind::DomainName
                | SymbolKind::ProblemName
                | SymbolKind::Requirement
                | SymbolKind::Action
                | SymbolKind::DASymbol
                | SymbolKind::Method
        ) {
            return Ok(true);
        }

        match symbol.name().as_str() {
            OBJECT_TYPE
                if ast_table.requirements().contains(&Typing)
                    || ast_table.requirements().contains(&Adl) =>
            {
                return Ok(true)
            }
            NUMBER_TYPE | TOTAL_TIME if ast_table.requirements().contains(&NumericFluents) => {
                return Ok(true)
            }
            DURATION_VARIABLE if ast_table.requirements().contains(&DurativeActions) => {
                return Ok(true)
            }
            _ => {}
        }

        // Skip if the declaration is a variable and its scope contains an atomic skeleton node.
        // The variables have local scope and does not need to be checked
        if matches!(declaration.kind(), SymbolKind::Variable)
            && (declaration
                .scope()
                .contains_ast_of_kind(AstKind::AtomicFormulaSkeleton, ast_table)?
                || declaration
                    .scope()
                    .contains_ast_of_kind(AstKind::AtomicFunctionSkeleton, ast_table)?
                || declaration // Add for HDDL
                    .scope()
                    .contains_ast_of_kind(AstKind::TaskDef, ast_table)?)
        {
            return Ok(true);
        }

        Ok(false)
    }

    /// Checks the usage of predicates and functions in the given symbol table to ensure they match
    /// their declarations.
    ///
    /// This function iterates over each symbol in the provided `symbol_table` and checks whether
    /// any usage of a predicate or function corresponds to its declaration. If any mismatch is
    /// found, an error is recorded with the appropriate line and column information. The function
    /// returns a result indicating whether any errors were found during the checking process.
    ///
    /// # Arguments
    /// * `symbol_table` - A reference to a `SymbolTable` that contains the symbols (predicates,
    ///   functions, etc.) to be checked.
    ///
    /// # Returns
    /// * `Result<bool, ParserInternalError>` - Returns `Ok(true)` if no errors were found,
    ///   `Ok(false)` if errors were found.  In case of an internal aiplan4rust error, it returns
    /// `Err(ParserInternalError)`.
    ///
    /// # Example
    /// ```rust
    /// let result = check_atomic_formula_usages(&symbol_table);
    /// match result {
    ///     Ok(true) => println!("No errors found."),
    ///     Ok(false) => println!("Errors found in predicate or function usages."),
    ///     Err(e) => eprintln!("An internal error occurred: {:?}", e),
    /// }
    /// ```
    ///
    /// # Algorithm
    /// 1. The function iterates through all symbols in the `symbol_table`.
    /// 2. For each symbol, it checks if it is a predicate or function.
    /// 3. It then checks each usage of the symbol to ensure it matches its declaration.
    /// 4. If any mismatch is found, an error is logged, and the `no_error` flag is set to `false`.
    /// 5. The function returns `Ok(true)` if no errors are encountered, or `Ok(false)` if errors
    ///   are found.
    ///
    /// # Error Handling
    /// The function handles errors by adding them to an error manager, providing the line and
    ///   column information for each mismatch.
    ///
    /// # Notes
    /// - This function relies on the `match_declaration_with_usage` method to check if the symbol
    ///   usage matches the declaration.
    /// - The error manager records all parsing errors related to symbol mismatches.
    pub fn check_atomic_formula_usages(
        &mut self,
        symbol_table: &SymbolTable,
        ast_table: &AstTable,
    ) -> Result<bool, ParserInternalError> {
        let mut no_error = true;

        let atomic_expression_checker = AtomicExpressionChecker::new(symbol_table, ast_table);

        // Iterate over each symbol in the symbol table.

        for symbol in symbol_table.values() {
            for declaration in symbol.declarations() {
                if !matches!(
                    declaration.kind(),
                    SymbolKind::Predicate | SymbolKind::Function
                ) {
                    continue;
                }
                for usage in symbol.usages() {
                    if !atomic_expression_checker
                        .check_domain_atomic_expression(declaration, usage)?
                    {
                        no_error = false;
                        let entry = ast_table.get_entry(usage.ast()).unwrap();
                        let (line, column) = entry.span().start_position();
                        let content = format!(
                            "Predicate or function '{}' used at line {} column {} does not match any declaration.",
                            symbol.name(),
                            line,
                            column
                        );
                        let error = ParsingError::new(
                            ParserErrorKind::ParseError, // Use a different error kind if needed
                            self.file_path.clone(),
                            line,
                            column,
                            content,
                        );
                        self.error_manager.add_error(error);
                    }
                }
            }
        }
        Ok(no_error)
    }

    //fn is_pddl_builtin_symbol(symbol: &str) -> bool {
    //    PDDL_BUILTIN_SYMBOLS.contains(&symbol)
    //}

    fn skip_undeclared_symbol_declaration(
        symbol: &Symbol,
        ast_table: &AstTable,
    ) -> Result<bool, ParserInternalError> {
        match symbol.name().as_str() {
            OBJECT_TYPE
                if ast_table.requirements().contains(&Typing)
                    || ast_table.requirements().contains(&Adl) =>
            {
                Ok(true)
            }
            NUMBER_TYPE | TOTAL_TIME if ast_table.requirements().contains(&NumericFluents) => {
                Ok(true)
            }
            DURATION_VARIABLE if ast_table.requirements().contains(&DurativeActions) => Ok(true),
            _ => Ok(false),
        }
    }

    fn skip_duplicated_declaration(declaration: &Declaration) -> Result<bool, ParserInternalError> {
        if matches!(
            declaration.kind(),
            SymbolKind::DomainName | SymbolKind::ProblemName
        ) {
            return Ok(true);
        }

        Ok(false)
    }

    /// Checks if a symbol is properly declared as a built-in symbol according to the PDDL
    /// specifications.
    ///
    /// This function verifies whether a given symbol matches the expected type of a built-in symbol
    /// based on the PDDL requirements present in the `ast_table`. For example, it checks if a
    /// symbol like `OBJECT_TYPE`, `NUMBER_TYPE`, or `TOTAL_TIME` is correctly declared with the
    /// appropriate kind (e.g., `PrimitiveType`, `Function`, `Variable`) based on the domain's
    /// requirements.
    ///
    /// # Arguments
    /// * `symbol`: A reference to the `Symbol` that needs to be checked.
    /// * `declaration`: A reference to the `Declaration` of the symbol, which contains type
    ///   information.
    /// * `ast_table`: A reference to the `AstTable` that contains the domain's requirements and
    ///   other metadata.
    ///
    /// # Returns
    /// * `Ok(true)` if the symbol's declaration matches the expected type and is correct according
    ///   to the PDDL requirements.
    /// * `Ok(false)` if the symbol's declaration is incorrect or doesn't match any recognized
    ///   built-in symbol declaration.
    ///
    /// # Errors
    /// If the symbol's declaration is invalid, an error is logged with the line and column number
    /// of the invalid declaration.

    fn check_pddl_builtin_symbol_declaration(
        &mut self,
        symbol: &Symbol,
        declaration: &Declaration,
        ast_table: &AstTable,
    ) -> Result<bool, ParserInternalError> {
        let (expected_kind, requirement, error_message) = match symbol.name().as_str() {
            OBJECT_TYPE
                if ast_table.requirements().contains(&Typing)
                    || ast_table.requirements().contains(&Adl) =>
            {
                (SymbolKind::PrimitiveType, ":typing", "builtin type")
            }
            NUMBER_TYPE if ast_table.requirements().contains(&NumericFluents) => (
                SymbolKind::PrimitiveType,
                ":numeric-fluents",
                "builtin type",
            ),
            TOTAL_TIME if ast_table.requirements().contains(&NumericFluents) => {
                (SymbolKind::Function, ":numeric-fluents", "builtin function")
            }
            DURATION_VARIABLE if ast_table.requirements().contains(&DurativeActions) => (
                SymbolKind::Variable,
                ":durative-actions",
                "builtin variable",
            ),
            _ => return Ok(false), // Aucun cas ne correspond, donc on retourne directement `false`
        };

        // Vérifier si le symbole a le bon type
        if *declaration.kind() != expected_kind {
            if let Some(entry) = ast_table.get_entry(declaration.ast()) {
                let (line, column) = entry.span().start_position();
                let content = format!(
                    "'{}' is a {} in a domain with {} requirement.",
                    symbol.name(),
                    error_message,
                    requirement
                );
                let error = ParsingError::new(
                    ParserErrorKind::ParseError,
                    self.file_path.clone(),
                    line,
                    column,
                    content,
                );
                self.error_manager.add_error(error);
            }
            return Ok(false);
        }

        Ok(true)
    }
}
