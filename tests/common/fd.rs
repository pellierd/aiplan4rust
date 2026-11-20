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
            let _ = lines.next(); // -1 ou axiom layer

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

/// Extrait et normalise tous les atomes/fluents possibles des domaines SAS+
/// pour les rendre comparables aux faits de la base Datalog.
pub fn extract_datalog_style_fluents(sas_content: &str) -> Result<HashSet<String>, String> {
    let raw_fluents = extract_fluents_from_content(sas_content)?;
    let mut datalog_atoms = HashSet::new();

    for f in raw_fluents {
        let cleaned = if f.starts_with("Atom ") {
            &f[5..]
        } else {
            f.as_str()
        };

        let trimmed = cleaned.trim();
        let trimmed_lower = trimmed.to_lowercase();

        if trimmed_lower.starts_with("negatedatom")
            || trimmed.contains("new-axiom@")
            || !trimmed.contains('(')
            || !trimmed.ends_with(')')
        {
            continue;
        }

        datalog_atoms.insert(trimmed.to_lowercase());
    }

    Ok(datalog_atoms)
}
pub fn extract_actions_from_content(sas_content: &str) -> Result<Vec<String>, String> {
    let mut actions = Vec::new();
    let mut lines = sas_content.lines().peekable();

    while let Some(line) = lines.next() {
        if line.trim() == "begin_operator" {
            if let Some(action_name) = lines.next() {
                actions.push(action_name.trim().to_string());
            }
        }
    }

    Ok(actions)
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
            "Fast Downward introuvable au chemin : {}. Définis la variable d'environnement FAST_DOWNWARD_PATH ou place le dossier fd/ au bon endroit.",
            fd_path.display()
        ));
    }

    // 1. Rendre TOUS les chemins absolus (essentiel pour le changement de dossier)
    let abs_fd_path = std::fs::canonicalize(fd_path)
        .map_err(|e| format!("Impossible de résoudre le chemin de FD : {}", e))?;
    let abs_domain = std::fs::canonicalize(domain_path)
        .map_err(|e| format!("Chemin domaine invalide : {}", e))?;
    let abs_problem = std::fs::canonicalize(problem_path)
        .map_err(|e| format!("Chemin problème invalide : {}", e))?;

    // 2. Extraire les noms pour le fichier personnalisé
    let domain_name = domain_path
        .parent()
        .and_then(|p| p.file_name())
        .and_then(|n| n.to_str())
        .unwrap_or("domain");
    let problem_name = problem_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("problem");

    // 3. Créer un dossier temporaire unique (sandbox) pour le parallélisme total
    let temp_dir = tempfile::tempdir()
        .map_err(|e| format!("Impossible de créer le dossier temporaire : {}", e))?;

    println!(
        "  [Oracle] Lancement de Fast Downward pour [{}/{}]...",
        domain_name, problem_name
    );

    // 4. Lancer FD dans la sandbox avec les chemins absolus
    let output = Command::new("python3")
        .arg(&abs_fd_path)
        .arg("--translate")
        .arg(&abs_domain)
        .arg(&abs_problem)
        .current_dir(temp_dir.path()) // Isole l'exécution et output.sas
        .output()
        .map_err(|e| format!("Impossible de lancer le processus Fast Downward : {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "Fast Downward a échoué avec le code de sortie : {}\nStderr: {}",
            output.status, stderr
        ));
    }

    // 5. Renommer "output.sas" avec le nom personnalisé du domaine et du problème
    let default_sas = temp_dir.path().join("output.sas");
    let custom_sas_name = format!("{}_{}.sas", domain_name, problem_name);
    let custom_sas_path = temp_dir.path().join(custom_sas_name);

    if default_sas.exists() {
        std::fs::rename(&default_sas, &custom_sas_path)
            .map_err(|e| format!("Impossible de renommer le fichier SAS : {}", e))?;
    }

    // 6. Lire le contenu du fichier personnalisé
    let sas_content = std::fs::read_to_string(&custom_sas_path).map_err(|e| {
        format!(
            "Impossible de lire le fichier SAS généré ({}) : {}",
            custom_sas_path.display(),
            e
        )
    })?;

    Ok(sas_content)
}
