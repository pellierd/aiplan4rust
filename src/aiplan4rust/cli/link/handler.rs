//! This module provides the CLI linking workflow for `aiplan4rust`.
//!
//! It defines functions to handle the `link` subcommand, validate domain and problem files,
//! filter inputs based on their typing (Raw or Parsed/IR), and perform linking of a single domain
//! with one or multiple problem files. The module also manages output serialization,
//! diagnostics reporting, and timing statistics.
//!
//! # Key Responsibilities
//!
//! - **Reading Inputs:** Handles reading of domain and problem files from paths, including error handling.
//! - **Validation:** Validates that domain and problem files are of compatible types (Raw or IR/Parsed)
//!   and that they are well-formed.
//! - **Filtering:** Filters out invalid or incompatible problem files, printing warnings for ignored files.
//! - **Linking:** Links each valid problem with the domain, producing lifted outputs if no errors occur.
//! - **Serialization:** Supports output in multiple formats (e.g., JSON, YAML) using `SerdeFormat`.
//! - **Diagnostics:** Tracks and prints errors and warnings encountered during linking.
//! - **CLI Integration:** Uses `clap::ArgMatches` to parse command-line arguments for the `link` subcommand.

use crate::aiplan4rust::cli::cli::{CURRENT_DIR, FILES_ARG, FORMAT_ARG, OUTPUT_ARG, OUT_DIR_ARG};
use crate::aiplan4rust::cli::error::CliError;
use crate::aiplan4rust::artefact::error::ArtefactError;
use crate::aiplan4rust::artefact::source::Source;
use crate::aiplan4rust::artefact::{IRContent, Artefact};
use crate::aiplan4rust::lang::Requirement;
use crate::aiplan4rust::lir::problem::LiftedProblem;
use crate::aiplan4rust::serialization::serde::SerdeFormat;
use crate::{Frontend, Renderer, Severity};
use clap::ArgMatches;
use colored::Colorize;
use std::fs;
use std::path::PathBuf;
use clap::error::ErrorKind;
use crate::aiplan4rust::cli::check::check_link_args;
use crate::aiplan4rust::cli::path::{default_lifted_output_path, output_path};

/// Handles the `link` CLI subcommand.
///
/// This function orchestrates the workflow for linking a single domain file
/// with one or more problem files. It determines whether the domain and problems
/// are in IR or Raw format, reads the inputs, validates them, computes output paths,
/// and delegates the linking and statistics accumulation to `link_inputs`.
///
/// # Behavior
///
/// - The first file in the CLI arguments is treated as the **domain**.
/// - All remaining files are treated as **problem files**.
/// - If the domain is IR, all problems must also be IR.
/// - If the domain is Raw, all problems must also be Raw.
/// - Each problem is linked individually with the domain, producing separate output files.
/// - If a single problem file is provided and an explicit output path is given,
///   it will be used; otherwise, default output paths are generated.
/// - Warnings are printed for any invalid domain or problem files.
///
/// # Arguments
///
/// * `matches` - A reference to `ArgMatches` containing parsed CLI arguments for the `link` subcommand.
///
/// # Returns
///
/// Returns `Ok(())` if all linking operations succeed.
/// Returns `Err(CliError)` if:
/// - No input files are provided,
/// - Domain or problem files cannot be read or are invalid,
/// - Domain and problem types are inconsistent,
/// - Any file reading or linking operation fails.
///
/// # Example
///
/// ```rust
/// let matches = build_link_subcommand().get_matches();
/// handle_link_command(&matches)?;
/// ```
pub fn handle_link_command(matches: &ArgMatches) -> Result<(), CliError> {
    // --- Validate CLI arguments ---
    check_link_args(matches)?;

    // --- Collect files from CLI arguments ---
    let files: Vec<String> = matches
        .get_many::<String>(FILES_ARG)
        .ok_or_else(CliError::missing_input_files)?
        .cloned()
        .collect();

    // --- Determine output format (default to JSON) ---
    let format = *matches
        .get_one::<SerdeFormat>(FORMAT_ARG)
        .ok_or_else(|| {
            clap::Error::raw(
                ErrorKind::MissingRequiredArgument,
                "No output format provided"
            )
        })?;

    // --- Optional output path ---
    let output_opt = matches.get_one::<String>(OUTPUT_ARG).map(PathBuf::from);

    // Determine the output directory
    let out_dir = matches
        .get_one::<String>(OUT_DIR_ARG)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(CURRENT_DIR));


    // --- Validate domain file ---
    let domain_file = PathBuf::from(&files[0]);
    let domain = match filter_domain(&domain_file) {
        Some(d) => d,
        None => {
            println!("Warning: no valid domain found, link command skipped");
            return Ok(());
        }
    };

    // --- Validate problem files ---
    let problem_files: Vec<PathBuf> = files[1..].iter().map(PathBuf::from).collect();
    let problems = filter_problems(&domain, problem_files)?;

    if problems.is_empty() {
        println!("Warning: no valid problem files found, nothing to do");
        return Ok(());
    }

    // --- Execute linking workflow with validated inputs ---
    link_inputs(&domain, &problems, format, &out_dir, &output_opt)?;

    Ok(())
}

