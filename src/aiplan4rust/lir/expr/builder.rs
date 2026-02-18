use ordered_float::OrderedFloat;
use crate::aiplan4rust::lang::{ArithmeticOp, AssignOp, BinaryComp, FunctionSymbolId, ObjectId, Optimization, PredicateSymbolId, PreferenceSymbolId, TaskLabelSymbolId, TaskSymbolId, Type, TypeId, TypedList, TypedSymbol, VariableId};
use crate::aiplan4rust::lang::BinaryComp::Less;
use crate::aiplan4rust::lir::expr::{Expr, ExprNode, ExprKind, ExprContent, ExprError};
use crate::aiplan4rust::tree::NodeId;
use crate::aiplan4rust::tree::builder::SyntaxTreeBuilder;

/// Ergonomic builder for `Expr` (expression trees).
///
/// Provides high-level helpers to construct expr using `ExprKind` + `ExprContent`.
/// Requires a mutable reference to a `StringInterner` for identifiers.
pub struct ExprBuilder {
    base: SyntaxTreeBuilder<ExprNode>,
}

#[allow(dead_code)]
impl ExprBuilder {
    /// Creates a new `ExprBuilder` with an empty syntax tree.
    ///
    /// This builder is used to construct expression trees using direct identifiers
    /// (like `VariableID`, `ObjectID`, etc.) instead of strings.
    pub fn new() -> Self {
        Self {
            base: SyntaxTreeBuilder::new(),
        }
    }

    /// Finalize builder into `Expr`
    pub fn finish(self) -> Expr {
        Expr::from_tree(self.base.finish())
    }

    /// Allocate a leaf node
    pub fn leaf(&mut self, node: ExprNode) -> NodeId {
        self.base.leaf(node)
    }

    /// Allocate a node with children
    pub fn node(&mut self, node: ExprNode, children: Vec<NodeId>) -> NodeId {
        self.base.node(node, children)
    }

    /// Set an existing node as the root
    pub fn set_root(&mut self, id: NodeId) -> Result<&mut Self, ExprError> {
        self.base.set_root(id)?;
        Ok(self)
    }

    /// Create a node with children and set it as root
    pub fn root_node(&mut self, node: ExprNode, children: Vec<NodeId>) -> Result<NodeId, ExprError> {
        Ok(self.base.root_node(node, children)?)
    }

    // -------------------------
    // Generic helpers
    // -------------------------

    /// Creates a unary node in the expression tree.
    ///
    /// A unary node is a node that has **exactly one child**.
    /// This function is useful for all logical or temporal operators
    /// that take a single argument, such as `not`, `at_start`, `always`, etc.
    ///
    /// # Arguments
    /// * `kind` - The type of node (`ExprKind`) to create.
    /// * `child` - NodeId of the single child.
    ///
    /// # Returns
    /// NodeId of the newly created node.
    fn unary(&mut self, kind: ExprKind, child: NodeId) -> NodeId {
        self.node(ExprNode::new(kind, ExprContent::None, None), vec![child])
    }

    /// Creates a binary node in the expression tree.
    ///
    /// A binary node is a node that has **exactly two children**.
    /// This function is useful for binary logical operators, implications,
    /// functional comparisons, and other expr that take two arguments.
    ///
    /// # Arguments
    /// * `kind` - The type of node (`ExprKind`) to create.
    /// * `left` - NodeId of the left child.
    /// * `right` - NodeId of the right child.
    ///
    /// # Returns
    /// NodeId of the newly created node.
    fn binary(&mut self, kind: ExprKind, left: NodeId, right: NodeId) -> NodeId {
        self.node(ExprNode::new(kind, ExprContent::None, None), vec![left, right])
    }

    /// Creates an N-ary node in the expression tree.
    ///
    /// An N-ary node is a node that can have an arbitrary number of children.
    /// This function is suitable for operators like `and`, `or`, `typed_list`,
    /// or any expression that can take a dynamic number of children.
    ///
    /// # Arguments
    /// * `kind` - The type of node (`ExprKind`) to create.
    /// * `children` - A vector of NodeIds representing the children.
    ///
    /// # Returns
    /// NodeId of the newly created node.
    fn nary(&mut self, kind: ExprKind, children: Vec<NodeId>) -> NodeId {
        self.node(ExprNode::new(kind, ExprContent::None, None), children)
    }

    // -------------------------
    // High-level helpers
    // -------------------------

    /// Creates a constant (object) node with the given identifier.
    ///
    /// This helper accepts any type that can be converted into an [`ObjectId`],
    /// making it easy to use either a typed ID or a raw `usize` (especially in tests).
    ///
    /// # Arguments
    /// * `id` - The identifier of the constant/object (e.g., an `ObjectID` or `usize`).
    ///
    /// # Returns
    /// The [`NodeId`] of the newly created leaf node in the expression tree.
    pub fn constant<I: Into<ObjectId>>(&mut self, id: I) -> NodeId {
        self.leaf(ExprNode::new(
            ExprKind::Constant,
            ExprContent::Constant(id.into()),
            None
        ))
    }

    /// Creates a variable node with the given identifier.
    ///
    /// This helper accepts any type that can be converted into a [`VariableId`],
    /// allowing the use of typed IDs or raw `usize`.
    ///
    /// Note: PDDL-specific naming (like the `?` prefix) should be handled
    /// during the initial encoding/interning phase, not in this builder.
    ///
    /// # Arguments
    /// * `id` - The identifier of the variable (e.g., a `VariableID` or `usize`).
    ///
    /// # Returns
    /// The [`NodeId`] of the newly created `Variable` leaf node.
    pub fn variable<I: Into<VariableId>>(&mut self, id: I) -> NodeId {
        self.leaf(ExprNode::new(
            ExprKind::Variable,
            ExprContent::Variable(id.into()),
            None,
        ))
    }

