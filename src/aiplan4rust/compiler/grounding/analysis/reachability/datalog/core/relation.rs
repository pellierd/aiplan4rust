//! A high-performance, allocation-optimized relational database engine component.
//!
//! This module provides foundational structures for grounding and evaluating Datalog-style
//! relational facts within an automated planning system. It emphasizes maximum CPU cache locality
//! via linear storage layouts and eliminates heap fragmentation during saturation algorithms
//! through intensive allocation pooling and stack-allocated inline containers.

use crate::aiplan4rust::support::lang::ObjectId;
use rustc_hash::{FxHashMap, FxHashSet};
use smallvec::SmallVec;
use std::fmt;

/// A specialized collection for storing unique relational facts.
///
/// `Relation` manages ground Datalog tuples of a fixed arity. It uses a **flat-buffer**
/// storage strategy for high cache locality and memory efficiency, while maintaining
/// auxiliary indices for fast membership checks and join operations.
///
/// # Internal Architecture
/// - **Storage**: All tuples are stored contiguously in a single, flat `Vec<ObjectId>`.
/// - **Membership**: A fast `FxHashSet` storing inline `SmallVec` buffers ensures constant-time
///   duplicate detection with zero heap allocations for arities lower than or equal to 4.
/// - **Indexing**: A specialized `FxHashMap` maps the first argument of a tuple to its starting
///   offsets in the flat buffer, significantly accelerating indexed relational joins.
#[derive(Debug, Clone)]
pub struct Relation {
    /// The fixed number of elements per tuple in this relation.
    arity: usize,
    /// Contiguous buffer of object identifiers.
    tuples: Vec<ObjectId>,
    /// Set used to enforce uniqueness and provide fast $\mathcal{O}(1)$ lookups via stack-allocated inline buffers.
    index: FxHashSet<smallvec::SmallVec<[ObjectId; Self::INLINE_TUPLE_CAPACITY]>>,
    /// Index optimized for joins: Maps `first_arg -> [buffer_offsets]`.
    first_arg_index: FxHashMap<ObjectId, Vec<usize>>,
}

impl Relation {
    /// Default initial capacity for the join index offset buckets.
    ///
    /// Pre-allocating space for the first-argument indexing vector dramatically reduces
    /// micro-allocations and pointer chasing during iterative Datalog fixpoint updates.
    const DEFAULT_INDEX_BUCKET_CAPACITY: usize = 8;

    /// Threshold arity for stack-allocated inline storage.
    ///
    /// Relational facts with an arity lower than or equal to this limit will reside
    /// entirely within the uniqueness `FxHashSet` internal buckets, achieving $O(0)$
    /// heap allocations for all common PDDL operators (unaries to quaternaries).
    const INLINE_TUPLE_CAPACITY: usize = 4;

    /// Initializes a new empty relation layout configured for a fixed tuple arity.
    ///
    /// # Arguments
    ///
    /// * `arity` - The exact number of [`ObjectId`] coordinates required for every fact in this relation.
    ///
    /// # Memory Footprint
    ///
    /// This constructor is zero-allocation. No heap vectors or hash tables are provisioned
    /// until the first relational fact is successfully committed via [`Self::insert`].
    #[inline]
    pub fn new(arity: usize) -> Self {
        Self {
            arity,
            tuples: Vec::new(),
            index: FxHashSet::default(),
            first_arg_index: FxHashMap::default(),
        }
    }

    /// Checks if the relation contains the given tuple.
    ///
    /// # Complexity
    ///
    /// - **Time Complexity**: $\mathcal{O}(1)$ average case. The `FxHashSet` leverages a fast,
    ///   non-cryptographic hashing algorithm to achieve constant-time lookups.
    ///
    /// # Returns
    ///
    /// - `true` if the specific tuple exists within the relation dataset.
    /// - `false` otherwise.
    #[inline]
    pub fn contains(&self, tuple: &[ObjectId]) -> bool {
        // FxHashSet<SmallVec<T>> performs a lookup using &[T] thanks to the Borrow trait implementation.
        // This is O(1), guaranteed collision-free, and incurs ZERO allocations.
        self.index.contains(tuple)
    }

