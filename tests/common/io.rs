use std::{fs, io};
use std::io::Read;
use std::path::{Path, PathBuf};

/// Reads the content of a file into a string.
///
/// # Arguments
///
/// * `path` - A reference to the file path to be read.
///
/// # Returns
///
/// A `Result` containing the file's content as a `String`, or an `io::Error`.
pub fn read_file(path: &Path) -> io::Result<String> {
    if !path.exists() {
        return Err(io::Error::new(io::ErrorKind::NotFound, "File does not exist"));
    }

    let mut source = String::new();
    let mut file = fs::File::open(path)?;
    file.read_to_string(&mut source)?;
    Ok(source)
}

/// Collects all `.pddl` or `.hddl` files from the specified directory.
///
/// # Arguments
///
/// * `domain_dir` - A reference to a path representing the directory to search.
///
/// # Returns
///
/// A `Result` containing a vector of `PathBuf` if successful, or an `io::Error`.
pub fn collect_domain_files(domain_dir: &Path) -> io::Result<Vec<PathBuf>> {
    let entries = fs::read_dir(domain_dir)?;

    let mut files = Vec::new();
    for entry in entries.filter_map(Result::ok) {
        let path = entry.path();
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            if ext.eq_ignore_ascii_case("pddl") || ext.eq_ignore_ascii_case("hddl") {
                files.push(path);
            }
        }
    }

    Ok(files)
}
