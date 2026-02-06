//! This module defines the tree data structures for representing **lifted** syntax problems
//! in hierarchical task network (HTN) and classical syntax domains.
//!
//! The primary struct [`Problem`] models a **lifted syntax problem**, meaning
//! that actions, methods, predicates, and tasks are represented with parameters (variables)
//! rather than fully instantiated ground elements.
//!
//! It captures all components necessary to specify a syntax problem instance at
//! the lifted level:
//!
//! - The **domain and problem identifiers** (`Ident`).
//! - The set of **requirements** (features used in the domain).
//! - The **types**, **constants**, and **objects** defining the domain vocabulary.
//! - The **lifted predicates** and **functions**, expressed as atomic formula skeletons.
//! - The **lifted actions** and **methods**, representing parametrized operators and HTN methods.
//! - The **lifted tasks**, forming the task skeletons for hierarchical syntax.
//! - The **initial state** and **goal conditions** expressed as symbolic expressions (`Expr`).
//! - The **global domain constraints** and **problem-specific constraints**.
//! - The **metric and length specifications** for optimization and bounding.
//! - The **initial task network**, describing the starting point of hierarchical tasks.
//!
//! This lifted representation enables symbolic reasoning and efficient syntax
//! by deferring grounding (instantiation) to a later phase.
//!
//! The module supports serialization/deserialization with `serde` for persistence and interoperability.
//!
//! # Example
//! ```
//! use aiplan4rust::lir::problem::Problem;
//! use aiplan4rust::lang::Ident;
//!
//! let mut problem = Problem::default();
//! problem.set_domain_name(Ident::new("blocksworld"));
//! problem.set_problem_name(Ident::new("tower"));
//!
//! assert_eq!(problem.domain_name().as_str(), "blocksworld");
//! assert_eq!(problem.problem_name().as_str(), "tower");
//! ```
//!
//! This module is essential for representing lifted HTN and classical syntax problems
//! before grounding and solving.

use crate::aiplan4rust::interner::{InternerError, StringInterner};
use crate::aiplan4rust::lang::{AtomSkeletonID, FunctionSkeletonID, FunctorID, ObjectID, PredicateID, Requirement, StringID, TaskSkeletonID, TaskSymbolID, Type, TypeID, TypedSymbol};
use crate::aiplan4rust::lir::atomic_skeleton::{
    AtomicFormulaSkeleton, AtomicFunctionSkeleton, AtomicTaskSkeleton,
};
use crate::aiplan4rust::lir::expr::Expr;
use crate::aiplan4rust::lir::problem::{normalize, InitialTaskNetwork, LiftedAction, LiftedDerivedPredicate, LiftedDurativeAction, LiftedMethod, LiftedProblem};
use crate::aiplan4rust::lir::problem::{DomainDef, ProblemDef};
use crate::aiplan4rust::lir::{renderers, LirError};
use crate::aiplan4rust::serialization::serde::SerdeSerializable;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fmt;
use std::fmt::{Display, Formatter};
use itertools::Itertools;
use crate::aiplan4rust::grounding::problem::SymbolTable;
use crate::aiplan4rust::linking::LinkedSemanticContext;
use crate::aiplan4rust::lir::problem::encode::{encoder, EncodingRegistry};
use crate::aiplan4rust::lir::renderers::LiftedSyntaxDisplay;

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Problem {
    interner: StringInterner,

    domain_name: StringID,
    problem_name: StringID,

    requirements: HashSet<Requirement>,

    type_symbols: SymbolTable<TypeID>,
    types: Vec<TypedSymbol<TypeID, TypeID>>,


    object_symbols: SymbolTable<ObjectID>,
    objects: Vec<TypedSymbol<ObjectID, TypeID>>,
    constant_offset: usize,

    predicates: SymbolTable<PredicateID>,
    atom_skeletons: Vec<AtomicFormulaSkeleton>,

    functors: SymbolTable<FunctorID>,
    function_skeletons: Vec<AtomicFunctionSkeleton>,

    task_symbols: SymbolTable<TaskSymbolID>,
    task_skeletons: Vec<AtomicTaskSkeleton>,



    domain_constraints: Expr,

    /// The list of derived predicates defined in this syntax problem.
    derived_predicates: Vec<LiftedDerivedPredicate>,

    /// The list of actions defined in this syntax problem.
    actions: Vec<LiftedAction>,

    /// The list of durative actions defined in this syntax problem.
    durative_actions: Vec<LiftedDurativeAction>,

    /// The list of methods defined in this syntax problem.
    methods: Vec<LiftedMethod>,


    /// The initial state of the problem.
    init: Expr,

    /// The goal of the problem.
    goal: Expr,

    /// The constraints defined in the problem, i.e., the constraints specific to an instance.
    /// No constraints are represented by an empty and `Expr'.
    problem_constraints: Expr,

    /// The metric specification of the problem.
    metric_spec: Expr,

    /// The length specification of the problem.
    /// The length-spec is deprecated since PDDL 2.1.
    length_spec: Expr,

    /// The initial task network of the problem.
    initial_task_network: InitialTaskNetwork,
}

