use std::fmt;
use std::collections::HashSet;
use serde::{Deserialize, Serialize};
use crate::aiplan4rust::grounding::error::GroundingError;
use crate::aiplan4rust::grounding::problem::fluent::Fluent;
use crate::aiplan4rust::grounding::problem::function::Function;
use crate::aiplan4rust::grounding::problem::index_table::IndexTable;
use crate::aiplan4rust::grounding::problem::ty::Ty;
use crate::aiplan4rust::interner::{Ident, InternerError, StringInterner};
use crate::aiplan4rust::lang::Requirement;
use crate::aiplan4rust::lir::problem::LiftedProblem;

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

    /// Symbol table for all predicates.
    predicates_symbols: IndexTable,

    /// Symbol table for all numeric functions.
    functions_symbols: IndexTable,

    /// Symbol table for all objects.
    objects_symbols: IndexTable,

    /// List of all types in the problem.
    types: Vec<Ty>,

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
            predicates_symbols: IndexTable::new(),
            functions_symbols: IndexTable::new(),
            objects_symbols: IndexTable::new(),
            types: Vec::new(),
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

    /// Returns a reference to all types.
    ///
    /// # Returns
    /// Reference to a `Vec<Ty>`.
    pub fn types(&self) -> &Vec<Ty> {
        &self.types
    }

    /// Returns a reference to all predicates (fluents).
    ///
    /// # Returns
    /// Reference to a `Vec<Fluent>`.
    pub fn predicates(&self) -> &Vec<Fluent> {
        &self.predicates
    }

    /// Returns a reference to all numeric functions.
    ///
    /// # Returns
    /// Reference to a `Vec<Function>`.
    pub fn functions(&self) -> &Vec<Function> {
        &self.functions
    }

    /// Returns a reference to all objects.
    ///
    /// # Returns
    /// Reference to a `Vec<Function>`.
    pub fn objects(&self) -> &Vec<Function> {
        &self.objects
    }

    /// Returns a reference to the interner.
    ///
    /// # Returns
    /// Reference to `StringInterner`.
    pub fn interner(&self) -> &StringInterner {
        &self.interner
    }

    /// Returns a reference to the types symbol table.
    ///
    /// # Returns
    /// Reference to `IndexTable` mapping types to indices.
    pub fn types_symbols(&self) -> &IndexTable {
        &self.types_symbols
    }

    /// Returns a reference to the predicates symbol table.
    ///
    /// # Returns
    /// Reference to `IndexTable` mapping predicates to indices.
    pub fn predicates_symbols(&self) -> &IndexTable {
        &self.predicates_symbols
    }

    /// Returns a reference to the functions symbol table.
    ///
    /// # Returns
    /// Reference to `IndexTable` mapping functions to indices.
    pub fn functions_symbols(&self) -> &IndexTable {
        &self.functions_symbols
    }

    /// Returns a reference to the objects symbol table.
    ///
    /// # Returns
    /// Reference to `IndexTable` mapping objects to indices.
    pub fn objects_symbols(&self) -> &IndexTable {
        &self.objects_symbols
    }
}

impl TryFrom<LiftedProblem> for Problem {
    type Error = GroundingError;

    /// Attempts to construct a grounded `Problem` from a `LiftedProblem`.
    ///
    /// This consumes the lifted problem's interner and requirements,
    /// initializing a new grounded problem with empty collections for types,
    /// predicates, functions, and objects.
    fn try_from(mut lifted_problem: LiftedProblem) -> Result<Self, Self::Error> {
        // Take ownership of the interner and requirements
        let interner = lifted_problem.take_interner();
        let requirements = lifted_problem.take_requirements();

        // Construct a new grounded problem
        Ok(Problem::new(interner, requirements))
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
