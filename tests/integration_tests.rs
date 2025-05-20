use std::path::Path;
use std::fs;
use aiplan4rust::{Frontend, Language, DiagnosticRenderer};

// Teste un seul domaine donné (un dossier contenant des problèmes et domaines)
fn test_domain(domain_dir: &Path, language: &Language) {
    let mut files = fs::read_dir(domain_dir)
        .expect("Cannot read domain subdir")
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().map(|s| s == "hddl").unwrap_or(false))
        .collect::<Vec<_>>();

    // Filtrer et trier uniquement les fichiers problème (pb*)
    let mut problem_files: Vec<_> = files.iter()
        .filter(|p| {
            p.file_name()
                .and_then(|f| f.to_str())
                .map(|f| f.starts_with("pb") && !f.contains("-domain"))
                .unwrap_or(false)
        })
        .cloned()
        .collect();

    problem_files.sort_by_key(|p| p.file_name().map(|f| f.to_os_string()));

    for problem_path in problem_files {
        let problem_stem = problem_path
            .file_stem()
            .unwrap()
            .to_string_lossy()
            .to_string();

        let domain_path1 = domain_dir.join("domain.hddl");
        let domain_path2 = domain_dir.join(format!("{}-domain.hddl", problem_stem));
        let domain_path = if domain_path2.exists() {
            domain_path2
        } else if domain_path1.exists() {
            domain_path1
        } else {
            panic!(
                "No domain file found for problem: {}",
                problem_path.display()
            );
        };

        println!(
            "Parsing domain: {} and problem: {}",
            domain_path.display(),
            problem_path.display()
        );

        let frontend = Frontend::new();
        let result = frontend.parse(
            domain_path.to_str().unwrap(),
            problem_path.to_str().unwrap(),
            language,
        );

        match result {
            Ok(linker_result) => {
                let mut renderer = DiagnosticRenderer::new(linker_result.diagnostic_manager());
                renderer.display();

                assert!(
                    linker_result.planning_task().is_some(),
                    "Expected linked planning task to be available for: {}",
                    problem_path.display()
                );
            }
            Err(e) => {
                eprintln!(
                    "Parsing error for domain: {} and problem: {}.\nError: {}",
                    domain_path.display(),
                    problem_path.display(),
                    e
                );
                panic!("Parsing failed");
            }
        }
    }
}

fn test_competition(competition_dir: &Path, language: &Language) {
    // Collecte tous les sous-dossiers
    let mut domain_dirs = fs::read_dir(competition_dir)
        .expect("Cannot read competition dir")
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
        .collect::<Vec<_>>();

    // Trie les dossiers par nom (alphabetique)
    domain_dirs.sort_by_key(|path| path.file_name().map(|f| f.to_os_string()));

    // Parcours les dossiers triés
    for domain_dir in domain_dirs {
        test_domain(&domain_dir, language);
    }
}

// Test global qui teste toutes les compétitions (plusieurs dossiers racines)
#[test]
fn test_all_competitions() {
    let ipc20_partial_order_coloring = Path::new("tests/integration/hddl/ipc20/partial-order/colouring");
    let ipc20_partial_order_monroe_fully_observable = Path::new("tests/integration/hddl/ipc20/partial-order/monroe-fully-observable");

    test_domain(ipc20_partial_order_coloring, &Language::HDDL);

    test_domain(ipc20_partial_order_monroe_fully_observable, &Language::HDDL);
}
