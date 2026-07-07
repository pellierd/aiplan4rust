use crate::aiplan4rust::support::lang::ObjectId;
use std::collections::HashMap;
use std::collections::HashSet;
use std::fmt;

/// A specialized collection for storing unique relational facts.
///
/// `Relation` manages ground Datalog tuples of a fixed arity. It uses a **flat-buffer** /// storage strategy for high cache locality and memory efficiency, while maintaining
/// auxiliary indices for fast membership checks and join operations.
///
/// # Internal Architecture
/// - **Storage**: All tuples are stored contiguously in a single `Vec<ObjectId>`.
/// - **Membership**: A `HashSet` provides $O(1)$ duplicate detection.
/// - **Indexing**: A `HashMap` maps the first argument of a tuple to its starting
///   offsets in the buffer, significantly accelerating relational joins.
#[derive(Debug, Clone)]
pub struct Relation {
    /// The fixed number of elements per tuple in this relation.
    arity: usize,
    /// Contiguous buffer of object identifiers.
    tuples: Vec<ObjectId>,
    /// Set used to enforce uniqueness and provide $O(1)$ lookups.
    index: HashSet<Vec<ObjectId>>,
    /// Index optimized for joins: Maps `first_arg -> [buffer_offsets]`.
    first_arg_index: HashMap<ObjectId, Vec<usize>>,
}

impl Relation {
    /// Initializes a new empty relation with a fixed arity.
    ///
    /// # Arguments
    /// * `arity` - The number of [`ObjectId`]s each tuple must contain.
    #[inline]
    pub fn new(arity: usize) -> Self {
        Self {
            arity,
            tuples: Vec::new(),
            index: HashSet::new(),
            first_arg_index: HashMap::new(),
        }
    }

    /// Checks if the relation contains the given tuple.
    ///
    /// This check is $O(1)$ and performs no allocations.
    #[inline]
    pub fn contains(&self, tuple: &[ObjectId]) -> bool {
        // HashSet<Vec<T>> permet de chercher avec un &[T] grâce à l'implémentation de Borrow
        // C'est O(1), Garanti sans collision, et ZÉRO allocation.
        self.index.contains(tuple)
    }

    /// Inserts a new fact into the relation.
    ///
    /// If the tuple is already present, the method returns `false` and
    /// no modifications are made.
    ///
    /// # Returns
    /// `true` if the fact was new and successfully added.
    pub fn insert(&mut self, tuple: &[ObjectId]) -> bool {
        if !self.index.contains(tuple) {
            let v = tuple.to_vec();

            // Calcul de l'offset AVANT l'insertion
            let start_offset = self.tuples.len();

            // Mise à jour de l'index sur le premier argument
            if let Some(&first_obj) = tuple.first() {
                self.first_arg_index
                    .entry(first_obj)
                    .or_default()
                    .push(start_offset);
            }

            self.tuples.extend_from_slice(tuple);
            self.index.insert(v);
            return true;
        }
        false
    }

    /// Provides a reference to the index mapping first arguments to buffer offsets.
    ///
    /// This is primarily used by the Datalog engine to perform indexed joins.
    #[inline]
    pub fn index_by_first_arg(&self) -> &HashMap<ObjectId, Vec<usize>> {
        &self.first_arg_index
    }

    /// Returns the arity (number of elements per tuple) of the relation.
    #[inline]
    pub fn arity(&self) -> usize {
        self.arity
    }

    /// Returns the number of unique facts stored in the relation.
    #[inline]
    pub fn len(&self) -> usize {
        self.index.len()
    }

    /// Returns `true` if the relation contains no facts.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.index.is_empty()
    }

    /// Returns a reference to the underlying flat data buffer.
    #[inline]
    pub fn data(&self) -> &[ObjectId] {
        &self.tuples
    }

    /// Returns an iterator yielding each fact as a shared slice of size [`arity`].
    ///
    /// This method abstracts the underlying flat-buffer storage layout. For standard
    /// relations (arity > 0), it chunks the contiguous buffer into exact slices.
    /// For propositional facts (arity = 0), it guarantees semantic correctness by
    /// yielding exactly one empty slice if the proposition is true, rather than
    /// skipping execution due to the empty data buffer.
    ///
    /// # Return Value
    ///
    /// An opaque `impl Iterator` yielding `&[ObjectId]` items. This avoids dynamic
    /// heap allocations (`Box<dyn Iterator>`) or virtual table lookups, allowing
    /// the compiler to fully inline and optimize downstream loops.
    ///
    /// # Safety & Panics
    ///
    /// Safe from execution panics. It explicitly prevents passing a step size of 0
    /// to [`slice::chunks_exact`], which would otherwise trigger an immediate runtime panic.
    pub fn iter(&self) -> impl Iterator<Item = &[ObjectId]> {
        // 1. If arity is 0 and the proposition is true, stage exactly one empty slice.
        let empty_arity_slice = if self.arity == 0 && !self.index.is_empty() {
            Some(&[][..])
        } else {
            None
        };

        // 2. Defensive guard: chunks_exact(0) panics in Rust.
        // Use 1 as a placeholder size if arity is 0 since self.tuples is empty anyway.
        let chunk_size = if self.arity == 0 { 1 } else { self.arity };

        // 3. Chain both sources under a unified structural type layout.
        empty_arity_slice
            .into_iter()
            .chain(self.tuples.chunks_exact(chunk_size))
    }

    /// Retrieves a specific tuple by its logical index.
    ///
    /// # Arguments
    /// * `index` - The position of the tuple (0 to `len() - 1`).
    pub fn get(&self, index: usize) -> Option<&[ObjectId]> {
        let start = index * self.arity;
        let end = start + self.arity;
        self.tuples.get(start..end)
    }
}

impl fmt::Display for Relation {
    /// Formats the relation as a mathematical set of tuples.
    ///
    /// This implementation provides a concise representation of the facts
    /// currently stored in the relation, using standard set notation.
    ///
    /// # Output Examples
    /// - **Empty**: `{empty}`
    /// - **Propositional (Arity 0)**: `State: TRUE⟿`
    /// - **Relational**: `{(c1, c2), (c3, c4)}`
    ///
    /// # Performance Note
    /// For very large relations, this method will iterate through the entire
    /// flat buffer. It is primarily intended for debugging and small-scale
    /// knowledge base inspection.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_empty() {
            return write!(f, "{{empty}}");
        }

        // Special case: Arity 0 represents a boolean proposition.
        // In Datalog, if an arity-0 relation exists in the DB, it is logically True.
        if self.arity == 0 {
            return write!(f, "State: TRUE⟿");
        }

        write!(f, "{{")?;
        let mut first = true;
        for chunk in self.iter() {
            if !first {
                write!(f, ", ")?;
            }
            first = false;

            write!(f, "(")?;
            for (j, obj) in chunk.iter().enumerate() {
                if j > 0 {
                    write!(f, ", ")?;
                }
                // Directly writing the ObjectId using its Display implementation
                write!(f, "{}", obj)?;
            }
            write!(f, ")")?;
        }
        write!(f, "}}")
    }
}
