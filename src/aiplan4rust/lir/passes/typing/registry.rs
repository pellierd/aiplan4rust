use std::fmt;
use indexmap::IndexMap;
use crate::aiplan4rust::lang::{Type, TypeId, TypedSymbol};

/// A registry responsible for the unification and management of composite types during the flattening pass.
///
/// The `TypeRegistry` ensures that ad-hoc `either` types are mapped to unique, stable `TypeId`s.
/// It tracks existing types from the problem definition and dynamically generates new atomic
/// identifiers for previously unseen type combinations.
#[derive(Clone, Debug)]
pub struct TypeRegistry {
    /// Maps a sorted, deduplicated signature of member types to a unique `TypeId`.
    cache: IndexMap<Vec<TypeId>, TypeId>,
    /// The number of pre-existing types in the problem, used as the base offset for new IDs.
    initial_count: usize,
    /// A monotonic counter used to assign unique identifiers to newly materialized types.
    next_id_counter: usize,
}

impl TypeRegistry {
    /// Creates a new `TypeRegistry` by indexing existing type definitions.
    ///
    /// This constructor populates the internal cache with all currently defined
    /// composite types to prevent the creation of redundant identifiers.
    ///
    /// # Parameters
    /// * `type_defs`: A slice of [`TypedSymbol`] representing the problem's initial type hierarchy.
    ///
    /// # Returns
    /// A initialized registry with the counter set at the end of the existing ID space.
    pub fn new(type_defs: &[TypedSymbol<TypeId, TypeId>]) -> Self {
        let mut cache = IndexMap::new();
        let initial_count = type_defs.len();

        for def in type_defs {
            let members = def.ty().members();
            // We only cache composite types (Either) as atomic types are self-resolving.
            if members.len() > 1 {
                let mut sig = members.to_vec();
                sig.sort_unstable();
                sig.dedup();
                cache.insert(sig, def.symbol());
            }
        }

        Self {
            cache,
            initial_count,
            next_id_counter: initial_count,
        }
    }

    /// Resolves a collection of member types into a single, unique `TypeId`.
    ///
    /// If the provided members represent a single type, that type's ID is returned directly.
    /// For multiple members, the registry performs **Unification**: it sorts and deduplicates
    /// the members to find an existing match or registers a new anonymous type.
    ///
    /// # Parameters
    /// * `members`: A slice of [`TypeId`] representing the components of an `either` type.
    ///
    /// # Returns
    /// * An existing [`TypeId`] if the signature has been seen before or exists in the domain.
    /// * A newly generated [`TypeId`] if this specific combination of types is unique.
    pub fn resolve(&mut self, members: &[TypeId]) -> TypeId {
        // Optimization: Atomic types resolve to themselves.
        if members.len() == 1 {
            return members[0];
        }

        // Generate a canonical signature for the composite type.
        let mut sig = members.to_vec();
        sig.sort_unstable();
        sig.dedup();

        if let Some(&id) = self.cache.get(&sig) {
            id
        } else {
            // Materialize a new ID for this unique combination.
            let id = TypeId::from(self.next_id_counter);
            self.cache.insert(sig, id);
            self.next_id_counter += 1;
            id
        }
    }

    /// Consumes the registry and returns an iterator over the newly materialized types.
    ///
    /// This is used during the final phase of the flattening pass to inject the new
    /// anonymous type definitions back into the global [`LiftedProblem`].
    ///
    /// # Returns
    /// An [`Iterator`] yielding tuples of the new [`TypeId`] and its corresponding [`Type`] definition.
    pub fn into_new_types(self) -> impl Iterator<Item = (TypeId, Type<TypeId>)> {
        let offset = self.initial_count;
        self.cache.into_iter()
            .filter(move |(_, id)| id.as_usize() >= offset)
            .map(|(sig, id)| (id, Type::either(sig)))
    }
}

impl fmt::Display for TypeRegistry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "=== TypeRegistry State ===")?;
        writeln!(f, "Initial types count: {}", self.initial_count)?;
        writeln!(f, "Next available ID  : {}", self.next_id_counter)?;
        writeln!(f, "Unification Cache  :")?;

        if self.cache.is_empty() {
            writeln!(f, "  (empty)")?;
        } else {
            for (sig, id) in &self.cache {
                // On marque les types qui ont été créés dynamiquement (matérialisés)
                let status = if id.as_usize() >= self.initial_count {
                    "[NEW]"
                } else {
                    "[EXISTING]"
                };

                writeln!(f, "  {:>10} {:?} -> TypeId({})", status, sig, id.as_usize())?;
            }
        }
        writeln!(f, "==========================")
    }
}
