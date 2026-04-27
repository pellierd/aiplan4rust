//! Module `lifted_syntax_display`
//!
//! Ce module définit le trait pour le rendu textuel (PDDL/HDDL) des éléments du LIR.
//! Il gère deux cas d'usage :
//! 1. Le rendu d'éléments dépendants d'un contexte externe (ex: Action, Expr).
//! 2. Le rendu d'éléments racines qui fournissent leur propre contexte (ex: DomainDef, Problem).

use crate::aiplan4rust::lir::store::renderers::RenderContext;
use std::fmt::{self, Write};

/// Trait unique pour le rendu syntaxique PDDL/HDDL.
pub trait LiftedSyntaxDisplay {
    // --- CAS 1 : RENDU AVEC CONTEXTE EXTERNE (Enfants) ---

    /// Formate l'élément en utilisant le [`RenderContext`] fourni.
    fn fmt_syntax(&self, f: &mut fmt::Formatter<'_>, ctx: &RenderContext) -> fmt::Result;

    /// Produit une String à partir d'un contexte donné.
    fn to_syntax_string_with_context(&self, ctx: &RenderContext) -> String
    where
        Self: Sized,
    {
        let mut s = String::new();
        let _ = write!(&mut s, "{}", self.as_syntax_with_context(ctx));
        s
    }

    /// Helper pour le formattage récursif : `write!(f, "{}", item.as_syntax(ctx))`
    fn as_syntax_with_context<'a>(
        &'a self,
        ctx: &'a RenderContext<'a>,
    ) -> LiftedSyntaxDisplayWrapper<'a, Self>
    where
        Self: Sized,
    {
        LiftedSyntaxDisplayWrapper { value: self, ctx }
    }

    // --- CAS 2 : RENDU AUTO-GÉRÉ (Racines) ---

    /// Formate l'élément en créant son propre contexte interne.
    /// Par défaut, renvoie une erreur si l'objet n'est pas une racine.
    fn fmt_syntax_self(&self, _f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Les types racines (DomainDef, Problem) doivent surcharger cette méthode.
        Err(fmt::Error)
    }

    /// Produit une String sans nécessiter de contexte externe.
    fn to_syntax_string(&self) -> String
    where
        Self: Sized,
    {
        let mut s = String::new();
        write!(&mut s, "{}", SelfSyntaxDisplayWrapper { value: self })
            .expect("Root syntax rendering failed: context could not be self-generated.");
        s
    }
}

// --- WRAPPERS DE FORMATEUR ---

/// Wrapper pour le rendu avec contexte externe.
pub struct LiftedSyntaxDisplayWrapper<'a, T: ?Sized> {
    pub value: &'a T,
    pub ctx: &'a RenderContext<'a>,
}

impl<'a, T: LiftedSyntaxDisplay + ?Sized> fmt::Display for LiftedSyntaxDisplayWrapper<'a, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.value.fmt_syntax(f, self.ctx)
    }
}

/// Wrapper pour le rendu auto-géré (Racine).
pub struct SelfSyntaxDisplayWrapper<'a, T: ?Sized> {
    pub value: &'a T,
}

impl<'a, T: LiftedSyntaxDisplay + ?Sized> fmt::Display for SelfSyntaxDisplayWrapper<'a, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.value.fmt_syntax_self(f)
    }
}
