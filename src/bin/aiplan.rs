use clap_builder::Command;
use aiplan4rust::aiplan4rust::cli::build_cli;
use aiplan4rust::aiplan4rust::cli::error::CliError;
use aiplan4rust::aiplan4rust::cli::ground::{handle_ground_command, GROUND_SUBCOMMAND};
use aiplan4rust::aiplan4rust::cli::link::{handle_link_command, LINK_SUBCOMMAND};
use aiplan4rust::aiplan4rust::cli::parse::{handle_parse_command, PARSE_SUBCOMMAND};

/// Main entry point for the application.
///
/// Parses command-line arguments and dispatches execution to the appropriate command handler.
fn main() {
    env_logger::init();

    let mut cli = build_cli();
    let args = cli.get_matches_mut();

    match args.subcommand() {
        Some((subcommand @ GROUND_SUBCOMMAND, sub_matches)) => {
            // Nouvelle sous-commande ground
            let result = handle_ground_command(sub_matches);
            handle_cli_result(result, &mut cli, subcommand);
        }
        Some((subcommand @ LINK_SUBCOMMAND, sub_matches)) => {
            let result = handle_link_command(sub_matches);
            handle_cli_result(result, &mut cli, subcommand);
        }
        Some((subcommand @ PARSE_SUBCOMMAND, sub_matches)) => {
            let result = handle_parse_command(sub_matches);
            handle_cli_result(result, &mut cli, subcommand);
        }
        _ => {
            eprintln!("Internal error: no subcommand provided");
            cli.print_long_help().unwrap();
            println!();
            std::process::exit(2);
        }
    }
}

fn handle_cli_result(result: Result<(), CliError>, cli: &mut Command, subcommand: &str) -> ! {
    match result {
        Ok(()) => std::process::exit(0),

        Err(CliError::Clap(e)) => {
            e.print().unwrap_or_else(|err| eprintln!("Failed to print error: {err}"));
            println!();

            // Affiche l'aide complète de la sous-commande
            if let Some(sub) = cli.find_subcommand_mut(subcommand) {
                println!();
                sub.print_help().unwrap_or_else(|err| eprintln!("Failed to print help: {err}"));
                println!();
            }

            std::process::exit(e.exit_code());
        }

        Err(other) => {
            eprintln!("Internal error: {other:?}");

            // Récupération des variables d'environnement
            let log_level = std::env::var("RUST_LOG").unwrap_or_default().to_lowercase();
            let backtrace = std::env::var("RUST_BACKTRACE").unwrap_or_default();

            // On n'affiche le bloc d'aide que si on n'est pas déjà en mode "verbeux"
            if !log_level.contains("debug") && !log_level.contains("trace") && backtrace != "1" {
                eprintln!("To help us fix this, please run the command again with:");
                eprintln!("    RUST_LOG=debug RUST_BACKTRACE=1 ./target/debug/aiplan ...");
            }

            eprintln!("This is a bug. Please report it at: https://github.com/aiplan4rust/issues");
            std::process::exit(2);
        }
    }
}
