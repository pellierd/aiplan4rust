use crate::aiplan4rust::grounding::substitution::error::GroundingEngineError;
use crate::aiplan4rust::grounding::substitution::Substitution;
use crate::aiplan4rust::lir::expr::{Expr, ExprKind};
use crate::aiplan4rust::tree::NodeId;

pub trait Substitutable {
    /// Parcourt l'arbre à partir de `node_id` et remplace les variables par les objets
    /// fournis dans la substitution. Retourne l'ID du nouveau nœud (ou l'existant).
    fn substitute(&mut self, node_id: NodeId, sub: &Substitution) -> Result<(), GroundingEngineError>;
}

impl Substitutable for Expr {
    fn substitute(&mut self, root_id: NodeId, sub: &Substitution) -> Result<(), GroundingEngineError>{
        // On utilise un parcours post-order ou un simple stack
        let mut stack = vec![root_id];

        while let Some(current_id) = stack.pop() {
            let current_node = self.try_node(current_id)?;
            let kind = current_node.kind();

            match kind {
                // C'EST ICI : tu interceptes le nœud Variable
                ExprKind::Variable => {
                    // On récupère l'ID de la variable (stocké dans le nœud)
                    let var_node = self.try_node(current_id)?;
                    let var_id = var_node.try_variable()?;

                    // Si elle est dans notre dictionnaire de substitution
                    if let Some(obj_id) = sub.get(&var_id) {
                        // On transforme le nœud Variable en nœud Constant (ObjectID)
                        // Tu as probablement une méthode comme set_to_object ou replace_with_constant
                        self.set_to_object(current_id, obj_id)?;
                    }
                }

                // Pour tous les autres nœuds, on continue de descendre vers les feuilles
                _ => {
                    stack.extend(current_node.children());
                }
            }
        }
        Ok(())
    }
}
