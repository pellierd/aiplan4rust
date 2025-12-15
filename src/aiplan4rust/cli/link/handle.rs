use std::time::Instant;
use clap::ArgMatches;
use colored::Colorize;
use crate::aiplan4rust::serialization::serde::{SerdeFormat, SerdeSerializable};
use crate::{Frontend, Renderer, Severity};
use crate::aiplan4rust::cli::cli::{FILES_ARG, FORMAT_ARG, OUTPUT_ARG};

/// Handles the `link` command logic.
///
/// This function retrieves the domain and problem files from the CLI arguments,
/// validates that exactly two files are provided, and calls the `link` function
/// to combine them into the specified output format.
///
/// # Arguments:
/// - `matches`: Parsed command-line arguments for the `link` subcommand.
pub fn handle_link_command(matches: &ArgMatches) {
    if let Some(files) = matches.get_many::<String>(FILES_ARG) {
        let files_vec: Vec<String> = files.cloned().collect();
        if files_vec.len() == 2 {
            let output = matches.get_one::<String>(OUTPUT_ARG).unwrap();
            let format = matches.get_one::<SerdeFormat>(FORMAT_ARG).unwrap();
            link_from_semantic_context(&files_vec[0], &files_vec[1], *format, output);
        } else {
            eprintln!("Error: You must provide exactly two files (domain and problem).")
        }
    }
}


fn link_from_semantic_context(domain_file: &str, problem_file: &str, format: SerdeFormat, output: &str) {
    let frontend = Frontend::new();

    // Appel de la méthode link sur frontend
    match frontend.link(domain_file, problem_file) {
        Ok(linker_result) => {
            if let Some(planning_task) = linker_result.linked_semantic_context() {
                if let Err(e) =
                    planning_task.serialize_to_file(format, output)
                {
                    eprintln!("Error saving file: {}", e);
                } else {
                    println!("Output saved to {}", output);
                }
            } else {
                let mut renderer = Renderer::new(linker_result.diagnostic_manager(), linker_result.interner());
                let _ =  renderer.display();
            }
        }
        Err(e) => {
            eprintln!("Error during linking: {}", e);
        }
    }
}

pub fn link_from_files(
    domain_file: &str,
    problem_file: &str,
    format: SerdeFormat,
    output: &str,
) {
    let start_time = Instant::now();

    println!(
        "{:>10} aiplan4rust v0.1.0 (domain: {}, problem: {})",
        "Parsing".green().bold(),
        domain_file,
        problem_file
    );

    let frontend = Frontend::new();
    match frontend.parse(domain_file, problem_file) {
        Ok(result) => {
            let mut renderer = Renderer::new(result.diagnostic_manager(), result.interner());
            let _ = renderer.display();


            // Count errors and warnings
            let dm = result.diagnostic_manager();
            let error_count = dm.count_diagnostics_of_severity(Severity::Error);
            let warning_count = dm.count_diagnostics_of_severity(Severity::Warning);

            // Elapsed time
            let elapsed = start_time.elapsed().as_secs_f32();

            println!(
                "{} {} error(s), {} warning(s) in {:.2}s",
                "Finished".green().bold(),
                error_count,
                warning_count,
                elapsed
            );

            if error_count > 0 {
                println!(
                    "{} No output file produced due to errors.",
                    "===>".blue().bold()
                );
            } else if let Some(lifted_problem) = result.lifted_problem() {
                if let Err(e) =
                    lifted_problem.serialize_to_file(format, output)
                {
                    eprintln!("Error saving file: {}", e);
                } else {
                    println!(
                        "{} Output saved to {}",
                        "===>".blue().bold(),
                        output
                    );
                }
            }
        }
        Err(e) => {
            eprintln!("{}", e);
        }
    }
}
