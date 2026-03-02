/*mod common;

use std::path::Path;
use crate::common::io::collect_domain_files;
use crate::common::pipeline::{run_full_grounding_pipeline}; // Ta fonction chapeau
use test_case::test_case;

/// Fonction de test générique pour le grounding
fn test_grounding_domain(domain_path: &str, quick_mode: bool) {
    let path = Path::new(domain_path);

    // On récupère les fichiers (domaine + problèmes)
    let files = collect_domain_files(path);
    let domain_file = files.iter().find(|p| p.to_string_lossy().contains("domain"))
        .expect("Fichier domaine manquant");

    let mut problems: Vec<_> = files.iter()
        .filter(|p| !p.to_string_lossy().contains("domain"))
        .collect();

    // Mode court pour le CI : on ne garde que le premier problème
    if quick_mode && !problems.is_empty() {
        problems.truncate(1);
    }

    for prob_path in problems {
        // Exécution du pipeline : Parse -> Analyze -> Encode -> Datalog Run
        let result = run_full_grounding_pipeline(domain_file, prob_path);

        assert!(result.is_ok(), "Le grounding a échoué pour {:?}", prob_path);

        let report = result.unwrap();
        // Vérification minimale : on doit avoir trouvé au moins un fait initial
        assert!(report.total_facts > 0, "Aucun fait généré pour {:?}", prob_path);

        println!(
            "Domain {:?} | Prob {:?}: {} actions, {} facts",
            path.file_name().unwrap(),
            prob_path.file_name().unwrap(),
            report.total_actions,
            report.total_facts
        );
    }
}*/
