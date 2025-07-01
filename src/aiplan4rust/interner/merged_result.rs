use crate::aiplan4rust::interner::StringInterner;
use crate::aiplan4rust::lang::Ident;

use std::collections::HashMap;
use std::fmt;
use std::mem::take;

/// Result of merging two [`StringInterner`] instances.
///
/// This structure contains the merged `interner` which holds the combined strings,
/// and a mapping from identifiers in the original "problem" interner to their corresponding
/// identifiers in the merged global interner.
///
/// This mapping is useful to translate identifiers from the problem context into the
/// unified global context after merging.
///
/// # Fields
///
/// - `interner`: The merged [`StringInterner`] containing all strings from both inputs.
/// - `problem_ident_map`: A [`HashMap`] mapping original problem [`Ident`] values
///   to their corresponding global [`Ident`] in the merged interner.
///
/// # Example
///
/// ```ignore
/// let merged_result = InternerMergeResult::new(merged_interner, problem_to_global_map);
/// println!("Merged interner has {} strings", merged_result.interner().len());
/// if let Some(global_id) = merged_result.problem_ident_map().get(&problem_id) {
///     println!("Problem id {:?} maps to global id {:?}", problem_id, global_id);
/// }
/// ```
#[derive(Debug, Clone)]
pub struct InternerMergeResult {
    interner: StringInterner,
    problem_ident_map: HashMap<Ident, Ident>,
}

impl InternerMergeResult {
    /// Creates a new `InternerMergeResult`.
    ///
    /// # Parameters
    ///
    /// - `interner`: The merged [`StringInterner`] instance.
    /// - `problem_ident_map`: A mapping from problem identifiers to merged global identifiers.
    ///
    /// # Returns
    ///
    /// A new instance of `InternerMergeResult`.
    pub fn new(
        interner: StringInterner,
        problem_ident_map: HashMap<Ident, Ident>,
    ) -> Self {
        Self {
            interner,
            problem_ident_map,
        }
    }

    /// Returns a reference to the merged [`StringInterner`].
    ///
    /// This interner contains all strings from both merged interners.
    pub fn interner(&self) -> &StringInterner {
        &self.interner
    }

    /// Extracts the merged `StringInterner` by taking it out of `self`,
    /// leaving an empty/default `StringInterner` in its place.
    ///
    /// Requires `&mut self`.
    pub fn take_interner(&mut self) -> StringInterner {
        take(&mut self.interner)
    }

    /// Returns a reference to the mapping from problem [`Ident`] to global [`Ident`].
    ///
    /// This map is used to translate identifiers from the problem interner
    /// into their equivalent in the merged global interner.
    pub fn problem_ident_map(&self) -> &HashMap<Ident, Ident> {
        &self.problem_ident_map
    }

    /// Takes (extracts) the mapping from problem [`Ident`] to global [`Ident`],
    /// leaving an empty map in its place.
    ///
    /// This allows consuming the map without cloning it.
    ///
    /// Requires a mutable reference to `self`.
    pub fn take_problem_ident_map(&mut self) -> HashMap<Ident, Ident> {
        take(&mut self.problem_ident_map)
    }

    /// Merges a domain and problem [`StringInterner`] into a unified [`InternerMergeResult`].
    ///
    /// This function clones the domain interner and extends it with all strings from the
    /// problem interner. It also creates a mapping from each identifier in the problem
    /// to its new identifier in the merged interner.
    ///
    /// # Parameters
    ///
    /// - `domain_interner`: A reference to the domain's [`StringInterner`].
    /// - `problem_interner`: A reference to the problem's [`StringInterner`].
    ///
    /// # Returns
    ///
    /// A new [`InternerMergeResult`] containing:
    /// - the merged `StringInterner`
    /// - a `HashMap<Ident, Ident>` mapping problem identifiers to merged identifiers
    pub fn from_domain_and_problem(
        domain_interner: &StringInterner,
        problem_interner: &StringInterner,
    ) -> Self {
        let mut interner = domain_interner.clone();
        let mut problem_ident_map = HashMap::new();

        for (old_id, s) in problem_interner.iter() {
            let new_id = interner.intern(s.to_string());
            problem_ident_map.insert(old_id, new_id);
        }

        InternerMergeResult::new(interner, problem_ident_map)
    }
}

impl fmt::Display for InternerMergeResult {
    /// Formats the `InternerMergeResult` for display, showing the
    /// merged interner and all identifier mappings.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "InternerMergeResult {{")?;
        writeln!(f, "  interner: {}", self.interner)?;
        writeln!(f, "  problem_ident_map: [")?;
        for (problem_id, global_id) in &self.problem_ident_map {
            writeln!(f, "    {:?} -> {:?}", problem_id, global_id)?;
        }
        writeln!(f, "  ]")?;
        writeln!(f, "}}")
    }
}