/// Links a single domain file with multiple problem files, producing lifted outputs.
///
/// This function performs two main steps over the problem files:
/// 1. **Compute output paths** for each problem, optionally using an explicit path
///    if only one problem file is provided.
/// 2. **Link the domain with each problem**, producing separate lifted outputs for each.
///
/// # Behavior
///
/// - The first argument (`domain`) is the domain input; all others are problem inputs.
/// - The domain can be either parsed (IR) or raw. All problems must match the domain typing.
/// - Each problem is linked individually; if a single problem is provided and an explicit
///   output path is given, it will be used.
/// - A summary of the operation is printed, including the number of problems linked and
///   the total elapsed time.
///
/// # Arguments
///
/// * `domain` - Reference to a validated `Input` representing the domain file.
/// * `problems` - Slice of validated `Input`s representing problem files.
/// * `format` - Serialization format for the output (`SerdeFormat`).
/// * `output_opt` - Optional explicit output path, used only if a single problem file.
///
/// # Returns
///
/// Returns `Ok(())` if all linking succeeds.
/// Returns `Err(CliError)` if any linking or output path computation fails.
///
/// # Example
///
/// ```rust
/// let domain = Input::read_from_file("domain.pddl")?;
/// let problems = vec![Input::read_from_file("problem1.pddl")?, Input::read_from_file("problem2.pddl")?];
/// link_inputs(&domain, &problems, SerdeFormat::Json, None)?;
/// ```
fn link_inputs(
    domain: &Source,
    problems: &Vec<Source>,
    format: SerdeFormat,
    out_dir: &PathBuf,
    output_opt: &Option<PathBuf>,
) -> Result<(), CliError> {
    use std::time::Instant;

    let start_time = Instant::now();

    for problem in problems {
        let output_path: PathBuf = match output_opt {
            Some(path) => output_path(&path.to_path_buf(), out_dir)?,
            None => default_lifted_output_path(domain.path(), problem.path(), out_dir)?,
        };

        // Linking
        if domain.is_parsed_domain() {
            link_from_parsed_input(domain, &problem, format, &output_path)?;
        } else {
            link_from_raw_input(domain, &problem, format, &output_path)?;
        }
    }

    // Résumé global si plusieurs problèmes
    if problems.len() > 1 {
        let total_time = start_time.elapsed().as_secs_f32();
        println!(
            "\n{} {} problem(s) linked in {:.2}s",
            "Finished".green().bold(),
            problems.len(),
            total_time
        );
    }

    Ok(())
}

