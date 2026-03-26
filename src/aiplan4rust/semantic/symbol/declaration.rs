//! Module for representing symbol declarations in the AI syntax and semantic analysis.
//!
//! This module defines the `Declaration` struct, which models a symbol's declaration
//! within the AST arena. It enriches basic syntax with semantic metadata such as
//! scope, origin, types, and arguments, while maintaining strict links to the
//! source code via `Span` and `NodeId`.
//!
//! # Core Concepts
//!
//! - **Symbol**: The identity of the declared symbol, including its identifier and kind (e.g., Variable, Predicate).
//! - **Scope**: The visibility and lifetime context (e.g., Global, Local, Action-level).
//! - **SymbolOrigin**: Indicates whether the declaration stems from a Domain or a Problem file.
//! - **Types & Arguments**: Semantic structures (`Type`, `TypedList`) capturing the symbol's signature.
//! - **Provenance Tracking**: A dual-layer tracking system using `Span` for general location and
//!   `NodeId` vectors (`ty_node_ids`, `argument_node_ids`) for precise mapping of each component.
//!
//! # Functionality
//!
//! The `Declaration` struct:
//! - Encapsulates all data required for semantic validation and type checking.
//! - Supports **Symbol Remapping**, allowing identifiers to be updated during AST transformations.
//! - Facilitates **Precise Diagnostics** by linking every semantic element (like a specific type in an `either`)
//!   to its original AST node.
//! - Provides specialized formatting via `fmt::Display` and `InternerDisplay` for human-readable
//!   output using interned strings.
//!
//! # Examples
//!
//! ```rust
//! use crate::aiplan4rust::semantic::symbol::{Declaration, Symbol, SymbolKind, Scope, SymbolOrigin};
//! use crate::aiplan4rust::lang::{Type, TypedList};
//! use crate::aiplan4rust::syntax::{Span, NodeId};
//!
//! // Creating a declaration for a variable 'x' of type 'object'
//! let symbol = Symbol::new(symbol_id, SymbolKind::Variable);
//! let declaration = Declaration::new(
//!     symbol,
//!     Scope::Global,
//!     SymbolOrigin::Domain,
//!     Some(Type::from(vec![type_id])), // Semantic type
//!     Some(vec![type_node_id]),        // Syntactic link
//!     None,                            // No arguments
//!     None,                            // No argument nodes
//!     span,
//!     main_node_id,
//!     None,                            // No imported scope
//! );
//!
//! println!("{}", declaration);
//! ```
//!
//! This module relies on `serde` for robust serialization/deserialization of the semantic model.

use crate::aiplan4rust::interner::{InternerDisplay, InternerError, SymbolInterner};
use crate::aiplan4rust::lang::Type;
use crate::aiplan4rust::lang::TypedList;
use crate::aiplan4rust::lang::{RemapSymbol, SymbolId};
use crate::aiplan4rust::semantic::symbol::Scope;
use crate::aiplan4rust::semantic::symbol::SymbolKind;
use crate::aiplan4rust::semantic::symbol::{Symbol, SymbolOrigin};
use crate::aiplan4rust::syntax::Span;
use crate::aiplan4rust::tree::NodeId;
use serde::Deserialize;
use serde::Serialize;
use std::collections::HashMap;
use std::fmt;

