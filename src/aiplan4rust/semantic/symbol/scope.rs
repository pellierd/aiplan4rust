use crate::aiplan4rust::frontend::ParserInternalError;
use crate::aiplan4rust::syntax::ast::{AstKind, AstNode};
use linked_hash_map::LinkedHashMap;
use serde::{Deserialize, Serialize};
use std::cmp::PartialEq;
use std::fmt;
use std::hash::Hash;
use crate::aiplan4rust::semantic::SemanticContext;

#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Scope {
    stack: Vec<usize>,
}

impl Scope {
    pub fn new(ast: usize, parent: Option<&Scope>) -> Self {
        let mut scope = Scope { stack: Vec::new() };

        if let Some(parent_scope) = parent {
            scope.stack.extend(parent_scope.stack.iter().cloned());
        }
        scope.stack.push(ast);
        scope
    }

    pub fn root() -> Self {
        Scope::new(0, None)
    }

    /// Returns `true` if `self` starts with the given `prefix` scope.
    pub fn starts_with(&self, prefix: &Scope) -> bool {
        self.stack.starts_with(&prefix.stack)
    }

    // Fonction pour créer un itérateur sur `Scope`
    pub fn iter(&self) -> impl Iterator<Item = &usize> {
        self.stack.iter()
    }

    // Fonction pour obtenir un itérateur mutable si nécessaire
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut usize> {
        self.stack.iter_mut()
    }

    /// Checks if the scope contains at least one AST node of the specified kind.
    ///
    /// This function iterates over the IDs stored in the scope's `stack` and uses the shared `bimap`
    /// to look up the corresponding AST node. If an AST node is not found in the `bimap` for a given ID,
    /// it returns a `ParserInternalError`, as this indicates an internal inconsistency in the scope
    /// construction. Otherwise, if at least one node matches the specified kind, the function returns
    /// `Ok(true)`. If none of the nodes match, it returns `Ok(false)`.
    ///
    /// # Parameters
    /// - `kind`: The AST kind to search for in the scope.
    ///
    /// # Returns
    /// - `Ok(true)` if any node in the scope's stack has the given kind.
    /// - `Ok(false)` if no node in the scope's stack has the given kind.
    /// - `Err(ParserInternalError)` if an expected ID is not found in the `bimap`.
    ///
    /// # Example
    /// ```rust
    /// let result = scope.contains_ast_of_kind(AstKind::Function);
    /// match result {
    ///     Ok(true) => println!("Scope contains a function node."),
    ///     Ok(false) => println!("Scope does not contain a function node."),
    ///     Err(e) => eprintln!("Error: {}", e),
    /// }
    /// ```
    pub fn contains_ast_of_kind(
        &self,
        kind: AstKind,
        context: &SemanticContext,
    ) -> Result<bool, ParserInternalError> {
        for id in &self.stack {
            match context.get(*id) {
                Some(ast) => {
                    if *ast.kind() == kind {
                        return Ok(true);
                    }
                }
                None => {
                    return Err(ParserInternalError::new(format!(
                        "ID {} not found in bimap",
                        id
                    )));
                }
            }
        }
        Ok(false)
    }

    pub fn display_with_map(
        &self,
        f: &mut fmt::Formatter<'_>,
        map: &LinkedHashMap<usize, &Box<AstNode>>,
    ) -> fmt::Result {
        let mut scope_strings: Vec<String> = Vec::new();
        for ast_index in self.stack.iter() {
            if let Some(ast) = map.get(ast_index) {
                let (line, column) = ast.start_position();
                let scope_string = format!("[{} {}:{}]", ast.kind(), line, column);
                scope_strings.push(scope_string);
            }
        }
        write!(f, "{}", scope_strings.join(" -> "))
    }
}

// Implémentation du trait Iterator pour Scope
impl Iterator for Scope {
    type Item = usize;  // Définition du type d'élément à itérer (ici usize)

    fn next(&mut self) -> Option<Self::Item> {
        self.stack.pop() // Retourne et retire le dernier élément de la pile
    }
}


impl fmt::Display for Scope {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.stack)
    }
}
