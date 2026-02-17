//! Module for representing symbol declarations in the AI syntax Rust syntax and semantic analysis.
//!
//! This module defines the `Declaration` struct, which models the declaration of a symbol in the
//! abstract syntax tree (AST) arena, enriched with semantic information such as scope, origin,
//! types, and arguments. It also integrates source location (`Span`) and AST node identity (`NodeId`).
//!
//! # Core Concepts
//!
//! - **SymbolRef**: A reference to a declared symbol, including its identifier and kind (variable, function, etc.).
//! - **Scope**: The visibility and lifetime context of the declaration (e.g., global, local).
//! - **SymbolOrigin**: Origin domain of the symbol, typically indicating if it comes from the domain model or problem context.
//! - **Type** and **TypedList**: Optional type annotations for the symbol and its parameters.
//! - **Span**: Source code range indicating where the declaration occurs.
//! - **NodeId**: The unique AST node identifier associated with the declaration.
//!
//! # Functionality
//!
//! The `Declaration` struct:
//! - Encapsulates all relevant data about a symbol declaration for semantic analysis.
//! - Supports remapping of identifiers, useful in symbol transformations or expr normalization.
//! - Provides formatting helpers to display symbol types and arguments, with or without resolving interned strings.
//! - Implements `fmt::Display` for human-readable string representations of declarations.
//! - Implements `InternerDisplay` to format declarations using a `StringInterner` for readable names.
//!
//! # Examples
//!
//! ```rust
//! use crate::aiplan4rust::semantic::symbol::{Declaration, SymbolRef, SymbolKind, Scope, SymbolOrigin};
//! use crate::aiplan4rust::lang::{Ident, Type, TypedList};
//! use crate::aiplan4rust::syntax::{Span, NodeId};
//!
//! let symbol_ref = SymbolRef::new(Ident::from("x"), SymbolKind::Variable);
//! let declaration = Declaration::new(
//!     symbol_ref,
//!     Scope::Global,
//!     SymbolOrigin::Domain,
//!     Some(Type::from(vec![Ident::from("int")])),
//!     None,
//!     Span::dummy(),
//!     NodeId(1),
//!     None,
//! );
//! println!("{}", declaration);
//! ```
//!
//! This module depends on serde for serialization and deserialization of declarations.

use crate::aiplan4rust::syntax::Span;
use crate::aiplan4rust::interner::{InternerDisplay, InternerError, SymbolInterner};
use crate::aiplan4rust::semantic::symbol::{SymbolOrigin, Symbol};
use crate::aiplan4rust::semantic::symbol::Scope;
use crate::aiplan4rust::semantic::symbol::SymbolKind;
use crate::aiplan4rust::tree::NodeId;
use crate::aiplan4rust::lang::{SymbolId, RemapSymbol};
use crate::aiplan4rust::lang::TypedList;
use crate::aiplan4rust::lang::Type;
use std::collections::HashMap;
use serde::Deserialize;
use serde::Serialize;
use std::fmt;