/// Links a parsed domain and problem input, producing a serialized output if successful.
///
/// This function performs the following steps:
/// 1. Creates a `Frontend` instance to handle linking operations.
/// 2. Invokes the frontend to link the parsed domain and problem inputs.
/// 3. Renders diagnostics (errors and warnings) to the console using `Renderer`.
/// 4. If the linking produces a lifted problem (semantic context), serializes and saves it
///    to the specified output path.
///
/// # Arguments
///
/// * `domain` - A reference to an `Input` representing the parsed domain file.
/// * `problem` - A reference to an `Input` representing the parsed problem file.
/// * `format` - The serialization format for the output (`SerdeFormat`).
/// * `output` - The path where the lifted problem will be saved.
///
/// # Returns
///
/// Returns `Ok(())` on success.
/// Returns a `CliError` if any of the following occur:
/// - Linking of the parsed inputs fails.
/// - Rendering of diagnostics fails.
/// - Saving the lifted problem fails.
///
/// # Behavior
///
/// - Diagnostics are always displayed, even if linking fails.
/// - Only produces an output file if a lifted problem is successfully created.
///
/// # Examples
///
/// ```no_run
/// let domain = Input::read_from_file("domain.prs").unwrap();
/// let problem = Input::read_from_file("pb01.prs").unwrap();
/// let output = PathBuf::from("pb01.lifted");
/// link_from_parsed_input(&domain, &problem, SerdeFormat::JSON, &output).unwrap();
/// ```
fn link_from_parsed_input(
    domain: &Source,
    problem: &Source,
    format: SerdeFormat,
    output: &PathBuf,
) -> Result<(), CliError> {
    // Create a frontend instance
    let frontend = Frontend::new();

    // Perform linking, propagate any errors
    let mut builder_result = frontend.link_from_parsed_input(domain, problem)?;

    // Display diagnostics
    let mut renderer = Renderer::new(
        builder_result.diagnostic_manager(),
        builder_result.interner(),
    );
    renderer.display()?;

    // If linking produced a semantic context, serialize it
    if let Some(lifted_problem) = builder_result.take_lifted_problem() {
        save_link_output(lifted_problem, format, output)?;
    }

    Ok(())
}

/// Links a raw domain and problem input, producing a serialized output if no errors occur.
///
/// This function performs the following steps:
/// 1. Logs the start of the linking process, including the domain and problem file paths.
/// 2. Uses the `Frontend` to parse the domain and problem inputs.
/// 3. Renders diagnostics (errors and warnings) to the console using `Renderer`.
/// 4. Counts the number of errors and warnings encountered during parsing.
/// 5. Prints a summary including elapsed time, errors, and warnings.
/// 6. If there are no errors, serializes and saves the lifted problem to the specified output path.
///
/// # Arguments
///
/// * `domain` - A reference to an `Input` representing the domain file. Must be a raw input.
/// * `problem` - A reference to an `Input` representing the problem file. Must be a raw input.
/// * `format` - The desired serialization format for the output (`SerdeFormat`).
/// * `output` - Path to the file where the serialized output will be saved.
///
/// # Returns
///
/// Returns `Ok(())` on success.
/// Returns a `CliError` if any of the following occur:
/// - Parsing of domain or problem fails.
/// - Diagnostics rendering fails.
/// - Saving the lifted problem fails.
///
/// # Behavior
///
/// - If parsing produces warnings but no errors, the output is still saved.
/// - If any errors are encountered, the output is not produced.
/// - Timing of the linking process is printed for user feedback.
///
/// # Examples
///
/// ```no_run
/// let domain = Input::read_from_file("domain.hddl").unwrap();
/// let problem = Input::read_from_file("pb01.hddl").unwrap();
/// let output = PathBuf::from("pb01.lifted");
/// link_from_raw_input(&domain, &problem, SerdeFormat::JSON, &output).unwrap();
/// ```
fn link_from_raw_input(
    domain: &Source,
    problem: &Source,
    format: SerdeFormat,
    output: &PathBuf,
) -> Result<(), CliError> {
    use std::time::Instant;

    let start_time = Instant::now();

    println!(
        "{} aiplan4rust v0.1.0 (domain: {}, problem: {})",
        "Parsing".green().bold(),
        domain.path().to_string_lossy(),
        problem.path().to_string_lossy()
    );

    let frontend = Frontend::new();

    // Parse domain and problem files
    let mut result = frontend.link_from_raw_input(domain, problem)?;

    // Display diagnostics (propagation via DiagnosticError)
    let mut renderer = Renderer::new(result.diagnostic_manager(), result.interner());
    renderer.display()?; // DiagnosticError se convertit en CliError

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
    } else if let Some(lifted_problem) = result.take_lifted_problem() {
        // Serialize the lifted problem
        save_link_output(lifted_problem, format, output)?;
    }

    Ok(())
}

