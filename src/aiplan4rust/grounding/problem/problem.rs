use crate::aiplan4rust::grounding::error::GroundingError;
use crate::aiplan4rust::grounding::problem::{builders, Type};
use crate::aiplan4rust::grounding::problem::Fluent;
use crate::aiplan4rust::grounding::problem::Function;
use crate::aiplan4rust::grounding::problem::IndexTable;

use crate::aiplan4rust::interner::{Ident, InternerError, StringInterner};
use crate::aiplan4rust::lang::Requirement;
use crate::aiplan4rust::lir::problem::LiftedProblem;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fmt;
use crate::aiplan4rust::grounding::problem::value_domain::ValueDomain;

/// Represents a fully grounded PDDL problem.
///
/// This structure stores all information about a PDDL problem, including
/// domain and problem identifiers, requirements, types, predicates, functions,
/// and objects. It also maintains internal symbol tables and an interner for
/// efficient string management.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Problem {
    /// Interned strings for efficient storage of identifiers.
    interner: StringInterner,

    /// The identifier of the domain.
    domain_id: Ident,

    /// The identifier of the problem.
    problem_id: Ident,

    /// Set of requirements declared for this problem.
    requirements: HashSet<Requirement>,

    /// Symbol table for all types.
    types_symbols: IndexTable,

    /// List of type of the problem
    types: Vec<Type>,

    /// List of value domains for types
    types_domains: Vec<ValueDomain>,

    /// Symbol table for all predicates.
    predicates_symbols: IndexTable,

    /// Symbol table for all numeric functions.
    functions_symbols: IndexTable,

    /// Symbol table for all objects.
    objects_symbols: IndexTable,

    /// List of all predicates (fluents) in the problem.
    predicates: Vec<Fluent>,

    /// List of all numeric functions in the problem.
    functions: Vec<Function>,

    /// List of all objects in the problem, represented as functions.
    objects: Vec<Function>,
}

impl Problem {
    /// Creates a new empty grounded `Problem` with the given requirements.
    ///
    /// All symbol tables (types, predicates, functions, objects) and vectors
    /// are initialized empty. The domain and problem identifiers are set
    /// to `Ident::default()` and must be assigned later using `set_domain_id`
    /// and `set_problem_id`.
    ///
    /// # Parameters
    /// - `interner`: The `StringInterner` storing all identifiers.
    /// - `requirements`: The set of PDDL requirements for this problem.
    ///
    /// # Returns
    /// A new `Problem` instance ready for incremental construction.
    pub fn new(interner: StringInterner, requirements: HashSet<Requirement>) -> Self {
        Self {
            interner,
            domain_id: Ident::default(),
            problem_id: Ident::default(),
            requirements,
            types_symbols: IndexTable::new(),
            types: Vec::new(),
            types_domains: Vec::new(),
            predicates_symbols: IndexTable::new(),
            functions_symbols: IndexTable::new(),
            objects_symbols: IndexTable::new(),
            predicates: Vec::new(),
            functions: Vec::new(),
            objects: Vec::new(),
        }
    }

    /// Returns the domain name as a string slice.
    ///
    /// # Returns
    /// `Ok(&str)` if the identifier exists in the interner, otherwise `Err(InternerError)`.
    pub fn domain_name(&self) -> Result<&str, InternerError> {
        self.interner.try_resolve_ident(self.domain_id)
    }

    /// Returns the problem name as a string slice.
    ///
    /// # Returns
    /// `Ok(&str)` if the identifier exists in the interner, otherwise `Err(InternerError)`.
    pub fn problem_name(&self) -> Result<&str, InternerError> {
        self.interner.try_resolve_ident(self.problem_id)
    }

    /// Returns the domain identifier.
    pub fn domain_id(&self) -> Ident {
        self.domain_id
    }

    /// Returns the problem identifier.
    pub fn problem_id(&self) -> Ident {
        self.problem_id
    }

    // Sets the domain identifier, validating it exists in the interner.
    pub fn set_domain_id(&mut self, id: Ident) -> Result<(), InternerError> {
        self.interner.try_resolve_ident(id)?;
        self.domain_id = id;
        Ok(())
    }

    /// Sets the problem identifier, validating it exists in the interner.
    pub fn set_problem_id(&mut self, id: Ident) -> Result<(), InternerError> {
        self.interner.try_resolve_ident(id)?;
        self.problem_id = id;
        Ok(())
    }