    /// Creates a function symbol (functor) node with the given identifier.
    ///
    /// This helper accepts any type that can be converted into a [`FunctionSymbolId`],
    /// making it easy to use either a pre-resolved ID or a raw `usize`.
    ///
    /// # Arguments
    /// * `id` - The identifier of the function symbol (e.g., a `FunctorID` or `usize`).
    ///
    /// # Returns
    /// The [`NodeId`] of the newly created `FunctionSymbol` leaf node.
    pub fn function_symbol<I: Into<FunctionSymbolId>>(&mut self, id: I) -> NodeId {
        self.leaf(ExprNode::new(
            ExprKind::FunctionSymbol,
            ExprContent::FunctionSymbol(id.into()),
            None
        ))
    }

    /// Creates a predicate node with the given identifier.
    ///
    /// This helper accepts any type that can be converted into a [`PredicateSymbolId`],
    /// allowing for the use of typed identifiers or raw `usize` for quick prototyping.
    ///
    /// # Arguments
    /// * `id` - The identifier of the predicate (e.g., a [`PredicateSymbolId`] or `usize`).
    ///
    /// # Returns
    /// The [`NodeId`] of the newly created `Predicate` leaf node.
    pub fn predicate<I: Into<PredicateSymbolId>>(&mut self, id: I) -> NodeId {
        self.leaf(ExprNode::new(
            ExprKind::Predicate,
            ExprContent::PredicateSymbol(id.into()),
            None,
        ))
    }

    /// Creates a task symbol node with the given identifier.
    ///
    /// This helper accepts any type that can be converted into a [`TaskSymbolId`],
    /// which is useful for HTN (Hierarchical Task Network) expr where
    /// task identifiers are already resolved.
    ///
    /// # Arguments
    /// * `id` - The identifier of the task symbol (e.g., a [`TaskSymbolId`] or `usize`).
    ///
    /// # Returns
    /// The [`NodeId`] of the newly created `TaskSymbol` leaf node.
    pub fn task_symbol<I: Into<TaskSymbolId>>(&mut self, id: I) -> NodeId {
        self.leaf(ExprNode::new(
            ExprKind::TaskSymbol,
            ExprContent::TaskSymbol(id.into()),
            None,
        ))
    }

    /// Creates a preference name node with the given identifier.
    ///
    /// This helper accepts any type that can be converted into a [`PreferenceSymbolId`].
    /// It is typically used for preference constraints in PDDL or HTN problems.
    ///
    /// # Arguments
    /// * `id` - The identifier of the preference (e.g., a [`PreferenceSymbolId`] or `usize`).
    ///
    /// # Returns
    /// The [`NodeId`] of the newly created `PrefName` leaf node.
    pub fn pref_name<I: Into<PreferenceSymbolId>>(&mut self, id: I) -> NodeId {
        self.leaf(ExprNode::new(
            ExprKind::PrefName,
            ExprContent::PreferenceSymbol(id.into()),
            None,
        ))
    }

    /// Creates a `FunctionTerm` node in the expression tree.
    ///
    /// A `FunctionTerm` represents the application of a function symbol to a list of arguments
    /// (variables, constants, or other expr).
    ///
    /// # Arguments
    /// * `id` - The identifier of the function symbol (e.g., a [`FunctionSymbolId`] or `usize`).
    /// * `args` - A vector of [`NodeId`] representing the argument nodes.
    ///
    /// # Returns
    /// The [`NodeId`] of the newly created `FunctionTerm` node.
    pub fn function_term<I: Into<FunctionSymbolId>>(&mut self, id: I, args: Vec<NodeId>) -> NodeId {
        let func_symbol = self.function_symbol(id);
        let mut children = vec![func_symbol];
        children.extend(args);
        self.nary(ExprKind::FunctionTerm, children)
    }

    /// Creates a numeric literal node.
    ///
    /// This node stores a floating-point value as an [`ExprNode`] with the kind [`ExprKind::Number`].
    /// The value is internally converted to an [`OrderedFloat`] to ensure compatibility
    /// with the rest of the expression tree logic.
    ///
    /// # Arguments
    /// * `value` - The numeric value (f64) to store in the node.
    ///
    /// # Returns
    /// The [`NodeId`] of the newly created `Number` leaf node.
    pub fn number(&mut self, value: f64) -> NodeId {
        self.leaf(ExprNode::new(
            ExprKind::Number,
            ExprContent::Number(OrderedFloat::from(value)),
            None,
        ))
    }

    /// Creates an `AtomicFormula` node: (predicate arg1 arg2 ...)
    ///
    /// This represents a logical atom where a predicate is applied to a set of arguments.
    /// The first child of the resulting node is always the predicate symbol.
    ///
    /// # Arguments
    /// * `id` - The identifier of the predicate (e.g., a [`PredicateSymbolId`] or `usize`).
    /// * `args` - A vector of [`NodeId`] representing the terms/arguments of the formula.
    ///
    /// # Returns
    /// The [`NodeId`] of the newly created `AtomicFormula` node.
    pub fn atomic_formula<I: Into<PredicateSymbolId>>(&mut self, id: I, args: Vec<NodeId>) -> NodeId {
        let predicate_node = self.predicate(id);
        let mut children = vec![predicate_node];
        children.extend(args);
        self.nary(ExprKind::AtomicFormula, children)
    }

