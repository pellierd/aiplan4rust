use crate::aiplan4rust::error::error_manager::ErrorManager;
use crate::aiplan4rust::error::parsing_error::ParserErrorKind::ParseError;
use crate::aiplan4rust::error::parsing_error::{ParserErrorKind, ParsingError};
use crate::aiplan4rust::frontend::ParserInternalError;
use crate::aiplan4rust::semantics::analyser_result::AnalyzerResult;
use crate::aiplan4rust::semantics::annotated_syntax_tree::AnnotatedSyntaxTree;
use crate::aiplan4rust::semantics::ast_table::AstEntry;
use crate::aiplan4rust::semantics::ast_table::AstTable;
use crate::aiplan4rust::semantics::atomic_expression_checker::AtomicExpressionChecker;
use crate::aiplan4rust::semantics::scope::Scope;
use crate::aiplan4rust::semantics::symbol::Usage;
use crate::aiplan4rust::semantics::symbol::{Declaration, Symbol, SymbolKind};
use crate::aiplan4rust::semantics::symbol_table::SymbolTable;
use crate::aiplan4rust::syntax::ast::AssignOp;
use crate::aiplan4rust::syntax::ast::AstKind;
use crate::aiplan4rust::syntax::ast::BinaryComp;
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
                    self.check_function_type(&symbol_table, &ast_table)?;
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

    /// Checks if there are any undeclared symbols used in the given symbol table.
    /// This function scans all usages of symbols in the `symbol_table` and verifies if
    /// each symbol has been declared correctly. If no declaration is found for a symbol,
    /// an error is generated and added to the error manager.
    ///
    /// # Arguments
    ///
    /// * `symbol_table` - A reference to the `SymbolTable` that contains all the symbols.
    /// * `ast_table` - A reference to the `AstTable` for looking up AST entries related to the usages.
    /// * `skip_symbols` - A list of `SymbolKind`s to skip during the check.
    ///
    /// # Returns
    ///
    /// * `Result<bool, ParserInternalError>` - Returns `Ok(true)` if no undeclared symbol was found,
    ///   otherwise `Ok(false)`. Returns an error if an internal parser error occurs.
    ///
    /// # Example
    ///
    /// ```
    /// let result = check_undeclared_symbol(&symbol_table, &ast_table, &[SymbolKind::Action]);
    /// match result {
    ///     Ok(true) => println!("No undeclared symbols found."),
    ///     Ok(false) => println!("Some undeclared symbols were found."),
    ///     Err(e) => println!("Error: {}", e),
    /// }
    /// ```
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
                // Skip the symbol if it meets the criteria (e.g., already declared or needs to be skipped).
                if Self::should_skip_symbol(symbol, ast_table, usage.kind(), skip_symbols)? {
                    continue;
                }

                // Check if the declaration for the symbol was found.
                if !Self::is_declaration_found(symbol, usage) {
                    no_error = false;
                    // If no declaration is found, report an undeclared symbol error.
                    self.report_undeclared_symbol(symbol, usage, ast_table);
                }
            }
        }

        Ok(no_error)
    }

    /// Determines if a symbol should be skipped during the undeclared symbol check.
    /// This decision is based on whether the symbol is predefined in PDDL (according to the
    /// requirements) or if the symbol's kind matches any entry in the `skip_symbols` list.
    ///
    /// # Arguments
    ///
    /// * `symbol` - The symbol to check. This is typically a symbol from the symbol table that may
    ///   be used in the program or expression being analyzed.
    /// * `ast_table` - The AST (Abstract Syntax Tree) table used to fetch relevant AST data,
    ///   including the problem's requirements (such as Typing, NumericFluents, etc.) that influence
    ///   whether a symbol is predefined in PDDL.
    /// * `usage_kind` - The kind of symbol usage, which determines the context in which the symbol
    ///   is being used, e.g., a task, action, primitive type, etc.
    /// * `skip_symbols` - A list of symbol kinds (e.g., `SymbolKind::Action`) that should be
    ///   skipped during the check.
    ///
    /// # Returns
    ///
    /// * `Result<bool, ParserInternalError>` - Returns `Ok(true)` if the symbol should be skipped
    ///   (either because it is a predefined PDDL symbol or its kind is in the `skip_symbols` list),
    ///   otherwise `Ok(false)`. If there is an error while checking if the symbol is a predefined
    ///   PDDL symbol, an `Err` is returned.
    ///
    /// # Example
    ///
    /// ```
    /// let should_skip = should_skip_symbol(
    ///     &symbol,
    ///     &ast_table,
    ///     SymbolKind::Action,
    ///     &[SymbolKind::Action]
    /// );
    /// assert_eq!(should_skip, Ok(true));  // Assuming the symbol kind matches and is in the skip list.
    /// ```
    fn should_skip_symbol(
        symbol: &Symbol,
        ast_table: &AstTable,
        usage_kind: &SymbolKind,
        skip_symbols: &[SymbolKind],
    ) -> Result<bool, ParserInternalError> {
        // Skip if the symbol is predefined in PDDL or if it matches a symbol kind in the skip list.
        Ok(Self::is_pddl_builtin_symbol(symbol, ast_table)? || skip_symbols.contains(usage_kind))
    }

    /// Checks if a declaration for the given symbol usage exists in the symbol's declarations.
    /// This function handles different `SymbolKind`s and checks if the corresponding declaration
    /// matches the usage within the given scope.
    ///
    /// # Arguments
    ///
    /// * `symbol` - The symbol to check for declaration.
    /// * `usage` - The usage of the symbol that needs to be checked.
    ///
    /// # Returns
    ///
    /// * `bool` - Returns `true` if a matching declaration was found, otherwise `false`.
    ///
    /// # Example
    ///
    /// ```
    /// let declaration_found = is_declaration_found(&symbol, &usage);
    /// ```
    fn is_declaration_found(symbol: &Symbol, usage: &Usage) -> bool {
        let usage_scope = usage.scope();

        // Common closure to check declarations for the given kind and scope
        // This closure checks if the declaration's scope starts with the usage scope and if the
        // declaration kind matches the usage kind.
        let check_declarations = |declaration: &Declaration| {
            usage_scope.starts_with(&declaration.scope()) && declaration.kind() == usage.kind()
        };

        // For PrimitiveType, we also check usages at the root scope
        // This is necessary because PrimitiveType can be used without declaration if it appears on
        // the right side of type declarations in PDDL.For example, types like "car" or "vehicle"
        // might not be explicitly declared but are understood in the domain context.
        let check_usages_at_root_scope = |usage: &Usage| {
            let root_scope = Scope::new(0, None);
            symbol
                .usages()
                .iter()
                .any(|u| usage.scope().starts_with(&root_scope) && u.kind() == usage.kind())
        };

        // For SymbolKind::Task, we also check if declaration.kind() is Action or Task.
        // This ensures we match tasks that are declared with Action or Task symbols.
        let check_primitive_task_declaration = |declaration: &Declaration| {
            usage_scope.starts_with(&declaration.scope())
                && (*declaration.kind() == SymbolKind::Action
                    || *declaration.kind() == SymbolKind::Task)
        };

        match usage.kind() {
            SymbolKind::Task => symbol
                .declarations()
                .iter()
                .any(check_primitive_task_declaration),
            SymbolKind::PrimitiveType => {
                // For PrimitiveType, we check the common declaration logic and also include checks
                // or usages at the root scope. This ensures that PrimitiveTypes can be considered
                // even if they aren't explicitly declared in the current scope.
                symbol.declarations().iter().any(check_declarations)
                    || symbol
                        .usages()
                        .iter()
                        .any(|u| check_usages_at_root_scope(u))
            }
            _ => symbol.declarations().iter().any(check_declarations),
        }
    }

    /// Generates an error report for a symbol that is used but not declared.
    /// This function formats the error message and adds it to the error manager.
    ///
    /// # Arguments
    ///
    /// * `symbol` - The undeclared symbol that was used.
    /// * `usage` - The usage instance of the undeclared symbol.
    /// * `ast_table` - The AST table used to retrieve the location of the usage.
    ///
    /// # Example
    ///
    /// ```
    /// report_undeclared_symbol(&symbol, &usage, &ast_table);
    /// ```
    fn report_undeclared_symbol(&mut self, symbol: &Symbol, usage: &Usage, ast_table: &AstTable) {
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
                    SymbolKind::Predicate
                        | SymbolKind::Function
                        | SymbolKind::Task // Add to check compound task in HTN
                        | SymbolKind::Action // Add to check primitive task in HTN
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
                            "{} '{}' does not match any declaration.",
                            usage.kind(),
                            symbol.name()
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

    /// Checks if a symbol is a predefined PDDL symbol based on the requirements in the given
    /// `ast_table`.
    ///
    /// This function takes into account the context of the PDDL problem, i.e., which requirements
    /// are enabled in the current problem (such as `Typing`, `Adl`, `NumericFluents`, and
    /// `DurativeActions`), to determine if a symbol is considered a predefined symbol in PDDL.
    ///
    /// # Arguments
    /// - `symbol`: The symbol to check.
    /// - `ast_table`: The abstract syntax tree table that holds information about the PDDL problem
    ///   requirements.
    ///
    /// # Returns
    /// - `Ok(true)` if the symbol is predefined and matches the requirements.
    /// - `Ok(false)` if the symbol is not predefined.
    /// - `Err(ParserInternalError)` if there is an internal error while checking the symbol.
    ///
    /// # Examples
    /// ```
    /// let symbol = Symbol::new("object_type");
    /// let result = is_pddl_builtin_symbol(&symbol, &ast_table);
    /// assert_eq!(result, Ok(true));
    /// ```
    fn is_pddl_builtin_symbol(
        symbol: &Symbol,
        ast_table: &AstTable,
    ) -> Result<bool, ParserInternalError> {
        match symbol.name().as_str() {
            // 'object_type' is a predefined symbol when 'Typing' or 'Adl' requirements are present.
            OBJECT_TYPE
                if ast_table.requirements().contains(&Typing)
                    || ast_table.requirements().contains(&Adl) =>
            {
                Ok(true)
            }

            // 'number_type' or 'total_time' are predefined when the 'NumericFluents' requirement is
            // present.
            NUMBER_TYPE | TOTAL_TIME if ast_table.requirements().contains(&NumericFluents) => {
                Ok(true)
            }

            // 'duration_variable' is predefined when the 'DurativeActions' requirement is present.
            DURATION_VARIABLE if ast_table.requirements().contains(&DurativeActions) => Ok(true),

            // Default case for any other symbols.
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

    /// Verifies the types of expressions used in function calls and assignment operations.
    ///
    /// This function checks the types of expressions in the abstract syntax tree (AST) to ensure
    /// that they are compatible for their respective operations, specifically focusing on equality
    /// checks, assignment operations, and other comparisons and assignments. The function processes
    /// the following cases:
    ///
    /// - Equality check (`=`) and assignment (`assign`): Verifies that the types of the operands
    ///   match.
    /// - Other comparisons (greater than, less than, etc.) and assignments (scale up, scale down,
    ///   etc.): Verifies that both operands are numeric or compatible for the operation.
    ///
    /// The function uses `AtomicExpressionChecker` to validate type compatibility for each
    /// operation and logs errors  if any type mismatches are found.
    ///
    /// # Parameters
    /// - `symbol_table`: A reference to the `SymbolTable` used to resolve variable types.
    /// - `ast_table`: A reference to the `AstTable` that contains the abstract syntax tree entries.
    ///
    /// # Returns
    /// - `Ok(true)` if no type mismatches were found for the expressions.
    /// - `Ok(false)` if one or more type mismatches were found, and errors were logged.
    /// - `Err(ParserInternalError)` if an internal error occurs while processing the AST.
    ///
    /// # Example
    /// ```rust
    /// let symbol_table = ...;
    /// let ast_table = ...;
    /// let result = checker.check_function_type(&symbol_table, &ast_table);
    /// ```
    pub fn check_function_type(
        &mut self,
        symbol_table: &SymbolTable,
        ast_table: &AstTable,
    ) -> Result<bool, ParserInternalError> {
        let mut no_error = true;

        let type_checker = AtomicExpressionChecker::new(symbol_table, ast_table);

        for ast in ast_table.values() {
            match ast.kind() {
                // Case for equality check (AssignOp::Assign and BinaryComp::Equal)
                AstKind::FComp(BinaryComp::Equal) | AstKind::Assign(AssignOp::Assign) => {
                    let (ty1, ty2) =
                        Self::get_binary_operation_types(ast, symbol_table, ast_table)?;

                    // Call check_equal_and_assign function to handle this case
                    no_error &=
                        self.check_equal_and_assignment_expression(ast, symbol_table, &ty1, &ty2)?;
                }

                // Case for other comparison and assignment operations (Greater, Less, ScaleUp, etc.)
                AstKind::FComp(BinaryComp::Greater)
                | AstKind::FComp(BinaryComp::GreaterEq)
                | AstKind::FComp(BinaryComp::Less)
                | AstKind::FComp(BinaryComp::LessEq)
                | AstKind::Assign(AssignOp::ScaleUp)
                | AstKind::Assign(AssignOp::ScaleDown)
                | AstKind::Assign(AssignOp::Increase)
                | AstKind::Assign(AssignOp::Decrease) => {
                    let (ty1, ty2) =
                        Self::get_binary_operation_types(ast, symbol_table, ast_table)?;

                    // Call check_other_cases function to handle these cases
                    no_error &= self.check_numeric_expression(ast, &ty1, &ty2)?;
                }

                _ => {}
            }
        }

        Ok(no_error)
    }

    /// Verifies that the types of the operands in an equality (`=`) or assignment (`assign`)
    /// expression are compatible.
    ///
    /// This function handles both equality (`=`) and assignment (`assign`) operations. The types of
    /// the left and right operands must be compatible for the expression to be valid. While
    /// equality (`=`) typically involves comparing operands of the same type, assignments
    /// (`assign`) can involve various types depending on the PDDL domain, including numbers, or
    /// other user-defined types.
    ///
    /// # Parameters
    /// - `ast`: A reference to the `AstEntry` representing the expression to check.
    /// - `symbol_table`: A reference to the `SymbolTable` used to resolve type information.
    /// - `ty1`: A reference to a vector of strings representing the type of the first operand.
    /// - `ty2`: A reference to a vector of strings representing the type of the second operand.
    ///
    /// # Returns
    /// Returns a `Result<bool, ParserInternalError>`.
    /// - `Ok(true)` if the types are compatible (i.e., the left and right operands match).
    /// - `Ok(false)` if there is a type incompatibility, and an error is logged.
    /// - `Err` if there is an internal parsing error.
    ///
    /// # Example
    /// ```rust
    /// let ast_entry = ...;
    /// let ty1 = vec!["object".to_string()]; // An object type in PDDL
    /// let ty2 = vec!["object".to_string()]; // Another object type in PDDL
    /// let result = checker.check_equal_and_assignment_expression(&ast_entry, &symbol_table, &ty1, &ty2);
    /// ```
    fn check_equal_and_assignment_expression(
        &mut self,
        ast: &AstEntry,
        symbol_table: &SymbolTable,
        ty1: &Vec<String>,
        ty2: &Vec<String>,
    ) -> Result<bool, ParserInternalError> {
        let mut no_error = true;

        if !AtomicExpressionChecker::match_type(ty1, ty2, symbol_table, &Scope::root_scope())? {
            no_error = false;
            let (line, column) = ast.span().start_position();
            let content = format!("Type incompatibility in expression {}: ", ast);
            let error = ParsingError::new(
                ParserErrorKind::ParseError,
                self.file_path.clone(),
                line,
                column,
                content,
            );
            self.error_manager.add_error(error);
        }

        Ok(no_error)
    }

    /// Verifies that the types of the operands in a numeric comparison or assignment expression
    /// are compatible with the numeric type (i.e., `number`).
    ///
    /// This function is specifically for handling operations like Greater, Less, etc., where the
    /// operands must be of the numeric type.
    ///
    /// # Parameters
    /// - `ast`: A reference to the `AstEntry` representing the expression to check.
    /// - `ty1`: A reference to a vector of strings representing the types of the first operand.
    /// - `ty2`: A reference to a vector of strings representing the types of the second operand.
    ///
    /// # Returns
    /// Returns a `Result<bool, ParserInternalError>`.
    /// - `Ok(true)` if the types are compatible with numeric operations (`number`).
    /// - `Ok(false)` if there is a type incompatibility, and an error is logged.
    /// - `Err` if there is an internal parsing error.
    ///
    /// # Example
    /// ```rust
    /// let ast_entry = ...;
    /// let ty1 = vec!["number".to_string()];
    /// let ty2 = vec!["number".to_string()];
    /// let result = checker.check_numeric_expression(&ast_entry, &ty1, &ty2);
    /// ```
    fn check_numeric_expression(
        &mut self,
        ast: &AstEntry,
        ty1: &Vec<String>,
        ty2: &Vec<String>,
    ) -> Result<bool, ParserInternalError> {
        let mut no_error = true;

        // Handle Greater, Less, etc.
        let number = vec![NUMBER_TYPE.to_string()];
        if ty1 != &number || ty2 != &number {
            no_error = false;
            let (line, column) = ast.span().start_position();
            let content = format!("Type incompatibility in expression {}: ", ast);
            let error = ParsingError::new(
                ParserErrorKind::ParseError,
                self.file_path.clone(),
                line,
                column,
                content,
            );
            self.error_manager.add_error(error);
        }

        Ok(no_error)
    }

    /// Retrieves the types of the two operands involved in a binary operation.
    ///
    /// This function is designed to validate that the given abstract syntax tree (AST) entry
    /// represents a binary operation with exactly two children. It then retrieves the types
    /// of both operands (the children) by consulting the provided symbol table and AST table.
    /// If any issues arise, such as missing children, undeclared types, or incorrect numbers
    /// of children, an error is returned.
    ///
    /// # Arguments
    ///
    /// * `ast` - A reference to the `AstEntry` representing the binary operation in the AST.
    /// * `symbol_table` - A reference to the `SymbolTable` that holds the variable types for the
    ///   scope.
    /// * `ast_table` - A reference to the `AstTable` that holds the full set of AST entries.
    ///
    /// # Returns
    ///
    /// This function returns a `Result` containing a tuple of two `Vec<String>` values representing
    /// the types of the two operands if successful. The types are derived from the symbol table and
    /// AST table based on the respective operands' positions in the AST. If an error occurs, a
    /// `ParserInternalError` is returned.
    ///
    /// # Errors
    ///
    /// The function may return an error in the following cases:
    /// - If the `ast` entry does not have exactly two children, a `ParserInternalError` is returned
    ///   with the message "Binary operations must have exactly two children."
    /// - If either of the two operands is missing from the `ast_table`, a `ParserInternalError`
    ///   with the message "Missing first argument." or "Missing second argument." will be returned
    ///   accordingly.
    /// - If either of the two operands does not have a declared type in the symbol table, a
    ///   `ParserInternalError` will be returned with the message "No type declared for the first
    ///   argument." or "No type declared for the second argument."
    ///
    /// # Example
    /// ```rust
    /// let (ty1, ty2) = get_binary_operation_types(&ast, &symbol_table, &ast_table)?;
    /// ```
    fn get_binary_operation_types(
        ast: &AstEntry,
        symbol_table: &SymbolTable,
        ast_table: &AstTable,
    ) -> Result<(Vec<String>, Vec<String>), ParserInternalError> {
        // Validate that there are exactly 2 children
        if ast.children().len() != 2 {
            return Err(ParserInternalError::new(
                "Binary operations must have exactly two children.".to_string(),
            ));
        }

        let arg1 = ast_table
            .get_entry(ast.children()[0])
            .ok_or_else(|| ParserInternalError::new("Missing first argument.".to_string()))?;
        let arg2 = ast_table
            .get_entry(ast.children()[1])
            .ok_or_else(|| ParserInternalError::new("Missing second argument.".to_string()))?;

        let ty1 =
            Self::get_type(ast.children()[0], arg1, symbol_table, ast_table)?.ok_or_else(|| {
                ParserInternalError::new("No type declared for the first argument.".to_string())
            })?;
        let ty2 =
            Self::get_type(ast.children()[1], arg2, symbol_table, ast_table)?.ok_or_else(|| {
                ParserInternalError::new("No type declared for the second argument.".to_string())
            })?;

        Ok((ty1, ty2))
    }

    /// Retrieves the type of an AST node based on its kind.
    ///
    /// This function handles several AST node types, including numbers, variables, constants,
    /// and function terms. It delegates the actual type retrieval to specific helper functions
    /// for each type of AST node.
    ///
    /// # Parameters
    /// - `index`: The index of the symbol in the symbol table.
    /// - `ast`: A reference to the AST entry to analyze.
    /// - `symbol_table`: A reference to the symbol table.
    /// - `ast_table`: A reference to the AST table.
    ///
    /// # Returns
    /// Returns a `Result` containing an `Option<Vec<String>>`, which represents the type of
    /// the AST node, or an error if the node type is unexpected.
    ///
    /// # Errors
    /// Returns a `ParserInternalError` if the AST node kind is not one of the expected types.
    pub fn get_type(
        index: usize,
        ast: &AstEntry,
        symbol_table: &SymbolTable,
        ast_table: &AstTable,
    ) -> Result<Option<Vec<String>>, ParserInternalError> {
        match ast.kind() {
            // Case 1: Directly a number -> Type is NUMBER_TYPE
            AstKind::Number(_) => Self::get_number_type(),

            // Case 2: Variable
            AstKind::Variable(symbol) => {
                Self::get_variable_type(index, symbol, symbol_table, ast_table)
            }

            // Case 3: Constant
            AstKind::Constant(symbol) => Self::get_constant_type(index, symbol, symbol_table),

            // Case 4: Function Term
            AstKind::FunctionTerm => {
                Self::get_function_term_type(index, ast, symbol_table, ast_table)
            }

            // Default case: Unexpected AST node
            _ => Err(ParserInternalError::new(format!(
                "Unexpected AST node kind found: {}",
                ast.kind()
            ))),
        }
    }

    /// Helper to return a type `NUMBER_TYPE`.
    ///
    /// This function returns the type for a number, which is predefined as `NUMBER_TYPE`.
    ///
    /// # Returns
    /// Returns a `Result` containing an `Option<Vec<String>>`, with a single element `NUMBER_TYPE`.
    fn get_number_type() -> Result<Option<Vec<String>>, ParserInternalError> {
        Ok(Some(vec![NUMBER_TYPE.to_string()]))
    }

    /// Helper to retrieve the type of a variable.
    ///
    /// This function handles the special case of a `DURATION_VARIABLE` and delegates to
    /// `get_declaration_type` for other variables.
    ///
    /// # Parameters
    /// - `index`: The index of the symbol in the symbol table.
    /// - `symbol`: The name of the variable symbol.
    /// - `symbol_table`: A reference to the symbol table.
    /// - `ast_table`: A reference to the AST table.
    ///
    /// # Returns
    /// Returns a `Result` containing an `Option<Vec<String>>`, representing the type of the variable,
    /// or an error if the variable has multiple declarations.
    fn get_variable_type(
        index: usize,
        symbol: &str,
        symbol_table: &SymbolTable,
        ast_table: &AstTable,
    ) -> Result<Option<Vec<String>>, ParserInternalError> {
        if symbol == DURATION_VARIABLE && ast_table.requirements().contains(&DurativeActions) {
            return Self::get_number_type();
        }
        Self::get_declaration_type(index, symbol, symbol_table)
    }

    /// Helper to retrieve the type of a constant.
    ///
    /// This function delegates to `get_declaration_type` to retrieve the type of a constant.
    ///
    /// # Parameters
    /// - `index`: The index of the symbol in the symbol table.
    /// - `symbol`: The name of the constant symbol.
    /// - `symbol_table`: A reference to the symbol table.
    ///
    /// # Returns
    /// Returns a `Result` containing an `Option<Vec<String>>`, representing the type of the constant,
    /// or an error if the constant has multiple declarations.
    fn get_constant_type(
        index: usize,
        symbol: &str,
        symbol_table: &SymbolTable,
    ) -> Result<Option<Vec<String>>, ParserInternalError> {
        Self::get_declaration_type(index, symbol, symbol_table)
    }

    /// Helper function to retrieve the type of a symbol from the symbol table.
    ///
    /// This function looks up a symbol in the symbol table and returns its associated type.
    /// If the symbol has multiple declarations, it returns an error.
    ///
    /// # Parameters
    /// - `index`: The index of the symbol in the symbol table.
    /// - `symbol`: The name of the symbol.
    /// - `symbol_table`: A reference to the symbol table.
    ///
    /// # Returns
    /// Returns a `Result` containing an `Option<Vec<String>>` representing the symbol's type,
    /// or an error if the symbol has multiple declarations.
    fn get_declaration_type(
        index: usize,
        symbol: &str,
        symbol_table: &SymbolTable,
    ) -> Result<Option<Vec<String>>, ParserInternalError> {
        let declarations = symbol_table.get_declaration_by_usage(index)?;
        if declarations.len() > 1 {
            return Err(ParserInternalError::new(format!(
                "Symbol '{}' has multiple declarations.",
                symbol
            )));
        }
        Ok(declarations
            .get(0)
            .map(|decl| decl.types().cloned())
            .unwrap_or(None))
    }

    /// Helper to handle `FunctionTerm` and retrieve its type.
    ///
    /// This function checks if the `FunctionTerm` has a valid functor and determines
    /// the type based on the associated function symbol.
    ///
    /// # Parameters
    /// - `index`: The index of the symbol in the symbol table.
    /// - `ast`: A reference to the AST entry representing the function term.
    /// - `symbol_table`: A reference to the symbol table.
    /// - `ast_table`: A reference to the AST table.
    ///
    /// # Returns
    /// Returns a `Result` containing an `Option<Vec<String>>`, representing the type of the
    /// function term, or an error if the functor is invalid or the term has no functor.
    fn get_function_term_type(
        index: usize,
        ast: &AstEntry,
        symbol_table: &SymbolTable,
        ast_table: &AstTable,
    ) -> Result<Option<Vec<String>>, ParserInternalError> {
        let children = ast.children();
        if children.is_empty() {
            return Err(ParserInternalError::new(
                "Function term has no functor (empty children).".to_string(),
            ));
        }

        let functor_index = children[0];
        let functor_entry = ast_table.get_entry(functor_index).ok_or_else(|| {
            ParserInternalError::new(format!("No AST entry found for index {}.", functor_index))
        })?;

        if let AstKind::FunctionSymbol(symbol) = functor_entry.kind() {
            if symbol == TOTAL_TIME && ast_table.requirements().contains(&NumericFluents) {
                return Self::get_number_type();
            }
            return Self::get_declaration_type(index, symbol, symbol_table);
        }

        Err(ParserInternalError::new(
            "First child of function term is not a FunctionSymbol.".to_string(),
        ))
    }
}
