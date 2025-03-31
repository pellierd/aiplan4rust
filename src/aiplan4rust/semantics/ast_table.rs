use crate::aiplan4rust::frontend::ParserInternalError;
use crate::aiplan4rust::syntax::ast::{Ast, AstKind, Requirement};
use crate::aiplan4rust::syntax::span::Span;
use linked_hash_map::IntoIter;
use linked_hash_map::LinkedHashMap;
use std::collections::{HashMap, HashSet};

use crate::aiplan4rust::semantics::symbol::Source;
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AstTable {
    map: LinkedHashMap<usize, AstEntry>,
    requirements: HashSet<Requirement>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AstEntry {
    kind: AstKind,
    span: Span,
    children: Vec<usize>,
}

impl AstTable {
    pub fn new() -> Self {
        AstTable {
            map: LinkedHashMap::new(),
            requirements: HashSet::new(),
        }
    }

    pub fn source(&self) -> Source {
        if let Some(first_node) = self.map.values().next() {
            match first_node.kind() {
                AstKind::Domain => Source::Domain,
                AstKind::Problem => Source::Problem,
                _ => Source::Unknown,
            }
        } else {
            Source::Unknown
        }
    }

    pub fn from(ast: &Ast) -> Result<AstTable, ParserInternalError> {
        // Vérification du type de l'AST
        match ast.kind() {
            AstKind::Domain | AstKind::Problem => {
                let index_table = ast.to_hash_map();
                let mut ast_table = AstTable::new();
                let mut next_index = 0;
                AstTable::from_rec(ast, &index_table, &mut ast_table, &mut next_index)?;
                Ok(ast_table)
            }
            _ => Err(ParserInternalError::new(
                "AST must be of type Domain or Problem".to_string(),
            )),
        }
    }

    fn from_rec(
        ast: &Ast,
        index_table: &HashMap<&Ast, usize>,
        ast_table: &mut AstTable,
        next_index: &mut usize,
    ) -> Result<(), ParserInternalError> {
        let entry = AstEntry::from(ast, index_table)?;
        if let AstKind::Requirement(req) = entry.kind() {
            ast_table.add_requirement(req.clone());
        }
        ast_table.insert(*next_index, entry);

        *next_index += 1; // Incrémente pour le prochain nœud

        for child in ast.children() {
            AstTable::from_rec(child.as_ref(), index_table, ast_table, next_index)?;
        }

        Ok(())
    }

    // Vérifie si un AstKind spécifique existe déjà dans la table
    pub fn contains_kind(&self, kind: AstKind) -> bool {
        self.map.iter().any(|(_, entry)| *entry.kind() == kind)
    }

    pub fn insert(&mut self, index: usize, entry: AstEntry) -> Option<AstEntry> {
        self.map.insert(index, entry)
    }

    // Accesseur pour obtenir un noeud à partir de son ID
    pub fn get_entry(&self, id: usize) -> Option<&AstEntry> {
        self.map.get(&id)
    }

    // Méthode pour ajouter un nouveau noeud à la table
    pub fn add_entry(&mut self, id: usize, entry: AstEntry) {
        self.map.insert(id, entry);
    }

    // Méthode pour vérifier si un noeud existe dans la table
    pub fn contains_entry(&self, id: usize) -> bool {
        self.map.contains_key(&id)
    }

    // Méthode pour ajouter un nouveau noeud à la table
    pub fn add_requirement(&mut self, requirement: Requirement) {
        self.requirements.insert(requirement);
    }

    pub fn requirements(&self) -> &HashSet<Requirement> {
        &self.requirements
    }

    /// Returns an iterator over the elements of the `AstTable`.
    ///
    /// The iterator yields pairs of `(key, value)`, where `key` is a `usize`
    /// and `value` is an `AstEntry`. The elements are borrowed immutably.
    ///
    /// # Example
    /// ```
    /// for (key, ast_entry) in ast_table.iter() {
    ///     println!("{key}: {:?}", ast_entry);
    /// }
    /// ```
    pub fn iter(&self) -> impl Iterator<Item = (&usize, &AstEntry)> {
        self.map.iter()
    }

    /// Returns a mutable iterator over the elements of the `AstTable`.
    ///
    /// The iterator yields pairs of `(key, value)`, where `key` is a `usize`
    /// and `value` is a mutable reference to an `AstEntry`. This allows
    /// modifications of the values during iteration.
    ///
    /// # Example
    /// ```
    /// for (key, ast_entry) in ast_table.iter_mut() {
    ///     ast_entry.some_field = new_value;
    /// }
    /// ```
    pub fn iter_mut(&mut self) -> impl Iterator<Item = (&usize, &mut AstEntry)> {
        self.map.iter_mut()
    }
    /// Returns an immutable iterator over the keys of the `AstTable`.
    ///
    /// The iterator yields immutable references to the keys (`usize`).
    /// This allows for read-only access to the keys without consuming the structure.
    ///
    /// # Example
    ///
    /// ```
    /// for key in ast_table.keys() {
    ///     println!("{key}");
    /// }
    /// ```
    pub fn keys(&self) -> impl Iterator<Item = &usize> {
        self.map.keys()
    }

    /// Returns an immutable iterator over the values of the `AstTable`.
    ///
    /// The iterator yields immutable references to the values (`AstEntry`).
    /// This allows for read-only access to the values without consuming the structure.
    ///
    /// # Example
    ///
    /// ```
    /// for value in ast_table.values() {
    ///     println!("{:?}", value);
    /// }
    /// ```
    pub fn values(&self) -> impl Iterator<Item = &AstEntry> {
        self.map.values()
    }
}

impl IntoIterator for AstTable {
    type Item = (usize, AstEntry);
    type IntoIter = IntoIter<usize, AstEntry>;

    /// Consumes the `AstTable` and returns an iterator over its elements.
    ///
    /// The iterator yields owned `(key, value)` pairs, consuming the `AstTable` in the process.
    /// This means that the original `AstTable` will no longer be accessible after calling this method.
    ///
    /// # Example
    /// ```
    /// for (key, ast_entry) in ast_table {
    ///     println!("{key}: {:?}", ast_entry);
    /// }
    /// ```
    fn into_iter(self) -> Self::IntoIter {
        self.map.into_iter()
    }
}

impl fmt::Display for AstTable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "AstTable {{")?;
        for (index, entry) in &self.map {
            writeln!(f, "  {}: {}", index, entry)?;
        }
        write!(f, "}}")
    }
}