    /// Creates a logical `AND` node with one or more child expr.
    ///
    /// In PDDL/HDDL, this represents a conjunction. An empty vector of children
    /// is technically allowed and usually represents a "True" constant in logical contexts.
    ///
    /// # Arguments
    /// * `children` - A vector of [`NodeId`]s, each pointing to a sub-expression to be conjoined.
    ///
    /// # Returns
    /// The [`NodeId`] of the newly created `And` node.
    pub fn and(&mut self, children: Vec<NodeId>) -> NodeId {
        self.nary(ExprKind::And, children)
    }

    /// Creates an empty logical `AND` node, representing a "True" constant.
    ///
    /// In PDDL and many logical frameworks, a conjunction with no operands
    /// is vacuously true. This is commonly used as a default precondition
    /// or an empty effect block.
    ///
    /// # Returns
    /// The [`NodeId`] of the newly created `And` node with no children.
    pub fn empty_and(&mut self) -> NodeId {
        self.nary(ExprKind::And, vec![])
    }

    /// Creates a logical `OR` node with one or more child expr.
    ///
    /// In PDDL/HDDL, this represents a disjunction. If the vector of children
    /// is empty, the expression is technically "False" (the identity element for OR).
    ///
    /// # Arguments
    /// * `children` - A vector of [`NodeId`]s, each pointing to a sub-expression to be disjoined.
    ///
    /// # Returns
    /// The [`NodeId`] of the newly created `Or` node.
    pub fn or(&mut self, children: Vec<NodeId>) -> NodeId {
        self.nary(ExprKind::Or, children)
    }

    /// Creates an empty logical `OR` node, representing a "False" constant.
    ///
    /// In logic and planning languages like PDDL, a disjunction with no operands
    /// is vacuously false. This is often used to represent an unsatisfiable
    /// condition or an initial state for an accumulator.
    ///
    /// # Returns
    /// The [`NodeId`] of the newly created `Or` node with no children.
    pub fn empty_or(&mut self) -> NodeId {
        self.nary(ExprKind::Or, vec![])
    }

    /// Creates a logical `NOT` node with a single child expression.
    ///
    /// In PDDL/HDDL, this represents the negation of a formula. It is an unary
    /// operator, meaning it must have exactly one child node representing
    /// the expression being negated.
    ///
    /// # Arguments
    /// * `expr` - The [`NodeId`] of the expression to negate.
    ///
    /// # Returns
    /// The [`NodeId`] of the newly created `Not` node.
    pub fn not(&mut self, expr: NodeId) -> NodeId {
        self.unary(ExprKind::Not, expr)
    }

    /// Creates an `Imply` node: (imply A B)
    ///
    /// In logic, this represents the material implication (A → B). It is a binary
    /// operator where the first child is the antecedent and the second is the consequent.
    ///
    /// # Arguments
    /// * `antecedent` - The [`NodeId`] of the condition (the "if" part).
    /// * `consequent` - The [`NodeId`] of the result (the "then" part).
    ///
    /// # Returns
    /// The [`NodeId`] of the newly created `Imply` node.
    pub fn imply(&mut self, antecedent: NodeId, consequent: NodeId) -> NodeId {
        self.binary(ExprKind::Imply, antecedent, consequent)
    }

    /// Creates a `Forall` node: (forall (vars...) body)
    ///
    /// This represents a universal quantifier. The variables and their associated
    /// types are stored directly in the node's content, while the quantified
    /// formula is attached as a child node.
    ///
    /// # Arguments
    /// * `vars` - A [`TypedList`] mapping [`VariableId`]s to their respective [`TypeId`]s.
    /// * `body` - The [`NodeId`] of the sub-expression within the scope of this quantifier.
    ///
    /// # Returns
    /// The [`NodeId`] of the newly created `Forall` node.
    pub fn forall(&mut self, vars: TypedList<VariableId, TypeId>, body: NodeId) -> NodeId {
        self.node(
            ExprNode::new(
                ExprKind::Forall,
                ExprContent::QuantifierVariables(vars),
                None,
            ),
            vec![body],
        )
    }

    /// Creates an `Exists` node: (exists (vars...) body)
    ///
    /// This represents an existential quantifier. The variables and their associated
    /// types are stored directly in the node's content, while the quantified
    /// formula is attached as a single child node.
    ///
    /// # Arguments
    /// * `vars` - A [`TypedList`] mapping [`VariableId`]s to their respective [`TypeId`]s.
    /// * `body` - The [`NodeId`] of the sub-expression within the scope of this quantifier.
    ///
    /// # Returns
    /// The [`NodeId`] of the newly created `Exists` node.
    pub fn exists(&mut self, vars: TypedList<VariableId, TypeId>, body: NodeId) -> NodeId {
        self.node(
            ExprNode::new(
                ExprKind::Exists,
                ExprContent::QuantifierVariables(vars),
                None,
            ),
            vec![body],
        )
    }

    /// Helper to convert a vector of TypedSymbols into a TypedList
    pub fn typed_variable_list(&mut self, vars: Vec<TypedSymbol<VariableId, TypeId>>) -> TypedList<VariableId, TypeId> {
        let mut list = TypedList::new();
        for typed_var in vars {
            list.push(typed_var);
        }
        list
    }

    /// Helper for a single typed symbol: (SymbolID, [TypeIDs])
    pub fn typed_variable(&mut self, id: usize, type_ids: &[usize]) -> TypedSymbol<VariableId, TypeId> {
        let ty = self.ty(type_ids);
        TypedSymbol::new(
            VariableId::from(id),
            ty,
        )
    }