/// Represents a formal declaration of a symbol within the AST.
///
/// This structure links semantic information (calculated during analysis) with
/// precise syntactic anchors (NodeIds) to enable accurate diagnostics,
/// symbol remapping, and source code patching.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Declaration {
    /// The actual symbol being declared, containing its identifier and kind.
    symbol: Symbol,

    /// The visibility context and lifetime of the declaration (e.g., Global, Local).
    scope: Scope,

    /// The source domain of the declaration, indicating if it originates
    /// from a Domain or a Problem file.
    origin: SymbolOrigin,

    /// Optional semantic typing information associated with the symbol.
    ty: Option<Type<SymbolId>>,

    /// Optional list of AST node identifiers corresponding to each type in `ty`.
    /// This allows pinpointing the exact syntax for every declared type.
    ty_node_ids: Option<Vec<NodeId>>,

    /// Optional list of typed parameters, used for symbols with signatures
    /// such as predicates or functions.
    arguments: Option<TypedList<SymbolId, SymbolId>>,

    /// Optional list of AST node identifiers for each individual argument name.
    /// Ensures a 1:1 mapping between semantic arguments and their syntactic origin.
    argument_node_ids: Option<Vec<NodeId>>,

    /// The specific range in the source code covered by this declaration.
    span: Span,

    /// The primary AST node identifier that represents this declaration.
    node_id: NodeId,

    /// The original scope context if this declaration was imported
    /// from an external module; otherwise `None`.
    imported_scope: Option<Scope>,
}

impl Declaration {
    /// Creates a new `Declaration` instance.
    ///
    /// Constructs a `Declaration` that represents the declaration of a symbol within
    /// a given scope, along with its semantic information (types, arguments) and
    /// its syntactic anchoring in the AST.
    ///
    /// # Arguments
    ///
    /// * `symbol` - The symbol being declared (identity and kind).
    /// * `scope` - The scope in which this declaration is valid (e.g., Global, Local).
    /// * `origin` - The origin source of the declaration (e.g., Domain or Problem).
    /// * `ty` - Optional semantic types associated with the symbol.
    /// * `ty_node_ids` - Optional AST node identifiers for the type annotations.
    /// * `arguments` - Optional typed list of parameters for predicates or functions.
    /// * `argument_node_ids` - Optional AST node identifiers for each individual argument.
    /// * `span` - The source code span locating the declaration.
    /// * `node_id` - The main AST node identifier for this declaration.
    /// * `imported_scope` - The original scope if the symbol was imported from another module.
    ///
    /// # Returns
    ///
    /// A new `Declaration` instance populated with the provided information.
    ///
    /// # Example
    ///
    /// ```rust
    /// let decl = Declaration::new(
    ///     symbol,
    ///     scope,
    ///     origin,
    ///     Some(types),
    ///     Some(type_nodes),
    ///     Some(args),
    ///     Some(arg_nodes),
    ///     span,
    ///     node_id,
    ///     None,
    /// );
    /// ```
    pub fn new(
        symbol: Symbol,
        scope: Scope,
        origin: SymbolOrigin,
        ty: Option<Type<SymbolId>>,
        ty_node_ids: Option<Vec<NodeId>>,
        arguments: Option<TypedList<SymbolId, SymbolId>>,
        argument_node_ids: Option<Vec<NodeId>>,
        span: Span,
        node_id: NodeId,
        imported_scope: Option<Scope>,
    ) -> Self {
        Declaration {
            symbol,
            scope,
            origin,
            ty,
            ty_node_ids,
            arguments,
            argument_node_ids,
            span,
            node_id,
            imported_scope,
        }
    }

    /// Returns a reference to the [`Symbol`] associated with this declaration.
    ///
    /// # Returns
    ///
    /// A reference to the underlying symbol.
    pub fn symbol(&self) -> &Symbol {
        &self.symbol
    }

    /// Returns the [`SymbolId`] of the declared symbol.
    ///
    /// # Returns
    ///
    /// The unique identifier of the symbol.
    pub fn symbol_ident(&self) -> SymbolId {
        self.symbol.id()
    }

    /// Returns the [`SymbolKind`] of the declared symbol.
    ///
    /// # Returns
    ///
    /// The kind of the symbol (e.g., Variable, PrimitiveType).
    pub fn symbol_kind(&self) -> SymbolKind {
        self.symbol.kind()
    }

    /// Returns a reference to the [`Scope`] in which the symbol is declared.
    ///
    /// # Returns
    ///
    /// A reference to the scope context.
    pub fn scope(&self) -> &Scope {
        &self.scope
    }

