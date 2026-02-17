//! Module handling the result of the semantic linking phase between domain and problem ASTs.
//!
//! This module defines the [`Result`] enum, which encapsulates the outcome
//! of the linking process that associates semantic information between a syntax domain
//! and problem representation.
//!
//! The linking process attempts to create a [`LinkedSemanticContext`] that combines
//! the domain and problem ASTs in a semantically consistent way. Alongside the linked context,
//! a [`DiagnosticManager`] collects any warnings, errors, or informational messages
//! generated during linking.
//!
//! # Typical Usage
//!
//! After performing linking, users receive a `LinkerResult` that:
//! - Contains a linked semantic context if linking succeeded.
//! - Provides access to diagnostics for troubleshooting or reporting.
//! - Allows access to the symbol interner regardless of linking success.

use crate::aiplan4rust::diagnostic::DiagnosticManager;
use crate::aiplan4rust::interner::SymbolInterner;
use crate::aiplan4rust::linking::LinkedSemanticContext;
use std::fmt;

/// Represents the result of the semantic linking process.
#[derive(Debug, Clone)]
pub enum Result {
    /// Linking succeeded with a linked context and diagnostics.
    Success {
        context: LinkedSemanticContext,
        diagnostic_manager: DiagnosticManager,
    },
    /// Linking failed; diagnostics and interner are preserved.
    Failure {
        diagnostic_manager: DiagnosticManager,
        interner: SymbolInterner,
    },
}

impl Result {
    /// Constructs a successful linking result.
    ///
    /// # Arguments
    /// * `context` – The successfully linked semantic context.
    /// * `diagnostic_manager` – Diagnostics collected during linking.
    ///
    /// # Returns
    /// A `LinkerResult::Success` variant.
    pub fn success(context: LinkedSemanticContext, diagnostic_manager: DiagnosticManager) -> Self {
        Self::Success {
            context,
            diagnostic_manager,
        }
    }

    /// Constructs a failed linking result.
    ///
    /// # Arguments
    /// * `diagnostic_manager` – Diagnostics explaining the failure.
    /// * `interner` – The interner used during linking.
    ///
    /// # Returns
    /// A `LinkerResult::Failure` variant.
    pub fn failure(diagnostic_manager: DiagnosticManager, interner: SymbolInterner) -> Self {
        Self::Failure {
            diagnostic_manager,
            interner,
        }
    }

    /// Returns `true` if linking succeeded.
    pub fn is_success(&self) -> bool {
        matches!(self, Self::Success { .. })
    }

    /// Returns `true` if linking failed.
    pub fn is_failure(&self) -> bool {
        matches!(self, Self::Failure { .. })
    }

    /// Returns a reference to the linked semantic context if available.
    pub fn linked_semantic_context(&self) -> Option<&LinkedSemanticContext> {
        match self {
            Self::Success { context, .. } => Some(context),
            Self::Failure { .. } => None,
        }
    }

    /// Returns a mutable reference to the linked semantic context if available.
    pub fn linked_semantic_context_mut(&mut self) -> Option<&mut LinkedSemanticContext> {
        match self {
            Self::Success { context, .. } => Some(context),
            Self::Failure { .. } => None,
        }
    }

    /// Takes ownership of the linked semantic context if available.
    pub fn take_linked_semantic_context(&mut self) -> Option<LinkedSemanticContext> {
        match self {
            Self::Success { context, .. } => Some(std::mem::take(context)),
            Self::Failure { .. } => None,
        }
    }

    /// Returns a reference to the diagnostic manager.
    pub fn diagnostic_manager(&self) -> &DiagnosticManager {
        match self {
            Self::Success {
                diagnostic_manager, ..
            } => diagnostic_manager,
            Self::Failure {
                diagnostic_manager, ..
            } => diagnostic_manager,
        }
    }

    /// Returns a mutable reference to the diagnostic manager.
    pub fn diagnostic_manager_mut(&mut self) -> &mut DiagnosticManager {
        match self {
            Self::Success {
                diagnostic_manager, ..
            } => diagnostic_manager,
            Self::Failure {
                diagnostic_manager, ..
            } => diagnostic_manager,
        }
    }

    /// Consumes and returns the diagnostic manager.
    pub fn take_diagnostic_manager(&mut self) -> DiagnosticManager {
        match self {
            Self::Success {
                diagnostic_manager, ..
            } => std::mem::take(diagnostic_manager),
            Self::Failure {
                diagnostic_manager, ..
            } => std::mem::take(diagnostic_manager),
        }
    }

    /// Returns a reference to the interner used during linking.
    ///
    /// For `Success`, the interner is retrieved from the context.
    /// For `Failure`, it is returned directly.
    pub fn interner(&self) -> &SymbolInterner {
        match self {
            Self::Success { context, .. } => context.interner(),
            Self::Failure { interner, .. } => interner,
        }
    }

    /// Consumes and returns the interner used during linking.
    ///
    /// For `Success`, takes it from the context.
    /// For `Failure`, takes it from the variant.
    pub fn take_interner(&mut self) -> SymbolInterner {
        match self {
            Self::Success { context, .. } => context.take_interner(),
            Self::Failure { interner, .. } => std::mem::take(interner),
        }
    }
}

impl fmt::Display for Result {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Success {
                context,
                diagnostic_manager,
            } => {
                writeln!(f, "Linking successful:\n{}", context)?;
                if !diagnostic_manager.is_empty() {
                    writeln!(f, "\nDiagnostics:")?;
                    for diag in diagnostic_manager.diagnostics() {
                        writeln!(f, "{}", diag)?;
                    }
                } else {
                    writeln!(f, "\nNo diagnostics reported.")?;
                }
            }
            Self::Failure {
                diagnostic_manager, ..
            } => {
                writeln!(f, "Linking failed.")?;
                for diag in diagnostic_manager.diagnostics() {
                    writeln!(f, "{}", diag)?;
                }
            }
        }
        Ok(())
    }
}