    /// Helper to create a list of TypeIDs from a slice of integers
    pub fn ty(&mut self, ids: &[usize]) -> Type<TypeId> {
        Type::either(ids.iter().map(|&id| TypeId::from(id)).collect())
    }

    /// Creates a `Preference` node: (preference name body)
    ///
    /// This represents a named preference constraint. The first child is the
    /// identifier of the preference, and the second child is the expression
    /// (the goal or constraint) being preferred.
    ///
    /// # Arguments
    /// * `id` - The identifier of the preference (e.g., a [`PreferenceSymbolId`] or `usize`).
    /// * `body` - The [`NodeId`] of the expression that forms the body of the preference.
    ///
    /// # Returns
    /// The [`NodeId`] of the newly created `Preference` node.
    pub fn preference<I: Into<PreferenceSymbolId>>(&mut self, id: I, body: NodeId) -> NodeId {
        let pref_symbol_node = self.pref_name(id);
        self.binary(ExprKind::Preference, pref_symbol_node, body)
    }

    /// Creates a `When` node: (when condition effect)
    ///
    /// This represents a conditional effect. It is a binary operator where
    /// the first child is the antecedent (the condition that must hold)
    /// and the second child is the consequent (the effect that occurs).
    ///
    /// # Arguments
    /// * `condition` - The [`NodeId`] of the logical formula governing the effect.
    /// * `effect` - The [`NodeId`] of the effect expression to be applied.
    ///
    /// # Returns
    /// The [`NodeId`] of the newly created `When` node.
    pub fn when(&mut self, condition: NodeId, effect: NodeId) -> NodeId {
        self.binary(ExprKind::When, condition, effect)
    }

    /// Creates a functional comparison (`FComp`) node: (op left right)
    ///
    /// Functional comparisons are used to compare two numeric expr
    /// (terms, fluents, or literals) using a binary operator.
    ///
    /// # Arguments
    /// * `op` - The binary comparison operator (e.g., [`BinaryComp::GreaterEq`], [`BinaryComp::Less`]).
    /// * `left` - The [`NodeId`] of the left-hand side expression.
    /// * `right` - The [`NodeId`] of the right-hand side expression.
    ///
    /// # Returns
    /// The [`NodeId`] of the newly created `FComp` node.
    pub fn fcomp(&mut self, op: BinaryComp, left: NodeId, right: NodeId) -> NodeId {
        self.node(
            ExprNode::new(
                ExprKind::FComp,
                ExprContent::BinaryComp(op),
                None
            ),
            vec![left, right],
        )
    }

    /// Creates a "less than" comparison node: (< left right)
    ///
    /// This is a convenience helper that constructs an [`ExprKind::FComp`] node
    /// using the [`BinaryComp::Less`] operator.
    ///
    /// # Arguments
    /// * `left` - The [`NodeId`] of the left-hand numeric expression.
    /// * `right` - The [`NodeId`] of the right-hand numeric expression.
    ///
    /// # Returns
    /// The [`NodeId`] of the newly created `FComp` node.
    pub fn less(&mut self, left: NodeId, right: NodeId) -> NodeId {
        self.fcomp(BinaryComp::Less, left, right)
    }

    /// Creates a "less than or equal to" comparison node: (<= left right)
    ///
    /// This is a convenience helper that constructs an [`ExprKind::FComp`] node
    /// using the [`BinaryComp::LessEq`] operator.
    ///
    /// # Arguments
    /// * `left` - The [`NodeId`] of the left-hand numeric expression.
    /// * `right` - The [`NodeId`] of the right-hand numeric expression.
    ///
    /// # Returns
    /// The [`NodeId`] of the newly created `FComp` node.
    pub fn less_eq(&mut self, left: NodeId, right: NodeId) -> NodeId {
        self.fcomp(BinaryComp::LessEq, left, right)
    }

    /// Creates a "greater than" comparison node: (> left right)
    ///
    /// This is a convenience helper that constructs an [`ExprKind::FComp`] node
    /// using the [`BinaryComp::Greater`] operator.
    ///
    /// # Arguments
    /// * `left` - The [`NodeId`] of the left-hand numeric expression.
    /// * `right` - The [`NodeId`] of the right-hand numeric expression.
    ///
    /// # Returns
    /// The [`NodeId`] of the newly created `FComp` node.
    pub fn greater(&mut self, left: NodeId, right: NodeId) -> NodeId {
        self.fcomp(BinaryComp::Greater, left, right)
    }

    /// Creates a "greater than or equal to" comparison node: (>= left right)
    ///
    /// This is a convenience helper that constructs an [`ExprKind::FComp`] node
    /// using the [`BinaryComp::GreaterEq`] operator.
    ///
    /// # Arguments
    /// * `left` - The [`NodeId`] of the left-hand numeric expression.
    /// * `right` - The [`NodeId`] of the right-hand numeric expression.
    ///
    /// # Returns
    /// The [`NodeId`] of the newly created `FComp` node.
    pub fn greater_eq(&mut self, left: NodeId, right: NodeId) -> NodeId {
        self.fcomp(BinaryComp::GreaterEq, left, right)
    }

    /// Creates a "numeric equality" comparison node: (= left right)
    ///
    /// This constructs an [`ExprKind::FComp`] node using the [`BinaryComp::Equal`] operator.
    /// In PDDL, this is used for comparing fluents or numeric values, and should be
    /// distinguished from logical equivalence or object identity depending on your
    /// solver's implementation.
    ///
    /// # Arguments
    /// * `left` - The [`NodeId`] of the left-hand numeric expression.
    /// * `right` - The [`NodeId`] of the right-hand numeric expression.
    ///
    /// # Returns
    /// The [`NodeId`] of the newly created `FComp` node.
    pub fn equal(&mut self, left: NodeId, right: NodeId) -> NodeId {
        self.fcomp(BinaryComp::Equal, left, right)
    }