#[allow(dead_code)]
impl Problem {
    /// Creates a new empty `PlanningProblem` with default identifiers
    /// and no requirements, types, constants, predicates, functions, or actions.
    ///
    /// # Example
    ///
    /// ```
    /// use aiplan4rust::aiplan4rust::lir::problem::LiftedProblem;
    /// let problem = LiftedProblem::new();
    /// assert!(problem.actions().is_empty());
    /// assert!(problem.type_symbol_table().is_empty());
    /// ```
    pub(crate) fn new(interner : StringInterner, requirements: HashSet<Requirement>) -> Self {
        Self {
            interner,
            domain_name: StringID::default(),
            problem_name: StringID::default(),
            requirements,
            type_symbols: SymbolTable::new(),
            types: Vec::new(),
            object_symbols: SymbolTable::new(),
            objects: Vec::new(),
            constant_offset: 0,
            predicates: SymbolTable::new(),
            atom_skeletons: Vec::new(),
            functors: SymbolTable::new(),
            function_skeletons: Vec::new(),
            task_symbols: SymbolTable::new(),
            task_skeletons: Vec::new(),
            domain_constraints: Expr::empty_or(),
            derived_predicates: Vec::new(),
            actions: Vec::new(),
            durative_actions: Vec::new(),
            methods: Vec::new(), // Add for HDDL
            init: Expr::empty_and(),
            goal: Expr::empty_or(),
            problem_constraints: Expr::empty_or(),
            metric_spec: Expr::metric_none(),
            length_spec: Expr::empty_length_spec(),
            initial_task_network: InitialTaskNetwork::default(), // Add for HDDL
        }
    }

    pub fn interner(&self) -> &StringInterner {
        &self.interner
    }

    pub fn interner_mut(&mut self) -> &mut StringInterner {
        &mut self.interner
    }

    pub fn set_interner(&mut self, interner: StringInterner) {
        self.interner = interner;
    }

    pub fn take_interner(&mut self) -> StringInterner {
        std::mem::take(&mut self.interner)
    }

    pub fn domain_id(&self) -> StringID {
        self.domain_name
    }

    pub fn set_domain_id(&mut self, id: StringID) -> Result<(), InternerError> {
        self.interner.try_resolve_ident(id)?;
        self.domain_name = id;
        Ok(())
    }

    pub fn domain_name(&self) -> Result<&str, InternerError> {
        self.interner.try_resolve_ident(self.domain_name)
    }

    pub fn problem_id(&self) -> StringID {
        self.problem_name.clone()
    }

    pub fn set_problem_id(&mut self, id: StringID) -> Result<(), InternerError> {
        self.interner.try_resolve_ident(id)?;
        self.problem_name = id;
        Ok(())
    }

    pub fn problem_name(&self) -> Result<&str, InternerError> {
        self.interner.try_resolve_ident(self.problem_name)
    }

    pub fn requirements(&self) -> &HashSet<Requirement> {
        &self.requirements
    }