/// Represents a declaration of a symbol in the abstract syntax arena (AST).
///
/// This struct captures detailed information about a symbol's declaration within
/// the program, including its identity, scope, origin, associated types, parameters,
/// AST syntax, and source code location.
///
/// # Fields
///
/// * `symbol_ref` - The reference to the symbol being declared, containing its identifier and kind.
/// * `scope` - The scope in which this declaration is valid (e.g., global, local).
/// * `origin` - The origin or source domain of the declaration, typically indicating
///   whether it belongs to the domain or problem context.
/// * `types` - An optional list of types associated with the symbol (e.g., return types or annotations).
/// * `arguments` - Optional lists of typed parameters or arguments, grouped by parameter lists, if applicable.
/// * `node_id` - The AST syntax identifier corresponding to this declaration.
/// * `span` - The source span indicating where this declaration occurs in the source code.
///
/// # Example
///
/// ```rust
/// let symbol_ref = SymbolRef::new(Ident::from("x"), SymbolKind::Variable);
/// let declaration = Declaration::new(
///     symbol_ref,
///     Scope::Global,
///     SymbolOrigin::Domain,   // Origin could be Domain or Problem
///     Some(vec![Ident::from("int")]),
///     None,
///     Span::dummy(),
///     NodeId(1),
/// );
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Declaration {
    /// Reference to the symbol declared.
    symbol: Symbol,

    /// The scope of the declaration.
    scope: Scope,

    /// The origin from which the declaration originates.
    origin: SymbolOrigin,

    /// Optional list of types associated with the symbol.
    types: Option<Type<SymbolId>>,

    /// Optional list of argument types, grouped in parameter lists.
    arguments: Option<TypedList<SymbolId, SymbolId>>,

    /// The span in source code where the declaration is located.
    span: Span,

    /// The AST syntax ID corresponding to this declaration.
    node_id: NodeId,

    /// Original scope if this declaration was imported, otherwise `None`.
    imported_scope: Option<Scope>,
}

impl Declaration {
    /// Creates a new `Declaration` instance.
    ///
    /// Constructs a `Declaration` that represents the declaration of a symbol within
    /// a given scope, along with optional type_checker information and arguments.
    ///
    /// # Parameters
    ///
    /// - `symbol_ref`: A reference to the symbol being declared.
    /// - `scope`: The scope in which this declaration is valid (e.g., function, module).
    /// - `origin`: The origin or source of the declaration (e.g., domain or problem).
    /// - `types`: An optional list of types associated with the symbol (e.g., return types or type_checker
    ///   annotations).
    /// - `arguments`: An optional list of typed symbols representing the parameters or arguments,
    ///   possibly grouped by parameter lists.
    /// - `span`: The source code span that locates where the declaration appears.
    /// - `node_id`: The AST syntax identifier corresponding to this declaration.
    ///
    /// # Returns
    ///
    /// A new `Declaration` instance populated with the provided information.
    ///
    /// # Example
    ///
    /// ```
    /// let decl = Declaration::new(
    ///     symbol_ref,
    ///     scope,
    ///     source,
    ///     Some(vec![type_ident]),
    ///     Some(vec![typed_arg]),
    ///     span,
    ///     node_id,
    /// );
    /// ```
    pub fn new(
        symbol: Symbol,
        scope: Scope,
        origin: SymbolOrigin,
        types: Option<Type<SymbolId>>,
        arguments: Option<TypedList<SymbolId, SymbolId>>,
        span: Span,
        node_id: NodeId,
        imported_scope: Option<Scope>,
    ) -> Self {
        Declaration {
            symbol,
            scope,
            origin,
            types,
            arguments,
            span,
            node_id,
            imported_scope
        }
    }


    /// Returns a reference to the [`SymbolRef`] associated with this usage.
    pub fn symbol(&self) -> &Symbol {
        &self.symbol
    }

    /// Returns the [`SymbolId`] of the referenced symbol.
    pub fn symbol_ident(&self) -> SymbolId {
        self.symbol.id()
    }

    /// Returns the [`SymbolKind`] of the referenced symbol.
    pub fn symbol_kind(&self) -> SymbolKind {
        self.symbol.kind()
    }

    /// Returns a reference to the [`Scope`] in which the symbol is used.
    pub fn scope(&self) -> &Scope {
        &self.scope
    }

    pub fn set_scope(&mut self, scope: Scope) {
        self.scope = scope;
    }

    /// Returns a reference to the [`SymbolOrigin`] indicating the origin of the symbol.
    pub fn origin(&self) -> SymbolOrigin {
        self.origin
    }

    /// Returns an optional reference to the list of types associated with the symbol.
    pub fn types(&self) -> Option<&Type<SymbolId>> {
        self.types.as_ref()
    }

    /// Returns an optional reference to the list of arguments associated with the symbol.
    pub fn arguments(&self) -> Option<&TypedList<SymbolId, SymbolId>> {
        self.arguments.as_ref()
    }

