use std::fmt;

/// A trait for types that can be represented as a syntax string for planning domain languages,
/// such as PDDL or HDDL.
///
/// This trait provides functionality to display an object in a domain-specific language
/// compatible string format. It includes methods to specify indentation depth for
/// pretty-printing nested structures.
///
/// Implementors must also implement `fmt::Display`, which defines the default string representation.
///
/// # Examples
///
/// ```rust
/// use aiplan4rust::SyntaxDisplay;
///
/// struct Example;
///
/// impl std::fmt::Display for Example {
///     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
///         write!(f, "example")
///     }
/// }
///
/// impl SyntaxDisplay for Example {}
///
/// let e = Example;
/// assert_eq!(e.to_syntax_string(), "example");
/// assert_eq!(e.to_syntax_string_with_depth(1), "    example"); // 2 spaces * depth * 2 = 4 spaces
/// ```
pub trait Display: fmt::Display {
    /// Converts the object to a syntax string with default depth (0).
    ///
    /// This method returns the string representation with no indentation.
    ///
    /// # Returns
    /// A `String` containing the syntax representation of the object.
    fn to_syntax_string(&self) -> String {
        self.to_syntax_string_with_depth(0)
    }

    /// Converts the object to a syntax string with a specified indentation depth.
    ///
    /// # Parameters
    /// - `depth`: The indentation level; each level corresponds to 2 spaces of indentation.
    ///
    /// # Returns
    /// A `String` containing the syntax representation of the object, indented accordingly.
    fn to_syntax_string_with_depth(&self, depth: usize) -> String {
        let offset = "  ".repeat(depth); // 2 spaces per depth level
        format!("{}{}", offset, self)
    }
}