/// Saves a "linked" problem (`LiftedProblem`) to a file.
///
/// This function performs the following steps:
/// 1. Ensures that all parent directories for the output file exist, creating them if necessary.
/// 2. Wraps the provided `LiftedProblem` in an `IRContent::LiftedProblem` container along with
///    the chosen serialization format.
/// 3. Creates an `Output` object to handle writing the content to disk.
/// 4. Writes the serialized problem to the specified output path.
/// 5. Resolves and prints the absolute path of the produced file for user confirmation.
///
/// # Type Parameters
/// - `P`: Any typing that can be converted into a `PathBuf` (e.g., `&str`, `String`, `PathBuf`).
///
/// # Arguments
/// * `lifted_problem` - The lifted problem (linked problem) to save.
/// * `format` - The serialization format to use (JSON, etc.).
/// * `output_path` - Path where the serialized output should be written.
///
/// # Returns
/// * `Ok(())` if the file was successfully saved.
/// * `Err(IOError)` if any I/O operation (creating directories or writing the file) fails.
///
/// # Example
/// ```rust
/// use aiplan4rust::io::save_link_output;
/// use aiplan4rust::serialization::serde::SerdeFormat;
///
/// let lifted_problem = ...; // Construct or obtain a LiftedProblem
/// save_link_output(lifted_problem, SerdeFormat::Json, "output/problem.lifted")?;
/// ```
pub fn save_link_output<P: Into<PathBuf>>(
    lifted_problem: LiftedProblem,
    format: SerdeFormat,
    output_path: P,
) -> Result<(), ArtefactError> {
    let output_path = output_path.into();

    // Create all parent directories if they do not exist
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)?;
    }

    // Wrap the lifted problem in IRContent for output
    let content = IRContent::LiftedProblem(lifted_problem, format);
    let output = Artefact::new_ir(output_path.clone(), content);

    // Write the content to disk
    output.write()?;

    // Resolve the absolute path for display
    let absolute_output = output_path
        .canonicalize()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| output_path.display().to_string());

    println!(
        "{} Output file produced ({})",
        "===>".blue().bold(),
        absolute_output
    );

    Ok(())
}

/// Lit et valide le domain : doit être RawDomain ou ParsedDomain
/// Validates a domain file.
///
/// This function attempts to read the domain file from the given path and checks
/// if it is either a RawDomain or ParsedDomain. If the domain cannot be read or
/// is of an incompatible typing, a warning is printed and `None` is returned.
///
/// # Parameters
///
/// * `path` - The path to the domain file to validate.
///
/// # Returns
///
/// * `Some(Input)` - If the domain is successfully read and is of a valid typing.
/// * `None` - If the domain could not be read or is not a valid Raw/Parsed domain.
pub fn filter_domain(path: &PathBuf) -> Option<Source> {
    match Source::try_from_path(path) {
        // Domain successfully read and has a valid typing
        Ok(d) if d.is_raw() || d.is_parsed_domain() => Some(d),

        // Domain read but typing is invalid
        Ok(d) => {
            println!(
                "Warning: domain '{}' ignored: must be a RawDomain or ParsedDomain",
                d.path().display()
            );
            None
        }

        // Domain file could not be read
        Err(e) => {
            println!(
                "Warning: unable to read domain file '{}': {}",
                path.display(),
                e
            );
            None
        }
    }
}