    /// Returns a reference to the [`Span`] in the source code.
    pub fn span(&self) -> &Span {
        &self.span
    }

    /// Returns the [`NodeId`] of the AST syntax associated with this usage.
    pub fn node_id(&self) -> NodeId {
        self.node_id
    }

    /// Sets the [`SymbolOrigin`] of this declaration.
    ///
    /// This method is public within the crate to allow controlled updates.
    pub fn set_origin(&mut self, origin: SymbolOrigin) {
        self.origin = origin;
    }

    /// Returns a reference to the imported scope if any.
    pub fn imported_scope(&self) -> Option<&Scope> {
        self.imported_scope.as_ref()
    }

    /// Sets the imported scope.
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
        if let Some(types) = &self.types {
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
        if let Some(types) = &self.types {
            write!(w, ", types: (")?;

            match types.members() {
                [] => { /* no types */ }
                [single] => {
                    match interner.resolve_symbol(*single) {
                        Some(name) => write!(w, "{}", name)?,
                        None => write!(w, "<uninterned:{}>", single)?,
                    }
                }
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
    /// including its AST syntax index, symbol kind, identifier, scope, source,
    /// and optionally the associated types and argument types.
    ///
    /// The output format looks like:
    /// `[index: <node_id>, kind: <symbol_kind>, ident: <identifier>, scope: <scope>, source:
    ///  <source>, types: [...], arguments: [...]]`
    ///
    /// # Arguments
    ///
    /// * `f` - The formatter used to write the string representation.
    ///
    /// # Returns
    ///
    /// A `fmt::Result` which is `Ok` if formatting succeeded, or an error if it failed.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Display the main elements: ast_old, kind, scope, and source
        write!(f, "[index: {}, kind: {}, ident: {}", self.node_id().as_usize(), self.symbol_kind(), self.symbol_ident())?;

        // Add scope and source at the end
        write!(
            f,
            ", scope: {}, source: {}, imported: {}",
            self.scope(),
            self.origin(),
            self.imported_scope().map_or("None".to_string(), |s| s.to_string())
        )?;

        // Call the format_types function to format the types
        self.fmt_types(f)?;

        // Call the format_arguments function to format the arguments
        self.fmt_arguments(f)?;

        // Close the bracket
        write!(f, "]")
    }
}

impl RemapSymbol for Declaration {
    /// Remaps all identifiers in this `Declaration`, including its main symbol,
    /// associated types, and argument identifiers, according to the provided mapping.
    ///
    /// Any `Ident` present in `map` is replaced with the corresponding new value.
    /// This is useful when symbol names are updated during transformations or
    /// domain/problem expr.
    ///
    /// # Parameters
    ///
    /// - `map`: A `HashMap<Ident, Ident>` mapping old identifiers to new identifiers.
    ///
    /// # Errors
    ///
    /// Returns [`InternerError`] if any argument identifier cannot be remapped according to `map`.
    fn remap_symbol(&mut self, map: &HashMap<SymbolId, SymbolId>) -> Result<(), InternerError>{
        // Remap the main symbol name
        if let Some(new_ident) = map.get(&self.symbol_ident()) {
            self.symbol.set_ident(new_ident.clone());
        }

        // Remap associated types
        if let Some(ref mut types) = self.types {
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
        let name_str = match interner.resolve_symbol(self.symbol_ident()) {
            Some(name) => name,
            None => "<uninterned>",
        };

        write!(
            f,
            "[index: {}, kind: {}, ident: {}",
            self.node_id().as_usize(),
            self.symbol_kind(),
            name_str
        )?;

        write!(
            f,
            ", scope: {}, source: {}, imported: {}",
            self.scope(),
            self.origin(),
            self.imported_scope().map_or("None".to_string(), |s| s.to_string())
        )?;

        // Appelle les helpers définies dans ce même impl
        self.fmt_types_with(f, interner)?;
        self.fmt_arguments_with(f, interner)?;

        write!(f, "]")
    }

}
