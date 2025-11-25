use ordered_float::OrderedFloat;
use crate::aiplan4rust::interner::StringInterner;
use crate::aiplan4rust::lang::{ArithmeticOp, AssignOp, BinaryComp, Optimization};
use crate::aiplan4rust::lang::BinaryComp::Less;
use crate::aiplan4rust::lir::expr::{Expr, ExprNode, ExprKind, ExprContent, ExprError};
use crate::aiplan4rust::syntax::tree::NodeId;
use crate::aiplan4rust::syntax::tree::builder::SyntaxTreeBuilder;

/// Ergonomic builder for `Expr` (expression trees).
///
/// Provides high-level helpers to construct expressions using `ExprKind` + `ExprContent`.
/// Requires a mutable reference to a `StringInterner` for identifiers.
pub struct ExprBuilder<'a> {
    base: SyntaxTreeBuilder<ExprNode>,
    interner: &'a mut StringInterner,
}

#[allow(dead_code)]
impl<'a> ExprBuilder<'a> {
    /// Create a new builder with a mutable reference to a `StringInterner`
    pub fn new(interner: &'a mut StringInterner) -> Self {
        Self {
            base: SyntaxTreeBuilder::new(),
            interner,
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
    /// functional comparisons, and other expressions that take two arguments.
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

    /// Create a constant node with the given name.
    ///
    /// # Arguments
    /// * `name` - Name of the constant.
    ///
    /// # Returns
    /// NodeId of the newly created Constant node.
    pub fn constant(&mut self, name: &str) -> NodeId {
        let id = self.interner.intern_ident(name);
        self.leaf(ExprNode::new(ExprKind::Constant, ExprContent::Ident(id), None))
    }

    /// Create a variable node with a PDDL-compliant name (prefixed with `?` if not already).
    ///
    /// # Arguments
    /// * `name` - Name of the variable (without `?` or with `?`).
    ///
    /// # Returns
    /// NodeId of the newly created variable node.
    pub fn variable(&mut self, name: &str) -> NodeId {
        // Ensure PDDL variable starts with '?'
        let pddl_name = if name.starts_with('?') {
            name.to_string()
        } else {
            format!("?{}", name)
        };
        let id = self.interner.intern_ident(&pddl_name);
        self.leaf(ExprNode::new(ExprKind::Variable, ExprContent::Ident(id), None))
    }

    /// Create a function symbol node with the given name.
    ///
    /// # Arguments
    /// * `name` - Name of the function symbol.
    ///
    /// # Returns
    /// NodeId of the newly created FunctionSymbol node.
    pub fn function_symbol(&mut self, name: &str) -> NodeId {
        let id = self.interner.intern_ident(name);
        self.leaf(ExprNode::new(ExprKind::FunctionSymbol, ExprContent::Ident(id), None))
    }

    /// Create a primitive type node with the given name.
    ///
    /// # Arguments
    /// * `name` - Name of the primitive type.
    ///
    /// # Returns
    /// NodeId of the newly created PrimitiveType node.
    pub fn primitive_type(&mut self, name: &str) -> NodeId {
        let id = self.interner.intern_ident(name);
        self.leaf(ExprNode::new(ExprKind::PrimitiveType, ExprContent::Ident(id), None))
    }

    /// Create a predicate node with the given name.
    ///
    /// # Arguments
    /// * `name` - Name of the predicate.
    ///
    /// # Returns
    /// NodeId of the newly created Predicate node.
    pub fn predicate(&mut self, name: &str) -> NodeId {
        let id = self.interner.intern_ident(name);
        self.leaf(ExprNode::new(ExprKind::Predicate, ExprContent::Ident(id), None))
    }

    /// Create a task symbol node with the given name.
    ///
    /// # Arguments
    /// * `name` - Name of the task symbol.
    ///
    /// # Returns
    /// NodeId of the newly created TaskSymbol node.
    pub fn task_symbol(&mut self, name: &str) -> NodeId {
        let id = self.interner.intern_ident(name);
        self.leaf(ExprNode::new(ExprKind::TaskSymbol, ExprContent::Ident(id), None))
    }

    /// Create a preference name node with the given name.
    ///
    /// # Arguments
    /// * `name` - Name of the preference.
    ///
    /// # Returns
    /// NodeId of the newly created PrefName node.
    pub fn pref_name(&mut self, name: &str) -> NodeId {
        let id = self.interner.intern_ident(name);
        self.leaf(ExprNode::new(ExprKind::PrefName, ExprContent::Ident(id), None))
    }

    /// Create a type node containing multiple child nodes representing types.
    ///
    /// # Arguments
    /// * `children` - Vector of NodeIds, each representing a type.
    ///
    /// # Returns
    /// NodeId of the newly created Type node.
    pub fn type_(&mut self, children: Vec<NodeId>) -> NodeId {
        self.nary(ExprKind::Type, children)
    }

    /// Create a TypedList node containing multiple typed elements.
    ///
    /// # Arguments
    /// * `elements` - Vector of NodeIds, each pointing to a `typed_symbol`.
    ///
    /// # Returns
    /// NodeId of the newly created TypedList node
    pub fn typed_list(&mut self, elements: Vec<NodeId>) -> NodeId {
        self.nary(ExprKind::TypedList, elements)
    }

    /// Create a typed symbol node from an existing element node and its type node.
    ///
    /// # Arguments
    /// * `element` - NodeId of the element
    /// * `ty` - NodeId of the type
    ///
    /// # Returns
    /// NodeId of the newly created typed symbol node
    pub fn typed_symbol(&mut self, element: NodeId, ty: NodeId) -> NodeId {
        self.binary(ExprKind::TypedSymbol, element, ty)
    }

    /// Create a `FunctionTerm` node in the expression tree.
    ///
    /// A `FunctionTerm` represents the application of a function symbol to a list of arguments
    /// (variables, constants, or other expressions) in the intermediate representation (IR) of a PDDL/HDDL expression.
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the function symbol, interned via the builder's `StringInterner`.
    /// * `args` - A vector of [`NodeId`] representing the argument nodes of the function.
    ///
    /// # Returns
    ///
    /// The [`NodeId`] of the newly created `FunctionTerm` node.
    pub fn function_term(&mut self, name: &str, args: Vec<NodeId>) -> NodeId {
        let func_symbol = self.function_symbol(name);
        let mut children = vec![func_symbol];
        children.extend(args);
        self.nary(ExprKind::FunctionTerm, children)
    }

    /// Create a numeric literal node.
    ///
    /// This node stores a floating-point value as an `ExprNode` with kind `Number`.
    ///
    /// # Arguments
    /// * `value` - The numeric value to store in the node.
    ///
    /// # Returns
    /// NodeId of the newly created `Number` node.
    pub fn number(&mut self, value: f64) -> NodeId {
        self.leaf(ExprNode::new(
            ExprKind::Number,
            ExprContent::Float(OrderedFloat::from(value)),
            None,
        ))
    }

    /// Create an AtomicFormula node: (predicate arg1 arg2 ...)
    ///
    /// # Arguments
    /// * `name` - Name of the predicate.
    /// * `args` - Vector of arguments as NodeId.
    ///
    /// # Returns
    /// NodeId of the created AtomicFormula node.
    pub fn atomic_formula(&mut self, name: &str, args: Vec<NodeId>) -> NodeId {
        let predicate_node = self.predicate(name);
        let mut children = vec![predicate_node];
        children.extend(args);
        self.nary(ExprKind::AtomicFormula, children)
    }

    /// Create a logical AND node with one or more child expressions.
    ///
    /// # Arguments
    /// * `children` - A vector of NodeIds, each pointing to an expression to be ANDed.
    ///
    /// # Returns
    /// NodeId of the newly created `And` node.
    pub fn and(&mut self, children: Vec<NodeId>) -> NodeId {
        self.nary(ExprKind::And, children)
    }

    /// Create an empty logical AND node, representing True.
    pub fn empty_and(&mut self) -> NodeId {
        self.nary(ExprKind::And, vec![])
    }

    /// Create a logical OR node with one or more child expressions.
    ///
    /// # Arguments
    /// * `children` - A vector of NodeIds, each pointing to an expression to be ORed.
    ///
    /// # Returns
    /// NodeId of the newly created `Or` node.
    pub fn or(&mut self, children: Vec<NodeId>) -> NodeId {
        self.nary(ExprKind::Or, children)
    }

    /// Create an empty logical OR node, representing False.
    pub fn empty_or(&mut self) -> NodeId {
        self.nary(ExprKind::Or, vec![])
    }

    /// Create a logical NOT node with a single child expression.
    ///
    /// # Arguments
    /// * `expr` - NodeId of the expression to negate.
    ///
    /// # Returns
    /// NodeId of the newly created `Not` node.
    pub fn not(&mut self, expr: NodeId) -> NodeId {
        self.unary(ExprKind::Not, expr)
    }

    /// Create an Imply node: (imply A B)
    ///
    /// # Arguments
    /// * `antecedent` - The left part of the implication.
    /// * `consequent` - The right part of the implication.
    ///
    /// # Returns
    /// NodeId of the created Imply node.
    pub fn imply(&mut self, antecedent: NodeId, consequent: NodeId) -> NodeId {
        self.binary(ExprKind::Imply, antecedent, consequent)
    }

    /// Create a Forall node: (forall (vars...) body)
    ///
    /// # Arguments
    /// * `vars` - A TypedList node containing all bound variables.
    /// * `body` - The expression quantified by the forall.
    ///
    /// # Returns
    /// NodeId of the created Forall node.
    pub fn forall(&mut self, vars: NodeId, body: NodeId) -> NodeId {
        self.binary(ExprKind::Forall, vars, body)
    }

    /// Create an `Exists` node.
    ///
    /// # Arguments
    /// * `vars` - NodeId of a node representing the bound variables (e.g., a `TypedList` of `TypedSymbol`s)
    /// * `body` - NodeId of the expression over which the variables are quantified
    ///
    /// # Returns
    /// NodeId of the newly created `Exists` node
    pub fn exists(&mut self, vars: NodeId, body: NodeId) -> NodeId {
        self.binary(ExprKind::Exists, vars, body)
    }

    /// Create a Preference node with a name and a body expression.
    ///
    /// # Arguments
    /// * `name` - Name of the preference (interned automatically)
    /// * `body` - NodeId of the expression that forms the body of the preference
    ///
    /// # Returns
    /// NodeId of the newly created Preference node
    pub fn preference(&mut self, name: &str, body: NodeId) -> NodeId {
        let pref_symbol_node = self.pref_name(name);
        self.binary(ExprKind::Preference, pref_symbol_node, body)
    }

    /// Create a When node with a condition and an effect expression.
    ///
    /// # Arguments
    /// * `condition` - NodeId of the condition expression
    /// * `effect` - NodeId of the effect expression
    ///
    /// # Returns
    /// NodeId of the newly created When node
    pub fn when(&mut self, condition: NodeId, effect: NodeId) -> NodeId {
        self.binary(ExprKind::When, condition, effect)
    }

    /// Create a functional comparison (FComp) node with a binary operator and two child expressions.
    ///
    /// # Arguments
    /// * `op` - Binary comparison operator (e.g., GreaterEq, Less, Equal)
    /// * `left` - NodeId of the left-hand expression
    /// * `right` - NodeId of the right-hand expression
    ///
    /// # Returns
    /// NodeId of the newly created FComp node
    fn fcomp(&mut self, op: BinaryComp, left: NodeId, right: NodeId) -> NodeId {
        self.node(
            ExprNode::new(ExprKind::FComp, ExprContent::BinaryComp(op), None),
            vec![left, right],
        )
    }

    /// Create a "<" comparison node.
    ///
    /// This constructs an FComp (`Functional Comparison`) expression of the form:
    /// `( < left right )`
    ///
    /// # Arguments
    /// * `left` – `NodeId` representing the left-hand expression.
    /// * `right` – `NodeId` representing the right-hand expression.
    ///
    /// # Returns
    /// A `NodeId` referencing the newly created FComp node using the `<` operator.
    pub fn less(&mut self, left: NodeId, right: NodeId) -> NodeId {
        self.fcomp(BinaryComp::Less, left, right)
    }

    /// Create a "<=" comparison node.
    ///
    /// This constructs an FComp (`Functional Comparison`) expression of the form:
    /// `( <= left right )`
    ///
    /// # Arguments
    /// * `left` – `NodeId` representing the left-hand expression.
    /// * `right` – `NodeId` representing the right-hand expression.
    ///
    /// # Returns
    /// A `NodeId` referencing the newly created FComp node using the `<=` operator.
    pub fn less_eq(&mut self, left: NodeId, right: NodeId) -> NodeId {
        self.fcomp(BinaryComp::LessEq, left, right)
    }

    /// Create a ">" comparison node.
    ///
    /// This constructs an FComp (`Functional Comparison`) expression of the form:
    /// `( > left right )`
    ///
    /// # Arguments
    /// * `left` – `NodeId` representing the left-hand expression.
    /// * `right` – `NodeId` representing the right-hand expression.
    ///
    /// # Returns
    /// A `NodeId` referencing the newly created FComp node using the `>` operator.
    pub fn greater(&mut self, left: NodeId, right: NodeId) -> NodeId {
        self.fcomp(BinaryComp::Greater, left, right)
    }

    /// Create a ">=" comparison node.
    ///
    /// This constructs an FComp (`Functional Comparison`) expression of the form:
    /// `( >= left right )`
    ///
    /// # Arguments
    /// * `left` – `NodeId` representing the left-hand expression.
    /// * `right` – `NodeId` representing the right-hand expression.
    ///
    /// # Returns
    /// A `NodeId` referencing the newly created FComp node using the `>=` operator.
    pub fn greater_eq(&mut self, left: NodeId, right: NodeId) -> NodeId {
        self.fcomp(BinaryComp::GreaterEq, left, right)
    }

    /// Create an "=" comparison node.
    ///
    /// This constructs an FComp (`Functional Comparison`) expression of the form:
    /// `( = left right )`
    ///
    /// # Arguments
    /// * `left` – `NodeId` representing the left-hand expression.
    /// * `right` – `NodeId` representing the right-hand expression.
    ///
    /// # Returns
    /// A `NodeId` referencing the newly created FComp node using the `=` operator.
    pub fn equal(&mut self, left: NodeId, right: NodeId) -> NodeId {
        self.fcomp(BinaryComp::Equal, left, right)
    }

    /// Create an assignment expression node with a specified operation.
    ///
    /// # Arguments
    /// * `op` - The type of assignment operation (e.g., `AssignOp::Set`, `AssignOp::Add`, etc.).
    /// * `target` - NodeId of the expression representing the target of the assignment.
    /// * `value` - NodeId of the expression representing the value to be assigned.
    ///
    /// # Returns
    /// A `NodeId` of the newly created `Assign` node in the expression tree.
    fn assign_expr(&mut self, op: AssignOp, target: NodeId, value: NodeId) -> NodeId {
        self.node(
            ExprNode::new(ExprKind::Assign, ExprContent::AssignOp(op), None),
            vec![target, value],
        )
    }

    /// Create an assignment that sets `target` to `value`.
    ///
    /// # Arguments
    /// * `target` - The `NodeId` of the expression whose value is being set.
    /// * `value` - The `NodeId` of the expression representing the new value.
    ///
    /// # Returns
    /// The `NodeId` of the created `Assign` node.
    pub fn assign(&mut self, target: NodeId, value: NodeId) -> NodeId {
        self.assign_expr(AssignOp::Assign, target, value)
    }

    /// Create an assignment that increases `target` by `value`.
    ///
    /// # Arguments
    /// * `target` - The `NodeId` of the expression being increased.
    /// * `value` - The `NodeId` of the expression representing the increment.
    ///
    /// # Returns
    /// The `NodeId` of the created `Assign` node.
    pub fn increase(&mut self, target: NodeId, value: NodeId) -> NodeId {
        self.assign_expr(AssignOp::Increase, target, value)
    }

    /// Create an assignment that decreases `target` by `value`.
    ///
    /// # Arguments
    /// * `target` - The `NodeId` of the expression being decreased.
    /// * `value` - The `NodeId` of the expression representing the decrement.
    ///
    /// # Returns
    /// The `NodeId` of the created `Assign` node.
    pub fn decrease(&mut self, target: NodeId, value: NodeId) -> NodeId {
        self.assign_expr(AssignOp::Decrease, target, value)
    }

    /// Create an assignment that scales `target` up by `value`.
    ///
    /// # Arguments
    /// * `target` - The `NodeId` of the expression being scaled.
    /// * `value` - The `NodeId` of the expression representing the scaling factor.
    ///
    /// # Returns
    /// The `NodeId` of the created `Assign` node.
    pub fn scale_up(&mut self, target: NodeId, value: NodeId) -> NodeId {
        self.assign_expr(AssignOp::ScaleUp, target, value)
    }

    /// Create an assignment that scales `target` down by `value`.
    ///
    /// # Arguments
    /// * `target` - The `NodeId` of the expression being scaled.
    /// * `value` - The `NodeId` of the expression representing the scaling factor.
    ///
    /// # Returns
    /// The `NodeId` of the created `Assign` node.
    pub fn scale_down(&mut self, target: NodeId, value: NodeId) -> NodeId {
        self.assign_expr(AssignOp::ScaleDown, target, value)
    }

    /// Create an `ArithmeticExp` node with the specified operator.
    ///
    /// # Arguments
    /// * `op` - The arithmetic operator to apply (e.g., `ArithmeticOp::Add`, `ArithmeticOp::Mul`).
    /// * `operands` - Vector of NodeIds representing the operands.
    ///
    /// # Returns
    /// NodeId of the newly created `Operation` node.
    fn arithmetic_exp(&mut self, op: ArithmeticOp, operands: Vec<NodeId>) -> NodeId {
        self.node(
            ExprNode::new(ExprKind::Operation, ExprContent::ArithmeticOp(op), None),
            operands,
        )
    }

    /// Create an addition node (`+`) over the given operands.
    ///
    /// # Arguments
    /// * `operands` - Vector of NodeIds to sum.
    ///
    /// # Returns
    /// NodeId of the newly created addition node.
    pub fn add(&mut self, operands: Vec<NodeId>) -> NodeId {
        self.arithmetic_exp(ArithmeticOp::Add, operands)
    }

    /// Create a subtraction node (`-`) over the given operands.
    ///
    /// # Arguments
    /// * `operands` - Vector of NodeIds to subtract.
    ///
    /// # Returns
    /// NodeId of the newly created subtraction node.
    pub fn sub(&mut self, operands: Vec<NodeId>) -> NodeId {
        self.arithmetic_exp(ArithmeticOp::Sub, operands)
    }

    /// Create a multiplication node (`*`) over the given operands.
    ///
    /// # Arguments
    /// * `operands` - Vector of NodeIds to multiply.
    ///
    /// # Returns
    /// NodeId of the newly created multiplication node.
    pub fn mul(&mut self, operands: Vec<NodeId>) -> NodeId {
        self.arithmetic_exp(ArithmeticOp::Mul, operands)
    }

    /// Create a division node (`/`) over the given operands.
    ///
    /// # Arguments
    /// * `operands` - Vector of NodeIds to divide.
    ///
    /// # Returns
    /// NodeId of the newly created division node.
    pub fn div(&mut self, operands: Vec<NodeId>) -> NodeId {
        self.arithmetic_exp(ArithmeticOp::Div, operands)
    }


    /// Create an `AtStart` node wrapping a single expression.
    ///
    /// # Arguments
    /// * `expr` - The `NodeId` of the expression that occurs at the start.
    ///
    /// # Returns
    /// A `NodeId` of the newly created `AtStart` node.
    pub fn at_start(&mut self, expr: NodeId) -> NodeId {
        self.unary(ExprKind::AtStart, expr)
    }

    /// Create an `AtEnd` node wrapping a single expression.
    ///
    /// # Arguments
    /// * `expr` - The `NodeId` of the expression that occurs at the end.
    ///
    /// # Returns
    /// A `NodeId` of the newly created `AtEnd` node.
    pub fn at_end(&mut self, expr: NodeId) -> NodeId {
        self.unary(ExprKind::AtEnd, expr)
    }

    /// Create an `Overall` node wrapping a single expression.
    ///
    /// # Arguments
    /// * `expr` - The `NodeId` of the expression to be wrapped.
    ///
    /// # Returns
    /// A `NodeId` of the newly created `Overall` node.
    pub fn overall(&mut self, expr: NodeId) -> NodeId {
        self.unary(ExprKind::Overall, expr)
    }
    /// Create an `Always` node wrapping a single expression.
    ///
    /// # Arguments
    /// * `expr` - The `NodeId` of the expression to be wrapped.
    ///
    /// # Returns
    /// A `NodeId` of the newly created `Always` node.
    pub fn always(&mut self, expr: NodeId) -> NodeId {
        self.unary(ExprKind::Always, expr)
    }

    /// Create a `Sometime` temporal operator node with a single child expression.
    ///
    /// # Arguments
    /// * `expr` - NodeId of the expression to which the `Sometime` operator applies.
    ///
    /// # Returns
    /// NodeId of the newly created `Sometime` node.
    pub fn sometime(&mut self, expr: NodeId) -> NodeId {
        self.unary(ExprKind::Sometime, expr)
    }

    /// Create a `Within` node with a numeric duration and an expression.
    ///
    /// # Arguments
    /// * `value` - The numeric bound (f64) for the `Within` operator.
    /// * `expr` - NodeId of the expression node to which `Within` applies.
    ///
    /// # Returns
    /// NodeId of the newly created `Within` node.
    pub fn within(&mut self, value: f64, expr: NodeId) -> NodeId {
        let duration_node = self.number(value);
        self.node(
            ExprNode::new(ExprKind::Within, ExprContent::None, None),
            vec![duration_node, expr],
        )
    }

    /// Create an `AtMostOnce` node with a single expression child.
    ///
    /// # Arguments
    /// * `expr` - NodeId of the expression to constrain to at most once.
    ///
    /// # Returns
    /// NodeId of the newly created `AtMostOnce` node.
    pub fn at_most_once(&mut self, expr: NodeId) -> NodeId {
        self.unary(ExprKind::AtMostOnce, expr)
    }

    /// Create a `SometimeAfter` node with two child expressions.
    ///
    /// # Arguments
    /// * `first` - NodeId of the first expression (the reference event).
    /// * `second` - NodeId of the second expression (the event that must occur after the first).
    ///
    /// # Returns
    /// NodeId of the newly created `SometimeAfter` node.
    pub fn sometime_after(&mut self, first: NodeId, second: NodeId) -> NodeId {
        self.binary(ExprKind::SometimeAfter, first, second)
    }

    /// Create a `SometimeBefore` node with two child expressions.
    ///
    /// # Arguments
    /// * `first` - NodeId of the first expression (the event that must occur before the second).
    /// * `second` - NodeId of the second expression (the reference event).
    ///
    /// # Returns
    /// NodeId of the newly created `SometimeBefore` node.
    pub fn sometime_before(&mut self, first: NodeId, second: NodeId) -> NodeId {
        self.binary(ExprKind::SometimeBefore, first, second)
    }

    /// Create an `AlwaysWithin` node with a numeric bound and two child expressions.
    ///
    /// # Arguments
    /// * `duration` - f64 value representing the time bound.
    /// * `first` - NodeId of the first expression (start event).
    /// * `second` - NodeId of the second expression (end event).
    ///
    /// # Returns
    /// NodeId of the newly created `AlwaysWithin` node.
    pub fn always_within(&mut self, duration: f64, first: NodeId, second: NodeId) -> NodeId {
        let number_node = self.number(duration);
        self.node(
            ExprNode::new(ExprKind::AlwaysWithin, ExprContent::None, None),
            vec![number_node, first, second],
        )
    }

    /// Create a `HoldDuring` node with start/end numeric bounds and a child expression.
    ///
    /// # Arguments
    /// * `start` - f64 value for the start time.
    /// * `end` - f64 value for the end time.
    /// * `expr` - NodeId of the expression to hold during the interval.
    ///
    /// # Returns
    /// NodeId of the newly created `HoldDuring` node.
    pub fn hold_during(&mut self, start: f64, end: f64, expr: NodeId) -> NodeId {
        let start_node = self.number(start);
        let end_node = self.number(end);
        self.node(
            ExprNode::new(ExprKind::HoldDuring, ExprContent::None, None),
            vec![start_node, end_node, expr],
        )
    }

    /// Create a `HoldAfter` node with a time bound and a child expression.
    ///
    /// # Arguments
    /// * `time` - f64 value representing the time after which the expression holds.
    /// * `expr` - NodeId of the expression to hold after the given time.
    ///
    /// # Returns
    /// NodeId of the newly created `HoldAfter` node.
    pub fn hold_after(&mut self, time: f64, expr: NodeId) -> NodeId {
        let time_node = self.number(time);
        self.node(
            ExprNode::new(ExprKind::HoldAfter, ExprContent::None, None),
            vec![time_node, expr],
        )
    }

    /// Create a `TimedInitialLiteral` node with a time and a child expression.
    ///
    /// # Arguments
    /// * `time` - f64 value representing the initial time.
    /// * `expr` - NodeId of the expression true at that time.
    ///
    /// # Returns
    /// NodeId of the newly created `TimedInitialLiteral` node.
    pub fn timed_initial_literal(&mut self, time: f64, expr: NodeId) -> NodeId {
        let time_node = self.number(time);
        self.node(
            ExprNode::new(ExprKind::TimedInitialLiteral, ExprContent::None, None),
            vec![time_node, expr],
        )
    }

    /// Create a Metric node with the specified optimization directive and child expression.
    ///
    /// # Arguments
    /// * `opt` - The optimization directive (Minimize or Maximize)
    /// * `expr` - NodeId of the expression representing the metric target
    ///
    /// # Returns
    /// NodeId of the newly created Metric node
    fn metric_exp(&mut self, opt: Optimization, expr: NodeId) -> NodeId {
        self.node(
            ExprNode::new(ExprKind::Metric, ExprContent::Optimization(opt), None),
            vec![expr],
        )
    }

    /// Create a Metric node that **minimizes** the given expression.
    ///
    /// # Arguments
    /// * `expr` - NodeId of the expression representing the metric to minimize
    ///
    /// # Returns
    /// NodeId of the newly created Metric node
    pub fn minimize(&mut self, expr: NodeId) -> NodeId {
        self.metric_exp(Optimization::Minimize, expr)
    }

    /// Create a Metric node that **maximizes** the given expression.
    ///
    /// # Arguments
    /// * `expr` - NodeId of the expression representing the metric to maximize
    ///
    /// # Returns
    /// NodeId of the newly created Metric node
    pub fn maximize(&mut self, expr: NodeId) -> NodeId {
        self.metric_exp(Optimization::Maximize, expr)
    }


    /// Create a `TotalTime` node.
    ///
    /// Represents the total time metric of a plan or task sequence in the LIR.
    ///
    /// # Returns
    /// NodeId of the newly created `TotalTime` node.
    pub fn total_time(&mut self) -> NodeId {
        self.leaf(ExprNode::new(ExprKind::TotalTime, ExprContent::None, None))
    }

    /// Create an `IsViolated` node for a given preference name.
    ///
    /// # Arguments
    /// * `name` - The string name of the preference to check.
    ///
    /// # Returns
    /// NodeId of the newly created `IsViolated` node.
    pub fn is_violated(&mut self, name: &str) -> NodeId {
        let pref_node = self.pref_name(name);
        self.unary(ExprKind::IsViolated, pref_node)
    }

    /// Create a Length node, optionally specifying serial and parallel lengths.
    ///
    /// # Arguments
    /// * `serial` - Optional f64 for the serial length.
    /// * `parallel` - Optional f64 for the parallel length.
    ///
    /// # Returns
    /// NodeId of the newly created Length node.
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

    /// Create a Serial node representing a serial length specification.
    ///
    /// # Arguments
    /// * `value` - f64 specifying the serial length.
    ///
    /// # Returns
    /// NodeId of the newly created Serial node.
    pub fn serial(&mut self, value: f64) -> NodeId {
        let number = self.number(value);
        self.unary(ExprKind::Serial, number)
    }

    /// Create a Parallel node representing a parallel length specification.
    ///
    /// # Arguments
    /// * `value` - f64 specifying the parallel length.
    ///
    /// # Returns
    /// NodeId of the newly created Parallel node.
    pub fn parallel(&mut self, value: f64) -> NodeId {
        let number = self.number(value);
        self.unary(ExprKind::Parallel, number)
    }

    /// Create a Task node consisting of a TaskSymbol and argument nodes.
    ///
    /// # Arguments
    /// * `name` - Name of the task symbol.
    /// * `arguments` - Vector of NodeIds representing argument nodes.
    ///
    /// # Returns
    /// NodeId of the newly created Task node.
    pub fn task(&mut self, name: &str, arguments: Vec<NodeId>) -> NodeId {
        let task_symbol = self.task_symbol(name);
        let mut children = vec![task_symbol];
        children.extend(arguments);
        self.nary(ExprKind::Task, children)
    }

    /// Create a TaskID node representing a task identifier.
    ///
    /// # Arguments
    /// * `name` - Name of the task identifier.
    ///
    /// # Returns
    /// NodeId of the newly created TaskID node.
    pub fn task_id(&mut self, name: &str) -> NodeId {
        let id = self.interner.intern_ident(name);
        self.leaf(ExprNode::new(ExprKind::TaskID, ExprContent::Ident(id), None))
    }

    /// Create a TaggedTask node with a TaskID and a Task as children.
    ///
    /// # Arguments
    /// * `id` - Name of the task identifier.
    /// * `task` - NodeId of the task node.
    ///
    /// # Returns
    /// NodeId of the newly created TaggedTask node.
    pub fn tagged_task(&mut self, id: &str, task: NodeId) -> NodeId {
        let task_id = self.task_id(id);
        self.binary(ExprKind::TaggedTask, task_id, task)
    }

    /// Create a TaskOrderingConstraint node comparing two tasks (< task1 task2).
    ///
    /// # Arguments
    /// * `task1` - NodeId of the first task.
    /// * `task2` - NodeId of the second task.
    ///
    /// # Returns
    /// NodeId of the newly created TaskOrderingConstraint node.
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