    pub fn requirements_mut(&mut self) -> &mut HashSet<Requirement> {
        &mut self.requirements
    }

    pub fn add_requirement(&mut self, requirement: Requirement) {
        self.requirements.insert(requirement);
    }

    pub fn add_requirements<I>(&mut self, iter: I)
    where
        I: IntoIterator<Item = Requirement>,
    {
        self.requirements.extend(iter);
    }

    pub fn is_required(&self, requirement: Requirement) -> bool {
        self.requirements.contains(&requirement)
    }

    pub fn set_requirements(&mut self, new_requirements: HashSet<Requirement>) {
        self.requirements = new_requirements;
    }

    pub fn take_requirements(&mut self) -> HashSet<Requirement> {
        std::mem::take(&mut self.requirements)
    }

    pub fn type_symbol_table(&self) -> &SymbolTable<TypeID> {
        &self.type_symbols
    }

    pub fn add_type_symbol(&mut self, symbol: StringID) -> TypeID {

        let id = self.type_symbols.insert(symbol);
        let idx = id.as_usize();
        if (idx >= self.types.len()) {
            let typed = TypedSymbol::new(id, Type::default());
            self.types.push(typed);

        }
        id
    }


    pub fn types(&self) -> &[TypedSymbol<TypeID, TypeID>] {
        &self.types
    }

    pub fn has_types(&self) -> bool {
        !self.types.is_empty()
    }

    pub fn add_type(&mut self, ty: TypedSymbol<TypeID, TypeID>) -> Result<TypeID, LirError> {
        let id = ty.symbol();
        let idx = id.as_usize();

        // 1. Vérification stricte : l'ID doit avoir été pré-alloué en Phase 1
        // Si l'index dépasse, c'est une erreur de cohérence entre les deux phases.
        //if idx >= self.types.len() {
            // On utilise l'erreur VariableNotFound (ou une erreur plus spécifique si tu préfères)
        //    return Err(LirError::type_not_found(;
        //}

        // 2. Mise à jour de la réservation par la définition réelle
        // On écrase le TypedSymbol "vide" par celui contenant les membres/parents
        self.types[idx] = ty;

        Ok(id)
    }

    /*pub fn add_types<I>(&mut self, iter: I)
    where
        I: IntoIterator<Item = TypedSymbol<TypeID, TypeID>>,
    {
        for ty in iter {
            self.add_type(ty);
        }
    }*/

    pub fn object_symbol_table(&self) -> &SymbolTable<ObjectID> {
        &self.object_symbols
    }

    pub fn add_object_symbol(&mut self, symbol: StringID) -> ObjectID {
        let id = self.object_symbols.insert(symbol);
        let idx = id.as_usize();
        if (idx >= self.objects.len()) {
            self.objects.push(TypedSymbol::new(id, Type::default()));
        }
        id
    }

    pub fn objects(&self) -> &[TypedSymbol<ObjectID, TypeID>] {
        &self.objects
    }

    pub fn domain_constants(&self) -> &[TypedSymbol<ObjectID, TypeID>] {
        &self.objects[..self.constant_offset]
    }

    pub fn has_domain_constants(&self) -> bool {
        self.constant_offset > 0
    }

    pub fn problem_objects(&self) -> &[TypedSymbol<ObjectID, TypeID>] {
        &self.objects[self.constant_offset..]
    }

    pub fn has_problem_objects(&self) -> bool {
        self.objects.len() > self.constant_offset
    }

    pub fn add_object(&mut self, obj: TypedSymbol<ObjectID, TypeID>) -> ObjectID {
        let id = obj.symbol();
        let idx = id.as_usize();

        self.objects[idx] = obj;

        id
    }

    /*pub fn add_objects<I>(&mut self, iter: I)
    where
        I: IntoIterator<Item = TypedSymbol<ObjectID, TypeID>>,
    {
        for obj in iter {
            self.add_object(obj);
        }
    }*/

    pub fn set_constant_offset(&mut self) {
        self.constant_offset = self.object_symbols.len();
    }