    /// Creates an assignment expression node: (op target value)
    ///
    /// This node represents a functional effect that modifies a fluent.
    /// Common operations include direct assignment, incrementing, or decrementing.
    ///
    /// # Arguments
    /// * `op` - The type of assignment operation (e.g., [`AssignOp::Assign`], [`AssignOp::Increase`]).
    /// * `target` - The [`NodeId`] of the fluent (function term) being modified.
    /// * `value` - The [`NodeId`] of the numeric expression to apply.
    ///
    /// # Returns
    /// The [`NodeId`] of the newly created `Assign` node.
    fn assign_expr(&mut self, op: AssignOp, target: NodeId, value: NodeId) -> NodeId {
        self.node(
            ExprNode::new(
                ExprKind::Assign,
                ExprContent::AssignOp(op),
                None
            ),
            vec![target, value],
        )
    }

    /// Creates an assignment node that sets `target` to `value`: (assign target value)
    ///
    /// This is a convenience helper for the [`AssignOp::Assign`] operation. It is
    /// typically used in action effects to reset a fluent to a specific numeric value.
    ///
    /// # Arguments
    /// * `target` - The [`NodeId`] of the fluent (function term) being set.
    /// * `value` - The [`NodeId`] of the numeric expression representing the new value.
    ///
    /// # Returns
    /// The [`NodeId`] of the newly created `Assign` node.
    pub fn assign(&mut self, target: NodeId, value: NodeId) -> NodeId {
        self.assign_expr(AssignOp::Assign, target, value)
    }

    /// Creates an increase assignment: (increase target value)
    ///
    /// This represents a functional effect where the current value of the `target`
    /// is incremented by the result of the `value` expression.
    ///
    /// # Arguments
    /// * `target` - The [`NodeId`] of the fluent (function term) to be increased.
    /// * `value` - The [`NodeId`] of the numeric expression representing the increment.
    ///
    /// # Returns
    /// The [`NodeId`] of the newly created `Assign` node.
    pub fn increase(&mut self, target: NodeId, value: NodeId) -> NodeId {
        self.assign_expr(AssignOp::Increase, target, value)
    }

    /// Creates a decrease assignment: (decrease target value)
    ///
    /// This represents a functional effect where the current value of the `target`
    /// is decremented by the result of the `value` expression.
    ///
    /// # Arguments
    /// * `target` - The [`NodeId`] of the fluent (function term) to be decreased.
    /// * `value` - The [`NodeId`] of the numeric expression representing the decrement.
    ///
    /// # Returns
    /// The [`NodeId`] of the newly created `Assign` node.
    pub fn decrease(&mut self, target: NodeId, value: NodeId) -> NodeId {
        self.assign_expr(AssignOp::Decrease, target, value)
    }

    /// Creates a scale-up assignment: (scale-up target value)
    ///
    /// This represents a functional effect where the current value of the `target`
    /// is multiplied by the result of the `value` expression.
    ///
    /// # Arguments
    /// * `target` - The [`NodeId`] of the fluent (function term) to be scaled up.
    /// * `value` - The [`NodeId`] of the numeric expression representing the factor.
    ///
    /// # Returns
    /// The [`NodeId`] of the newly created `Assign` node.
    pub fn scale_up(&mut self, target: NodeId, value: NodeId) -> NodeId {
        self.assign_expr(AssignOp::ScaleUp, target, value)
    }

    /// Creates a scale-down assignment: (scale-down target value)
    ///
    /// This represents a functional effect where the current value of the `target`
    /// is divided by the result of the `value` expression.
    ///
    /// # Arguments
    /// * `target` - The [`NodeId`] of the fluent (function term) to be scaled down.
    /// * `value` - The [`NodeId`] of the numeric expression representing the divisor.
    ///
    /// # Returns
    /// The [`NodeId`] of the newly created `Assign` node.
    pub fn scale_down(&mut self, target: NodeId, value: NodeId) -> NodeId {
        self.assign_expr(AssignOp::ScaleDown, target, value)
    }

    /// Creates an arithmetic expression node: (op operands...)
    ///
    /// This node represents a functional operation (addition, multiplication, etc.)
    /// applied to one or more numeric sub-expr.
    ///
    /// # Arguments
    /// * `op` - The arithmetic operator to apply (e.g., [`ArithmeticOp::Add`], [`ArithmeticOp::Mul`]).
    /// * `operands` - A [`Vec<NodeId>`] of the numeric expr to be operated upon.
    ///
    /// # Returns
    /// The [`NodeId`] of the newly created `Operation` node.
    fn arithmetic_exp(&mut self, op: ArithmeticOp, operands: Vec<NodeId>) -> NodeId {
        self.node(
            ExprNode::new(
                ExprKind::Operation,
                ExprContent::ArithmeticOp(op),
                None
            ),
            operands,
        )
    }

    /// Creates an addition node: (+ operands...)
    ///
    /// This constructs an [`ExprKind::Operation`] node using the [`ArithmeticOp::Add`]
    /// operator. It can take any number of operands, representing their cumulative sum.
    ///
    /// # Arguments
    /// * `operands` - A [`Vec<NodeId>`] of numeric expr to be added together.
    ///
    /// # Returns
    /// The [`NodeId`] of the newly created addition node.
    pub fn add(&mut self, operands: Vec<NodeId>) -> NodeId {
        self.arithmetic_exp(ArithmeticOp::Add, operands)
    }

