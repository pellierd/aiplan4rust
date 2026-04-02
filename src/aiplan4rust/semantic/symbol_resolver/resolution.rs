use crate::aiplan4rust::arena::NodeId;
use crate::aiplan4rust::semantic::signature_matcher::result::MatchResult;
use serde::{Deserialize, Serialize};
use std::fmt;

/// La structure produite par le SymbolResolver.
/// Elle fait le pont entre la déclaration trouvée et le résultat du matching.
#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct Resolution {
    declaration: NodeId,
    status: MatchResult,
}

impl Resolution {
    /// Crée une nouvelle résolution liant une déclaration à son résultat de matching.
    pub fn new(declaration: NodeId, status: MatchResult) -> Self {
        Self {
            declaration,
            status,
        }
    }

    /// Retourne l'identifiant de la déclaration vers laquelle le symbole pointe.
    pub fn declaration(&self) -> NodeId {
        self.declaration
    }

    /// Retourne le détail du résultat du matching (Match, Upcast, NoMatch).
    pub fn status(&self) -> &MatchResult {
        &self.status
    }

    /// Indique si la résolution est un succès sémantique.
    pub fn is_resolved(&self) -> bool {
        !matches!(self.status, MatchResult::NoMatch(_))
    }
}

impl fmt::Display for Resolution {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Resolution(declaration: {}, status: {})",
            self.declaration, self.status
        )
    }
}