    pub fn is_constant(&self, id: ObjectID) -> bool {
        id.as_usize() < self.constant_offset
    }

    pub fn is_object(&self, id: ObjectID) -> bool {
        let idx = id.as_usize();
        idx >= self.constant_offset && idx < self.objects.len()
    }

    pub fn predicate_symbol_table(&self) -> &SymbolTable<PredicateID> {
        &self.predicates
    }

    pub fn atom_skeletons(&self) -> &[AtomicFormulaSkeleton] {
        &self.atom_skeletons
    }

    pub fn has_predicates(&self) -> bool {
        !self.atom_skeletons.is_empty()
    }

    pub fn add_atom_skeleton(&mut self, atom_skeleton: AtomicFormulaSkeleton) -> (PredicateID, AtomSkeletonID) {
        let predicate_id = self.predicates.insert(atom_skeleton.symbol());
        let skeleton_id = AtomSkeletonID::from(self.atom_skeletons.len());
        self.atom_skeletons.push(atom_skeleton);
        (predicate_id, skeleton_id)
    }

    pub fn add_atom_skeletons<I>(&mut self, iter: I)
    where
        I: IntoIterator<Item = AtomicFormulaSkeleton>,
    {
        for atom in iter {
            self.add_atom_skeleton(atom);
        }
    }

    pub fn functor_symbol_table(&self) -> &SymbolTable<FunctorID> {
        &self.functors
    }

    pub fn function_skeletons(&self) -> &[AtomicFunctionSkeleton] {
        &self.function_skeletons
    }

    pub fn has_functions(&self) -> bool {
        !self.function_skeletons.is_empty()
    }

    pub fn add_function_skeleton(&mut self, function: AtomicFunctionSkeleton) -> (FunctorID, FunctionSkeletonID) {
        let functor_id = self.functors.insert(function.symbol());
        let skeleton_id = FunctionSkeletonID::from(self.function_skeletons.len());
        self.function_skeletons.push(function);
        (functor_id, skeleton_id)
    }

    pub fn add_function_skeletons<I>(&mut self, iter: I)
    where
        I: IntoIterator<Item = AtomicFunctionSkeleton>,
    {
        for function in iter {
            self.add_function_skeleton(function);
        }
    }

    pub fn task_symbol_table(&self) -> &SymbolTable<TaskSymbolID> {
        &self.task_symbols
    }

    pub fn task_skeletons(&self) -> &[AtomicTaskSkeleton] {
        &self.task_skeletons
    }

    pub fn has_tasks(&self) -> bool {
        !self.task_skeletons.is_empty()
    }

    pub fn add_task_skeleton(&mut self, task_skeleton: AtomicTaskSkeleton) -> (TaskSymbolID, TaskSkeletonID) {
        let task_symbol_id = self.task_symbols.insert(task_skeleton.symbol());
        let task_skeleton_id = TaskSkeletonID::new(self.task_skeletons.len());
        self.task_skeletons.push(task_skeleton);
        (task_symbol_id, task_skeleton_id)
    }

    pub fn add_task_skeletons<I>(&mut self, iter: I)
    where
        I: IntoIterator<Item = AtomicTaskSkeleton>,
    {
        for task in iter {
            self.add_task_skeleton(task);
        }
    }


    pub fn domain_constraints(&self) -> &Expr {
        &self.domain_constraints
    }

    pub fn set_domain_constraints(&mut self, constraints: Expr) {
        self.domain_constraints = constraints;
    }


    pub fn derived_predicates(&self) -> &Vec<LiftedDerivedPredicate> {
        &self.derived_predicates
    }


    pub fn add_derived_predicate(&mut self, predicate: LiftedDerivedPredicate) {
        self.derived_predicates.push(predicate);
    }

    pub fn actions(&self) -> &[LiftedAction] {
        &self.actions
    }

    pub fn add_action(&mut self, action: LiftedAction) {
        self.actions.push(action);
    }

    pub fn durative_actions(&self) -> &[LiftedDurativeAction] {
        &self.durative_actions
    }

    pub fn add_durative_action(&mut self, action: LiftedDurativeAction) {
        self.durative_actions.push(action);
    }

