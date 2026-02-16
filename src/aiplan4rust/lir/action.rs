//! Module définissant la structure `Action`, représentant soit une action instantanée, soit durative.
//!
//! Cette structure unifiée simplifie le grounding tout en préservant la sémantique PDDL.
//! L'interface est conçue pour ressembler à une structure à plat pour la facilité d'usage.

use std::fmt;
use std::fmt::Formatter;
use crate::aiplan4rust::lang::{ActionSymbolID, TypeID, VariableID};
use crate::aiplan4rust::lang::TypedList;
use crate::aiplan4rust::lir::atomic_skeleton::NamedTypedList;
use crate::aiplan4rust::lir::expr::Expr;
use serde::{Deserialize, Serialize};
use crate::aiplan4rust::lir::renderers;
use crate::aiplan4rust::lir::renderers::{LiftedSyntaxDisplay, RenderContext};

/// Représente une action dans le LIR, qui peut être soit instantanée, soit durative.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Action {
    /// L'en-tête de l'action (symbole et paramètres).
    header: NamedTypedList<ActionSymbolID>,

    /// Le corps spécifique de l'action.
    body: ActionBody,
}

/// Énumération interne pour distinguer les types d'actions tout en gardant un LIR unifié.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ActionBody {
    /// Action instantanée standard.
    Snap {
        precondition: Expr,
        effect: Expr,
    },
    /// Action temporelle avec durée et conditions temporelles.
    Durative {
        duration: Expr,
        condition: Expr,
        effect: Expr,
    },
}

impl Default for ActionBody {
    fn default() -> Self {
        Self::Snap {
            precondition: Expr::default(),
            effect: Expr::default(),
        }
    }
}

impl Default for Action {
    fn default() -> Self {
        Self {
            header: NamedTypedList::default(),
            body: ActionBody::default(),
        }
    }
}

#[allow(dead_code)]
impl Action {
    /// Crée une nouvelle action instantanée.
    pub fn new_snap(
        name: ActionSymbolID,
        parameters: TypedList<VariableID, TypeID>,
        precondition: Expr,
        effect: Expr,
    ) -> Self {
        Self {
            header: NamedTypedList::new(name, parameters),
            body: ActionBody::Snap { precondition, effect },
        }
    }

    /// Crée une nouvelle action durative.
    pub fn new_durative(
        name: ActionSymbolID,
        parameters: TypedList<VariableID, TypeID>,
        duration: Expr,
        condition: Expr,
        effect: Expr,
    ) -> Self {
        Self {
            header: NamedTypedList::new(name, parameters),
            body: ActionBody::Durative { duration, condition, effect },
        }
    }

    // --- Accesseurs Communs (Interface "à plat") ---

    /// Retourne la signature complète (nom + paramètres).
    pub fn signature(&self) -> &NamedTypedList<ActionSymbolID> {
        &self.header
    }

    pub fn name(&self) -> ActionSymbolID {
        self.header.symbol()
    }

    pub fn set_name(&mut self, name: ActionSymbolID) {
        self.header.set_symbol(name);
    }

    pub fn parameters(&self) -> &TypedList<VariableID, TypeID> {
        self.header.parameters()
    }

    pub fn parameters_mut(&mut self) -> &mut TypedList<VariableID, TypeID> {
        self.header.parameters_mut()
    }

    pub fn set_parameters(&mut self, parameters: TypedList<VariableID, TypeID>) {
        self.header.set_parameters(parameters);
    }

    pub fn is_durative(&self) -> bool {
        matches!(self.body, ActionBody::Durative { .. })
    }

    // --- Interface Unifiée pour le Grounding ---

    /// Retourne la condition logique de l'action.
    /// Renvoie 'precondition' pour les actions simples et 'condition' pour les duratives.
    pub fn precondition(&self) -> &Expr {
        match &self.body {
            ActionBody::Snap { precondition, .. } => precondition,
            ActionBody::Durative { condition, .. } => condition,
        }
    }

    pub fn precondition_mut(&mut self) -> &mut Expr {
        match &mut self.body {
            ActionBody::Snap { precondition, .. } => precondition,
            ActionBody::Durative { condition, .. } => condition,
        }
    }

    pub fn set_precondition(&mut self, expr: Expr) {
        match &mut self.body {
            ActionBody::Snap { precondition, .. } => *precondition = expr,
            ActionBody::Durative { condition, .. } => *condition = expr,
        }
    }

    /// Retourne l'expression de l'effet (commun aux deux types).
    pub fn effect(&self) -> &Expr {
        match &self.body {
            ActionBody::Snap { effect, .. } => effect,
            ActionBody::Durative { effect, .. } => effect,
        }
    }

    pub fn effect_mut(&mut self) -> &mut Expr {
        match &mut self.body {
            ActionBody::Snap { effect, .. } => effect,
            ActionBody::Durative { effect, .. } => effect,
        }
    }

    pub fn set_effect(&mut self, expr: Expr) {
        match &mut self.body {
            ActionBody::Snap { effect, .. } => *effect = expr,
            ActionBody::Durative { effect, .. } => *effect = expr,
        }
    }

    /// Retourne la durée si l'action est durative, sinon None.
    pub fn duration(&self) -> Option<&Expr> {
        match &self.body {
            ActionBody::Durative { duration, .. } => Some(duration),
            ActionBody::Snap { .. } => None,
        }
    }

    pub fn duration_mut(&mut self) -> Option<&mut Expr> {
        match &mut self.body {
            ActionBody::Durative { duration, .. } => Some(duration),
            ActionBody::Snap { .. } => None,
        }
    }

    pub fn set_duration(&mut self, expr: Expr) {
        match &mut self.body {
            ActionBody::Durative { duration, .. } => *duration = expr,
            ActionBody::Snap { .. } => {
                // Optionnel : on pourrait convertir l'action en durative ici,
                // mais pour l'instant on ignore ou on pourrait paniquer selon ta préférence.
            }
        }
    }

    // --- Helpers Internes ---

    pub(crate) fn body(&self) -> &ActionBody {
        &self.body
    }
}

impl fmt::Display for Action {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        renderers::default::render_action(f, self)
    }
}

impl LiftedSyntaxDisplay for Action {
    fn fmt_syntax(&self, f: &mut Formatter<'_>, ctx: &RenderContext) -> fmt::Result {
        renderers::syntax::action::render(f, self, ctx)
    }
}
