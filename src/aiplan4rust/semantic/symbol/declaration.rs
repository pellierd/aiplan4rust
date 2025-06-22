use std::collections::HashMap;
use crate::aiplan4rust::syntax::Span;
use crate::aiplan4rust::interner::StringInterner;
use crate::aiplan4rust::semantic::symbol::{SymbolRef, SymbolSource};
use crate::aiplan4rust::semantic::symbol::Scope;
use crate::aiplan4rust::semantic::symbol::SymbolKind;
use crate::aiplan4rust::semantic::symbol::TypedSymbol;

use serde::Deserialize;
use serde::Serialize;
use std::fmt;
use crate::aiplan4rust::semantic::arena::NodeId;
use crate::aiplan4rust::syntax::elements::Ident;

/// Represents a declaration in the abstract syntax tree (AST).
///
/// This struct holds information about a symbol declared in the program, including its
/// associated AST node, symbol type, scope, source, and optional types and arguments.
///
/// # Fields
///
/// * `ast_old` - The AST node index of the declaration.
/// * `kind` - The type or kind of the symbol declared (e.g., variable, function).
/// * `scope` - The scope in which the declaration is valid.
/// * `source` - The source from which the declaration originates (e.g., file or module).
/// * `types` - An optional list of types associated with the symbol, if any. This can be `None` if
///   not provided.
/// * `arguments` - An optional list of argument types, grouped in parameter lists, if applicable.
///
/// # Example
///
/// ```rust
/// let declaration = Declaration {
///     ast_old: 1,
///     kind: SymbolKind::Variable,
///     scope: Scope::Global,
///     source: Source::File("main.rs".into()),
///     types: Some(vec!["int".into()]),
///     arguments: None,
/// };
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Declaration {

    symbol_ref: SymbolRef,

    /// The scope of the declaration
    scope: Scope,

    /// The source of the declaration
    source: SymbolSource,

    /// Optional list of types associated with the symbol.
    types: Option<Vec<Ident>>,

    /// Optional list of argument types, grouped in parameter lists.
    arguments: Option<Vec<TypedSymbol>>,

    /// The AST node of the declaration
    node_id: NodeId,

    span: Span,

}

impl Declaration {
    /// Constructor to create a new `Declaration`
    pub fn new(
        symbol_ref: SymbolRef,
        scope: Scope,
        source: SymbolSource,
        types: Option<Vec<Ident>>,
        arguments: Option<Vec<TypedSymbol>>,
        span : Span,
        node_id: NodeId,
    ) -> Self {
        Declaration {
            symbol_ref,
            scope,
            source,
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

    /// Returns a reference to the [`SymbolSource`] indicating the origin of the symbol.
    pub fn source(&self) -> SymbolSource {
        self.source
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

    /// Sets the [`SymbolSource`] of this declaration.
    ///
    /// This method is public within the crate to allow controlled updates.
    pub fn set_source(&mut self, source: SymbolSource) {
        self.source = source;
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
                    match interner.get_str(*single) {
                        Some(name) => write!(w, "{})", name)?,
                        None => write!(w, "<uninterned:{}>)", single)?,
                    }
                }
                _ => {
                    write!(w, "either")?;
                    for ty in types {
                        match interner.get_str(*ty) {
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
    pub fn to_string_with_interner(&self, interner: &StringInterner) -> String {
        let mut out = String::new();
        let _ = self.fmt_with_interner(&mut out, interner);
        out
    }

    pub fn fmt_with_interner(
        &self,
        w: &mut dyn fmt::Write,
        interner: &StringInterner,
    ) -> fmt::Result {
        // Récupère la chaîne correspondant à `self.name` via l'interner, ou affiche <uninterned> sinon
        let name_str = match interner.get_str(self.symbol_ident()) {
            Some(name) => name,
            None => "<uninterned>",
        };

        // Affiche les éléments principaux : ast_old, kind, et le nom résolu
        write!(
            w,
            "[index: {}, kind: {}, ident: {}",
            self.node_id(),
            self.symbol_kind(),
            name_str
        )?;

        // Ajoute scope et source
        write!(w, ", scope: {}, source: {}", self.scope(), self.source())?;

        // Appelle la version avec interner pour formater les types
        self.fmt_types_with_interner(w, interner)?;

        // Appelle la version avec interner pour formater les arguments
        self.fmt_arguments_with_interner(w, interner)?;

        // Ferme la bracket
        write!(w, "]")
    }
}

impl fmt::Display for Declaration {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Display the main elements: ast_old, kind, scope, and source
        write!(f, "[index: {}, kind: {}, ident: {}", self.node_id(), self.symbol_kind(), self.symbol_ident())?;

        // Add scope and source at the end
        write!(f, ", scope: {}, source: {}", self.scope(), self.source())?;

        // Call the format_types function to format the types
        self.fmt_types(f)?;

        // Call the format_arguments function to format the arguments
        self.fmt_arguments(f)?;

        // Close the bracket
        write!(f, "]")
    }
}
