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
use crate::aiplan4rust::semantic::symbol::declaration::DeclarationError;
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

    /// The source file of the declaration, indicating if it originates
    /// from a Domain or a Problem file.
    origin: SymbolOrigin,

    /// Optional semantic typing information associated with the symbol.
    ty: Option<Type<SymbolId>>,

    /// Optional list of AST node identifiers corresponding to each type in `ty`.
    /// Provides the exact syntactic source for every declared type.
    type_sources: Option<Vec<NodeId>>,

    /// Optional list of typed parameters, used for symbols with signatures
    /// such as predicates or functions.
    arguments: Option<TypedList<SymbolId, SymbolId>>,

    /// Optional list of AST node identifiers for each individual argument name.
    /// Ensures a 1:1 mapping between semantic arguments and their syntactic source.
    argument_sources: Option<Vec<NodeId>>,

    /// The specific range in the source code covered by this declaration.
    span: Span,

    /// The primary AST node identifier representing this declaration's main definition.
    source: NodeId,

    /// The original scope context if this declaration was imported
    /// from an external module; otherwise `None`.
    imported_scope: Option<Scope>,

    /// The AST node identifier of the original declaration if this is an alias.
    /// Primarily used to link Problem declarations back to their Domain source.
    alias: Option<NodeId>,

    /// List of AST node identifiers (`Usage`) formally resolved to this declaration.
    /// Represents the result of the bidirectional linking between usage and definition.
    usages: Vec<NodeId>,

    /// For `SymbolKind::Predicate`, contains the identifiers of `DerivedPredicate`
    /// nodes (axioms) that provide a logical implementation for this symbol.
    derivations: Vec<NodeId>,

    /// For `SymbolKind::DerivedPredicate`, contains the identifier of the base
    /// `Predicate` node (the signature) from which this implementation originates.
    derived_source: Option<NodeId>,

    /// Indique si cette déclaration est une définition logique (axiome)
    /// ou une signature de prédicat standard.
    derived: bool,
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
        type_sources: Option<Vec<NodeId>>,
        arguments: Option<TypedList<SymbolId, SymbolId>>,
        argument_sources: Option<Vec<NodeId>>,
        span: Span,
        source: NodeId,
        imported_scope: Option<Scope>,
        alias: Option<NodeId>,
    ) -> Self {
        Declaration {
            symbol,
            scope,
            origin,
            ty,
            type_sources,
            arguments,
            argument_sources,
            span,
            source,
            imported_scope,
            alias,
            usages: vec![],
            derivations: vec![],
            derived_source: None,
            derived: false,
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
    /// * `scope` - The new visibility context (e.g., Global, Local) to associate with this declaration.
    pub fn set_scope(&mut self, scope: Scope) {
        self.scope = scope;
    }

    /// Returns an optional reference to the typing information associated with the symbol.
    ///
    /// # Returns
    /// * `Some(&Type<SymbolId>)` - A reference to the semantic type definition if the symbol is typed.
    /// * `None` - If the symbol has no associated type.
    pub fn ty(&self) -> Option<&Type<SymbolId>> {
        self.ty.as_ref()
    }

    /// Returns an optional slice of the AST node identifiers corresponding to the types.
    ///
    /// These sources allow mapping each semantic type back to its precise syntactic location.
    ///
    /// # Returns
    /// * `Some(&[NodeId])` - A slice of node identifiers representing the type components.
    /// * `None` - If no type source nodes are recorded.
    pub fn type_sources(&self) -> Option<&[NodeId]> {
        self.type_sources.as_deref()
    }

    /// Récupère le type de la déclaration elle-même (le type fourni) ou renvoie une erreur de type manquant.
    pub fn try_type(&self) -> Result<&Type<SymbolId>, DeclarationError> {
        self.ty()
            .ok_or_else(|| DeclarationError::missing_symbol_types(self.symbol().id()))
    }

    /// Sets the semantic types associated with this declaration.
    ///
    /// # Arguments
    /// * `ty` - The semantic type definition to assign to this symbol.
    pub fn set_ty(&mut self, ty: Type<SymbolId>) {
        self.ty = Some(ty);
    }

    /// Sets the AST node identifiers corresponding to the typing syntax.
    ///
    /// # Arguments
    /// * `node_ids` - A vector of node identifiers representing the type components in the source tree.
    pub fn set_type_sources(&mut self, node_ids: Vec<NodeId>) {
        self.type_sources = Some(node_ids);
    }

    /// Clears both semantic types and their associated syntax node identifiers.
    pub fn clear_type(&mut self) {
        self.ty = None;
        self.type_sources = None;
    }

    /// Returns an optional reference to the parameters associated with the symbol.
    ///
    /// # Returns
    /// * `Some(&TypedList<SymbolId, SymbolId>)` - A reference to the argument list (e.g., for predicates).
    /// * `None` - If the symbol does not have arguments.
    pub fn arguments(&self) -> Option<&TypedList<SymbolId, SymbolId>> {
        self.arguments.as_ref()
    }

    /// Sets the list of arguments for symbols like predicates or functions.
    ///
    /// # Arguments
    /// * `arguments` - The typed parameter list to assign.
    pub fn set_arguments(&mut self, arguments: TypedList<SymbolId, SymbolId>) {
        self.arguments = Some(arguments);
    }

    pub fn try_get_arg_type(&self, index: usize) -> Result<&Type<SymbolId>, DeclarationError> {
        self.arguments()
            .and_then(|args| args.get(index))
            .map(|arg| arg.ty())
            .ok_or_else(|| DeclarationError::argument_index_out_of_bounds(index))
    }

    /// Returns an optional slice of the AST node identifiers corresponding to the arguments.
    ///
    /// This provides a direct link between each semantic parameter and its original syntax node.
    ///
    /// # Returns
    /// * `Some(&[NodeId])` - A slice of node identifiers representing the argument nodes.
    /// * `None` - If no argument source nodes are recorded.
    pub fn argument_sources(&self) -> Option<&[NodeId]> {
        self.argument_sources.as_deref()
    }

    /// Sets the AST node identifiers corresponding to the argument syntax.
    ///
    /// # Arguments
    /// * `ids` - A vector of node identifiers representing the arguments in the source tree.
    pub fn set_argument_sources(&mut self, ids: Vec<NodeId>) {
        self.argument_sources = Some(ids);
    }

    /// Clears both semantic arguments and their associated syntax node identifiers.
    pub fn clear_arguments(&mut self) {
        self.arguments = None;
        self.argument_sources = None;
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

    /// Returns the primary AST node identifier associated with this declaration.
    ///
    /// This node represents the main syntactic definition of the symbol in the source code.
    ///
    /// # Returns
    /// * `NodeId` - The unique identifier of the associated AST node.
    pub fn source(&self) -> NodeId {
        self.source
    }

    /// Sets the primary AST node identifier for this declaration.
    ///
    /// # Arguments
    /// * `node_id` - The new AST node identifier to be associated with this declaration.
    pub fn set_source(&mut self, node_id: NodeId) {
        self.source = node_id;
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

    /// Returns the original AST node identifier if this declaration is an alias.
    ///
    /// # Returns
    /// * `Some(NodeId)` - The identifier of the source of truth.
    /// * `None` - If this declaration is the primary definition.
    pub fn alias(&self) -> Option<NodeId> {
        self.alias
    }

    /// Defines the source node for this declaration, effectively creating an alias.
    ///
    /// This is typically used during the linkage phase to point from a
    /// Problem-specific declaration back to its original Domain definition.
    ///
    /// # Arguments
    /// * `node_id` - The identifier of the original declaration node.
    pub fn set_alias(&mut self, node_id: NodeId) {
        self.alias = Some(node_id);
    }

    /// Returns whether this declaration is an alias to another node.
    ///
    /// # Returns
    /// * `true` - If the declaration is an alias.
    /// * `false` - If it is a primary declaration.
    pub fn is_alias(&self) -> bool {
        self.alias.is_some()
    }

    /// Registers a resolved usage for this declaration.
    ///
    /// # Arguments
    /// * `usage_node_id` - The AST node identifier where this symbol is used.
    ///
    /// # Returns
    /// * `true` - If the usage was newly added.
    /// * `false` - If the usage was already registered.
    pub fn add_usage(&mut self, usage_node_id: NodeId) -> bool {
        #[cfg(debug_assertions)]
        {
            // En mode Debug : on garde la sécurité maximale
            if self.usages.contains(&usage_node_id) {
                // Optionnel : tu peux panic! ici si tu veux être sûr
                // de corriger tes bugs de parcours d'AST.
                return false;
            }
        }

        // En mode Release : le bloc au-dessus est supprimé.
        // On fait le push en O(1) sans se poser de questions.
        self.usages.push(usage_node_id);
        true
    }

    /// Returns a slice of all AST node identifiers resolved to this declaration.
    ///
    /// # Returns
    /// * `&[NodeId]` - A slice containing all registered usage identifiers.
    pub fn usages(&self) -> &[NodeId] {
        &self.usages
    }

    /// Returns the list of `DerivedPredicate` nodes that define this symbol's logic.
    ///
    /// # Returns
    /// * `&[NodeId]` - A slice of identifiers for the associated axiom nodes.
    pub fn derivations(&self) -> &[NodeId] {
        &self.derivations
    }

    /// Returns the base `Predicate` signature node that this derived predicate implements.
    ///
    /// # Returns
    /// * `Some(NodeId)` - The ID of the abstract predicate signature.
    /// * `None` - If this symbol is not a derived predicate.
    pub fn derived_source(&self) -> Option<NodeId> {
        self.derived_source
    }

    /// Establishes a link to a derived predicate (axiom) for this symbol.
    ///
    /// # Arguments
    /// * `derived_node_id` - The AST node identifier of the `DerivedPredicate`.
    ///
    /// # Returns
    /// * `true` - If the derivation link was newly established.
    /// * `false` - If the link already existed.
    pub fn add_derivation(&mut self, derived_node_id: NodeId) -> bool {
        #[cfg(debug_assertions)]
        {
            // On garde le check en mode Debug pour valider la logique du compilateur
            if self.derivations.contains(&derived_node_id) {
                return false;
            }
        }

        // En mode Release (--release), on fonce en O(1)
        self.derivations.push(derived_node_id);
        true
    }

    /// Sets the base predicate signature node that this derived predicate refers to.
    ///
    /// # Arguments
    /// * `node_id` - The identifier of the base `Predicate` signature node.
    pub fn set_derived_source(&mut self, node_id: NodeId) {
        self.derived_source = Some(node_id);
    }

    /// Retourne vrai si cette déclaration est un axiome dérivé.
    pub fn is_derived(&self) -> bool {
        self.derived
    }

    /// Définit si cette déclaration est un axiome dérivé.
    pub fn set_derived(&mut self, derived: bool) {
        self.derived = derived;
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
        write!(w, ", types: (")?;

        if let Some(types) = &self.ty {
            let members = types.members();
            match members.len() {
                0 => {} // Reste vide : ()
                1 => {
                    // Un seul type
                    let ty = members[0];
                    match interner.resolve_symbol(ty) {
                        Some(name) => write!(w, "{}", name)?,
                        None => write!(w, "<uninterned:{}>", ty)?,
                    }
                }
                _ => {
                    // Plusieurs types (PDDL 'either')
                    write!(w, "either")?;
                    for ty in members {
                        match interner.resolve_symbol(*ty) {
                            Some(name) => write!(w, " {}", name)?,
                            None => write!(w, " <uninterned:{}>", ty)?,
                        }
                    }
                }
            }
        }

        write!(w, ")")
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
        write!(w, ", arguments: (")?;

        if let Some(arguments) = &self.arguments {
            for (i, typed_symbol) in arguments.iter().enumerate() {
                if i > 0 {
                    write!(w, " ")?;
                }
                typed_symbol.fmt_with_interner(w, interner)?;
            }
        }

        write!(w, ")")
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
        // 1. Remap the main symbol name
        if let Some(&new_ident) = map.get(&self.symbol_ident()) {
            self.symbol.set_ident(new_ident);
        }

        // 2. Remap associated types
        if let Some(types) = self.ty.as_mut() {
            for ident in types.iter_mut() {
                if let Some(&new_ident) = map.get(ident) {
                    *ident = new_ident;
                }
            }
        }

        // 3. Remap argument identifiers (recursive call)
        if let Some(args) = self.arguments.as_mut() {
            for arg in args.iter_mut() {
                arg.remap_symbol(map)?;
            }
        }

        Ok(())
    }
}

impl fmt::Display for Declaration {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // 1. Core Identity
        // On affiche l'ID du nœud, le genre, le flag de dérivation et l'ID du symbole.
        write!(
            f,
            "[node: {}, kind: {}, derived: {}, ident: {}",
            self.source(),
            self.symbol_kind(),
            self.is_derived(), // Ton nouveau flag booléen
            self.symbol_ident()
        )?;

        // 2. Context and Provenance
        write!(
            f,
            ", scope: {}, origin: {}, alias: {}, imported: {}",
            self.scope(),
            self.origin(),
            self.alias()
                .map_or("None".to_string(), |id| id.as_usize().to_string()),
            self.imported_scope()
                .map_or("None".to_string(), |s| s.to_string())
        )?;

        // 3. Types and Source Mapping
        self.fmt_types(f)?;
        if let Some(nodes) = self.type_sources() {
            write!(f, " (sources: ")?;
            for (i, node) in nodes.iter().enumerate() {
                if i > 0 {
                    write!(f, ", ")?;
                }
                write!(f, "{}", node)?;
            }
            write!(f, ")")?;
        }

        // 4. Arguments (Signatures) and Source Mapping
        self.fmt_arguments(f)?;
        if let Some(nodes) = self.argument_sources() {
            write!(f, " (sources: ")?;
            for (i, node) in nodes.iter().enumerate() {
                if i > 0 {
                    write!(f, ", ")?;
                }
                write!(f, "{}", node)?;
            }
            write!(f, ")")?;
        }

        // 5. Predicate Linkage (Base <-> Derived)
        if !self.derivations().is_empty() {
            write!(f, ", derivations: [")?;
            for (i, node) in self.derivations().iter().enumerate() {
                if i > 0 {
                    write!(f, ", ")?;
                }
                write!(f, "{}", node)?;
            }
            write!(f, "]")?;
        }

        if let Some(source) = self.derived_source() {
            write!(f, ", derived_from: {}", source)?;
        }

        // 6. Usage Tracking
        if !self.usages().is_empty() {
            write!(f, ", usages: [")?;
            for (i, node) in self.usages().iter().enumerate() {
                if i > 0 {
                    write!(f, ", ")?;
                }
                write!(f, "{}", node)?;
            }
            write!(f, "]")?;
        }

        write!(f, "]")
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

        // 1. Core Identity
        // Ajout du flag 'derived' ici pour voir immédiatement si c'est un axiome.
        write!(
            f,
            "[node: {}, kind: {}, derived: {}, ident: {}",
            self.source().as_usize(),
            self.symbol_kind(),
            self.is_derived(), // Utilisation de ton nouveau booléen
            name_str
        )?;

        // 2. Context and Provenance
        write!(
            f,
            ", scope: {}, origin: {}, alias: {}, imported: {}",
            self.scope(),
            self.origin(),
            self.alias()
                .map_or("None".to_string(), |id| id.as_usize().to_string()),
            self.imported_scope()
                .map_or("None".to_string(), |s| s.to_string())
        )?;

        // 3. Semantic Types and Source Mapping
        self.fmt_types_with(f, interner)?;
        if let Some(nodes) = self.type_sources() {
            write!(f, " (sources: ")?;
            for (i, node) in nodes.iter().enumerate() {
                if i > 0 {
                    write!(f, ", ")?;
                }
                write!(f, "{}", node.as_usize())?;
            }
            write!(f, ")")?;
        }

        // 4. Arguments (Signatures) and Source Mapping
        self.fmt_arguments_with(f, interner)?;
        if let Some(nodes) = self.argument_sources() {
            write!(f, " (sources: ")?;
            for (i, node) in nodes.iter().enumerate() {
                if i > 0 {
                    write!(f, ", ")?;
                }
                write!(f, "{}", node.as_usize())?;
            }
            write!(f, ")")?;
        }

        // 5. Predicate Linkage (Base <-> Derived)
        // Utile pour les Predicates standards qui ont des définitions logiques.
        if !self.derivations().is_empty() {
            write!(f, ", derivations: [")?;
            for (i, node) in self.derivations().iter().enumerate() {
                if i > 0 {
                    write!(f, ", ")?;
                }
                write!(f, "{}", node.as_usize())?;
            }
            write!(f, "]")?;
        }

        // Utile pour les Axiomes (derived: true) pour remonter à la signature.
        if let Some(source) = self.derived_source() {
            write!(f, ", derived_from: {}", source.as_usize())?;
        }

        // 6. Usage Tracking
        if !self.usages().is_empty() {
            write!(f, ", usages: [")?;
            for (i, node) in self.usages().iter().enumerate() {
                if i > 0 {
                    write!(f, ", ")?;
                }
                write!(f, "{}", node.as_usize())?;
            }
            write!(f, "]")?;
        }

        write!(f, "]")
    }
}