/// Validates a list of problem files against the provided domain.
///
/// This function reads all problem files from the given paths, filters out
/// invalid or unreadable files, and then validates them according to the
/// typing of the domain (raw or parsed). Only problems that are compatible
/// with the domain typing and language/hierarchy are returned.
///
/// # Parameters
///
/// * `domain` - The reference to the domain input used for validation.
/// * `problem_paths` - A vector of paths to problem files to validate.
///
/// # Returns
///
/// * `Ok(Vec<Input>)` - A vector of valid `Input` problems compatible with the domain.
/// * `Err(CliError)` - If any I/O or validation error occurs during processing.
///
/// # Notes
///
/// Problems that cannot be read are ignored and a warning is printed.
pub fn filter_problems(
    domain: &Source,
    problem_paths: Vec<PathBuf>,
) -> Result<Vec<Source>, CliError> {
    // Read and collect all problems that can be successfully read from files
    let problems: Vec<Source> = problem_paths
        .into_iter()
        .filter_map(|p| match Source::try_from_path(&p) {
            Ok(p) => Some(p),
            Err(e) => {
                // Warn if a problem file cannot be read
                println!(
                    "Warning: unable to read problem file '{}', it will be ignored: {}",
                    p.display(),
                    e
                );
                None
            }
        })
        .collect();

    // Depending on the domain typing, filter the problems accordingly
    if domain.is_raw() {
        Ok(filter_raw_problems(domain, problems)?)
    } else if domain.is_parsed_domain() {
        Ok(filter_parsed_problems(domain, problems)?)
    } else {
        // Domain typing unrecognized, return an empty list
        Ok(vec![])
    }
}

/// Filters a list of problem inputs against a validated raw domain.
///
/// This function performs several checks to ensure that each problem is compatible
/// with the given domain:
///
/// 1. The input must be a problem (not a domain or unrelated file).
/// 2. The problem must be in raw format.
/// 3. The problem's language must match the language of the domain.
///
/// Any problem failing one of these checks is ignored, and a warning message
/// is printed to stdout. Valid problems are collected and returned.
///
/// # Parameters
///
/// * `domain` - A reference to a validated raw domain input. The domain's language
///   is used to filter compatible problems.
/// * `problems` - A vector of candidate problem inputs to be validated.
///
/// # Returns
///
/// * `Ok(Vec<Input>)` - A vector containing only the valid raw problems.
/// * `Err(CliError)` - If the domain's raw content cannot be accessed, this error
///   is returned.
///
/// # Warnings
///
/// Warnings are printed for each problem that is ignored, specifying the reason:
///
/// * Not a problem input.
/// * Not in raw format.
/// * Language mismatch with the domain.
///
/// # Examples
///
/// ```rust
/// # use aiplan4rust::io::Input;
/// # use aiplan4rust::cli::error::CliError;
/// # fn example(domain: &Input, problems: Vec<Input>) -> Result<(), CliError> {
/// let valid_problems = filter_raw_problems(domain, problems)?;
/// println!("{} valid problems found", valid_problems.len());
/// # Ok(())
/// # }
/// ```
///
/// # Notes
///
/// This function assumes that the domain input has already been validated as raw.
/// It does not perform any transformation on the problems; it only filters them
/// based on the criteria described above.
fn filter_raw_problems(domain: &Source, problems: Vec<Source>) -> Result<Vec<Source>, CliError> {
    // --- Extract the language of the domain ---
    // Fail early if the domain raw content is not accessible
    let domain_lang = domain.try_raw_content()?.language();

    // --- Prepare a vector to accumulate valid problems ---
    let mut valid_problems = Vec::new();

    // --- Iterate over each candidate problem ---
    for p in problems {
        // Step 1: Check if the input is a problem
        if p.is_problem() {
            // Step 2: Check if the problem is raw
            if p.is_raw() {
                // Step 3: Try extracting the raw content
                let c = p.try_raw_content()?;
                // Step 4: Compare problem language with domain language
                if c.language() == domain_lang {
                    // Problem is valid: add to the list
                    valid_problems.push(p);
                } else {
                    // Language mismatch
                    println!(
                        "Warning: problem '{}' ignored: language does not match domain",
                        p.path().display()
                    );
                }
            } else {
                // Problem is not raw
                println!(
                    "Warning: problem '{}' ignored: must be a raw problem",
                    p.path().display()
                );
            }
        } else {
            // Input is not a problem
            println!(
                "Warning: problem '{}' ignored: not a problem file",
                p.path().display()
            );
        }
    }

    // --- Return all valid raw problems ---
    Ok(valid_problems)
}