    /// Sets the visibility scope of the declaration.
    ///
    /// # Arguments
    ///
    /// * `scope` - The new scope to associate with this declaration.
    pub fn set_scope(&mut self, scope: Scope) {
        self.scope = scope;
    }

    /// Returns an optional reference to the typing information associated with the symbol.
    ///
    /// # Returns
    ///
    /// `Some(&Type<SymbolId>)` if the symbol is typed, or `None` otherwise.
    pub fn ty(&self) -> Option<&Type<SymbolId>> {
        self.ty.as_ref()
    }

    /// Returns an optional slice of the AST node identifiers corresponding to the types.
    ///
    /// These IDs allow mapping each semantic type to its precise syntax location for diagnostics or patching.
    ///
    /// # Returns
    ///
    /// `Some(&[NodeId])` representing the type nodes, or `None` if no type nodes are recorded.
    pub fn ty_node_ids(&self) -> Option<&[NodeId]> {
        self.ty_node_ids.as_deref()
    }

    /// Sets the semantic types associated with this declaration.
    ///
    /// # Arguments
    ///
    /// * `ty` - The type definition to assign.
    pub fn set_ty(&mut self, ty: Type<SymbolId>) {
        self.ty = Some(ty);
    }

    /// Sets the AST node identifiers corresponding to the typing syntax.
    ///
    /// # Arguments
    ///
    /// * `node_ids` - A vector of node IDs representing the type components in the source tree.
    pub fn set_ty_node_ids(&mut self, node_ids: Vec<NodeId>) {
        self.ty_node_ids = Some(node_ids);
    }

    /// Clears both semantic types and their associated syntax node identifiers.
    pub fn clear_ty(&mut self) {
        self.ty = None;
        self.ty_node_ids = None;
    }

    /// Returns an optional reference to the list of arguments (parameters) associated with the symbol.
    ///
    /// # Returns
    ///
    /// `Some(&TypedList<SymbolId, SymbolId>)` if the symbol has arguments, or `None` otherwise.
    pub fn arguments(&self) -> Option<&TypedList<SymbolId, SymbolId>> {
        self.arguments.as_ref()
    }

    /// Sets the list of arguments for symbols like predicates or functions.
    ///
    /// # Arguments
    ///
    /// * `arguments` - The typed parameter list to assign.
    pub fn set_arguments(&mut self, arguments: TypedList<SymbolId, SymbolId>) {
        self.arguments = Some(arguments);
    }

    /// Returns an optional slice of the AST node identifiers corresponding to the arguments.
    ///
    /// This provides a direct link between each parameter and its original syntax node.
    ///
    /// # Returns
    ///
    /// `Some(&[NodeId])` representing the argument nodes, or `None` if no IDs are recorded.
    pub fn argument_node_ids(&self) -> Option<&[NodeId]> {
        self.argument_node_ids.as_deref()
    }

    /// Sets the AST node identifiers corresponding to the argument syntax.
    ///
    /// # Arguments
    ///
    /// * `ids` - A vector of node IDs representing the arguments in the source tree.
    pub fn set_argument_node_ids(&mut self, ids: Vec<NodeId>) {
        self.argument_node_ids = Some(ids);
    }

    /// Clears both semantic arguments and their associated syntax node identifiers.
    pub fn clear_arguments(&mut self) {
        self.arguments = None;
        self.argument_node_ids = None;
    }

    /// Returns the source code [`Span`] where the declaration is located.
    ///
    /// Since [`Span`] implements [`Copy`], this method returns a value rather than a reference,
    /// making it easier to use without worrying about lifetimes.
    ///
    /// # Returns
    ///
    /// The span indicating the start and end positions in the source.
    pub fn span(&self) -> Span {
        self.span
    }

    /// Sets the source code span associated with this declaration.
    ///
    /// # Arguments
    ///
    /// * `span` - The new span to assign.
    pub fn set_span(&mut self, span: Span) {
        self.span = span;
    }