    pub fn methods(&self) -> &[LiftedMethod] {
        &self.methods
    }

    pub fn add_method(&mut self, method: LiftedMethod) {
        self.methods.push(method);
    }

    pub fn init(&self) -> &Expr {
        &self.init
    }

    pub fn set_init(&mut self, init_expr: Expr) {
        self.init = init_expr;
    }

    pub fn goal(&self) -> &Expr {
        &self.goal
    }

    pub fn set_goal(&mut self, goal_expr: Expr) {
        self.goal = goal_expr;
    }

    pub fn problem_constraints(&self) -> &Expr {
        &self.problem_constraints
    }

    pub fn set_problem_constraints(&mut self, constraints: Expr) {
        self.problem_constraints = constraints;
    }

    pub fn metric_spec(&self) -> &Expr {
        &self.metric_spec
    }

    pub fn set_metric_spec(&mut self, metric: Expr) {
        self.metric_spec = metric;
    }
    pub fn length_spec(&self) -> &Expr {
        &self.length_spec
    }

    pub fn set_length_spec(&mut self, length_spec: Expr) {
        self.length_spec = length_spec;
    }

    pub fn initial_task_network(&self) -> &InitialTaskNetwork {
        &self.initial_task_network
    }

    pub fn set_initial_task_network(&mut self, initial_task_network: InitialTaskNetwork) {
        self.initial_task_network = initial_task_network;
    }

    /// Normalizes all expressions and normalizable components of this `Problem`.
    ///
    /// This includes:
    /// - Problem-level expressions: `domain_constraints`, `init`, `goal`,
    ///   `problem_constraints`, `metric_spec`, `length_spec`.
    /// - All actions (`precondition` and `effect`).
    /// - All methods (`precondition` and task network logical constraints).
    /// - The initial task network (`logical_constraints` only).
    ///
    /// # Errors
    ///
    /// Returns a `LirError` if normalization of any expression fails.
    pub fn normalize(&mut self) -> Result<(), LirError> {
        Ok(normalize::normalize_problem(self)?)
    }

    /// Returns a wrapper around the domain view of this problem.
    ///
    /// The returned [`DomainDef`] provides read-only access to all domain-level
    /// information, such as types, constants, predicates, functions, actions,
    /// methods, requirements, and domain-level constraints.
    ///
    /// # Returns
    /// A [`DomainDef`] instance borrowing the current `LiftedProblem`.
    ///
    /// # Example
    ///
    /// ```rust
    /// let problem: LiftedProblem = /* obtain LiftedProblem */;
    /// let domain = problem.domain_def();
    /// println!("Domain name: {}", domain.domain_name());
    /// ```
    pub fn domain_def(&self) -> DomainDef<'_> {
        DomainDef::new(self)
    }

    /// Returns a wrapper around the problem view of this problem.
    ///
    /// The returned [`ProblemDef`] provides read-only access to all problem-specific
    /// information, such as objects, initial state, goal state, problem constraints,
    /// metric and length specifications, and the initial task network.
    ///
    /// # Returns
    /// A [`ProblemDef`] instance borrowing the current `LiftedProblem`.
    ///
    /// # Example
    ///
    /// ```rust
    /// let problem: LiftedProblem = /* obtain LiftedProblem */;
    /// let problem_def = problem.problem_def();
    /// println!("Problem name: {}", problem_def.problem_name());
    /// ```
    pub fn problem_def(&self) -> ProblemDef<'_> {
        ProblemDef::new(self)
    }
}