/// Filters and validates parsed (IR) problem inputs against a parsed domain.
///
/// This function checks that each problem:
/// - is a parsed problem input (not raw, not a domain),
/// - has a valid parsed semantic context,
/// - satisfies the domain requirements (e.g. hierarchy support).
///
/// # Behavior
///
/// - Incompatible or irrelevant inputs are **ignored with a warning** printed to stdout.
/// - Semantic or structural errors when extracting parsed content are treated as **hard errors**
///   and cause the function to return `Err`.
/// - The domain is assumed to be already validated as a parsed domain.
///
/// # Warnings
///
/// A problem input is ignored (with a warning) if:
/// - it is not a parsed problem,
/// - it is not hierarchical while the domain requires hierarchy.
///
/// # Errors
///
/// This function returns an error if:
/// - the domain parsed content cannot be extracted,
/// - a problem claims to be parsed but its semantic content is invalid.
///
/// # Returns
///
/// A vector containing only the parsed problem inputs that are compatible
/// with the given domain.
///
/// # Examples
///
/// ```no_run
/// let valid_problems = filter_parsed_problems(&domain, problems)?;
/// ```
///
/// # Design notes
///
/// This function follows a CLI-oriented design:
/// - **validation issues** are reported as warnings,
/// - **internal inconsistencies** are surfaced as errors,
/// - control flow remains explicit and readable (no `continue`, no deep nesting).
fn filter_parsed_problems(domain: &Source, problems: Vec<Source>) -> Result<Vec<Source>, CliError> {
    // Extract the parsed semantic context from the domain.
    // This must succeed; otherwise the domain is invalid.
    let domain_sc = domain.try_parsed_content()?;

    // Check whether the domain requires hierarchical constructs.
    let domain_requires_hierarchy = domain_sc.is_required(Requirement::Hierarchy);

    // Accumulate only the problems that are valid for this domain.
    let mut valid_problems = Vec::new();

    // Iterate over all candidate problem inputs.
    for p in problems {
        // The input must be a parsed problem (not a domain, not raw).
        if !p.is_parsed_problem() {
            println!(
                "Warning: problem IR '{}' ignored: expected a parsed problem",
                p.path().display()
            );
        } else {
            // Extract the parsed semantic context of the problem.
            // If this fails, it is considered a hard error.
            let problem_sc = p.try_parsed_content()?;

            // If the domain is hierarchical, the problem must be hierarchical as well.
            if domain_requires_hierarchy && !problem_sc.is_required(Requirement::Hierarchy) {
                println!(
                    "Warning: problem IR '{}' ignored: must be hierarchical because the domain is hierarchical",
                    p.path().display()
                );
            } else {
                // The problem is compatible with the domain and can be kept.
                valid_problems.push(p);
            }
        }
    }

    // Return the list of valid parsed problems.
    Ok(valid_problems)
}