    /// Returns the [`NodeId`] of the main AST node associated with this declaration.
    ///
    /// # Returns
    ///
    /// The unique identifier of the AST node.
    pub fn node_id(&self) -> NodeId {
        self.node_id
    }

    /// Sets the [`NodeId`] of the main AST node associated with this declaration.
    ///
    /// # Arguments
    ///
    /// * `node_id` - The new AST node identifier.
    pub fn set_node_id(&mut self, node_id: NodeId) {
        self.node_id = node_id;
    }

    /// Returns the [`SymbolOrigin`] indicating where the symbol was defined.
    ///
    /// # Returns
    ///
    /// The origin of the declaration (e.g., Domain or Problem).
    pub fn origin(&self) -> SymbolOrigin {
        self.origin
    }

    /// Sets the origin of the declaration.
    ///
    /// # Arguments
    ///
    /// * `origin` - The new origin to assign.
    pub fn set_origin(&mut self, origin: SymbolOrigin) {
        self.origin = origin;
    }

    /// Returns an optional reference to the imported scope.
    ///
    /// This field is populated if the declaration originates from an external source or module.
    ///
    /// # Returns
    ///
    /// `Some(&Scope)` if imported, or `None` if the declaration is native to the current context.
    pub fn imported_scope(&self) -> Option<&Scope> {
        self.imported_scope.as_ref()
    }

    /// Sets or clears the original scope of an imported declaration.
    ///
    /// # Arguments
    ///
    /// * `scope` - The optional scope to assign.
    pub fn set_imported_scope(&mut self, scope: Option<Scope>) {
        self.imported_scope = scope;
    }

    /// Formats the types of the declaration for display.
    ///
    /// If the declaration has a list of types, this function formats them and writes them
    /// to the formatter. The types are displayed in a parenthesized list. If there is only
    /// one type_checker, it is directly displayed. If there are multiple types, they are prefixed
    /// with the word "either" and separated by spaces.
    ///
    /// # Arguments
    ///
    /// * `f` - The formatter where the types will be written.
    ///
    /// # Returns
    ///
    /// This function returns a `fmt::Result`, which indicates success or failure
    /// in formatting.
    ///
    /// # Example
    ///
    /// ```rust
    /// let declaration = Declaration { ... };
    /// println!("{}", declaration.format_types());
    /// ```
    fn fmt_types(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(types) = &self.ty {
            write!(f, ", types: (")?;
            if types.is_empty() {
                write!(f, ")")?;
            } else if types.len() == 1 {
                // Directly format the single type_checker
                if let Some(single_type) = types.iter().next() {
                    write!(f, "{})", single_type)?;
                }
            } else {
                // Format multiple types with 'either'
                write!(f, "either")?;
                for ty in types.iter() {
                    write!(f, " {}", ty)?;
                }
                write!(f, ")")?;
            }
        }
        Ok(())
    }

    /// Formats the arguments of the declaration for display.
    ///
    /// If the declaration has a list of arguments, this function formats them
    /// and writes them to the formatter. The arguments are displayed as a comma-separated
    /// list enclosed in parentheses. Each argument is printed with its name, separated by spaces.
    ///
    /// # Arguments
    ///
    /// * `f` - The formatter where the arguments will be written.
    ///
    /// # Returns
    ///
    /// This function returns a `fmt::Result`, which indicates success or failure
    /// in formatting.
    ///
    /// # Example
    ///
    /// ```rust
    /// let declaration = Declaration { ... };
    /// println!("{}", declaration.format_arguments());
    /// ```
    fn fmt_arguments(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(arguments) = &self.arguments {
            write!(f, ", arguments: (")?;
            for (i, argument) in arguments.iter().enumerate() {
                if i > 0 {
                    write!(f, " ")?;
                }
                // Affiche le nom de l'argument avec son index
                write!(f, "{}", argument)?;
            }
            write!(f, ")")?;
        }
        Ok(())
    }

