use std::fmt;
use std::str::FromStr;

const JSON: &str = "json";
const YAML: &str = "yaml";

/// Represents the possible file formats.
#[derive(Debug, Clone)]
pub enum FileFormat {
    /// Represents the JSON file format.
    Json,
    /// Represents the YAML file format.
    Yaml,
}

impl FromStr for FileFormat {
    type Err = String;

    /// Converts a string into a `FileFormat` variant.
    ///
    /// # Arguments
    ///
    /// * `s` - A string slice that represents the file format.
    ///
    /// # Returns
    ///
    /// This function returns `Ok(FileFormat::Json)` for the string "json",
    /// `Ok(FileFormat::Yaml)` for the string "yaml", and an error string
    /// if the input is invalid.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::str::FromStr;
    /// assert_eq!(FileFormat::from_str("json"), Ok(FileFormat::Json));
    /// assert_eq!(FileFormat::from_str("yaml"), Ok(FileFormat::Yaml));
    /// assert_eq!(FileFormat::from_str("txt"), Err("Invalid format: txt".to_string()));
    /// ```
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            JSON => Ok(FileFormat::Json),
            YAML => Ok(FileFormat::Yaml),
            _ => Err(format!("Invalid format: {}", s)),
        }
    }
}

impl FileFormat {
    /// Returns the file extension associated with the format.
    ///
    /// # Returns
    ///
    /// * "json" for `FileFormat::Json`.
    /// * "yaml" for `FileFormat::Yaml`.
    ///
    /// # Examples
    ///
    /// ```
    /// let json_format = FileFormat::Json;
    /// assert_eq!(json_format.extension(), "json");
    ///
    /// let yaml_format = FileFormat::Yaml;
    /// assert_eq!(yaml_format.extension(), "yaml");
    /// ```
    pub fn extension(&self) -> &'static str {
        match self {
            FileFormat::Json => JSON,
            FileFormat::Yaml => YAML,
        }
    }
}

impl fmt::Display for FileFormat {
    /// Formats the `FileFormat` as a string.
    ///
    /// # Arguments
    ///
    /// * `f` - The formatter to write the string to.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::fmt::Display;
    /// let json_format = FileFormat::Json;
    /// assert_eq!(format!("{}", json_format), "json");
    ///
    /// let yaml_format = FileFormat::Yaml;
    /// assert_eq!(format!("{}", yaml_format), "yaml");
    /// ```
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let format_str = match self {
            FileFormat::Json => JSON,
            FileFormat::Yaml => YAML,
        };
        write!(f, "{}", format_str)
    }
}
