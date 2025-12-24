
use aiplan4rust::aiplan4rust::cli::build_cli;
use aiplan4rust::aiplan4rust::cli::link::{handle_link_command, LINK_SUBCOMMAND};
use aiplan4rust::aiplan4rust::cli::parse::cli::PARSE_SUBCOMMAND;
use aiplan4rust::aiplan4rust::cli::parse::handle_parse_command;

/// Main entry point for the application.
///
/// Parses command-line arguments and dispatches execution to the appropriate command handler.
fn main() {
    env_logger::init();

    let matches = build_cli().get_matches();

    // Récupérer le résultat et gérer les erreurs
    let result = if let Some(matches) = matches.subcommand_matches(LINK_SUBCOMMAND) {
        handle_link_command(matches)
    } else if let Some(matches) = matches.subcommand_matches(PARSE_SUBCOMMAND) {
        handle_parse_command(matches)
    } else {
        // Aucun sous-commande fourni
        eprintln!("No subcommand provided.");
        std::process::exit(1);
    };

    // Vérifier si une erreur est survenue et l'afficher
    if let Err(err) = result {
        eprintln!("Error: {:?}", err);
        std::process::exit(1); // Optionnel : sortir avec code d'erreur
    }
}
