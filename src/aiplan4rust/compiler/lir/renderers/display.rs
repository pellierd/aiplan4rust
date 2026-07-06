//! Context-aware rendering traits for Lifted Intermediate Representation (LIR).
//!
//! This module defines a specialized display system for PDDL and HDDL structures.
//! Unlike standard formatting, LIR components rely on a [`LirRenderContext`] to
//! resolve internal database identifiers into human-readable symbols.
//!
//! # Architecture
//!
//! The module is organized around two support traits:
//! * [`LiftedSyntaxDisplay`]: Focused on generating valid PDDL/HDDL source code.
//! * [`LiftedDebugDisplay`]: Focused on structural diagnostics and interning verification.
//!
//! # Implementation Strategy
//!
//! To keep the API surface clean, all intermediate formatting wrappers are kept
//! **private**. Users interact with the rendering engine solely through the public
//! trait methods, ensuring a seamless and robust abstraction.

use crate::aiplan4rust::compiler::lir::renderers::LirRenderContext;
use std::fmt::{self, Write};

// =============================================================================
// 1. LIFTED SYNTAX DISPLAY
// =============================================================================

/// Trait for generating standard-compliant PDDL/HDDL syntax.
///
/// This trait should be implemented by any LIR element that needs to be
/// exported to a planner or displayed as valid domain code.
pub trait LiftedSyntaxDisplay {
    /// The support formatting logic for PDDL/HDDL generation.
    ///
    /// ### Parameters
    /// - `f`: The standard format writer.
    /// - `ctx`: The [`LirRenderContext`] used to resolve IDs (Types, Predicates, etc.).
    ///
    /// ### Returns
    /// - `fmt::Result`: Success or a formatting error.
    fn fmt_syntax(&self, f: &mut fmt::Formatter<'_>, ctx: &LirRenderContext) -> fmt::Result;

    /// Generates a standalone [`String`] representation of the element.
    ///
    /// Useful for logging or simple string manipulation where a writer is not available.
    ///
    /// ### Parameters
    /// - `ctx`: The rendering context.
    ///
    /// ### Returns
    /// - `String`: The formatted PDDL/HDDL syntax.
    fn to_syntax_string(&self, ctx: &LirRenderContext) -> String
    where
        Self: Sized,
    {
        let mut s = String::new();
        let _ = write!(&mut s, "{}", self.as_syntax(ctx));
        s
    }

    /// Wraps the element to enable standard `Display` compatibility.
    ///
    /// This method is the primary way to render elements within `println!` or `write!`.
    ///
    /// ### Parameters
    /// - `ctx`: The rendering context.
    ///
    /// ### Returns
    /// - A private wrapper implementing [`fmt::Display`].
    fn as_syntax<'a>(&'a self, ctx: &'a LirRenderContext<'a>) -> impl fmt::Display + 'a
    where
        Self: Sized,
    {
        LiftedSyntaxDisplayWrapper { value: self, ctx }
    }

    /// Self-contained rendering for root elements (e.g., Domain, Problem).
    ///
    /// This method is used when the element possesses its own internal interner
    /// and does not require an external context.
    fn fmt_syntax_self(&self, _f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Err(fmt::Error)
    }
}

// =============================================================================
// 2. LIFTED DEBUG DISPLAY
// =============================================================================

/// Trait for technical and structural inspection.
///
/// Implementations of this trait provide a detailed view of the LIR, typically
/// including resolved names alongside their raw internal IDs (e.g., `[p#42]`).
pub trait LiftedDebugDisplay {
    /// The support formatting logic for structural debugging.
    ///
    /// ### Parameters
    /// - `f`: The standard format writer.
    /// - `ctx`: The [`LirRenderContext`] used for symbol resolution.
    fn fmt_debug(&self, f: &mut fmt::Formatter<'_>, ctx: &LirRenderContext) -> fmt::Result;

    /// Generates a detailed debug [`String`].
    ///
    /// ### Parameters
    /// - `ctx`: The rendering context.
    fn to_debug_string(&self, ctx: &LirRenderContext) -> String
    where
        Self: Sized,
    {
        let mut s = String::new();
        let _ = write!(&mut s, "{}", self.as_debug(ctx));
        s
    }

    /// Wraps the element to enable standard `Display` compatibility for debug views.
    ///
    /// ### Parameters
    /// - `ctx`: The rendering context.
    fn as_debug<'a>(&'a self, ctx: &'a LirRenderContext<'a>) -> impl fmt::Display + 'a
    where
        Self: Sized,
    {
        LiftedDebugDisplayWrapper { value: self, ctx }
    }

    /// Self-contained debug rendering for root elements.
    fn fmt_debug_self(&self, _f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Err(fmt::Error)
    }
}

// =============================================================================
// PRIVATE DISPLAY WRAPPERS
// =============================================================================

/// Private bridge between `LiftedSyntaxDisplay` and `fmt::Display`.
struct LiftedSyntaxDisplayWrapper<'a, T: ?Sized> {
    value: &'a T,
    ctx: &'a LirRenderContext<'a>,
}

impl<'a, T: LiftedSyntaxDisplay + ?Sized> fmt::Display for LiftedSyntaxDisplayWrapper<'a, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.value.fmt_syntax(f, self.ctx)
    }
}

/// Private bridge between `LiftedDebugDisplay` and `fmt::Display`.
struct LiftedDebugDisplayWrapper<'a, T: ?Sized> {
    value: &'a T,
    ctx: &'a LirRenderContext<'a>,
}

impl<'a, T: LiftedDebugDisplay + ?Sized> fmt::Display for LiftedDebugDisplayWrapper<'a, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.value.fmt_debug(f, self.ctx)
    }
}
