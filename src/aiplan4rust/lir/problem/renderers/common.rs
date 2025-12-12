pub(crate) fn writeln_centered(
    f: &mut impl std::fmt::Write,
    title: &str,
    width: usize,
    fill: char,
) -> std::fmt::Result {
    if title.len() >= width {
        // Si le titre est trop long, on l'écrit tel quel
        writeln!(f, "{}", title)
    } else {
        let total_fill = width - title.len();
        let left_fill = total_fill / 2;
        let right_fill = total_fill - left_fill;
        writeln!(f, "{}{}{}", fill.to_string().repeat(left_fill), title, fill.to_string().repeat(right_fill))
    }
}