impl AstEntry {
    pub fn new(kind: AstKind, span: Span, children: Vec<usize>) -> Self {
        AstEntry {
            kind,
            span,
            children,
        }
    }

    pub fn from(
        ast: &Ast,
        index_table: &HashMap<&Ast, usize>,
    ) -> Result<AstEntry, ParserInternalError> {
        let mut children = Vec::new();

        for child in ast.children() {
            match index_table.get(child.as_ref()) {
                Some(&index) => children.push(index),
                None => {
                    return Err(ParserInternalError::new(format!(
                        "Node not found in index table: {:?}",
                        child
                    )))
                }
            }
        }

        Ok(AstEntry::new(
            ast.kind().clone(),
            ast.span().clone(),
            children,
        ))
    }

    // Accesseur pour obtenir le `kind` d'un noeud
    pub fn kind(&self) -> &crate::aiplan4rust::syntax::ast::AstKind {
        &self.kind
    }

    // Accesseur pour obtenir les enfants du noeud
    pub fn children(&self) -> &Vec<usize> {
        &self.children
    }

    // Accesseur pour obtenir la portée (span) du noeud
    pub fn span(&self) -> &Span {
        &self.span
    }

    // Méthode pour vérifier si un noeud a des enfants
    pub fn has_children(&self) -> bool {
        !self.children.is_empty()
    }

    /// Retrieves the key for the given AST node based on its type.
    /// This function is used to extract the key for symbols used in the symbol table.
    /// If the node is a constant, variable, or one of the predefined symbols, the key is the
    /// symbol's name. For `FunctionTerm` and `AtomicFormula`, it returns a key formatted as
    /// `name/arity` based on their first child.
    ///
    /// The key is used to store and look up symbols in the symbol table. This function ensures that
    /// each symbol has a unique identifier based on its structure, which is useful for semantics
    /// analysis and symbol resolution.
    ///
    /// # Returns
    /// - Ok(String): The key derived from the node.
    /// - Err(ParserInternalError): An error if the node cannot be processed or if it does not meet
    ///   the expected structure.
    ///
    /// # Errors
    /// - If the node has no children or its first child is not a `FunctionSymbol` or
    ///   `PredicateSymbol`.
    /// - If the node is of an unexpected kind.
    pub fn get_key(&self, ast_table: &AstTable) -> Result<String, ParserInternalError> {
        match &self.kind {
            // For symbols like constants, variables, action symbols, etc., return the symbol's
            // name directly.
            AstKind::Constant(name)
            | AstKind::Variable(name)
            | AstKind::PrimitiveType(name)
            | AstKind::DomainName(name)
            | AstKind::ProblemName(name)
            | AstKind::ActionSymbol(name)
            | AstKind::DASymbol(name)
            | AstKind::PrefName(name) => Ok(name.to_string()),
            // For `FunctionTerm` and `AtomicFormula`, derive the key from their first child
            AstKind::FunctionTerm | AstKind::AtomicFormula => {
                // Check if the node has children
                if let Some(child_index) = self.children.first() {
                    let child = ast_table.get_entry(*child_index).unwrap();
                    // Check the type of the first child (it should be either a FunctionSymbol or
                    // PredicateSymbol)
                    match &child.kind {
                        AstKind::FunctionSymbol(name) => Ok(name.to_string()),
                        AstKind::Predicate(name) => Ok(name.to_string()),
                        _ => {
                            // If the first child is neither a FunctionSymbol nor a PredicateSymbol, return an error
                            Err(ParserInternalError::new(
                                format!(
                                    "First child must be a 'FunctionSymbol' or 'PredicateSymbol', but found: {:?}.",
                                    child.kind
                                )
                            ))
                        }
                    }
                } else {
                    // If there are no children, return an error
                    Err(ParserInternalError::new(
                        "No children found for 'FunctionTerm' or 'AtomicFormula'.".to_string(),
                    ))
                }
            }
            // Handle unexpected AST node kinds
            _ => {
                // Return an error if the AST node kind is not recognized
                Err(ParserInternalError::new(format!(
                    "Unexpected AST kind: {:?}",
                    self.kind
                )))
            }
        }
    }
}

impl fmt::Display for AstEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "AstEntry {{ kind: {:?}, span: {}, children: {:?} }}",
            self.kind, self.span, self.children
        )
    }
}