    /// Returns a reference to the set of requirements.
    ///
    /// # Returns
    /// Reference to `HashSet<Requirement>`.
    pub fn requirements(&self) -> &HashSet<Requirement> {
        &self.requirements
    }

    // ---------- Type symbol table ----------

    /// Returns a reference to the type symbols table.
    pub fn types_symbols(&self) -> &IndexTable {
        &self.types_symbols
    }

    /// Returns a mutable reference to the type symbols table.
    pub fn types_symbols_mut(&mut self) -> &mut IndexTable {
        &mut self.types_symbols
    }

    /// Replaces the current type symbols table with the provided one.
    pub fn set_types_symbols(&mut self, table: IndexTable) {
        self.types_symbols = table;
    }

    // ---------- Type parents ----------

    /// Returns a reference to the list of parent types.
    pub fn types(&self) -> &Vec<Type> {
        &self.types
    }

    /// Returns a mutable reference to the list of parent types.
    pub fn types_mut(&mut self) -> &mut Vec<Type> {
        &mut self.types
    }

    /// Replaces the current type parents list with the provided one.
    pub fn set_types(&mut self, types: Vec<Type>) {
        self.types = types;
    }

    // ---------- Type domains ----------

    /// Returns a reference to the type domains (values associated with each type).
    pub fn types_domains(&self) -> &Vec<ValueDomain> {
        &self.types_domains
    }

    /// Returns a mutable reference to the type domains.
    pub fn types_domains_mut(&mut self) -> &mut Vec<ValueDomain> {
        &mut self.types_domains
    }

    /// Replaces the current type domains with the provided one.
    pub fn set_types_domains(&mut self, domains: Vec<ValueDomain>) {
        self.types_domains = domains;
    }

    // ------------------- PREDICATES -------------------

    /// Returns the symbol table mapping predicate identifiers to indices.
    pub fn predicates_symbols(&self) -> &IndexTable {
        &self.predicates_symbols
    }

    /// Returns the symbol table for predicates, mutable.
    fn predicates_symbols_mut(&mut self) -> &mut IndexTable {
        &mut self.predicates_symbols
    }

    /// Replaces the predicates symbol table with the provided one.
    fn set_predicates_symbols(&mut self, table: IndexTable) {
        self.predicates_symbols = table;
    }

    /// Returns a reference to the list of all predicates.
    pub fn predicates(&self) -> &Vec<Fluent> {
        &self.predicates
    }

    /// Returns a mutable reference to the list of predicates.
    fn predicates_mut(&mut self) -> &mut Vec<Fluent> {
        &mut self.predicates
    }

    /// Replaces the current list of predicates with the provided one.
    fn set_predicates(&mut self, predicates: Vec<Fluent>) {
        self.predicates = predicates;
    }

    // ------------------- FUNCTIONS -------------------

    /// Returns the symbol table mapping numeric function identifiers to indices.
    pub fn functions_symbols(&self) -> &IndexTable {
        &self.functions_symbols
    }

    /// Returns a mutable reference to the functions symbol table.
    fn functions_symbols_mut(&mut self) -> &mut IndexTable {
        &mut self.functions_symbols
    }

    /// Replaces the functions symbol table with the provided one.
    fn set_functions_symbols(&mut self, table: IndexTable) {
        self.functions_symbols = table;
    }

    /// Returns a reference to the list of all numeric functions.
    pub fn functions(&self) -> &Vec<Function> {
        &self.functions
    }

    /// Returns a mutable reference to the list of functions.
    fn functions_mut(&mut self) -> &mut Vec<Function> {
        &mut self.functions
    }

    /// Replaces the current list of functions with the provided one.
    fn set_functions(&mut self, functions: Vec<Function>) {
        self.functions = functions;
    }

    // ------------------- OBJECTS -------------------

    /// Returns the symbol table mapping object identifiers to indices.
    pub fn objects_symbols(&self) -> &IndexTable {
        &self.objects_symbols
    }

    /// Returns a mutable reference to the objects symbol table.
    fn objects_symbols_mut(&mut self) -> &mut IndexTable {
        &mut self.objects_symbols
    }

    /// Replaces the objects symbol table with the provided one.
    fn set_objects_symbols(&mut self, table: IndexTable) {
        self.objects_symbols = table;
    }

    /// Returns a reference to the list of all objects.
    pub fn objects(&self) -> &Vec<Function> {
        &self.objects
    }

