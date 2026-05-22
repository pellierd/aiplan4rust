use crate::aiplan4rust::lang::{TypeId, TypedSymbol};
use crate::aiplan4rust::lir::store::problem_old::DomainDef;
use crate::aiplan4rust::lir::store::renderers_old;
use crate::aiplan4rust::lir::store::renderers_old::syntax::{
    action, atomic_formula_skeleton, atomic_function_skeleton, derived_predicate, expr, method,
    task, typed_list,
};
use crate::aiplan4rust::lir::store::renderers_old::RenderContext;
use std::fmt::{self, Formatter};

/// Rendu complet d'une définition de domaine.
pub fn render(f: &mut Formatter<'_>, domain: &DomainDef<'_>, ctx: &RenderContext) -> fmt::Result {
    // 1. En-tête du domaine
    write!(
        f,
        "(define (domain {})",
        ctx.resolve_symbol(domain.domain_name())
    )?;

    // 2. Requirements
    let reqs = domain.requirements();
    if !reqs.is_empty() {
        write!(f, "\n  (:requirements")?;
        for req in reqs {
            write!(f, "\n    {}", req.to_string().to_lowercase())?; // ou via une fonction dédiée
        }
        write!(f, "\n  )")?;
    }

    // 3. Types
    if domain.has_type_defs() {
        write!(f, "\n  (:types\n    ")?;
        render_type_def(f, domain.type_defs(), ctx)?;
        write!(f, "\n  )")?;
    }

    // 4. Constants
    if domain.has_constant_defs() {
        write!(f, "\n  (:constants\n    ")?;
        typed_list::render_typed_object_list(f, domain.constant_defs(), ctx)?;
        write!(f, "\n  )")?;
    }

    // 5. Predicates
    if domain.has_predicate_defs() {
        write!(f, "\n  (:predicates")?;
        for pred in domain.predicate_defs() {
            write!(f, "\n    ")?;
            atomic_formula_skeleton::render(f, pred, ctx)?;
        }
        write!(f, "\n  )")?;
    }

    // 6. Functions
    if domain.has_function_defs() {
        write!(f, "\n  (:functions")?;
        for func in domain.functions_defs() {
            write!(f, "\n    ")?;
            atomic_function_skeleton::render(f, func, ctx)?;
        }
        write!(f, "\n  )")?;
    }

    // 7. Tasks (Spécifique HDDL)
    if domain.has_task_defs() {
        for task in domain.task_defs() {
            write!(f, "\n  ")?;
            task::render(f, task, ctx)?;
        }
    }

    // 11. Constraints (Domain level)
    if !domain.constraints().is_empty() {
        write!(f, "\n  (:constraints ")?;
        expr::render(f, domain.constraints(), ctx)?;
        write!(f, ")")?;
    }

    // 9. Derived Predicates
    for derived in domain.derived_predicates() {
        write!(f, "\n")?;
        derived_predicate::render(f, derived, ctx)?;
    }

    // 8. Actions & Durative Actions
    for action in domain.action_defs() {
        write!(f, "\n")?;
        action::render(f, action, ctx)?;
    }

    // 10. Methods (HDDL)
    for method in domain.method_defs() {
        write!(f, "\n")?;
        method::render(f, method, ctx)?;
    }

    // Fermeture finale du domaine
    write!(f, "\n)")?;

    Ok(())
}

fn render_type_def(
    f: &mut fmt::Formatter<'_>,
    types: &[TypedSymbol<TypeId, TypeId>],
    ctx: &RenderContext,
) -> fmt::Result {
    for (i, ty_symbol) in types.iter().enumerate() {
        // 1. Saut de ligne entre chaque déclaration (sauf avant la première)
        if i > 0 {
            write!(f, "\n    ")?;
        }

        // 2. Nom du typing actuel
        let name = ctx.resolve_type(ty_symbol.symbol());
        write!(f, "{}", name)?;

        // 3. Rendu du typing parent (ex: ' - vehicle')
        // Note: Assure-toi que render_type_list gère bien l'espace avant le '-'
        renderers_old::syntax::ty::render(f, ty_symbol.ty(), ctx)?;
    }
    Ok(())
}
