use crate::aiplan4rust::grounding::problem::numeric_fluent::NumericFluent;
use crate::aiplan4rust::grounding::problem::registry::value::ValueRegistry;
use crate::aiplan4rust::grounding::problem::Fluent;

use crate::aiplan4rust::cli::io::serialization::SerdeSerializable;
use crate::aiplan4rust::core::interner::{InternerError, SymbolInterner};
use crate::aiplan4rust::lang::ids::{FunctionSymbolId, ObjectId, PredicateSymbolId, TypeId};
use crate::aiplan4rust::lang::{Requirement, SymbolId, TaskSymbolId, TypedList};
use crate::aiplan4rust::lir::problem::skeleton::{
    AtomicFormulaSkeleton, AtomicFunctionSkeleton, AtomicTaskSkeleton,
};
use crate::aiplan4rust::lir::problem::{LiftedProblem, SymbolRegistry};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fmt;

/// Represents a fully grounded PDDL problem.
///
/// This structure stores all information about a PDDL problem, including
/// domain and problem identifiers, requirements, types, predicates, functions,
/// and objects. It also maintains internal symbol tables and an interner for
/// efficient string management.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Problem {
    interner: SymbolInterner,
    domain_name: SymbolId,
    problem_name: SymbolId,
    requirements: HashSet<Requirement>,

    type_symbols: SymbolRegistry<TypeId>,
    object_symbols: SymbolRegistry<ObjectId>,
    predicate_symbols: SymbolRegistry<PredicateSymbolId>,
    function_symbols: SymbolRegistry<FunctionSymbolId>,
    task_symbols: SymbolRegistry<TaskSymbolId>,

    type_defs: TypedList<TypeId, TypeId>,
    object_defs: TypedList<ObjectId, TypeId>,
    predicate_defs: Vec<AtomicFormulaSkeleton>,
    functions_def: Vec<AtomicFunctionSkeleton>,
    task_defs: Vec<AtomicTaskSkeleton>,

    constant_offset: usize,
    /// List of value domains for types
    type_objects: ValueRegistry,

    /// List of fluents (predicates grounded)
    fluents: Vec<Fluent>,

    /// List of numeric-fluents
    numeric_fluents: Vec<NumericFluent>,
}

impl Problem {
    /// Creates a new `Problem` by consuming a `LiftedProblem`.
    ///
    /// This implementation performs an ownership transfer of all internal
    /// data structures, ensuring no deep clones are required.
    pub fn from(mut lifted: LiftedProblem) -> Self {
        // Extract the interner
        let interner = lifted.take_interner();

        Self {
            interner,
            domain_name: lifted.domain_name(),
            problem_name: lifted.problem_name(),
            requirements: lifted.take_requirements(),

            // Symbol Tables: Mapping IDs to their string representations
            // These methods move the individual symbol maps out of the lifted problem
            type_symbols: lifted.take_type_symbols(),
            object_symbols: lifted.take_object_symbols(),
            predicate_symbols: lifted.take_predicate_symbols(),
            function_symbols: lifted.take_function_symbols(),
            task_symbols: lifted.take_task_symbols(),

            // Definitions: The "lifted" blueprints for the domain elements
            type_defs: lifted.take_type_defs(),
            object_defs: lifted.take_object_defs(),
            predicate_defs: lifted.take_predicate_defs(),
            functions_def: lifted.take_function_defs(),
            task_defs: lifted.take_task_defs(),

            // Offsets and Metadata
            constant_offset: lifted.constant_offset(),

            // Value domains and fluents are initialized empty.
            // They serve as placeholders for the subsequent grounding phase.
            type_objects: ValueRegistry::empty(),
            fluents: Vec::new(),
            numeric_fluents: Vec::new(),
        }
    }

