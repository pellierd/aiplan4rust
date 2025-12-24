use std::fs;
use std::path::Path;





/// Ensure that the parent directory of the given path exists.
/// If it does not exist, attempt to create it. On failure, prints an error and exits.
///
/// # Arguments
///
/// * `path` - The path whose parent directory should be ensured.
pub fn ensure_parent_dir_exists<P: AsRef<Path>>(path: P) {
    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        if !parent.exists() {
            if let Err(e) = fs::create_dir_all(parent) {
                eprintln!("Error creating output directory {}: {}", parent.display(), e);
                std::process::exit(1);
            }
        }
    }
}