/*impl SyntaxDisplay for Problem {
    /// Formats the entire problem, including both domain and problem definitions,
    /// as a syntax string.
    ///
    /// This implementation delegates rendering to the syntax renderers for
    /// the domain (`render_domain_def`) and the problem (`render_problem_def`),
    /// inserting a newline between them.
    ///
    /// # Arguments
    ///
    /// * `f` - The formatter to write the syntax string into.
    ///
    /// # Returns
    ///
    /// A [`fmt::Result`] indicating success or failure.
    fn fmt_syntax(&self, f: &mut Formatter<'_>) -> fmt::Result {
        renderers::syntax::problem::render(f, &self);
        writeln!(f)?;
        renderers::syntax_old::render_problem_def(f, &self.problem_def(), &self.interner())
    }
}

impl SelfInternerDisplay for Problem {
    /// Formats the problem using its internal `StringInterner`, including both
    /// domain and problem definitions.
    ///
    /// This implementation delegates rendering to the interner-aware renderers
    /// for the domain (`render_domain_def`) and the problem (`render_problem_def`),
    /// inserting a newline between them.
    ///
    /// # Arguments
    ///
    /// * `f` - The formatter to write the string into.
    ///
    /// # Returns
    ///
    /// A [`fmt::Result`] indicating success or failure.
    fn fmt_interner(&self, f: &mut Formatter<'_>) -> fmt::Result {
        //renderers::interner::render_domain_def(f, &self.domain_def(), &self.interner())?;
        //writeln!(f)?;
        //renderers::interner::render_problem_def(f, &self.problem_def(), &self.interner())
        writeln!(f)
    }
}*/

impl TryFrom<LinkedSemanticContext> for Problem {
    type Error = LirError;

    /// Attempts to create a `LiftedProblem` from a fully linked and semantically verified
    /// `LinkedSemanticContext`.
    ///
    /// This conversion performs the following steps:
    /// 1. Consumes the `StringInterner` from the context to manage identifiers.
    /// 2. Consumes the required `Requirement`s from the context to reflect the actual
    ///    semantic requirements used in the domain and problem.
    /// 3. Creates a new `LiftedProblem` initialized with the interner and requirements.
    /// 4. Extracts all domain-level elements (types, constants, predicates, functions,
    ///    actions, methods) from the context and populates the problem.
    /// 5. Extracts all problem-level elements (objects, initial state, goals,
    ///    constraints, metrics, initial task network) and populates the problem.
    /// 6. Normalizes all expressions in the problem to canonical form.
    ///
    /// After this conversion, the `LiftedProblem` contains a fully constructed,
    /// semantically consistent, and normalized representation of the lifted problem.
    ///
    /// # Errors
    ///
    /// Returns a `LirError` if any extraction or normalization step fails.
    fn try_from(mut context: LinkedSemanticContext) -> Result<Self, Self::Error> {

        // 1. Consume interner and required requirements from the context
        let interner = context.take_interner();
        let requirements = context.take_required_requirements();

        // 2. Create a new lifted problem with interner and requirements
        let mut problem = LiftedProblem::new(interner, requirements);

        // 3. Extract domain-level elements
        let domain_symbol_table = context.take_domain_table();
        let domain_syntax_tree = context.take_domain_syntax_tree();
        let mut registry = EncodingRegistry::new(domain_symbol_table);

        encoder::encode_domain(&domain_syntax_tree, &mut registry, &mut problem)?;

        // 4. Extract problem-level elements
        let problem_symbol_table = context.take_problem_table();
        let problem_syntax_tree = context.take_problem_syntax_tree();
        registry.set_symbol_table(problem_symbol_table);
        encoder::encode_problem(&problem_syntax_tree, &mut registry, &mut problem)?;

        // 5. Normalize all expressions in the problem
        normalize::normalize_problem(&mut problem)?;

        // 6. Return the fully constructed and normalized problem

        let domain_def = problem.domain_def();
        println!("Domain: \n{}", domain_def.to_syntax_string());

        println!("Problem : \n{}", problem.problem_def().to_syntax_string());

        Ok(problem)
    }
}


impl Display for Problem {
    /// Implements standard Rust [`Display`] for the problem.
    ///
    /// Delegates to the default interner-aware renderer for the entire problem.
    ///
    /// # Arguments
    ///
    /// * `f` - The formatter to write the output into.
    ///
    /// # Returns
    ///
    /// A [`fmt::Result`] indicating success or failure.
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        renderers::default::render_problem(f, self)
    }
}


impl SerdeSerializable for Problem { }