    /// Creates a subtraction node: (- operands...)
    ///
    /// This constructs an [`ExprKind::Operation`] node using the [`ArithmeticOp::Sub`]
    /// operator.
    ///
    /// * If one operand is provided, it represents unary negation: (- a) => -a.
    /// * If multiple operands are provided, it represents left-associative subtraction:
    ///   (- a b c) => (a - b - c).
    ///
    /// # Arguments
    /// * `operands` - A [`Vec<NodeId>`] of numeric expr.
    ///
    /// # Returns
    /// The [`NodeId`] of the newly created subtraction node.
    pub fn sub(&mut self, operands: Vec<NodeId>) -> NodeId {
        self.arithmetic_exp(ArithmeticOp::Sub, operands)
    }

    /// Creates a multiplication node: (* operands...)
    ///
    /// This constructs an [`ExprKind::Operation`] node using the [`ArithmeticOp::Mul`]
    /// operator. It represents the product of all expr contained in the
    /// `operands` vector.
    ///
    /// # Arguments
    /// * `operands` - A [`Vec<NodeId>`] of numeric expr to be multiplied.
    ///
    /// # Returns
    /// The [`NodeId`] of the newly created multiplication node.
    pub fn mul(&mut self, operands: Vec<NodeId>) -> NodeId {
        self.arithmetic_exp(ArithmeticOp::Mul, operands)
    }

    /// Creates a division node: (/ operands...)
    ///
    /// This constructs an [`ExprKind::Operation`] node using the [`ArithmeticOp::Div`]
    /// operator.
    ///
    /// # Arguments
    /// * `operands` - A [`Vec<NodeId>`] of numeric expr. Usually, this contains
    ///   two nodes representing the dividend and the divisor.
    ///
    /// # Returns
    /// The [`NodeId`] of the newly created division node.
    pub fn div(&mut self, operands: Vec<NodeId>) -> NodeId {
        self.arithmetic_exp(ArithmeticOp::Div, operands)
    }

    /// Creates an `AtStart` temporal node: (at start expr)
    ///
    /// This node is used in temporal planning to constrain an expression
    /// (condition or effect) to the beginning of the action's execution.
    ///
    /// # Arguments
    /// * `expr` - The [`NodeId`] of the expression to be wrapped in the temporal constraint.
    ///
    /// # Returns
    /// The [`NodeId`] of the newly created `AtStart` node.
    pub fn at_start(&mut self, expr: NodeId) -> NodeId {
        self.unary(ExprKind::AtStart, expr)
    }

    /// Creates an `AtEnd` temporal node: (at end expr)
    ///
    /// This node is used in temporal planning to anchor an expression
    /// (condition or effect) to the end of the action's execution interval.
    ///
    /// # Arguments
    /// * `expr` - The [`NodeId`] of the expression to be wrapped in the temporal constraint.
    ///
    /// # Returns
    /// The [`NodeId`] of the newly created `AtEnd` node.
    pub fn at_end(&mut self, expr: NodeId) -> NodeId {
        self.unary(ExprKind::AtEnd, expr)
    }

    /// Creates an `Overall` temporal node: (over all expr)
    ///
    /// This node represents a temporal invariant. In the context of PDDL, it ensures
    /// that the specified condition remains true throughout the entire duration
    /// of an action's execution.
    ///
    /// # Arguments
    /// * `expr` - The [`NodeId`] of the condition expression to be maintained.
    ///
    /// # Returns
    /// The [`NodeId`] of the newly created `Overall` node.
    pub fn overall(&mut self, expr: NodeId) -> NodeId {
        self.unary(ExprKind::Overall, expr)
    }

    /// Creates an `Always` constraint node: (always expr)
    ///
    /// This node represents a global trajectory constraint. It asserts that the
    /// given expression must hold true in every state of the plan execution.
    ///
    /// # Arguments
    /// * `expr` - The [`NodeId`] of the condition expression that must always hold.
    ///
    /// # Returns
    /// The [`NodeId`] of the newly created `Always` node.
    pub fn always(&mut self, expr: NodeId) -> NodeId {
        self.unary(ExprKind::Always, expr)
    }

    /// Creates a `Sometime` temporal node: (sometime expr)
    ///
    /// This node represents a modal operator asserting that the given expression
    /// must hold true in at least one state during the plan execution.
    ///
    /// # Arguments
    /// * `expr` - The [`NodeId`] of the condition that must eventually be satisfied.
    ///
    /// # Returns
    /// The [`NodeId`] of the newly created `Sometime` node.
    pub fn sometime(&mut self, expr: NodeId) -> NodeId {
        self.unary(ExprKind::Sometime, expr)
    }

    /// Creates a `Within` node representing a temporal constraint.
    ///
    /// This node defines a time window during which an expression must be satisfied.
    /// The generated structure places the duration as the first child and the
    /// expression as the second.
    ///
    /// # Arguments
    ///
    /// * `value` - The numeric time bound (duration) for the `Within` operator.
    /// * `expr` - The `NodeId` of the expression to which the constraint applies.
    pub fn within(&mut self, value: f64, expr: NodeId) -> NodeId {
        // 1. Create a leaf node for the numeric duration
        let duration_node = self.number(value);

        // 2. Create the parent Within node linking the duration and the expression
        self.node(
            // ExprContent::None is used because the data is stored in the children
            ExprNode::new(ExprKind::Within, ExprContent::None, None),
            vec![duration_node, expr],
        )
    }

    /// Creates an `AtMostOnce` node representing a cardinality constraint.
    ///
    /// This node specifies that the given expression or action can occur at most one time
    /// within the problem's scope.
    ///
    /// # Arguments
    ///
    /// * `expr` - The `NodeId` of the expression to which the constraint applies.
    pub fn at_most_once(&mut self, expr: NodeId) -> NodeId {
        self.unary(ExprKind::AtMostOnce, expr)
    }

