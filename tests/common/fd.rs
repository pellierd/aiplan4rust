// --- Helpers de Parsing FD ---

use std::collections::HashSet;
use std::path::Path;
use std::process::Command;

pub fn extract_predicate_names(fluents: &[String]) -> HashSet<String> {
    let mut preds = HashSet::new();
    for f in fluents {
        let cleaned = if f.starts_with("Atom ") {
            &f[5..]
        } else {
            f.as_str()
        };
        if let Some(idx) = cleaned.find('(') {
            preds.insert(cleaned[..idx].trim().to_lowercase());
        } else {
            preds.insert(cleaned.trim().to_lowercase());
        }
    }
    preds
}

pub fn extract_fluents_from_content(sas_content: &str) -> Result<Vec<String>, String> {
    let mut fluents = Vec::new();
    let mut lines = sas_content.lines();

    while let Some(line) = lines.next() {
        let trimmed = line.trim();
        if trimmed == "begin_variable" {
            let _ = lines.next(); // var name/id
            let _ = lines.next(); // axiom layer
            if let Some(count_line) = lines.next() {
                if let Ok(count) = count_line.trim().parse::<usize>() {
                    for _ in 0..count {
                        if let Some(val_line) = lines.next() {
                            fluents.push(val_line.trim().to_string());
                        }
                    }
                }
            }
        }
    }
    Ok(fluents)
}

pub fn run_fast_downward_and_get_content(
    domain_path: &Path,
    problem_path: &Path,
) -> Result<String, String> {
    let fd_path_str =
        std::env::var("FAST_DOWNWARD_PATH").unwrap_or_else(|_| "fd/fast-downward.py".to_string());

    let fd_path = Path::new(&fd_path_str);
    if !fd_path.exists() {
        return Err(format!(
            "Fast Downward introuvable au chemin : {}",
            fd_path.display()
        ));
    }

    let output = Command::new("python3")
        .arg(fd_path)
        .arg("--translate")
        .arg(domain_path)
        .arg(problem_path)
        .output()
        .map_err(|e| format!("Impossible de lancer FD : {}", e))?;

    if !output.status.success() {
        return Err(format!(
            "FD a échoué : {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    std::fs::read_to_string("output.sas")
        .map_err(|e| format!("Impossible de lire output.sas : {}", e))
}