    /// Returns a mutable reference to the list of objects.
    fn objects_mut(&mut self) -> &mut Vec<Function> {
        &mut self.objects
    }

    /// Replaces the current list of objects with the provided one.
    fn set_objects(&mut self, objects: Vec<Function>) {
        self.objects = objects;
    }

    /// Returns a reference to the interner.
    ///
    /// # Returns
    /// Reference to `StringInterner`.
    pub fn interner(&self) -> &StringInterner {
        &self.interner
    }
}

impl TryFrom<LiftedProblem> for Problem {
    type Error = GroundingError;

    /// Attempts to construct a grounded `Problem` from a `LiftedProblem`.
    ///
    /// This involves several steps:
    /// 1. Take ownership of shared resources like interner and requirements.
    /// 2. Initialize the grounded `Problem` structure.
    /// 3. Build symbol tables for types, predicates, functions, and objects.
    /// 4. Build the types table using the type symbols.
    /// 5. Build objects (constants and object fluents) based on symbols and types.
    fn try_from(mut lifted_problem: LiftedProblem) -> Result<Self, Self::Error> {
        // --- STEP 1: Take ownership of shared resources ---
        // Extract interner and requirements from the lifted problem
        let interner = lifted_problem.take_interner();
        let requirements = lifted_problem.take_requirements();

        // --- STEP 2: Initialize the grounded problem ---
        // Create an empty Problem with the interner and requirements
        let mut problem = Problem::new(interner, requirements);

        // --- STEP 3: Build symbol tables ---
        // 3a: Types
        let types_symbols = builders::build_type_symbols_table(&lifted_problem);
        problem.set_types_symbols(types_symbols);

        // 3b: Predicates
        let lifted_predicates = lifted_problem.predicates();
        let predicates_symbols = builders::build_predicates_symbols_table(lifted_predicates);
        problem.set_predicates_symbols(predicates_symbols);

        // 3c: Numeric functions
        let lifted_functions = lifted_problem.functions();
        let functions_symbols = builders::build_functions_symbols_table(lifted_functions);
        problem.set_functions_symbols(functions_symbols);

        // 3d: Objects (constants, objects and object fluents)
        //let lifted_constants = lifted_problem.constants();
        //let lifted_objects = lifted_problem.objects();
  /*      let object_symbol_table = builders::build_objects_symbols_table(
            lifted_constants,
            lifted_objects,
            lifted_functions,
        );
        problem.set_objects_symbols(object_symbol_table);*/

        // --- STEP 4: Build the types table ---
        //let types = builders::build_types_table(lifted_types, &problem.types_symbols())?;
        //problem.set_types(types);

        // --- STEP 5: Build grounded objects ---
        // Objects and constants are converted into grounded Function instances
        // Object fluents will be added later during full grounding
        /*let objects_vec = builders::build_objects_table(
            lifted_constants,
            lifted_objects,
            problem.types_symbols(),
            problem.objects_symbols(),
        )?;
        problem.set_objects(objects_vec);*/

        // --- STEP 6: Build the value domains of types of the problem ---
        let types_domains = builders::build_types_domains(
            problem.objects(),
            problem.types_symbols().len(),
        );
        problem.set_types_domains(types_domains);

        // Return the fully initialized (grounded) Problem
        Ok(problem)
    }
}


impl fmt::Display for Problem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Résolution des noms, ignore les erreurs pour l'affichage compact
        let domain_name = self.domain_name().unwrap_or("<unknown>");
        let problem_name = self.problem_name().unwrap_or("<unknown>");

        writeln!(f, "Problem: {} ({})", problem_name, domain_name)?;
        writeln!(f, "Requirements: {:?}", self.requirements)?;

        // Types
        writeln!(f, "Types ({}):", self.types.len())?;
        for ty in &self.types {
            writeln!(f, "  {}", ty)?;
        }

        // Predicates / Fluents
        writeln!(f, "Predicates ({}):", self.predicates.len())?;
        for pred in &self.predicates {
            writeln!(f, "  {}", pred)?;
        }

        // Numeric functions
        writeln!(f, "Functions ({}):", self.functions.len())?;
        for func in &self.functions {
            writeln!(f, "  {}", func)?;
        }

        // Objects
        writeln!(f, "Objects ({}):", self.objects.len())?;
        for obj in &self.objects {
            writeln!(f, "  {}", obj)?;
        }

        Ok(())
    }
}
