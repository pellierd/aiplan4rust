use clap::{ArgMatches, Error};
use clap::error::ErrorKind;
use crate::aiplan4rust::cli::cli::{FILES_ARG, OUTPUT_ARG};

/// Validates the arguments for the `parse` subcommand.
///
/// Ensures that the `-o/--output` flag is only used when a single input file
/// is provided. If multiple input files are given, using `-o/--output` will
/// result in an error.
///
/// # Arguments
///
/// * `matches` - The parsed command-line arguments from `clap`.
///
/// # Errors
///
/// Returns a [`clap::Error`] with `ArgumentConflict` kind if `-o/--output`
/// is provided with more than one input file.
///
/// # Example
///
/// ```rust
/// use clap::ArgMatches;
/// use crate::aiplan4rust::cli::validate::check_parse_args;
///
/// # let matches: ArgMatches = todo!();
/// check_parse_args(&matches)?;
/// ```
pub fn check_parse_args(matches: &ArgMatches) -> Result<(), Error> {
    check_output_flag_with_exact_count(
        matches,
        FILES_ARG,
        OUTPUT_ARG,
        1, // parse: -o/--output only allowed for single file
        "`-o/--output` cannot be used with multiple input files; use `-d/--out-dir` instead",
    )
}

/// Validates the arguments for the `link` subcommand.
///
/// Ensures that the `-o/--output` flag is only used when exactly two input files
/// are provided (domain and problem). Using `-o/--output` with a different
/// number of input files will result in an error.
///
/// # Arguments
///
/// * `matches` - The parsed command-line arguments from `clap`.
///
/// # Errors
///
/// Returns a [`clap::Error`] with `ArgumentConflict` kind if `-o/--output`
/// is provided with a number of input files different from two.
///
/// # Example
///
/// ```rust
/// use clap::ArgMatches;
/// use crate::aiplan4rust::cli::validate::check_link_args;
///
/// # let matches: ArgMatches = todo!();
/// check_link_args(&matches)?;
/// ```
pub fn check_link_args(matches: &ArgMatches) -> Result<(), Error> {
    check_output_flag_with_exact_count(
        matches,
        FILES_ARG,
        OUTPUT_ARG,
        2, // link: -o/--output requires exactly two files
        "`-o/--output` requires exactly two input files (domain and problem)",
    )
}

/// Checks that a specific output argument is only used when the number of input files
/// matches `expected_count`.
///
/// # Arguments
/// * `matches` - The clap matches to inspect.
/// * `files_arg` - The name of the input files argument (e.g., `FILES_ARG`).
/// * `output_arg` - The name of the output argument (e.g., `OUTPUT_ARG`).
/// * `expected_count` - The number of input files required for the output argument.
/// * `message` - The error message if the number of files is invalid.
fn check_output_flag_with_exact_count(
    matches: &ArgMatches,
    files_arg: &str,
    output_arg: &str,
    expected_count: usize,
    message: &str,
) -> Result<(), Error> {
    let file_count = matches
        .get_many::<String>(files_arg)
        .map(|v| v.len())
        .unwrap_or(0);

    let output_provided = matches.contains_id(output_arg);

    if output_provided && file_count != expected_count {
        return Err(Error::raw(ErrorKind::ArgumentConflict, message));
    }

    Ok(())
}
