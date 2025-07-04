use serde::{Serialize, de::DeserializeOwned};
use crate::aiplan4rust::frontend::ParserInternalError;
use crate::aiplan4rust::serialization::format::Format;

pub trait Serializable: Serialize + DeserializeOwned {
    /// Sérialise l'objet en chaîne JSON ou YAML selon le format donné.
    fn serialize_to_string(&self, format: Format) -> Result<String, ParserInternalError> {
        match format {
            Format::Json => serde_json::to_string_pretty(self)
                .map_err(|e| ParserInternalError::new(format!("Erreur de sérialisation JSON : {}", e))),
            Format::Yaml => serde_yaml::to_string(self)
                .map_err(|e| ParserInternalError::new(format!("Erreur de sérialisation YAML : {}", e))),
        }
    }

    /// Sérialise l'objet dans un fichier JSON ou YAML selon le format donné.
    fn serialize_to_file(&self, format: Format, path: &str) -> Result<(), ParserInternalError> {
        let content = self.serialize_to_string(format)?;
        std::fs::write(path, content)
            .map_err(|e| ParserInternalError::new(format!("Erreur écriture fichier : {}", e)))
    }

    /// Désérialise un objet depuis une chaîne JSON ou YAML.
    fn deserialize_from_str(s: &str, format: Format) -> Result<Self, ParserInternalError> where Self: Sized {
        match format {
            Format::Json => serde_json::from_str(s)
                .map_err(|e| ParserInternalError::new(format!("Erreur désérialisation JSON : {}", e))),
            Format::Yaml => serde_yaml::from_str(s)
                .map_err(|e| ParserInternalError::new(format!("Erreur désérialisation YAML : {}", e))),
        }
    }

    /// Désérialise un objet depuis un fichier JSON ou YAML.
    fn deserialize_from_file(path: &str, format: Format) -> Result<Self, ParserInternalError> where Self: Sized {
        let content = std::fs::read_to_string(path)
            .map_err(|e| ParserInternalError::new(format!("Erreur lecture fichier : {}", e)))?;
        Self::deserialize_from_str(&content, format)
    }
}
