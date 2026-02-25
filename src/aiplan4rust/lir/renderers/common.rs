use std::fmt;
use std::fmt::Formatter;
use crate::aiplan4rust::lir::expr::Expr;

// Taille standard d'un palier d'indentation (nombre d'espaces).
pub const INDENT_SIZE: usize = 2;

/// Écrit l'indentation correspondant au niveau demandé.
pub fn write_indent(f: &mut Formatter<'_>, level: usize) -> fmt::Result {
    for _ in 0..(level * INDENT_SIZE) {
        write!(f, " ")?;
    }
    Ok(())
}

/// Writes a centered title to a formatter with optional fill characters on both sides.
///
/// This function prints the given `title` centered within a line of total `width` characters.
/// The remaining space on either side of the title is filled with the specified `fill` character.
/// If the title is longer than or equal to the specified `width`, it is printed as-is without filling.
///
/// # Parameters
/// - `f`: A mutable reference to any type implementing `std::fmt::Write`. This is where the output is written.
/// - `title`: The string to be centered on the line.
/// - `width`: The total width of the line including the title and fill characters.
/// - `fill`: The character used to fill the space on the left and right of the title.
///
/// # Returns
/// Returns `std::fmt::Result` indicating whether writing to the formatter was successful.
///
/// # Example
///
/// ```rust
/// use std::fmt::Write;
/// let mut output = String::new();
/// writeln_centered(&mut output, "HELLO", 10, '-')?;
/// assert_eq!(output, "---HELLO---\n");
/// ```
pub(crate) fn writeln_centered(
    f: &mut impl std::fmt::Write,
    title: &str,
    width: usize,
    fill: char,
) -> std::fmt::Result {
    if title.len() >= width {
        // If the title is longer than the width, print it as-is
        writeln!(f, "{}", title)
    } else {
        let total_fill = width - title.len();
        let left_fill = total_fill / 2;
        let right_fill = total_fill - left_fill;
        writeln!(
            f,
            "{}{}{}",
            fill.to_string().repeat(left_fill),
            title,
            fill.to_string().repeat(right_fill)
        )
    }
}

pub(crate) fn render_labeled_typed_list<I, T>(
    f: &mut fmt::Formatter<'_>,
    label: &str,
    items: I
) -> fmt::Result
where
    I: IntoIterator<Item = T>,
    T: fmt::Display
{
    writeln!(f, "{}:", label)?;

    let mut iter = items.into_iter().peekable();

    if iter.peek().is_none() {
        writeln!(f, "  <None>")?;
    } else {
        for item in iter {
            writeln!(f, "  {}", item)?;
        }
    }
    Ok(())
}
pub(crate) fn render_labeled_expr(f: &mut fmt::Formatter<'_>, label: &str, expr: &Expr) -> fmt::Result {
    writeln!(f, "{}:", label)?;

    // On vérifie si l'expression est vide via son root_id
    if expr.root_id().is_none() {
        writeln!(f, "  <None>")?;
    } else {
        // L'expression n'est pas vide, on l'affiche ligne par ligne
        let content = format!("{}", expr);
        for line in content.lines() {
            writeln!(f, "  {}", line)?;
        }
    }
    Ok(())
}
