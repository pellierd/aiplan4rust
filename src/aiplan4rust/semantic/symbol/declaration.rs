use crate::aiplan4rust::syntax::Span;
use crate::aiplan4rust::interner::StringInterner;
use crate::aiplan4rust::semantic::symbol::{SymbolRef, SymbolOrigin};
use crate::aiplan4rust::semantic::symbol::Scope;
use crate::aiplan4rust::semantic::symbol::SymbolKind;
use crate::aiplan4rust::semantic::symbol::TypedSymbol;
use crate::aiplan4rust::tree::NodeId;
use crate::aiplan4rust::syntax::elements::Ident;

use std::collections::HashMap;
use serde::Deserialize;
use serde::Serialize;
use std::fmt;

/// Represents a declaration of a symbol in the abstract syntax tree (AST).
///
/// This struct captures detailed information about a symbol's declaration within
/// the program, including its identity, scope, origin, associated types, parameters,
/// AST node, and source code location.
///
/// # Fields
///
/// * `symbol_ref` - The reference to the symbol being declared, containing its identifier and kind.
/// * `scope` - The scope in which this declaration is valid (e.g., global, local).
/// * `origin` - The origin or source domain of the declaration, typically indicating
///   whether it belongs to the domain or problem context.
/// * `types` - An optional list of types associated with the symbol (e.g., return types or annotations).
/// * `arguments` - Optional lists of typed parameters or arguments, grouped by parameter lists, if applicable.
/// * `node_id` - The AST node identifier corresponding to this declaration.
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
    // Reference to the symbol declared.
    symbol_ref: SymbolRef,

    // The scope of the declaration.
    scope: Scope,

    // The origin from which the declaration originates.
    origin: SymbolOrigin,

    // Optional list of types associated with the symbol.
    types: Option<Vec<Ident>>,

    // Optional list of argument types, grouped in parameter lists.
    arguments: Option<Vec<TypedSymbol>>,

    // The span in source code where the declaration is located.
    span: Span,

    // The AST node ID corresponding to this declaration.
    node_id: NodeId,
}

