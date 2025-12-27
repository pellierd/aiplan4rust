use std::fs;
use std::path::{Path, PathBuf};
use clap::ArgMatches;
use colored::Colorize;
use crate::aiplan4rust::serialization::serde::SerdeFormat;
use crate::{Frontend, Renderer, Severity};
use crate::aiplan4rust::cli::cli::{FILES_ARG, FORMAT_ARG, OUTPUT_ARG};
use crate::aiplan4rust::cli::error::CliError;
use crate::aiplan4rust::io::{Extension, IRContent, Output};
use crate::aiplan4rust::io::error::IOError;
use crate::aiplan4rust::io::input::Input;
use crate::aiplan4rust::lir::problem::LiftedProblem;

/// Handles the `link` CLI subcommand.
///
/// This function orchestrates the workflow for linking a single domain file
/// with one or more problem files. It determines whether the domain and problems
/// are in IR or Raw format, reads the inputs, computes output paths, and delegates
/// the linking and statistics accumulation to `link_inputs`.
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
    // Collect files from CLI arguments
    let files: Vec<String> = matches
        .get_many::<String>(FILES_ARG)
        .ok_or_else(CliError::missing_input_files)?
        .cloned()
        .collect();

    // Determine output format (default: JSON)
    let format = *matches
        .get_one::<SerdeFormat>(FORMAT_ARG)
        .ok_or_else(|| CliError::invalid_argument("No output format provided"))?;

    // Optional output path (used only if a single problem file)
    let output_opt = matches.get_one::<String>(OUTPUT_ARG).map(PathBuf::from);

    // Separate domain and problem files
    let domain_file = PathBuf::from(&files[0]);
    let problem_files: Vec<PathBuf> = files[1..].iter().map(PathBuf::from).collect();

    // Delegate the linking workflow without passing matches
    link_inputs(&domain_file, &problem_files, format, output_opt)?;

    Ok(())
}

/// Links a single domain file with multiple problem files.
///
/// This function performs two passes over the problem files:
/// 1. **Filter and read valid problem inputs** according to the domain type (IR or Raw),
///    computing the output paths.
/// 2. **Link the domain with each valid problem**, producing lifted output files
///    and accumulating statistics.
///
/// # Behavior
///
/// - The first file is treated as the **domain**; the rest are **problem files**.
/// - The domain file can be either IR or Raw. All problem files must match the domain type.
/// - Each problem is linked individually with the domain, producing separate output files.
/// - If a single problem file is provided and an explicit output path is given, it is used.
/// - Prints a summary including the number of linked and ignored problems, and the total elapsed time.
///
/// # Arguments
///
/// * `domain_file` - Path to the domain file (IR or Raw).
/// * `problem_files` - Slice of paths to problem files (must match domain type).
/// * `format` - Serialization format for output files (e.g., JSON, YAML).
/// * `output_opt` - Optional explicit output path, used only if a single problem file.
///
/// # Returns
///
/// Returns `Ok(())` if all linking succeeds, or `Err(CliError)` if:
/// - The domain file is invalid,
/// - Problem files are inconsistent with the domain type,
/// - Any reading or linking operation fails.
///
/// # Example
///
/// ```rust
/// let domain = Path::new("domain.pddl");
/// let problems = vec![PathBuf::from("problem1.pddl"), PathBuf::from("problem2.pddl")];
/// link_inputs(domain, &problems, SerdeFormat::Json, None)?;
/// ```
fn link_inputs(
    domain_file: &Path,
    problem_files: &[PathBuf],
    format: SerdeFormat,
    output_opt: Option<PathBuf>,
) -> Result<(), CliError> {
    use std::time::Instant;

    // Read domain and determine type (IR or Raw)
    let domain = read_domain_input(domain_file)?;
    let expect_ir = domain.is_ir();
    let expect_raw = domain.is_raw();

    // Initialize statistics
    let mut files_linked = 0usize;
    let mut files_ignored = 0usize;
    let start_time = Instant::now();

    // First pass: read problems and filter by type
    let mut valid_problems: Vec<(Input, PathBuf)> = Vec::new();
    for problem_file in problem_files {
        if let Some(problem) = read_input_problem(problem_file, expect_ir, expect_raw)? {
            let output_path = if problem_files.len() == 1 {
                output_opt.clone().unwrap_or_else(|| {
                    Output::default_output_path(domain_file, Some(problem_file), Extension::Lifted, None)
                        .expect("failed to determine default output path")
                })
            } else {
                Output::default_output_path(domain_file, Some(problem_file), Extension::Lifted, None)?
            };
            valid_problems.push((problem, output_path));
        } else {
            files_ignored += 1;
        }
    }

    // Second pass: link domain with each valid problem
    for (problem, output_path) in valid_problems {
        if expect_ir {
            link_from_parsed_input(&domain, &problem, format, &output_path)?;
        } else {
            link_from_raw_input(&domain, &problem, format, &output_path)?;
        }
        files_linked += 1;
    }

    // Print global summary if multiple problems were processed
    if problem_files.len() > 1 {
        let total_time = start_time.elapsed().as_secs_f32();
        println!(
            "\n{} {} problem(s) linked successfully, {} problem(s) ignored in {:.2}s",
            "Finished".green().bold(),
            files_linked,
            files_ignored,
            total_time
        );
    }

    Ok(())
}



