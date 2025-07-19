
use crate::aiplan4rust::AiplanError;
use crate::aiplan4rust::interner::StringInterner;
use crate::aiplan4rust::serialization::planning::PlanningFormat;
use crate::aiplan4rust::syntax::SyntaxDisplay;

/// Trait for serializing and deserializing syntax structures that require an [`Interner`].
///
/// This trait allows serialization to and deserialization from multiple Serde-supported formats,
/// with support for providing an `Interner` as contextual information.
///
/// Supported formats:
/// - JSON
/// - YAML
/// - TOML
/// - CBOR
/// - MessagePack
///
/// Errors are returned as [`AiplanError`].
pub trait Serializable: SyntaxDisplay {
    /// Serializes the object into a string in the specified format.
    ///
    /// # Arguments
    ///
    /// * `interner` - The interner to use for symbol resolution during serialization.
    /// * `format` - The desired output format.
    ///
    /// # Returns
    ///
    /// A `String` representing the serialized object, or an error if serialization fails.
    fn serialize_to_string(
        &self,
        interner: &StringInterner,
    ) -> Result<String, AiplanError>;

    /// Serializes the object and writes it to a file in the specified format.
    ///
    /// # Arguments
    ///
    /// * `interner` - The interner to use for symbol resolution.
    /// * `format` - The desired output format.
    /// * `path` - The output file path.
    ///
    /// # Returns
    ///
    /// `Ok(())` on success, or an error if serialization or writing fails.
    fn serialize_to_file(
        &self,
        interner: &StringInterner,
        path: &str,
    ) -> Result<(), AiplanError>;

    /// Deserializes an object from a string in the specified format.
    ///
    /// # Arguments
    ///
    /// * `s` - String slice containing the serialized data.
    /// * `interner` - The interner to use for resolving symbols.
    /// * `format` - The format of the serialized data.
    ///
    /// # Returns
    ///
    /// The deserialized object, or an error if deserialization fails.
    fn deserialize_from_str(
        s: &str,
        interner: &mut StringInterner,
    ) -> Result<Self, AiplanError>
    where
        Self: Sized;

    /// Deserializes an object from a file in the specified format.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the input file.
    /// * `interner` - The interner to use for resolving symbols.
    /// * `format` - The format of the serialized data.
    ///
    /// # Returns
    ///
    /// The deserialized object, or an error if deserialization fails.
    fn deserialize_from_file(
        path: &str,
        interner: &mut StringInterner,
    ) -> Result<Self, AiplanError>
    where
        Self: Sized;

    /// Infers the serialization format from the file path extension.
    ///
    /// # Arguments
    ///
    /// * `path` - The file path.
    ///
    /// # Returns
    ///
    /// The corresponding `Format`, or an error if the extension is missing or unsupported.
    fn format_from_path(path: &str) -> Result<PlanningFormat, AiplanError> {
        let ext = std::path::Path::new(path)
            .extension()
            .and_then(|e| e.to_str())
            .ok_or_else(|| AiplanError::new("File has no extension".to_string()))?;

        ext.parse::<PlanningFormat>()
    }
}