    /// Creates a new empty grounded `Problem` with the given requirements.
    ///
    /// All symbol tables (types, predicates, functions, objects) and vectors
    /// are initialized empty. The domain and problem identifiers are set
    /// to `Ident::debug()` and must be assigned later using `set_domain_id`
    /// and `set_problem_id`.
    ///
    ///
    /// # Returns
    /// A new `Problem` instance ready for incremental construction.
    pub fn new() -> Self {
        Self {
            interner: SymbolInterner::new(),
            domain_name: SymbolId::default(),
            problem_name: SymbolId::default(),
            requirements: HashSet::new(),
            type_symbols: SymbolRegistry::new(),
            predicate_symbols: SymbolRegistry::new(),
            predicate_defs: Vec::new(),
            function_symbols: SymbolRegistry::new(),
            functions_def: Vec::new(),
            task_symbols: SymbolRegistry::new(),
            object_symbols: SymbolRegistry::new(),
            type_objects: ValueRegistry::empty(),
            object_defs: TypedList::new(),
            fluents: Vec::new(),
            numeric_fluents: Vec::new(),
            type_defs: TypedList::new(),
            task_defs: Vec::new(),
            constant_offset: 0,
        }
    }

    /// Returns the domain name as a string slice.
    ///
    /// # Returns
    /// `Ok(&str)` if the identifier exists in the interner, otherwise `Err(InternerError)`.
    pub fn domain_name(&self) -> Result<&str, InternerError> {
        self.interner.try_resolve_symbol(self.domain_name)
    }

    /// Returns the problem name as a string slice.
    ///
    /// # Returns
    /// `Ok(&str)` if the identifier exists in the interner, otherwise `Err(InternerError)`.
    pub fn problem_name(&self) -> Result<&str, InternerError> {
        self.interner.try_resolve_symbol(self.problem_name)
    }

    /// Returns the domain identifier.
    pub fn domain_id(&self) -> SymbolId {
        self.domain_name
    }

    /// Returns the problem identifier.
    pub fn problem_id(&self) -> SymbolId {
        self.problem_name
    }

    // Sets the domain identifier, validating it exists in the interner.
    pub fn set_domain_id(&mut self, id: SymbolId) -> Result<(), InternerError> {
        self.interner.try_resolve_symbol(id)?;
        self.domain_name = id;
        Ok(())
    }

