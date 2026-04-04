/*/// La structure produite par le SymbolResolver.
/// Elle fait le pont entre la déclaration trouvée et le résultat du matching.
#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct ResolutionC {
    declaration: NodeId,
    status: MatchResult,
}

impl ResolutionC {
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

impl fmt::Display for ResolutionC {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Resolution(declaration: {}, status: {})",
            self.declaration, self.status
        )
    }
}*/
