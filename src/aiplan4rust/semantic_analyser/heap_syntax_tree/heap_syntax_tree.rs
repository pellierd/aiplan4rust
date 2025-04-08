use crate::aiplan4rust::frontend::ParserInternalError;
use crate::aiplan4rust::syntax::elements::Requirement;
use crate::aiplan4rust::syntax::tree::SyntaxNode;
use crate::aiplan4rust::syntax::tree::SyntaxNodeKind;
use linked_hash_map::IntoIter;
use linked_hash_map::LinkedHashMap;
use std::collections::{HashMap, HashSet};

use crate::aiplan4rust::semantic_analyser::heap_syntax_tree::HeapSyntaxNode;
use crate::aiplan4rust::semantic_analyser::symbol::Source;
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HeapSyntaxTree {
    map: LinkedHashMap<usize, HeapSyntaxNode>,
    requirements: HashSet<Requirement>,
}

impl HeapSyntaxTree {
    pub fn new() -> Self {
        HeapSyntaxTree {
            map: LinkedHashMap::new(),
            requirements: HashSet::new(),
        }
    }

    pub fn source(&self) -> Source {
        if let Some(first_node) = self.map.values().next() {
            match first_node.kind() {
                SyntaxNodeKind::Domain => Source::Domain,
                SyntaxNodeKind::Problem => Source::Problem,
                _ => Source::Unknown,
            }
        } else {
            Source::Unknown
        }
    }

    pub fn from(ast: &SyntaxNode) -> Result<HeapSyntaxTree, ParserInternalError> {
        // Vérification du type de l'AST
        match ast.kind() {
            SyntaxNodeKind::Domain | SyntaxNodeKind::Problem => {
                let index_table = ast.to_hash_map();
                let mut ast_table = HeapSyntaxTree::new();
                let mut next_index = 0;
                HeapSyntaxTree::from_rec(ast, &index_table, &mut ast_table, &mut next_index)?;
                Ok(ast_table)
            }
            _ => Err(ParserInternalError::new(
                "AST must be of type Domain or Problem".to_string(),
            )),
        }
    }

    fn from_rec(
        ast: &SyntaxNode,
        index_table: &HashMap<&SyntaxNode, usize>,
        ast_table: &mut HeapSyntaxTree,
        next_index: &mut usize,
    ) -> Result<(), ParserInternalError> {
        let entry = HeapSyntaxNode::from(ast, index_table)?;
        if let SyntaxNodeKind::Requirement(req) = entry.kind() {
            ast_table.add_requirement(req.clone());
        }
        ast_table.insert(*next_index, entry);

        *next_index += 1; // Incrémente pour le prochain nœud

        for child in ast.children() {
            HeapSyntaxTree::from_rec(child.as_ref(), index_table, ast_table, next_index)?;
        }

        Ok(())
    }

    // Vérifie si un AstKind spécifique existe déjà dans la table
    pub fn contains_kind(&self, kind: SyntaxNodeKind) -> bool {
        self.map.iter().any(|(_, entry)| *entry.kind() == kind)
    }

    pub fn insert(&mut self, index: usize, entry: HeapSyntaxNode) -> Option<HeapSyntaxNode> {
        self.map.insert(index, entry)
    }

    // Accesseur pour obtenir un noeud à partir de son ID
    pub fn get_entry(&self, id: usize) -> Option<&HeapSyntaxNode> {
        self.map.get(&id)
    }

    // Méthode pour ajouter un nouveau noeud à la table
    pub fn add_entry(&mut self, id: usize, entry: HeapSyntaxNode) {
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
    pub fn iter(&self) -> impl Iterator<Item = (&usize, &HeapSyntaxNode)> {
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
    pub fn iter_mut(&mut self) -> impl Iterator<Item = (&usize, &mut HeapSyntaxNode)> {
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
    pub fn values(&self) -> impl Iterator<Item = &HeapSyntaxNode> {
        self.map.values()
    }
}

impl IntoIterator for HeapSyntaxTree {
    type Item = (usize, HeapSyntaxNode);
    type IntoIter = IntoIter<usize, HeapSyntaxNode>;

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

impl fmt::Display for HeapSyntaxTree {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "AstTable {{")?;
        for (index, entry) in &self.map {
            writeln!(f, "  {}: {}", index, entry)?;
        }
        write!(f, "}}")
    }
}
