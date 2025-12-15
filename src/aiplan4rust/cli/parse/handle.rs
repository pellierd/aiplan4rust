use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;
use clap::ArgMatches;
use clap::error::ErrorKind;
use colored::Colorize;
use crate::aiplan4rust::cli::aiplan_cli::{FILES_ARG, FORMAT_ARG, OUTPUT_ARG, OUT_DIR_ARG};
use crate::aiplan4rust::serialization::serde::{SerdeExtension, SerdeFormat, SerdeSerializable};
use crate::{Frontend, Renderer, Severity};

/// Handles the `parse` command logic.
///
/// This function retrieves the domain and/or problem files from the CLI arguments,
/// determines whether one or two files are provided, and calls the appropriate parsing function.
/// If an output file is not specified, a default name is generated.
///
/// # Arguments:
/// - `matches`: Parsed command-line arguments for the `parse` subcommand.
/// Handles the `parse` command logic.
pub fn handle_parse_command(matches: &ArgMatches) {
    // Validate arguments first
    if let Err(err) = validate_parse_args(matches) {
        err.print().expect("Error printing clap error");
        std::process::exit(1);
    }

    // Retrieve the input files
    let files: Vec<String> = matches
        .get_many::<String>(FILES_ARG)
        .unwrap()
        .cloned()
        .collect();

    let format = *matches.get_one::<SerdeFormat>(FORMAT_ARG).unwrap();

    match files.len() {
        1 => {
            let input_file = &files[0];
            let output = matches
                .get_one::<String>(OUTPUT_ARG)
                .cloned()
                .unwrap_or_else(|| generate_parsed_filename(input_file, format, None));

            // Ensure parent directories exist
            ensure_parent_dir_exists(&output);
            parse(input_file, format, &output);
        }
        _ => {
            // Multiple files
            let out_dir = matches
                .get_one::<String>(OUT_DIR_ARG)
                .cloned()
                .unwrap_or_else(|| ".".to_string()); // default to current directory

            ensure_dir_exists(&out_dir);

            for input_file in &files {
                let output_path = generate_parsed_filename(input_file, format, Some(&out_dir));

                // Ensure parent directories exist for each file
                ensure_parent_dir_exists(&output_path);
                parse(input_file, format, &output_path);

            }
        }
    }
}

fn parse(input_file: &str, format: SerdeFormat, output: &str) {
    let start_time = Instant::now(); // Démarre le chronomètre

    let full_path = Path::new(input_file)
        .canonicalize()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| input_file.to_string());

    println!(
        "\n{:>10} aiplan4rust v0.1.0 ({})",
        "Parsing".green().bold(),
        full_path
    );

    let frontend = Frontend::new();
    match frontend.parse_file(input_file) {
        Ok(result) => {
            let mut renderer = Renderer::new(result.diagnostic_manager(), result.interner()) ;
            let _ = renderer.display();

            // Compte les erreurs et les warnings
            let error_count = result.diagnostic_manager().count_diagnostics_of_severity(Severity::Error);
            let warning_count = result.diagnostic_manager().count_diagnostics_of_severity(Severity::Warning);

            // Chronomètre
            let elapsed_time = start_time.elapsed().as_secs_f32();

            // Affichage du message de fin
            println!(
                "{} {} error(s), {} warning(s) target(s) in {:.2}s",
                "Finished".green().bold(),
                format!("{}", error_count),
                format!("{}", warning_count),
                elapsed_time
            );

            // Si des erreurs sont présentes, indiquer qu'aucun fichier n'a été produit
            if error_count > 0 {
                println!(
                    "{} No output file produced due to errors.",
                    "===> ".blue().bold());
            } else {
                // Si aucun problème, afficher que le fichier a été produit
                if let Some(context) = result.semantic_context() {
                    if let Err(e) = context.serialize_to_file(format, output) {
                        eprintln!("Error saving file: {}", e);
                    } else {
                        let absolute_output = Path::new(output)
                            .canonicalize()
                            .map(|p| p.display().to_string())
                            .unwrap_or_else(|_| output.to_string());

                        println!(
                            "{} Output file produced ({})",
                            "===> ".blue().bold(),
                            absolute_output
                        );
                    }
                }
            }
        }
        Err(e) => {
            eprintln!("{}", e);
        }
    }
}

/// Generates the output filename based on input file, format, and optional output directory.
fn generate_parsed_filename(
    input_file: &str,
    format: SerdeFormat,
    out_dir: Option<&str>,
) -> String {
    let base_name = Path::new(input_file)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(input_file);
    let extension = SerdeExtension::from(format).as_str();

    match out_dir {
        Some(dir) => {
            let mut path = PathBuf::from(dir);
            path.push(format!("{}.{}", base_name, extension));
            path.to_string_lossy().into_owned()
        }
        None => format!("{}.{}", base_name, extension),
    }
}

/// Ensures that the parent directory of a given file path exists.
/// Exits the process on error.
fn ensure_parent_dir_exists(path: &str) {
    if let Some(parent) = Path::new(path).parent() {
        if !parent.exists() {
            if let Err(e) = fs::create_dir_all(parent) {
                eprintln!("Error creating output directory {}: {}", parent.display(), e);
                std::process::exit(1);
            }
        }
    }
}

/// Ensures that the given directory exists.
/// Exits the process on error.
fn ensure_dir_exists(dir: &str) {
    if !Path::new(dir).exists() {
        if let Err(e) = fs::create_dir_all(dir) {
            eprintln!("Error creating directory {}: {}", dir, e);
            std::process::exit(1);
        }
    }
}

/// Validates the parse subcommand arguments.
///
/// Rules:
/// 1. If multiple files are provided, `-o/--output` is forbidden.
/// 2. If multiple files are provided, `-d/--out-dir` must be specified.
/// 3. `-f/--format` is always optional and defaults to Json.
fn validate_parse_args(matches: &ArgMatches) -> Result<(), clap::Error> {
    let files: Vec<&String> = matches
        .get_many::<String>(FILES_ARG)
        .unwrap_or_default()
        .collect();

    let output_provided = matches.get_one::<String>(OUTPUT_ARG).is_some();
    let out_dir_provided = matches.get_one::<String>(OUT_DIR_ARG).is_some();

    if files.len() > 1 {
        if output_provided {
            return Err(clap::Error::raw(
                ErrorKind::ArgumentConflict,
                "`-o/--output` is only valid for a single input file; use `-d/--out-dir` for multiple files",
            ));
        }

        if !out_dir_provided {
            return Err(clap::Error::raw(
                ErrorKind::MissingRequiredArgument,
                "Multiple input files require `-d/--out-dir` to specify the output directory",
            ));
        }
    }

    Ok(())
}