/// Reads a domain file and ensures it is either IR or Raw.
///
/// # Arguments
/// * `domain_path` - Path to the domain file.
///
/// # Returns
/// * `Ok(Input)` if the domain file is valid (IR or Raw)
/// * `Err(CliError)` if the file is not a regular file or not IR/Raw
fn read_domain_input(domain_path: &Path) -> Result<Input, CliError> {
    if !domain_path.is_file() {
        return Err(CliError::invalid_argument(&format!(
            "'{}' is not a file",
            domain_path.display()
        )));
    }

    let domain = Input::read_from_file(domain_path)?;

    if !domain.is_ir() && !domain.is_raw() {
        return Err(CliError::invalid_argument(
            "Domain file must be IR or Raw",
        ));
    }

    Ok(domain)
}

fn read_input_problem(input_path: &Path, expect_ir: bool, expect_raw: bool) -> Result<Option<Input>, CliError> {
    // 1. Check if path is a file
    if !input_path.is_file() {
        println!(
            "{} '{}' is not a file — ignored",
            "warning:".yellow().bold(),
            input_path.display()
        );
        return Ok(None);
    }

    // 2. Read the input
    let input = Input::read_from_file(input_path)?;

    // 3. Check type consistency
    if (expect_ir && !input.is_ir()) || (expect_raw && !input.is_raw()) {
        let kind = if input.is_ir() {
            "IR file"
        } else if input.is_raw() {
            "Raw file"
        } else if input.is_text() {
            "unknown text file"
        } else if input.is_binary() {
            "binary file"
        } else {
            "unknown content"
        };

        println!(
            "{} '{}' type is inconsistent with expected type — ignored ({})",
            "warning:".yellow().bold(),
            input_path.display(),
            kind
        );
        return Ok(None);
    }

    Ok(Some(input))
}

fn link_from_parsed_input(
    domain: &Input,
    problem: &Input,
    format: SerdeFormat,
    output: &PathBuf,
) -> Result<(), CliError> {
    // Create a frontend instance
    let frontend = Frontend::new();

    // Perform linking, propagate any errors
    let mut builder_result = frontend.link_from_parsed_input(domain, problem)?;

    // If linking produced a semantic context, serialize it
    if let Some(lifted_problem) = builder_result.take_lifted_problem() {
        save_link_output(lifted_problem, format, output)?;
        println!("Output saved to {}", output.to_string_lossy());
    } else {
        // Otherwise, render diagnostics
        let mut renderer = Renderer::new(
            builder_result.diagnostic_manager(),
            builder_result.interner(),
        );
        renderer.display()?;
    }

    Ok(())
}

pub fn link_from_raw_input(
    domain: &Input,
    problem: &Input,
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
    let mut result = frontend.link_from_raw_input(domain, problem)?; // AiplanError se convertit en CliError

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
        println!(
            "{} Output saved to {}",
            "===>".blue().bold(),
            output.to_string_lossy()
        );
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
/// - `P`: Any type that can be converted into a `PathBuf` (e.g., `&str`, `String`, `PathBuf`).
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
) -> Result<(), IOError> {
    let output_path = output_path.into();

    // Create all parent directories if they do not exist
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)?;
    }

    // Wrap the lifted problem in IRContent for output
    let content = IRContent::LiftedProblem(lifted_problem, format);
    let output = Output::new_ir(output_path.clone(), content);

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

/*pub fn validate_domain_and_problems(
    domain_path: &PathBuf,
    problem_paths: &[PathBuf],
) -> Result<(Input, Vec<Input>), CliError> {
    let domain_input = Input::read_from_file(domain_path)?;

    // Détecte le type global et la langue du domain
    let (domain_type_is_raw, domain_lang) = if domain_input.is_raw_domain() {
        (true, domain_input.raw_content().unwrap().language())
    } else if domain_input.is_parsed_domain() {
        (false, domain_input.try_ir_content().language())
    } else {
        return Err(CliError::invalid_argument(format!(
            "Domain file '{}' must be a raw domain or parsed IR domain",
            domain_path.display()
        )));
    };

    let mut problem_inputs = Vec::with_capacity(problem_paths.len());

    for p_path in problem_paths {
        let p_input = Input::read_from_file(p_path)?;

        // Vérifie le type global et le rôle
        let compatible = if domain_type_is_raw {
            p_input.is_raw_problem() && p_input.try_raw_content()?.language() == domain_lang
        } else {
            p_input.is_parsed_problem() && p_input.try_ir_content()?.language() == domain_lang
        };

        if !compatible {
            return Err(CliError::invalid_argument(format!(
                "Problem file '{}' is incompatible with domain '{}'",
                p_path.display(), domain_path.display()
            )));
        }

        problem_inputs.push(p_input);
    }

    Ok((domain_input, problem_inputs))
}*/