    /// Formats the type_checker information of the declaration using a `StringInterner`
    /// to resolve interned identifiers into human-readable strings.
    ///
    /// This function writes a formatted representation of the declaration's types
    /// into the given formatter. The formatting follows these rules:
    /// - If there are no types, it writes `", types: ()"`.
    /// - If there is a single type_checker, it writes it directly: `", types: (type_name)"`.
    /// - If there are multiple types, it writes them in PDDL's `either` syntax:
    ///   `", types: (either type1 type2 ...)"`.
    ///
    /// If a type_checker identifier cannot be resolved by the interner, it is printed as `<uninterned:ID>`.
    ///
    /// # Arguments
    ///
    /// * `w` - The formatter to write the output to.
    /// * `interner` - The `StringInterner` used to resolve interned type_checker identifiers.
    ///
    /// # Returns
    ///
    /// Returns a [`fmt::Result`] indicating success or failure during formatting.
    ///
    fn fmt_types_with(
        &self,
        w: &mut std::fmt::Formatter<'_>,
        interner: &SymbolInterner,
    ) -> fmt::Result {
        if let Some(types) = &self.ty {
            write!(w, ", types: (")?;

            match types.members() {
                [] => { /* no types */ }
                [single] => match interner.resolve_symbol(*single) {
                    Some(name) => write!(w, "{}", name)?,
                    None => write!(w, "<uninterned:{}>", single)?,
                },
                _ => {
                    write!(w, "either")?;
                    for ty in types.iter() {
                        match interner.resolve_symbol(*ty) {
                            Some(name) => write!(w, " {}", name)?,
                            None => write!(w, " <uninterned:{}>", ty)?,
                        }
                    }
                }
            }

            write!(w, ")")?;
        } else {
            write!(w, ", types: ()")?;
        }

        Ok(())
    }

    /// Formats the arguments of the declaration using a `StringInterner`
    /// to resolve interned identifiers into human-readable strings.
    ///
    /// This function writes a formatted list of the declaration's arguments
    /// into the given formatter. Each argument is resolved through the interner,
    /// and printed in a space-separated list inside parentheses.
    ///
    /// If an argument identifier cannot be resolved, it is printed as `<uninterned:ID>`.
    ///
    /// # Arguments
    ///
    /// * `w` - The formatter to write the output to.
    /// * `interner` - The `StringInterner` used to resolve interned argument identifiers.
    ///
    /// # Returns
    ///
    /// Returns a [`fmt::Result`] indicating success or failure during formatting.
    ///
    fn fmt_arguments_with(
        &self,
        w: &mut std::fmt::Formatter<'_>,
        interner: &SymbolInterner,
    ) -> fmt::Result {
        if let Some(arguments) = &self.arguments {
            write!(w, ", arguments: (")?;

            for (i, typed_symbol) in arguments.iter().enumerate() {
                if i > 0 {
                    write!(w, " ")?;
                }
                typed_symbol.fmt_with_interner(w, interner)?;
            }

            write!(w, ")")?;
        }

        Ok(())
    }
}

impl fmt::Display for Declaration {
    /// Formats the `Declaration` for display purposes.
    ///
    /// This implementation writes a structured representation of the declaration,
    /// including its AST node ID, symbol kind, identifier (raw ID), scope, origin,
    /// and optionally the associated types, arguments, and their corresponding
    /// syntax node IDs.
    ///
    /// # Arguments
    ///
    /// * `f` - The formatter used to write the string representation.
    ///
    /// # Returns
    ///
    /// A `fmt::Result` indicating success or failure.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // 1. Basic information: node_id, kind, and raw symbol identifier
        write!(
            f,
            "[node: {}, kind: {}, ident: {}",
            self.node_id().as_usize(),
            self.symbol_kind(),
            self.symbol_ident()
        )?;

        // 2. Context: scope, origin, and imported scope
        write!(
            f,
            ", scope: {}, origin: {}, imported: {}",
            self.scope(),
            self.origin(),
            self.imported_scope()
                .map_or("None".to_string(), |s| s.to_string())
        )?;

