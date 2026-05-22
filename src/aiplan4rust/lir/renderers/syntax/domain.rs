use crate::aiplan4rust::lir::problem::DomainDef;
use crate::aiplan4rust::lir::renderers::syntax::{
    action, atom, derived_predicate, expr, function, method, task, typed_list,
};
use crate::aiplan4rust::lir::renderers::RenderContext;
use std::fmt::{self, Formatter};

/// Rendu complet d'une définition de domaine au format PDDL/HDDL (Syntaxe).
pub fn render(f: &mut Formatter<'_>, domain: &DomainDef<'_>, ctx: &RenderContext) -> fmt::Result {
    // 1. En-tête : (define (domain nom))
    write!(
        f,
        "(define (domain {})",
        ctx.resolve_symbol(domain.domain_name())
    )?;

    // 2. Requirements : (:requirements :strips :typing ...)
    let reqs = domain.requirements();
    if !reqs.is_empty() {
        write!(f, "\n  (:requirements")?;
        for req in reqs {
            // On s'assure du format ":keyword" en minuscules
            let req_str = req.to_string().to_lowercase();
            let clean_req = if req_str.starts_with(':') {
                req_str
            } else {
                format!(":{}", req_str)
            };
            write!(f, " {}", clean_req)?;
        }
        write!(f, ")")?;
    }

    // 3. Types : (:types type1 - parent type2 - parent)
    if domain.has_type_defs() {
        write!(f, "\n  (:types ")?;
        typed_list::render_typed_type_list(f, domain.type_defs().as_slice(), ctx)?;
        write!(f, ")")?;
    }

    // 4. Constants : (:constants c1 c2 - type)
    if domain.has_constant_defs() {
        write!(f, "\n  (:constants ")?;
        typed_list::render_typed_object_list(f, domain.constant_defs(), ctx)?;
        write!(f, ")")?;
    }

    // 5. Predicates
    if domain.has_predicate_defs() {
        write!(f, "\n  (:predicates")?;
        for pred in domain.predicate_defs() {
            write!(f, "\n    ")?;
            atom::render(f, pred, ctx)?;
        }
        write!(f, "\n  )")?;
    }

    // 6. Functions
    if domain.has_function_defs() {
        write!(f, "\n  (:functions")?;
        for func in domain.functions_defs() {
            write!(f, "\n    ")?;
            function::render(f, func, ctx)?;
        }
        write!(f, "\n  )")?;
    }

    // 7. Tasks (HDDL)
    if domain.has_task_defs() {
        for t in domain.task_defs() {
            write!(f, "\n  ")?;
            task::render(f, t, ctx)?;
        }
    }

    // 8. Constraints (Domain level)
    if !domain.constraints().is_some() {
        write!(f, "\n  (:constraints ")?;
        expr::render(f, domain.constraints(), ctx)?;
        write!(f, ")")?;
    }

    // 9. Derived Predicates
    for derived in domain.derived_predicates() {
        write!(f, "\n")?;
        derived_predicate::render(f, derived, ctx)?;
    }

    // 10. Actions & Durative Actions
    for a in domain.action_defs() {
        write!(f, "\n")?;
        action::render(f, a, ctx)?;
    }

    // 11. Methods (HDDL)
    for m in domain.method_defs() {
        write!(f, "\n")?;
        method::render(f, m, ctx)?;
    }

    // Fermeture finale du domaine
    write!(f, "\n)")
}