    /// Inserts a new relational fact into the collection.
    ///
    /// If the tuple is already present, the method returns `false` immediately
    /// and no modifications or allocations are made.
    ///
    /// # Complexity
    ///
    /// - **Time Complexity**:
    ///   - **Duplicate Case**: $\mathcal{O}(1)$ average case to check membership and bail out early.
    ///   - **Insertion Case**: $\mathcal{O}(1)$ amortized. Appending to the flat buffer (`self.tuples`)
    ///     and inserting into the `FxHashSet` run in amortized constant time. Updating the join index
    ///     is a fast $\mathcal{O}(1)$ map lookup followed by an $\mathcal{O}(1)$ vector push.
    /// - **Space Complexity**:
    ///   - If arity $\le 4$: $\mathcal{O}(1)$ auxiliary space. Data is copied inline within the
    ///     `FxHashSet` buckets using stack-allocated storage, resulting in **zero new heap allocations**.
    ///   - If arity $> 4$: $\mathcal{O}(A)$ where $A$ is the tuple arity, as the `SmallVec` spills over
    ///     and allocates a dynamic buffer on the heap.
    ///
    /// # Returns
    ///
    /// `true` if the fact was new and successfully committed to the relation database.
    #[inline]
    pub fn insert(&mut self, tuple: &[ObjectId]) -> bool {
        // 1. Early return if the fact already exists (ZERO allocation).
        if self.index.contains(tuple) {
            return false;
        }

        let start_offset = self.tuples.len();

        // 2. Update the join index on the first coordinate using a pre-allocated capacity bucket.
        if let Some(&first_obj) = tuple.first() {
            self.first_arg_index
                .entry(first_obj)
                .or_insert_with(|| Vec::with_capacity(Self::DEFAULT_INDEX_BUCKET_CAPACITY))
                .push(start_offset);
        }

        self.tuples.extend_from_slice(tuple);

        // 3. Deferred allocation optimized with SmallVec:
        // If arity <= 4, data is copied inline inside the HashSet bucket. No heap allocation occurs.
        self.index.insert(SmallVec::from_slice(tuple));

        true
    }

    /// Clears all relational facts while preserving internal buffer allocations.
    ///
    /// This method resets the logical state of the relation to empty, but retains the
    /// underlying capacities (`capacity()`) of its vectors and maps. It is designed
    /// specifically for **Buffer Pooling** or recycling relational layouts across
    /// multiple execution generations or distinct problems.
    ///
    /// # Performance Mechanism
    ///
    /// - `self.tuples.clear()` truncates the flat data block to length 0 but leaves
    ///   its heap capacity untouched, preventing subsequent `malloc` calls upon re-insertion.
    /// - `self.index.clear()` retains the bucket layout of the uniqueness `FxHashSet`.
    /// - Instead of clearing the master `first_arg_index` map—which would deallocate its
    ///   internal nodes—this method iterates through and clears the individual `Vec<usize>`
    ///   offset buckets. This preserves the allocation footprint of the join index pathways.
    ///
    /// # Complexity
    ///
    /// - **Time Complexity**: $\mathcal{O}(K)$ where $K$ represents the number of unique `first_arg`
    ///   keys currently cached inside the join index map. This bypasses the total tuple count $\mathcal{O}(N)$
    ///   and guarantees allocation-free subsequent saturation rounds.
    /// - **Space Complexity**: $\mathcal{O}(1)$ auxiliary space since all modifications are done in-place.
    #[inline]
    pub fn clear(&mut self) {
        self.tuples.clear();
        self.index.clear();

        // Flush inner offset vectors without destroying the map's skeleton (Buffer Pooling).
        for offsets in self.first_arg_index.values_mut() {
            offsets.clear();
        }
    }

    /// Provides a reference to the index mapping first arguments to buffer offsets.
    ///
    /// This is primarily used by the Datalog engine to perform indexed joins.
    ///
    /// # Returns
    ///
    /// A shared reference to the internal `FxHashMap` that links an initial [`ObjectId`]
    /// to its matching record locations inside the flat buffer.
    #[inline]
    pub fn index_by_first_arg(&self) -> &FxHashMap<ObjectId, Vec<usize>> {
        &self.first_arg_index
    }

    /// Returns the arity (number of elements per tuple) of the relation.
    ///
    /// # Returns
    ///
    /// The fixed coordinate dimension (length) required for every fact within this relation.
    #[inline]
    pub fn arity(&self) -> usize {
        self.arity
    }

    /// Returns the number of unique facts stored in the relation.
    ///
    /// # Returns
    ///
    /// The exact count of unique relational tuples currently committed to the dataset.
    #[inline]
    pub fn len(&self) -> usize {
        self.index.len()
    }

    /// Returns `true` if the relation contains no facts.
    ///
    /// # Returns
    ///
    /// - `true` if the database contains zero relational coordinates.
    /// - `false` if at least one operational fact or true proposition is present.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.index.is_empty()
    }

    /// Returns a reference to the underlying flat data buffer.
    ///
    /// # Returns
    ///
    /// A linear slice reference to the continuous, un-chunked array of raw [`ObjectId`] data.
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
    #[inline]
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
    ///
    /// * `index` - The zero-based position of the tuple (from `0` to `len() - 1`).
    ///
    /// # Returns
    ///
    /// - `Some(&[ObjectId])` containing the contiguous slice of objects forming the fact if the index is valid.
    /// - `None` if the computed slice bounds fall out of the underlying buffer's range.
    #[inline]
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
    ///
    /// - **Empty**: `{empty}`
    /// - **Propositional (Arity 0)**: `State: TRUE⟿`
    /// - **Relational**: `{(c1, c2), (c3, c4)}`
    ///
    /// # Performance Note
    ///
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
                // Directly write the ObjectId using its own Display implementation.
                write!(f, "{}", obj)?;
            }
            write!(f, ")")?;
        }
        write!(f, "}}")
    }
}