    /// Creates a `SometimeAfter` node representing a temporal ordering constraint.
    ///
    /// This node specifies that if the first expression occurs, the second expression
    /// must occur at some point following it.
    ///
    /// # Arguments
    ///
    /// * `first` - The `NodeId` of the reference event (the trigger).
    /// * `second` - The `NodeId` of the event that must follow the first.
    pub fn sometime_after(&mut self, first: NodeId, second: NodeId) -> NodeId {
        self.binary(ExprKind::SometimeAfter, first, second)
    }

    /// Creates a `SometimeBefore` node representing a temporal ordering constraint.
    ///
    /// This node specifies that if the second expression occurs, the first expression
    /// must have occurred at some point prior to it.
    ///
    /// # Arguments
    ///
    /// * `first` - The `NodeId` of the event that must precede the second.
    /// * `second` - The `NodeId` of the reference event.
    pub fn sometime_before(&mut self, first: NodeId, second: NodeId) -> NodeId {
        self.binary(ExprKind::SometimeBefore, first, second)
    }

    /// Creates an `AlwaysWithin` node representing a bounded temporal constraint.
    ///
    /// This node specifies that whenever the first expression (the trigger) occurs,
    /// the second expression must occur within a specific time duration.
    ///
    /// # Arguments
    ///
    /// * `duration` - The numeric f64 value representing the maximum time allowed
    ///   between the two events.
    /// * `first` - The `NodeId` of the first expression (the start event).
    /// * `second` - The `NodeId` of the second expression (the end event).
    pub fn always_within(&mut self, duration: f64, first: NodeId, second: NodeId) -> NodeId {
        let number_node = self.number(duration);
        self.node(
            ExprNode::new(ExprKind::AlwaysWithin, ExprContent::None, None),
            vec![number_node, first, second],
        )
    }

    /// Creates a `HoldDuring` node representing a persistent interval constraint.
    ///
    /// This node specifies that the given expression must remain true (hold)
    /// throughout the entire time interval defined by the start and end bounds.
    ///
    /// # Arguments
    ///
    /// * `start` - The numeric f64 value for the beginning of the time interval.
    /// * `end` - The numeric f64 value for the end of the time interval.
    /// * `expr` - The `NodeId` of the expression that must be maintained during this period.
    pub fn hold_during(&mut self, start: f64, end: f64, expr: NodeId) -> NodeId {
        let start_node = self.number(start);
        let end_node = self.number(end);
        self.node(
            ExprNode::new(ExprKind::HoldDuring, ExprContent::None, None),
            vec![start_node, end_node, expr],
        )
    }

    /// Creates a `HoldAfter` node representing a temporal persistence constraint.
    ///
    /// This node specifies that the given expression must remain true (hold)
    /// from a specific point in time onwards.
    ///
    /// # Arguments
    ///
    /// * `time` - The numeric f64 value representing the start time from which the expression must hold.
    /// * `expr` - The `NodeId` of the expression that must be maintained.
    pub fn hold_after(&mut self, time: f64, expr: NodeId) -> NodeId {
        let time_node = self.number(time);
        self.node(
            ExprNode::new(ExprKind::HoldAfter, ExprContent::None, None),
            vec![time_node, expr],
        )
    }

    /// Creates a `TimedInitialLiteral` node representing a timed fact.
    ///
    /// This node specifies that a particular expression or literal becomes true
    /// at a specific point in time, typically used for exogenous events or initial conditions.
    ///
    /// # Arguments
    ///
    /// * `time` - The numeric f64 value representing the exact time at which the expression occurs.
    /// * `expr` - The `NodeId` of the expression that becomes true at the given time.
    pub fn timed_initial_literal(&mut self, time: f64, expr: NodeId) -> NodeId {
        let time_node = self.number(time);
        self.node(
            ExprNode::new(ExprKind::TimedInitialLiteral, ExprContent::None, None),
            vec![time_node, expr],
        )
    }

    /// Creates a `Metric` node representing the optimization goal of the problem.
    ///
    /// This node defines how the solver should evaluate the quality of a solution,
    /// using a specific directive (Minimize or Maximize) applied to a target expression.
    ///
    /// # Arguments
    ///
    /// * `opt` - The `Optimization` directive, specifying whether to minimize or maximize the metric.
    /// * `expr` - The `NodeId` of the expression (e.g., total cost, time, or resource usage) to be optimized.
    fn metric_exp(&mut self, opt: Optimization, expr: NodeId) -> NodeId {
        self.node(
            // The optimization directive is stored directly in the node's content
            ExprNode::new(ExprKind::Metric, ExprContent::Optimization(opt), None),
            vec![expr],
        )
    }

    /// Creates a `Metric` node that specifies a minimization goal.
    ///
    /// This is a convenience wrapper around `metric_exp` that sets the optimization
    /// directive to `Minimize`. It is typically used to reduce costs, time, or resource consumption.
    ///
    /// # Arguments
    ///
    /// * `expr` - The `NodeId` of the expression to be minimized.
    pub fn minimize(&mut self, expr: NodeId) -> NodeId {
        self.metric_exp(Optimization::Minimize, expr)
    }

    /// Creates a `Metric` node that specifies a maximization goal.
    ///
    /// This is a convenience wrapper around `metric_exp` that sets the optimization
    /// directive to `Maximize`. It is commonly used for goals like maximizing
    /// utility, profit, or resource efficiency.
    ///
    /// # Arguments
    ///
    /// * `expr` - The `NodeId` of the expression to be maximized.
    pub fn maximize(&mut self, expr: NodeId) -> NodeId {
        self.metric_exp(Optimization::Maximize, expr)
    }

