use clap_builder::Command;
use aiplan4rust::aiplan4rust::cli::build_cli;
use aiplan4rust::aiplan4rust::cli::error::CliError;
use aiplan4rust::aiplan4rust::cli::link::{handle_link_command, LINK_SUBCOMMAND};
use aiplan4rust::aiplan4rust::cli::parse::cli::PARSE_SUBCOMMAND;
use aiplan4rust::aiplan4rust::cli::parse::handle_parse_command;

/// Main entry point for the application.
///
/// Parses command-line arguments and dispatches execution to the appropriate command handler.
fn main() {
    env_logger::init();

    let mut cli = build_cli();
    let args = cli.get_matches_mut();

    match args.subcommand() {
        Some((subcommand @ LINK_SUBCOMMAND, sub_matches)) => {
            let result = handle_link_command(sub_matches);
            handle_cli_result(result, &mut cli, subcommand);
        }
        Some((name @ PARSE_SUBCOMMAND, sub_matches)) => {
            let result = handle_parse_command(sub_matches);
            handle_cli_result(result, &mut cli, name);
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
            eprintln!("Internal error: {other:?}"); // ou .debug()
            //eprintln!("Backtrace:\n{:?}", std::backtrace::Backtrace::capture());
            eprintln!("This is a bug. Please report it.");
            std::process::exit(2);
        }
    }
}