        // 3. Types and their node IDs
        self.fmt_types(f)?;
        if let Some(nodes) = self.ty_node_ids() {
            write!(f, " (nodes: ")?;
            for (i, node) in nodes.iter().enumerate() {
                if i > 0 {
                    write!(f, ", ")?;
                }
                write!(f, "{}", node.as_usize())?;
            }
            write!(f, ")")?;
        }

        // 4. Arguments and their node IDs
        self.fmt_arguments(f)?;
        if let Some(nodes) = self.argument_node_ids() {
            write!(f, " (nodes: ")?;
            for (i, node) in nodes.iter().enumerate() {
                if i > 0 {
                    write!(f, ", ")?;
                }
                write!(f, "{}", node.as_usize())?;
            }
            write!(f, ")")?;
        }

        write!(f, "]")
    }
}

impl RemapSymbol for Declaration {
    /// Remaps all identifiers in this `Declaration`, including its main symbol,
    /// associated types, and argument identifiers, according to the provided mapping.
    ///
    /// Any `Ident` present in `map` is replaced with the corresponding new value.
    /// This is useful when symbol names are updated during transformations or
    /// domain/problem logic.
    ///
    /// # Parameters
    ///
    /// - `map`: A `HashMap<Ident, Ident>` mapping old identifiers to new identifiers.
    ///
    /// # Errors
    ///
    /// Returns [`InternerError`] if any argument identifier cannot be remapped according to `map`.
    fn remap_symbol(&mut self, map: &HashMap<SymbolId, SymbolId>) -> Result<(), InternerError> {
        // Remap the main symbol name
        if let Some(new_ident) = map.get(&self.symbol_ident()) {
            self.symbol.set_ident(new_ident.clone());
        }

        // Remap associated types
        if let Some(ref mut types) = self.ty {
            for ident in types.iter_mut() {
                if let Some(new_ident) = map.get(ident) {
                    *ident = new_ident.clone();
                }
            }
        }

        // Remap argument identifiers
        if let Some(ref mut args) = self.arguments {
            for arg in args.iter_mut() {
                arg.remap_symbol(map)?;
            }
        }
        Ok(())
    }
}
impl InternerDisplay for Declaration {
    fn fmt_with_interner(
        &self,
        f: &mut fmt::Formatter<'_>,
        interner: &SymbolInterner,
    ) -> fmt::Result {
        let name_str = interner
            .resolve_symbol(self.symbol_ident())
            .unwrap_or("<uninterned>");

        // 1. Informations de base
        write!(
            f,
            "[node: {}, kind: {}, ident: {}",
            self.node_id().as_usize(),
            self.symbol_kind(),
            name_str
        )?;

        // 2. Contexte
        write!(
            f,
            ", scope: {}, origin: {}, imported: {}",
            self.scope(),
            self.origin(),
            self.imported_scope()
                .map_or("None".to_string(), |s| s.to_string())
        )?;

        // 3. Types sémantiques ET leurs NodeIds
        self.fmt_types_with(f, interner)?;
        if let Some(nodes) = self.ty_node_ids() {
            write!(f, " (nodes: ")?;
            for (i, node) in nodes.iter().enumerate() {
                if i > 0 {
                    write!(f, ", ")?;
                }
                write!(f, "{}", node.as_usize())?;
            }
            write!(f, ")")?;
        }

        // 4. Arguments sémantiques ET leurs NodeIds
        self.fmt_arguments_with(f, interner)?;
        if let Some(nodes) = self.argument_node_ids() {
            write!(f, " (nodes: ")?;
            for (i, node) in nodes.iter().enumerate() {
                if i > 0 {
                    write!(f, ", ")?;
                }
                write!(f, "{}", node.as_usize())?;
            }
            write!(f, ")")?;
        }

        write!(f, "]")
    }
}