    /// Sets the problem identifier, validating it exists in the interner.
    pub fn set_problem_id(&mut self, id: SymbolId) -> Result<(), InternerError> {
        self.interner.try_resolve_symbol(id)?;
        self.problem_name = id;
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

    /// Returns a reference to the typing symbols table.
    pub fn type_symbols(&self) -> &SymbolRegistry<TypeId> {
        &self.type_symbols
    }

    /// Returns a mutable reference to the typing symbols table.
    pub fn type_symbols_mut(&mut self) -> &mut SymbolRegistry<TypeId> {
        &mut self.type_symbols
    }

    /// Replaces the current typing symbols table with the provided one.
    pub fn set_type_symbols(&mut self, table: SymbolRegistry<TypeId>) {
        self.type_symbols = table;
    }

    // ---------- Type parents ----------

    /// Returns a reference to the list of parent types (as `Option<TypeID>`).
    pub fn type_defs(&self) -> &TypedList<TypeId, TypeId> {
        &self.type_defs
    }

    /// Returns a mutable reference to the list of parent types (as `Option<TypeID>`).
    pub fn type_def_mut(&mut self) -> &mut TypedList<TypeId, TypeId> {
        &mut self.type_defs
    }

    /// Replaces the current typing parents list with the provided one.
    pub fn set_type_defs(&mut self, types: TypedList<TypeId, TypeId>) {
        self.type_defs = types;
    }

    // ---------- Type objects ----------

    /// Returns a reference to the typing domains (values associated with each typing).
    pub fn type_objects(&self) -> &ValueRegistry {
        &self.type_objects
    }

    /// Returns a mutable reference to the typing domains.
    pub fn type_objects_mut(&mut self) -> &mut ValueRegistry {
        &mut self.type_objects
    }

    /// Replaces the current typing domains with the provided one.
    pub fn set_type_objects(&mut self, registry: ValueRegistry) {
        self.type_objects = registry;
    }

    // ------------------- PREDICATES -------------------

    /// Returns the symbol table mapping predicate identifiers to indices.
    pub fn predicate_symbols(&self) -> &SymbolRegistry<PredicateSymbolId> {
        &self.predicate_symbols
    }

    /// Returns the symbol table for predicates, mutable.
    fn predicate_symbols_mut(&mut self) -> &mut SymbolRegistry<PredicateSymbolId> {
        &mut self.predicate_symbols
    }

    /// Replaces the predicates symbol table with the provided one.
    fn set_predicate_symbols(&mut self, table: SymbolRegistry<PredicateSymbolId>) {
        self.predicate_symbols = table;
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
    pub fn function_symbols(&self) -> &SymbolRegistry<FunctionSymbolId> {
        &self.function_symbols
    }

    /// Returns a mutable reference to the functions symbol table.
    fn function_symbols_mut(&mut self) -> &mut SymbolRegistry<FunctionSymbolId> {
        &mut self.function_symbols
    }

    /// Replaces the functions symbol table with the provided one.
    fn set_function_symbols(&mut self, table: SymbolRegistry<FunctionSymbolId>) {
        self.function_symbols = table;
    }

    /// Returns a reference to the list of all **numeric fluents** (functions that return numeric values).
    ///
    /// Each `NumericFluent` represents a grounded numeric function in the problem,
    /// including its symbol, parameters, and return typing.
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
    pub fn object_symbols(&self) -> &SymbolRegistry<ObjectId> {
        &self.object_symbols
    }

    /// Returns a mutable reference to the objects symbol table.
    fn object_symbols_mut(&mut self) -> &mut SymbolRegistry<ObjectId> {
        &mut self.object_symbols
    }

    /// Replaces the objects symbol table with the provided one.
    fn set_object_symbols(&mut self, table: SymbolRegistry<ObjectId>) {
        self.object_symbols = table;
    }

    /// Returns a reference to the list of all objects.
    pub fn object_defs(&self) -> &TypedList<ObjectId, TypeId> {
        &self.object_defs
    }

    /// Returns a mutable reference to the list of objects.
    fn object_defs_mut(&mut self) -> &mut TypedList<ObjectId, TypeId> {
        &mut self.object_defs
    }

    /// Replaces the current list of objects with the provided one.
    fn set_object_defs(&mut self, objects: TypedList<ObjectId, TypeId>) {
        self.object_defs = objects;
    }

    /// Returns a reference to the interner.
    ///
    /// # Returns
    /// Reference to `StringInterner`.
    pub fn interner(&self) -> &SymbolInterner {
        &self.interner
    }

    /// Returns a mutable reference to the interner.
    ///
    /// # Returns
    /// Mutable reference to `StringInterner`.
    pub fn interner_mut(&mut self) -> &mut SymbolInterner {
        &mut self.interner
    }
}

impl fmt::Display for Problem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let domain_name = self.domain_name().unwrap_or("<unknown>");
        let problem_name = self.problem_name().unwrap_or("<unknown>");

        writeln!(f, "Domain: {}\nProblem: {}\n", domain_name, problem_name)?;

        if self.requirements.is_empty() {
            writeln!(f, "Requirements: none")?;
        } else {
            writeln!(f, "Requirements:")?;
            for req in &self.requirements {
                writeln!(f, "{}", req)?;
            }
        }

        // ---------- Types ----------
        writeln!(f, "\nTypes Symbols Table:")?;
        write!(f, "{}", self.type_symbols())?;

        // ---------- Predicates ----------
        writeln!(f, "\nPredicates Symbols Table:")?;
        write!(f, "{}", self.predicate_symbols())?;

        // ---------- Functions ----------
        writeln!(f, "\nFunctions Symbols Table:")?;
        write!(f, "{}", self.function_symbols())?;

        // ---------- Objects ----------
        writeln!(f, "\nObjects Symbols Table:")?;
        write!(f, "{}", self.object_symbols())?;

        // ---------- Type definitions ----------
        writeln!(f, "\nType definition:")?;
        if self.type_defs().is_empty() {
            writeln!(f, "<None>")?;
        } else {
            for (idx, values) in self.type_defs().iter().enumerate() {
                writeln!(f, "{}: {}", idx, values)?;
            }
        }

        // ---------- Object Type Domains ----------
        writeln!(f, "\nType Domains Table:\n{}\n", self.type_objects())?;

        // ---------- Fluents ----------
        writeln!(f, "\nFluents Table:")?;
        if self.fluents().is_empty() {
            writeln!(f, "<None>")?;
        } else {
            for (idx, _fluent) in self.fluents().iter().enumerate() {
                write!(f, "{}: ", idx)?;
                //self.fmt_fluent_with_interner(f, fluent)?;
                writeln!(f)?;
            }
        }

        Ok(())
    }
}