    /// Creates a `TotalTime` node.
    ///
    /// This node represents the total duration or makespan of a plan or task sequence.
    /// It is typically used as a variable within a metric expression to be minimized.
    ///
    /// # Returns
    ///
    /// The `NodeId` of the newly created leaf node representing the total time metric.
    pub fn total_time(&mut self) -> NodeId {
        self.leaf(ExprNode::new(ExprKind::TotalTime, ExprContent::None, None))
    }

    /// Creates an `IsViolated` node to check the status of a soft constraint.
    ///
    /// This node evaluates to true if the named preference has not been satisfied
    /// in the current plan. It is typically used in metric expr to penalize
    /// the violation of specific soft goals.
    ///
    /// # Arguments
    ///
    /// * `id` - The identifier of the preference to check (e.g., a [`PreferenceSymbolId`] or `usize`).
    pub fn is_violated<I: Into<PreferenceSymbolId>>(&mut self, id: I) -> NodeId {
        let pref_node = self.pref_name(id);
        self.unary(ExprKind::IsViolated, pref_node)
    }

    /// Creates a `Length` node representing the temporal or structural span of an expression.
    ///
    /// This node can optionally incorporate serial and parallel components. If provided,
    /// these components are added as child nodes to define the specific constraints
    /// of the length metric.
    ///
    /// # Arguments
    ///
    /// * `serial` - An optional f64 value representing the sequential length component.
    /// * `parallel` - An optional f64 value representing the concurrent length component.
    pub fn length(&mut self, serial: Option<f64>, parallel: Option<f64>) -> NodeId {
        let mut children = Vec::new();
        if let Some(s) = serial {
            children.push(self.serial(s));
        }
        if let Some(p) = parallel {
            children.push(self.parallel(p));
        }
        self.nary(ExprKind::Length, children)
    }

    /// Creates a `Serial` node representing a sequential duration specification.
    ///
    /// This node is typically used as a component within a `Length` expression to
    /// define the duration of tasks executed in sequence.
    ///
    /// # Arguments
    ///
    /// * `value` - The numeric f64 value specifying the serial length.
    pub fn serial(&mut self, value: f64) -> NodeId {
        let number = self.number(value);
        self.unary(ExprKind::Serial, number)
    }

    /// Creates a `Parallel` node representing a concurrent duration specification.
    ///
    /// This node is typically used as a component within a `Length` expression to
    /// define the duration of tasks that are executed in parallel.
    ///
    /// # Arguments
    ///
    /// * `value` - The numeric f64 value specifying the parallel length.
    pub fn parallel(&mut self, value: f64) -> NodeId {
        let number = self.number(value);
        self.unary(ExprKind::Parallel, number)
    }

    /// Creates a `Task` node representing a task instance with its associated arguments.
    ///
    /// This node combines a task symbol identifier with a set of parameters or arguments.
    ///
    /// # Arguments
    ///
    /// * `id` - The identifier of the task symbol (e.g., a [`TaskSymbolId`] or `usize`).
    /// * `arguments` - A vector of `NodeId`s representing the arguments passed to the task.
    pub fn task<I: Into<TaskSymbolId>>(&mut self, id: I, arguments: Vec<NodeId>) -> NodeId {
        let task_symbol = self.task_symbol(id);
        let mut children = vec![task_symbol];
        children.extend(arguments);
        self.nary(ExprKind::Task, children)
    }

    /// Creates a `TaskID` node representing a unique identifier for a task instance.
    ///
    /// This node stores a task label identifier as a leaf node. It is typically used
    /// to reference specific task instances within temporal or causal constraints.
    ///
    /// # Arguments
    ///
    /// * `id` - The identifier of the task label (e.g., a [`TaskLabelSymbolId`] or `usize`).
    pub fn task_id<I: Into<TaskLabelSymbolId>>(&mut self, id: I) -> NodeId {
        self.leaf(ExprNode::new(
            ExprKind::TaskID,
            ExprContent::TaskLabelSymbol(id.into()),
            None,
        ))
    }

    /// Creates a `TaggedTask` node that associates a specific identifier with a task.
    ///
    /// This node represents a labeled task instance, linking a unique `TaskID` (the tag)
    /// to a `Task` definition.
    ///
    /// # Arguments
    ///
    /// * `id` - The identifier to be used as the task's tag (e.g., a [`TaskLabelSymbolId`] or `usize`).
    /// * `task` - The `NodeId` of the task expression being tagged.
    pub fn tagged_task<I: Into<TaskLabelSymbolId>>(&mut self, id: I, task: NodeId) -> NodeId {
        let task_id_node = self.task_id(id);
        self.binary(ExprKind::TaggedTask, task_id_node, task)
    }

    /// Creates a `TaskOrderingConstraint` node representing a strictly sequential relationship.
    ///
    /// This node enforces a temporal order between two tasks, specifically that
    /// the first task must be completed before the second task can begin.
    ///
    /// # Arguments
    ///
    /// * `task1` - The `NodeId` of the task that must occur first.
    /// * `task2` - The `NodeId` of the task that must occur second.
    pub fn task_ordering_constraint(
        &mut self,
        task1: NodeId,
        task2: NodeId,
    ) -> NodeId {
        self.node(
            ExprNode::new(ExprKind::TaskOrderingConstraint, ExprContent::BinaryComp(Less), None),
            vec![task1, task2],
        )
    }
}
