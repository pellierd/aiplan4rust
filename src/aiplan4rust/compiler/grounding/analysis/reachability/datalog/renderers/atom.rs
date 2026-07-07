use crate::aiplan4rust::compiler::grounding::analysis::reachability::datalog::core::atom::Atom;
use crate::aiplan4rust::compiler::grounding::analysis::reachability::datalog::core::term::Term;
use crate::aiplan4rust::compiler::lir::problem::LiftedProblem;
use crate::aiplan4rust::support::lang::SymbolId;
use crate::analysis::reachability::datalog::renderers::DatalogRenderContext;

pub fn render(ctx: &DatalogRenderContext, atom: &Atom) -> String {
    let sk_id = atom.skeleton_id();
    let is_neg = atom.is_negated();

    // 1. On résout les arguments (?v0, objet#1)
    let terms_str = resolve_terms(ctx, atom.terms());

    // 2. Cas particulier de l'égalité
    if atom.is_equality() && terms_str.len() == 2 {
        let op = if is_neg { "!=" } else { "==" };
        return format!("{} {} {}", terms_str[0], op, terms_str[1]);
    }

    // 3. On résout le nom du prédicat avec son préfixe (p:not_at)
    let pred_name = resolve_predicate_name(ctx, sk_id.as_usize(), is_neg);

    // 4. Formatage final
    format!("{}({})", pred_name, terms_str.join(", "))
}

/// Transforme les termes (variables ou constantes) en chaînes lisibles.
fn resolve_terms(ctx: &DatalogRenderContext, terms: &[Term]) -> Vec<String> {
    let interner = ctx.problem.interner();
    terms
        .iter()
        .map(|t| match t {
            Term::Constant(id) => {
                if let Some(symbol_id) = ctx.problem.object_symbols().get_ident(*id) {
                    interner
                        .resolve_symbol(*symbol_id)
                        .map(|s| format!("{}#{}", s, id.as_usize()))
                        .unwrap_or_else(|| format!("o#{}", id.as_usize()))
                } else {
                    format!("o#{}", id.as_usize())
                }
            }
            Term::Variable(_) => format!("{}", t),
        })
        .collect()
}

/// Détermine le nom complet du prédicat en fonction des seuils du RenderContext.
/*fn resolve_predicate_name(ctx: &RenderContext, raw_index: usize, is_neg: bool) -> String {
    let neg_prefix = if is_neg { "not_" } else { "" };
    let interner = ctx.problem.interner();

    if raw_index < ctx.fluence_threshold {
        // Cas 1 : Fluents PDDL
        let name = ctx
            .problem
            .predicate_defs()
            .get(raw_index)
            .and_then(|def| ctx.problem.predicate_symbols().get_ident(def.symbol()))
            .and_then(|sym| interner.resolve_symbol(*sym))
            .map(|s| s.to_string())
            .unwrap_or_else(|| format!("{}", raw_index));
        format!("p:{}{}", neg_prefix, name)
    } else if raw_index < ctx.action_base_id {
        // Cas 2 : Types
        let type_name = resolve_type_label(ctx, raw_index);
        format!("t:{}{}", neg_prefix, type_name)
    } else if raw_index < ctx.action_threshold {
        // Cas 3 : Actions
        let action_idx = raw_index - ctx.action_base_id;
        let name = ctx
            .problem
            .action_defs()
            .get(action_idx)
            .and_then(|def| ctx.problem.action_symbols().get_ident(def.name()))
            .and_then(|sym| interner.resolve_symbol(*sym))
            .map(|s| s.to_string())
            .unwrap_or_else(|| format!("{}", action_idx));
        format!("@{}{}", neg_prefix, name)
    } else {
        // Cas 4 : Auxiliaires
        format!("{}aux_{}", neg_prefix, raw_index)
    }
}*/

/// Détermine le nom complet du prédicat en fonction des seuils du RenderContext.
fn resolve_predicate_name(ctx: &DatalogRenderContext, raw_index: usize, is_neg: bool) -> String {
    let neg_prefix = if is_neg { "not_" } else { "" };

    if raw_index < ctx.fluence_threshold {
        let sym_id = ctx
            .problem
            .predicate_defs()
            .get(raw_index)
            .and_then(|def| ctx.problem.predicate_symbols().get_ident(def.symbol()));

        // Utilise .map(|id| (*id).into()) pour convertir en SymbolId proprement
        let name = resolve_name(
            sym_id.map(|id| (*id).into()),
            ctx.problem,
            &format!("{}", raw_index),
        );
        format!("p:{}{}", neg_prefix, name)
    } else if raw_index < ctx.action_threshold {
        let action_idx = raw_index - ctx.action_base_id;
        let sym_id = ctx
            .problem
            .action_defs()
            .get(action_idx)
            .and_then(|def| ctx.problem.action_symbols().get_ident(def.name()));

        // Même chose ici : conversion Safe via into()
        let name = resolve_name(
            sym_id.map(|id| (*id).into()),
            ctx.problem,
            &format!("{}", action_idx),
        );
        format!("@{}{}", neg_prefix, name)
    } else {
        format!("{}aux_{}", neg_prefix, raw_index)
    }
}

/// Helper spécifique pour extraire le nom d'un type.
fn resolve_type_label(ctx: &DatalogRenderContext, raw_index: usize) -> String {
    ctx.type_to_skeleton
        .iter()
        .position(|&id| id.as_usize() == raw_index)
        .map(|type_idx| {
            if type_idx == ctx.problem.type_defs().len() {
                "ROOT_TYPE".to_string()
            } else {
                let type_def = &ctx.problem.type_defs()[type_idx];

                let sym_id = ctx.problem.type_symbols().get_ident(type_def.symbol());

                // On déréférence id avec * pour passer de &TypeSymbolId à TypeSymbolId
                // avant le .into()
                resolve_name(
                    sym_id.map(|id| (*id).into()),
                    ctx.problem,
                    &format!("{}", type_idx),
                )
            }
        })
        .unwrap_or_else(|| format!("unk_{}", raw_index))
}
pub fn resolve_name(
    symbol_id: Option<SymbolId>, // On passe par valeur, pas par référence
    lifted_problem: &LiftedProblem,
    fallback: &str,
) -> String {
    symbol_id
        .and_then(|sid| lifted_problem.interner().resolve_symbol(sid))
        .map(|s| s.to_string())
        .unwrap_or_else(|| fallback.to_string())
}