/*impl Problem {
    /// Format a single ObjectFluent using the problem's symbol tables.
    pub fn fmt_object_fluent_with_interner<W: fmt::Write>(
        &self,
        f: &mut W,
        object_fluent: &ObjectFluent,
    ) -> fmt::Result {
        // Récupérer le nom du fluent
        let fluent_name = self
            .functions_symbols()
            .get_string(object_fluent.symbol())
            .unwrap_or("<unknown-fluent>");

        write!(f, "({}", fluent_name)?;

        // Parcourir les paramètres (toujours ObjectID)
        for obj_id in object_fluent.arguments() {
            let obj_name = self
                .objects_symbols()
                .get_string(*obj_id)
                .unwrap_or("<unknown-object>");
            write!(f, " {}", obj_name)?;
        }

        write!(f, ")")?;
        Ok(())
    }

    /// Format a single Fluent using the problem's symbol tables.
    pub fn fmt_fluent_with_interner<W: fmt::Write>(
        &self,
        f: &mut W,
        fluent: &Fluent,
    ) -> fmt::Result {
        let predicate_name = self
            .predicates_symbols()
            .get_string(fluent.symbol())
            .unwrap_or("<unknown-predicate>");
        write!(f, "({}", predicate_name)?;

        for param in fluent.parameters() {
            match param {
                ArgumentID::Object(obj_id) => {
                    let param_str = self
                        .objects_symbols()
                        .get_string(*obj_id)
                        .unwrap_or("<unknown-parameter>");
                    write!(f, " {}", param_str)?;
                }
                ArgumentID::ObjectFluent(obj_fluent_id) => {
                    // Utiliser la fonction dédiée pour ObjectFluent
                    let obj_fluent = self
                        .objects_fluents()
                        .get(obj_fluent_id.as_usize());

                    if let Some(of) = obj_fluent {
                        self.fmt_object_fluent_with_interner(f, of)?;
                    } else {
                        write!(f, " <unknown-object-fluent>")?;
                    }
                }
            }
        }

        write!(f, ")")?;
        Ok(())
    }

    /// Convert an ObjectFluent into a String.
    pub fn to_string_object_fluent(&self, object_fluent: &ObjectFluent) -> String {
        let mut s = String::new();
        self.fmt_object_fluent_with_interner(&mut s, object_fluent)
            .unwrap_or_else(|_| s.push_str("<object-fluent-format-error>"));
        s
    }

    /// Convert a Fluent into a String.
    pub fn to_string_fluent(&self, fluent: &Fluent) -> String {
        let mut s = String::new();
        self.fmt_fluent_with_interner(&mut s, fluent)
            .unwrap_or_else(|_| s.push_str("<fluent-format-error>"));
        s
    }
}*/

impl SerdeSerializable for Problem {}