impl Declaration {
    /// Creates a new `Declaration` instance.
    ///
    /// Constructs a `Declaration` that represents the declaration of a symbol within
    /// a given scope, along with optional type information and arguments.
    ///
    /// # Parameters
    ///
    /// - `symbol_ref`: A reference to the symbol being declared.
    /// - `scope`: The scope in which this declaration is valid (e.g., function, module).
    /// - `origin`: The origin or source of the declaration (e.g., domain or problem).
    /// - `types`: An optional list of types associated with the symbol (e.g., return types or type
    ///   annotations).
    /// - `arguments`: An optional list of typed symbols representing the parameters or arguments,
    ///   possibly grouped by parameter lists.
    /// - `span`: The source code span that locates where the declaration appears.
    /// - `node_id`: The AST node identifier corresponding to this declaration.
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
        symbol_ref: SymbolRef,
        scope: Scope,
        origin: SymbolOrigin,
        types: Option<Vec<Ident>>,
        arguments: Option<Vec<TypedSymbol>>,
        span: Span,
        node_id: NodeId,
    ) -> Self {
        Declaration {
            symbol_ref,
            scope,
            origin,
            types,
            arguments,
            span,
            node_id,
        }
    }


    /// Returns a reference to the [`SymbolRef`] associated with this usage.
    pub fn symbol_ref(&self) -> &SymbolRef {
        &self.symbol_ref
    }

    /// Returns the [`Ident`] of the referenced symbol.
    pub fn symbol_ident(&self) -> Ident {
        self.symbol_ref.ident()
    }

    /// Returns the [`SymbolKind`] of the referenced symbol.
    pub fn symbol_kind(&self) -> SymbolKind {
        self.symbol_ref.kind()
    }

    /// Returns a reference to the [`Scope`] in which the symbol is used.
    pub fn scope(&self) -> &Scope {
        &self.scope
    }

    /// Returns a reference to the [`SymbolOrigin`] indicating the origin of the symbol.
    pub fn origin(&self) -> SymbolOrigin {
        self.origin
    }

    /// Returns an optional reference to the list of types associated with the symbol.
    pub fn types(&self) -> Option<&Vec<Ident>> {
        self.types.as_ref()
    }

    /// Returns an optional reference to the list of arguments associated with the symbol.
    pub fn arguments(&self) -> Option<&Vec<TypedSymbol>> {
        self.arguments.as_ref()
    }

    /// Returns a reference to the [`Span`] in the source code.
    pub fn span(&self) -> &Span {
        &self.span
    }

    /// Returns the [`NodeId`] of the AST node associated with this usage.
    pub fn node_id(&self) -> NodeId {
        self.node_id
    }

    /// Sets the [`SymbolOrigin`] of this declaration.
    ///
    /// This method is public within the crate to allow controlled updates.
    pub fn set_origin(&mut self, origin: SymbolOrigin) {
        self.origin = origin;
    }

    /// Remaps all [`Ident`] values in this declaration using the provided mapping.
    ///
    /// This updates:
    /// - The identifier of the referenced symbol.
    /// - All associated types (if any).
    /// - All argument identifiers in parameter lists (if any).
    ///
    /// Useful during transformations or normalizations where symbol names are changed.
    ///
    /// # Arguments
    ///
    /// * `map` - A mapping from old [`Ident`]s to new [`Ident`]s.
    pub fn remap_idents(&mut self, map: &HashMap<Ident, Ident>) {
        // Remap the main symbol name
        if let Some(new_ident) = map.get(&self.symbol_ident()) {
            self.symbol_ref.set_ident(new_ident.clone());
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
                arg.remap_idents(map);
            }
        }
    }

    /// Formats the types of the declaration for display.
    ///
    /// If the declaration has a list of types, this function formats them and writes them
    /// to the formatter. The types are displayed in a parenthesized list. If there is only
    /// one type, it is directly displayed. If there are multiple types, they are prefixed
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
                // Directly format the single type
                if let Some(single_type) = types.iter().next() {
                    write!(f, "{})", single_type)?;
                }
            } else {
                // Format multiple types with 'either'
                write!(f, "either")?;
                for ty in types {
                    write!(f, " {}", ty)?;
                }
                write!(f, ")")?;
            }
        }
        Ok(())
    }

    /// Formats the type information of the declaration using a `StringInterner`
    /// to resolve identifier names.
    ///
    /// This function writes a human-readable representation of the declaration's
    /// associated types into the given [`std::fmt::Formatter`], resolving interned
    /// identifiers into strings using the provided [`StringInterner`].
    ///
    /// - If there are no types, it writes `", types: ()"`.
    /// - If there is one type, it writes it directly: `", types: (type_name)"`.
    /// - If there are multiple types, it uses PDDL's `either` syntax:
    ///   `", types: (either type1 type2 ...)"`.
    ///
    /// # Arguments
    ///
    /// * `f` - A mutable reference to the formatter used for output.
    /// * `interner` - A reference to the `StringInterner` used to resolve identifiers.
    ///
    /// # Returns
    ///
    /// A [`fmt::Result`] indicating success or failure during formatting.
    ///
    /// # Example Output
    ///
    /// - `types: ()`
    /// - `types: (object)`
    /// - `types: (either vehicle robot)`
    ///
    /// Uninterned identifiers will be printed as `<uninterned:ID>`.
    pub fn fmt_types_with_interner(
        &self,
        w: &mut dyn fmt::Write,
        interner: &StringInterner,
    ) -> fmt::Result {
        if let Some(types) = &self.types {
            write!(w, ", types: (")?;

            match types.as_slice() {
                [] => write!(w, ")")?,
                [single] => {
                    match interner.resolve(*single) {
                        Some(name) => write!(w, "{})", name)?,
                        None => write!(w, "<uninterned:{}>)", single)?,
                    }
                }
                _ => {
                    write!(w, "either")?;
                    for ty in types {
                        match interner.resolve(*ty) {
                            Some(name) => write!(w, " {}", name)?,
                            None => write!(w, " <uninterned:{}>", ty)?,
                        }
                    }
                    write!(w, ")")?;
                }
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
    fn fmt_arguments(&self, w: &mut dyn fmt::Write,) -> fmt::Result {
        if let Some(arguments) = &self.arguments {
            write!(w, ", arguments: (")?;
            for (i, argument) in arguments.iter().enumerate() {
                if i > 0 {
                    write!(w, " ")?;
                }
                // Affiche le nom de l'argument avec son index
                write!(w, "{}", argument)?;
            }
            write!(w, ")")?;
        }
        Ok(())
    }

    /// Formats the arguments of the declaration using a `StringInterner`
    /// to resolve interned identifiers into strings.
    ///
    /// This function writes a human-readable list of the declaration's arguments
    /// into the given [`std::fmt::Formatter`]. Each argument is resolved using
    /// the provided `StringInterner`.
    ///
    /// # Arguments
    ///
    /// * `f` - The formatter to write the output to.
    /// * `interner` - The `StringInterner` used to look up string values from argument identifiers.
    ///
    /// # Example Output
    ///
    /// - `arguments: (x y z)`
    ///
    /// If an identifier is not found in the interner, it will be printed as `<uninterned:ID>`.
    pub fn fmt_arguments_with_interner(
        &self,
        w: &mut dyn fmt::Write,
        interner: &StringInterner,
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
    /// Converts the `Usage` to a `String` using a given `StringInterner`.
    ///
    /// This method formats the `Usage` by resolving symbol identifiers through the
    /// provided interner, producing a human-readable string representation.
    ///
    /// # Arguments
    ///
    /// * `interner` - A reference to a `StringInterner` used to resolve interned strings.
    ///
    /// # Returns
    ///
    /// A `String` representing the formatted `Usage`.
    pub fn to_string_with_interner(&self, interner: &StringInterner) -> String {
        let mut out = String::new();
        let _ = self.fmt_with_interner(&mut out, interner);
        out
    }

    /// Formats the `Usage` to a formatter using a `StringInterner` to resolve identifiers.
    ///
    /// Writes a detailed representation of the usage, including its AST node index,
    /// symbol kind, resolved identifier name, scope, source, and optionally formatted
    /// types and arguments.
    ///
    /// # Arguments
    ///
    /// * `w` - A mutable reference to a writer implementing `fmt::Write`.
    /// * `interner` - A reference to a `StringInterner` used to resolve interned strings.
    ///
    /// # Returns
    ///
    /// Returns a `fmt::Result` indicating success or failure of the formatting operation.
    pub fn fmt_with_interner(
        &self,
        w: &mut dyn fmt::Write,
        interner: &StringInterner,
    ) -> fmt::Result {
        // Récupère la chaîne correspondant à `self.name` via l'interner, ou affiche <uninterned> sinon
        let name_str = match interner.resolve(self.symbol_ident()) {
            Some(name) => name,
            None => "<uninterned>",
        };

        // Affiche les éléments principaux : node_id, kind, et le nom résolu
        write!(
            w,
            "[index: {}, kind: {}, ident: {}",
            self.node_id().as_usize(),
            self.symbol_kind(),
            name_str
        )?;

        // Ajoute scope et source
        write!(w, ", scope: {}, source: {}", self.scope(), self.origin())?;

        // Appelle la version avec interner pour formater les types
        self.fmt_types_with_interner(w, interner)?;

        // Appelle la version avec interner pour formater les arguments
        self.fmt_arguments_with_interner(w, interner)?;

        // Ferme la bracket
        write!(w, "]")
    }
}

impl fmt::Display for Declaration {
    /// Formats the `Declaration` for display purposes.
    ///
    /// This implementation writes a structured representation of the declaration,
    /// including its AST node index, symbol kind, identifier, scope, source,
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
        write!(f, ", scope: {}, source: {}", self.scope(), self.origin())?;

        // Call the format_types function to format the types
        self.fmt_types(f)?;

        // Call the format_arguments function to format the arguments
        self.fmt_arguments(f)?;

        // Close the bracket
        write!(f, "]")
    }
}
