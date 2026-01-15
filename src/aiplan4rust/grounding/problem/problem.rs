use crate::aiplan4rust::grounding::error::GroundingError;
use crate::aiplan4rust::grounding::problem::builders;
use crate::aiplan4rust::grounding::problem::Fluent;
use crate::aiplan4rust::grounding::problem::SymbolTable;
use crate::aiplan4rust::interner::{Ident, InternerError, StringInterner};
use crate::aiplan4rust::lang::Requirement;
use crate::aiplan4rust::lir::problem::LiftedProblem;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fmt;
use std::rc::Rc;
use crate::aiplan4rust::grounding::problem::ids::{FunctionID, ObjectID, PredicateID, TypeID};
use crate::aiplan4rust::grounding::problem::numeric_fluent::NumericFluent;
use crate::aiplan4rust::grounding::problem::object::Object;
use crate::aiplan4rust::grounding::problem::object_fluent::ObjectFluent;
use crate::aiplan4rust::grounding::problem::value_domain::ValueDomain;
use crate::aiplan4rust::lir::problem::flatten::flatten;

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
    types_symbols: SymbolTable<TypeID>,

    /// Symbol table for all predicates.
    predicates_symbols: SymbolTable<PredicateID>,

    /// Symbol table for all numeric functions.
    functions_symbols: SymbolTable<FunctionID>,

    /// Symbol table for all objects.
    objects_symbols: SymbolTable<ObjectID>,

    /// List of types of the problem
    type_parents_table: Vec<TypeID>,

    /// List of value domains for types
    types_domains: Vec<ValueDomain>,

    /// List of objects
    objects: Vec<Object>,

    /// List of fluents (predicates grounded)
    fluents: Vec<Fluent>,

    /// List of object-fluents
    objects_fluents: Vec<ObjectFluent>,

    /// List of numeric-fluents
    numeric_fluents: Vec<NumericFluent>,
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
        let rc_interner = Rc::new(interner);
        Self {
            interner: Rc::try_unwrap(rc_interner.clone())
                .unwrap_or_else(|rc| (*rc).clone()),
            domain_id: Ident::default(),
            problem_id: Ident::default(),
            requirements,
            types_symbols: SymbolTable::new(Rc::clone(&rc_interner)),
            predicates_symbols: SymbolTable::new(Rc::clone(&rc_interner)),
            functions_symbols: SymbolTable::new(Rc::clone(&rc_interner)),
            objects_symbols: SymbolTable::new(Rc::clone(&rc_interner)),
            type_parents_table: Vec::new(),
            types_domains: Vec::new(),
            objects: Vec::new(),
            fluents: Vec::new(),
            objects_fluents: Vec::new(),
            numeric_fluents: Vec::new(),
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
    pub fn types_symbols(&self) -> &SymbolTable<TypeID> {
        &self.types_symbols
    }

    /// Returns a mutable reference to the type symbols table.
    pub fn types_symbols_mut(&mut self) -> &mut SymbolTable<TypeID> {
        &mut self.types_symbols
    }

    /// Replaces the current type symbols table with the provided one.
    pub fn set_types_symbols(&mut self, table: SymbolTable<TypeID>) {
        self.types_symbols = table;
    }

    // ---------- Type parents ----------

    /// Returns a reference to the list of parent types.
    pub fn type_parent_table(&self) -> &Vec<TypeID> {
        &self.type_parents_table
    }

    /// Returns a mutable reference to the list of parent types.
    pub fn type_parent_table_mut(&mut self) -> &mut Vec<TypeID> {
        &mut self.type_parents_table
    }

    /// Replaces the current type parents list with the provided one.
    pub fn set_type_parent_table(&mut self, types: Vec<TypeID>) {
        self.type_parents_table = types;
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
    pub fn predicates_symbols(&self) -> &SymbolTable<PredicateID> {
        &self.predicates_symbols
    }

    /// Returns the symbol table for predicates, mutable.
    fn predicates_symbols_mut(&mut self) -> &mut SymbolTable<PredicateID> {
        &mut self.predicates_symbols
    }

    /// Replaces the predicates symbol table with the provided one.
    fn set_predicates_symbols(&mut self, table: SymbolTable<PredicateID>) {
        self.predicates_symbols = table;
    }

    /// Returns a reference to the list of all predicates.
    pub fn fluents(&self) -> &Vec<Fluent> {
        &self.fluents
    }

    /// Returns a mutable reference to the list of predicates.
    fn fluents_mut(&mut self) -> &mut Vec<Fluent> {
        &mut self.fluents
    }

    /// Replaces the current list of predicates with the provided one.
    fn set_fluents(&mut self, fluents: Vec<Fluent>) {
        self.fluents = fluents;
    }

    // ------------------- FUNCTIONS -------------------

    /// Returns the symbol table mapping numeric function identifiers to indices.
    pub fn functions_symbols(&self) -> &SymbolTable<FunctionID> {
        &self.functions_symbols
    }

    /// Returns a mutable reference to the functions symbol table.
    fn functions_symbols_mut(&mut self) -> &mut SymbolTable<FunctionID> {
        &mut self.functions_symbols
    }

    /// Replaces the functions symbol table with the provided one.
    fn set_functions_symbols(&mut self, table: SymbolTable<FunctionID>) {
        self.functions_symbols = table;
    }

    /// Returns a reference to the list of all **numeric fluents** (functions that return numeric values).
    ///
    /// Each `NumericFluent` represents a grounded numeric function in the problem,
    /// including its symbol, parameters, and return type.
    pub fn numeric_fluents(&self) -> &Vec<NumericFluent> {
        &self.numeric_fluents
    }

    /// Returns a mutable reference to the list of **numeric fluents**.
    ///
    /// Allows modifying the collection of numeric functions directly.
    fn numeric_fluents_mut(&mut self) -> &mut Vec<NumericFluent> {
        &mut self.numeric_fluents
    }

    /// Replaces the current list of **numeric fluents** with the provided vector.
    ///
    /// # Parameters
    /// - `numeric_fluents`: A vector of `NumericFluent` objects to replace the current list.
    fn set_numeric_fluents(&mut self, numeric_fluents: Vec<NumericFluent>) {
        self.numeric_fluents = numeric_fluents;
    }

    // ------------------- OBJECTS -------------------

    /// Returns the symbol table mapping object identifiers to indices.
    pub fn objects_symbols(&self) -> &SymbolTable<ObjectID> {
        &self.objects_symbols
    }

    /// Returns a mutable reference to the objects symbol table.
    fn objects_symbols_mut(&mut self) -> &mut SymbolTable<ObjectID> {
        &mut self.objects_symbols
    }

    /// Replaces the objects symbol table with the provided one.
    fn set_objects_symbols(&mut self, table: SymbolTable<ObjectID>) {
        self.objects_symbols = table;
    }

    /// Returns a reference to the list of all objects.
    pub fn objects(&self) -> &Vec<Object> {
        &self.objects
    }

    /// Returns a mutable reference to the list of objects.
    fn objects_mut(&mut self) -> &mut Vec<Object> {
        &mut self.objects
    }

    /// Replaces the current list of objects with the provided one.
    fn set_objects(&mut self, objects: Vec<Object>) {
        self.objects = objects;
    }

    /// Returns a reference to the list of all object-fluents.
    pub fn objects_fluents(&self) -> &Vec<ObjectFluent> {
        &self.objects_fluents
    }

    /// Returns a mutable reference to the list of object-fluents.
    pub fn objects_fluents_mut(&mut self) -> &mut Vec<ObjectFluent> {
        &mut self.objects_fluents
    }

    /// Replaces the current list of object-fluents with the provided one.
    pub fn set_objects_fluents(&mut self, object_fluents: Vec<ObjectFluent>) {
        self.objects_fluents = object_fluents;
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

        flatten::flatten_types(&mut lifted_problem)?;

        let interner = lifted_problem.take_interner();
        let requirements = lifted_problem.take_requirements();
        let mut problem = Problem::new(interner, requirements);

        builders::build_type_symbols_table(&lifted_problem, problem.types_symbols_mut());
        builders::build_predicates_symbols_table(&lifted_problem, problem.predicates_symbols_mut());
        builders::build_functions_symbols_table(&lifted_problem, problem.functions_symbols_mut());
        builders::build_objects_symbols_table(&lifted_problem, problem.objects_symbols_mut());

        let types = builders::build_type_parent_table(&lifted_problem, &problem.types_symbols_mut())?;
        problem.set_type_parent_table(types);

        let objects = builders::build_objects_table(
            &lifted_problem,
            problem.objects_symbols(),
            problem.types_symbols(),
        )?;
        problem.set_objects(objects);

        let mut types_domains = builders::build_object_type_value_domains_table(
            &lifted_problem,
            problem.objects_symbols(),
            problem.functions_symbols(),
            problem.types_symbols(),
        )?;

        // Optionnel : mettre à jour les ValueDomain avec les ObjectFluentID
        let object_fluents_table = builders::build_object_fluents_table(
            &lifted_problem,
            problem.functions_symbols(),
            problem.types_symbols(),
            &mut types_domains
        )?;

        builders::build_object_fluent_type_value_domain(&mut types_domains, &object_fluents_table);
        problem.set_types_domains(types_domains);

        let fluents = builders::build_fluents_table(
            &lifted_problem,
            problem.predicates_symbols(),
            problem.types_symbols(),
            problem.types_domains()
        )?;
        problem.set_fluents(fluents);

        Ok(problem)
    }
}


impl fmt::Display for Problem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let domain_name = self.domain_name().unwrap_or("<unknown>");
        let problem_name = self.problem_name().unwrap_or("<unknown>");

        writeln!(f, "Problem: {} (Domain: {})", problem_name, domain_name)?;
        writeln!(f, "Requirements: {:?}", self.requirements)?;

        // ---------- Types ----------
        writeln!(f, "\nTypes ({}):", self.types_symbols().len())?;
        writeln!(f, "{}", self.types_symbols())?;

        // ---------- Predicates ----------
        writeln!(f, "\nPredicates ({}):", self.predicates_symbols().len())?;
        writeln!(f, "{}", self.predicates_symbols())?;

        // ---------- Functions ----------
        writeln!(f, "\nFunctions ({}):", self.functions_symbols().len())?;
        writeln!(f, "{}", self.functions_symbols())?;

        // ---------- Objects ----------
        writeln!(f, "\nObjects ({}):", self.objects_symbols().len())?;
        writeln!(f, "{}", self.objects_symbols())?;

        /*// ---------- Object-Fluents ----------
        writeln!(f, "\nObject-Fluents ({}):", self.objects_fluents.len())?;
        for of in &self.objects_fluents {
            let name = self.functions_symbols.get_string(of.symbol).unwrap_or("<unknown>");
            let args: Vec<String> = of.parameters
                .iter()
                .map(|p| self.objects_symbols.get_string(*p).unwrap_or("<unknown>").to_string())
                .collect();
            let ret_type = self.types_symbols.get_string(of.ty).unwrap_or("<unknown>");
            writeln!(f, "  {}({}) : {}", name, args.join(", "), ret_type)?;
        }*/

        Ok(())
    }
}
