use std::fmt;

/// A trait for types that can be represented as a PDDL (Planning Domain Definition Language) string.
///
/// This trait provides functionality to display an object in a PDDL-compatible string format. It includes
/// options to specify the depth of indentation for more complex PDDL representations.
pub trait PDDLDisplay: fmt::Display {
    /// Converts the object to a PDDL-formatted string with default depth (0).
    ///
    /// This method uses a default indentation level (depth 0) to convert the object into a PDDL string. It serves as
    /// a shorthand for objects that do not require special indentation.
    ///
    /// # Returns
    /// A `String` containing the PDDL representation of the object.
    fn to_pddl_string(&self) -> String {
        self.to_pddl_string_with_depth(0)
    }

    /// Converts the object to a PDDL-formatted string with a specified depth for indentation.
    ///
    /// This method allows custom formatting where the `depth` parameter specifies how much indentation (in spaces)
    /// should be applied to the resulting string. The `depth` controls the level of nesting in the PDDL string.
    ///
    /// # Parameters
    /// - `depth`: The indentation level, with each level adding two spaces of indentation.
    ///
    /// # Returns
    /// A `String` containing the PDDL representation of the object, formatted with the given depth.
    fn to_pddl_string_with_depth(&self, depth: usize) -> String {
        let offset = "  ".repeat(depth * 2); // Indentation based on depth
        format!("{}{}", offset, self)
    }
}
