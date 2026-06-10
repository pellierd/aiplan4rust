//! Module définissant la structure `Action`, représentant soit une action instantanée, soit durative.
//!
//! Cette structure unifiée simplifie le grounding tout en préservant la sémantique PDDL.
//! L'interface est conçue pour ressembler à une structure à plat pour la facilité d'usage.

use crate::aiplan4rust::compiler::lir::expr::ExprId;
use crate::aiplan4rust::compiler::lir::problem::skeleton::NamedTypedList;
use crate::aiplan4rust::compiler::lir::problem::SymbolRegistry;
use crate::aiplan4rust::compiler::lir::renderers;
use crate::aiplan4rust::compiler::lir::renderers::{
    LiftedDebugDisplay, LiftedSyntaxDisplay, RenderContext,
};
use crate::aiplan4rust::support::lang::TypedList;
use crate::aiplan4rust::support::lang::{ActionSymbolId, TypeId, VariableId};
use core::fmt::Formatter;
use serde::{Deserialize, Serialize};

/// Représente une action dans le LIR, qui peut être soit instantanée, soit durative.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Action {
    /// L'en-tête de l'action (symbole et paramètres).
    header: NamedTypedList<ActionSymbolId>,

    /// Le corps spécifique de l'action.
    body: ActionBody,

    variable_symbols: SymbolRegistry<VariableId>,
}

/// Énumération interne pour distinguer les types d'actions tout en gardant un LIR unifié.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "typing")]
pub enum ActionBody {
    /// Action instantanée standard.
    Snap {
        precondition: ExprId,
        effect: ExprId,
    },
    /// Action temporelle avec durée et conditions temporelles.
    Durative {
        duration: ExprId,
        condition: ExprId,
        effect: ExprId,
    },
}

impl Default for ActionBody {
    fn default() -> Self {
        Self::Snap {
            precondition: ExprId::default(),
            effect: ExprId::default(),
        }
    }
}

impl Default for Action {
    fn default() -> Self {
        Self {
            header: NamedTypedList::default(),
            body: ActionBody::default(),
            variable_symbols: SymbolRegistry::new(),
        }
    }
}

#[allow(dead_code)]
impl Action {
    /// Crée une nouvelle action instantanée.
    pub fn new_snap(
        name: ActionSymbolId,
        parameters: TypedList<VariableId, TypeId>,
        precondition: ExprId,
        effect: ExprId,
    ) -> Self {
        Self {
            header: NamedTypedList::new(name, parameters),
            body: ActionBody::Snap {
                precondition,
                effect,
            },
            variable_symbols: SymbolRegistry::new(),
        }
    }

    /// Crée une nouvelle action durative.
    pub fn new_durative(
        name: ActionSymbolId,
        parameters: TypedList<VariableId, TypeId>,
        duration: ExprId,
        condition: ExprId,
        effect: ExprId,
    ) -> Self {
        Self {
            header: NamedTypedList::new(name, parameters),
            body: ActionBody::Durative {
                duration,
                condition,
                effect,
            },
            variable_symbols: SymbolRegistry::new(),
        }
    }

    /// Permet d'ajouter les symboles après la création de manière élégante.
    /// Usage : Action::new_simple(...).with_symbols(ma_table)
    pub fn with_variable_symbols(mut self, symbols: SymbolRegistry<VariableId>) -> Self {
        self.variable_symbols = symbols;
        self
    }

    // --- Accesseurs Communs (Interface "à plat") ---

    /// Retourne la signature complète (nom + paramètres).
    pub fn signature(&self) -> &NamedTypedList<ActionSymbolId> {
        &self.header
    }

    pub fn name(&self) -> ActionSymbolId {
        self.header.symbol()
    }

    pub fn set_name(&mut self, name: ActionSymbolId) {
        self.header.set_symbol(name);
    }

    pub fn parameters(&self) -> &TypedList<VariableId, TypeId> {
        self.header.parameters()
    }

    pub fn parameters_mut(&mut self) -> &mut TypedList<VariableId, TypeId> {
        self.header.parameters_mut()
    }

    pub fn set_parameters(&mut self, parameters: TypedList<VariableId, TypeId>) {
        self.header.set_parameters(parameters);
    }

    pub fn is_durative(&self) -> bool {
        matches!(self.body, ActionBody::Durative { .. })
    }

    // --- Interface Unifiée pour le Grounding ---

    /// Retourne la condition logique de l'action.
    /// Renvoie 'precondition' pour les actions simples et 'condition' pour les duratives.
    pub fn precondition(&self) -> ExprId {
        match &self.body {
            ActionBody::Snap { precondition, .. } => *precondition,
            ActionBody::Durative { condition, .. } => *condition,
        }
    }

    pub fn set_precondition(&mut self, expr: ExprId) {
        match &mut self.body {
            ActionBody::Snap { precondition, .. } => *precondition = expr,
            ActionBody::Durative { condition, .. } => *condition = expr,
        }
    }

    /// Retourne l'expression de l'effet (commun aux deux types).
    pub fn effect(&self) -> ExprId {
        match &self.body {
            ActionBody::Snap { effect, .. } => *effect,
            ActionBody::Durative { effect, .. } => *effect,
        }
    }

    pub fn set_effect(&mut self, expr: ExprId) {
        match &mut self.body {
            ActionBody::Snap { effect, .. } => *effect = expr,
            ActionBody::Durative { effect, .. } => *effect = expr,
        }
    }

    /// Retourne la durée si l'action est durative, sinon None.
    pub fn duration(&self) -> Option<ExprId> {
        match &self.body {
            ActionBody::Durative { duration, .. } => Some(*duration),
            ActionBody::Snap { .. } => None,
        }
    }

    pub fn set_duration(&mut self, expr: ExprId) {
        match &mut self.body {
            ActionBody::Durative { duration, .. } => *duration = expr,
            ActionBody::Snap { .. } => {
                // Optionnel : on pourrait convertir l'action en durative ici,
                // mais pour l'instant on ignore ou on pourrait paniquer selon ta préférence.
            }
        }
    }

    /// Accès en lecture seule à la table des noms (symboles) des variables.
    /// À utiliser pour le rendu ou les messages d'erreur.
    pub fn variable_symbols(&self) -> &SymbolRegistry<VariableId> {
        &self.variable_symbols
    }

    /// Accès mutable à la table des noms des variables.
    pub fn variable_symbols_mut(&mut self) -> &mut SymbolRegistry<VariableId> {
        &mut self.variable_symbols
    }

    // --- Helpers Internes ---

    pub(crate) fn body(&self) -> &ActionBody {
        &self.body
    }
}

impl LiftedSyntaxDisplay for Action {
    /// Rendu PDDL propre (soit (:action ...) soit (:durative-action ...)).
    fn fmt_syntax(&self, f: &mut Formatter<'_>, ctx: &RenderContext) -> std::fmt::Result {
        renderers::syntax::action::render(f, self, ctx)
    }
}

impl LiftedDebugDisplay for Action {
    /// Rendu structurel pour le debug (Type d'action, Header ID, Body Expr Trees, Local Symbols).
    fn fmt_debug(&self, f: &mut Formatter<'_>, ctx: &RenderContext) -> std::fmt::Result {
        renderers::debug::action::render(f, self, ctx)
    }
}
