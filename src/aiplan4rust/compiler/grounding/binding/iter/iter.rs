use crate::aiplan4rust::compiler::grounding::binding::iter::BindingsIteratorError;
use crate::aiplan4rust::compiler::grounding::binding::Bindings;
use crate::aiplan4rust::compiler::grounding::problem::registry::value::ValueRegistry;
use crate::aiplan4rust::support::lang::{ObjectId, TypeId, TypedList, VariableId};
use std::fmt;

pub struct BindingsIterator<'a> {
    variables: &'a TypedList<VariableId, TypeId>,
    /// Domaines réordonnés directement à la construction pour un accès direct O(1) sans indirection
    domains: Vec<&'a [ObjectId]>,
    indices: Vec<usize>,
    /// Table de correspondance : odomètre_index -> syntaxe_index
    permutation: Vec<usize>,
    current_bindings: Bindings,
    exhausted: bool,
    total_count: usize,
    is_first: bool,
}

impl<'a> BindingsIterator<'a> {
    pub fn new(
        variables: &'a TypedList<VariableId, TypeId>,
        value_registry: &'a ValueRegistry,
    ) -> Result<Self, BindingsIteratorError> {
        let raw_domains = value_registry.get_variable_domains(variables)?;
        let arity = raw_domains.len();

        let mut permutation: Vec<usize> = (0..arity).collect();
        permutation.sort_by_key(|&idx| std::cmp::Reverse(raw_domains[idx].len()));

        // On construit le vecteur de domaines directement dans le bon ordre
        let mut domains = Vec::with_capacity(arity);
        for &orig_idx in &permutation {
            domains.push(raw_domains[orig_idx]);
        }

        let total_count = match arity {
            0 => 0,
            _ => domains
                .iter()
                .map(|d| d.len())
                .try_fold(1usize, |acc, x| acc.checked_mul(x))
                .ok_or_else(|| BindingsIteratorError::combinatorial_explosion(arity))?,
        };

        let is_domain_empty = arity > 0 && domains.iter().any(|d| d.is_empty());
        let exhausted = arity == 0 || total_count == 0 || is_domain_empty;
        let indices = vec![0; arity];

        let mut current_bindings = Bindings::with_capacity(arity);
        for typed_var in variables {
            current_bindings.insert(typed_var.symbol(), ObjectId::default());
        }

        Ok(Self {
            domains,
            variables,
            indices,
            permutation,
            current_bindings,
            exhausted,
            total_count,
            is_first: true,
        })
    }

    pub fn next(&mut self) -> Option<&Bindings> {
        if self.exhausted {
            return None;
        }

        let arity = self.variables.len();

        if self.is_first {
            self.is_first = false;

            if arity > 0 {
                for i in 0..arity {
                    let orig_idx = self.permutation[i];
                    let val = self.domains[i][self.indices[i]];
                    self.current_bindings
                        .insert(self.variables[orig_idx].symbol(), val);
                }
            } else {
                self.exhausted = true;
            }
        } else {
            if arity == 0 {
                self.exhausted = true;
                return None;
            }

            let changed_idx = self.prepare_next()?;

            for i in changed_idx..arity {
                let orig_idx = self.permutation[i];
                let val = self.domains[i][self.indices[i]];
                self.current_bindings
                    .insert(self.variables[orig_idx].symbol(), val);
            }
        }

        Some(&self.current_bindings)
    }

    #[inline]
    fn prepare_next(&mut self) -> Option<usize> {
        if self.indices.is_empty() {
            self.exhausted = true;
            return None;
        }

        // Accès direct sans indirection mémoire : le CPU adore
        for i in (0..self.indices.len()).rev() {
            if self.indices[i] + 1 < self.domains[i].len() {
                self.indices[i] += 1;
                return Some(i);
            }
            self.indices[i] = 0;
        }

        self.exhausted = true;
        None
    }

    pub fn skip_at(&mut self, target_var_idx: usize) {
        // Sécurité critique : On retrouve la position de la variable dans l'odomètre trié
        if let Some(pos) = self.permutation.iter().position(|&p| p == target_var_idx) {
            let arity = self.indices.len();
            let suffix_start = pos + 1;
            if suffix_start < arity {
                for i in suffix_start..arity {
                    self.indices[i] = self.domains[i].len().saturating_sub(1);
                }
            }
        }
    }

    pub fn has_next(&self) -> bool {
        !self.exhausted
    }

    pub fn reset(&mut self) {
        self.indices.fill(0);
        self.is_first = true;
        let arity = self.indices.len();
        let is_really_empty = arity > 0 && self.domains.iter().any(|d| d.is_empty());
        self.exhausted = self.total_count == 0 || is_really_empty;
    }

    pub fn total_count(&self) -> usize {
        self.total_count
    }

    pub fn remaining_count(&self) -> usize {
        if self.exhausted {
            return 0;
        }

        let mut remaining = 0usize;
        let mut multiplier = 1usize;

        for i in (0..self.indices.len()).rev() {
            let card = self.domains[i].len();
            let current_idx = self.indices[i];

            let diff = card.saturating_sub(1).saturating_sub(current_idx);
            remaining = remaining.saturating_add(diff.saturating_mul(multiplier));
            multiplier = multiplier.saturating_mul(card);
        }

        remaining.saturating_add(1)
    }
}

impl<'a> fmt::Display for BindingsIterator<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let current = self.total_count().saturating_sub(self.remaining_count());
        let total = self.total_count();
        let progress = if total > 0 {
            (current as f64 / total as f64) * 100.0
        } else {
            100.0
        };
        write!(
            f,
            "BindingsIterator [Progress: {}/{} ({:.2}%) | Arity: {}]",
            current,
            total,
            progress,
            self.indices.len()
        )
    }
}
