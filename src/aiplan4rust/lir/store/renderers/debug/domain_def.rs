use crate::aiplan4rust::lir::store::expr::ExprId;
use crate::aiplan4rust::lir::store::problem::DomainDef;
use crate::aiplan4rust::lir::store::renderers::debug::common::writeln_centered;
use crate::aiplan4rust::lir::store::renderers::debug::{
    action, atom, expr, function, method, typed_list,
};
use crate::aiplan4rust::lir::store::renderers::RenderContext;
use std::fmt;

/// Renders a human-readable representation of a `DomainDef` into a formatter.
pub fn render(f: &mut fmt::Formatter<'_>, domain: &DomainDef, ctx: &RenderContext) -> fmt::Result {
    // === HEADER ===
    writeln_centered(f, " DOMAIN DEFINITION ", 80, '=')?;
    writeln!(
        f,
        "  DOMAIN NAME  : {}\n",
        ctx.resolve_symbol(domain.domain_name())
    )?;

    // === REQUIREMENTS ===
    writeln_centered(f, " REQUIREMENTS ", 80, '-')?;
    if domain.requirements().is_empty() {
        writeln!(f, "  <None>")?;
    } else {
        let mut reqs: Vec<_> = domain.requirements().iter().collect();
        reqs.sort();
        for r in reqs {
            writeln!(f, "  - {:?}", r)?;
        }
    }
    writeln!(f)?;

    // === TYPES ===
    writeln_centered(f, " TYPES HIERARCHY ", 80, '-')?;
    if domain.type_defs().is_empty() {
        writeln!(f, "  <None>")?;
    } else {
        write!(f, "  ")?;
        typed_list::render_type_typed_list(f, domain.type_defs().as_slice(), ctx)?;
        writeln!(f)?;
    }

    // === CONSTANTS (Utilise la slice de TypedList) ===
    writeln_centered(f, " DOMAIN CONSTANTS ", 80, '-')?;
    let constants = domain.constant_defs(); // Retourne &[TypedSymbol]
    if constants.is_empty() {
        writeln!(f, "  <None>")?;
    } else {
        typed_list::render_object_typed_list(f, constants, ctx)?;
    }
    writeln!(f)?;

    // === PREDICATES ===
    writeln_centered(f, " PREDICATES ", 80, '-')?;
    if domain.predicate_defs().is_empty() {
        writeln!(f, "  <None>")?;
    } else {
        for p in domain.predicate_defs() {
            write!(f, "  ")?;
            // Signature : (nom ?arg1 - type)
            atom::render(f, p, ctx)?;
            writeln!(f)?;
        }
    }
    writeln!(f)?;

    // === FUNCTIONS ===
    writeln_centered(f, " FUNCTIONS (FLUENTS) ", 80, '-')?;
    if domain.functions_defs().is_empty() {
        writeln!(f, "  <None>")?;
    } else {
        for func in domain.functions_defs() {
            write!(f, "  ")?;
            function::render(f, func, ctx)?;
            writeln!(f)?;
        }
    }
    writeln!(f)?;

    // === DOMAIN CONSTRAINTS ===
    writeln_centered(f, " DOMAIN CONSTRAINTS ", 80, '-')?;
    let dc = domain.constraints();
    if dc == ExprId::EMPTY_AND {
        writeln!(f, "  <None/Always Satisfied>\n")?;
    } else {
        expr::render(f, dc, ctx)?;
        writeln!(f)?;
    }

    // === ACTIONS ===
    writeln_centered(f, " ACTIONS ", 80, '-')?;
    if domain.action_defs().is_empty() {
        writeln!(f, "  <None>")?;
    } else {
        for action in domain.action_defs() {
            action::render(f, action, ctx)?;
            writeln!(f)?;
        }
    }

    // === METHODS (HDDL) ===
    if !domain.method_defs().is_empty() {
        writeln_centered(f, " METHODS (HDDL) ", 80, '-')?;
        for method in domain.method_defs() {
            method::render(f, method, ctx)?;
            writeln!(f)?;
        }
    }

    writeln!(f, "\n{}", "=".repeat(80))
}
