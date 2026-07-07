use crate::aiplan4rust::compiler::grounding::analysis::reachability::datalog::core::rule::Rule;
use crate::analysis::reachability::datalog::renderers::{atom, DatalogRenderContext};

/// Affiche l'ensemble des règles du programme
pub fn render(ctx: &DatalogRenderContext, rules: &[Rule]) {
    println!("\n>>> DOMAIN LOGIC COMPILATION (Datalog Rules)\n");

    if rules.is_empty() {
        println!("  (No rules compiled)");
    } else {
        for (i, rule) in rules.iter().enumerate() {
            // L'index est affiché ici pour numéroter les règles
            print!("  RULE #{:<3} ", i);
            render_rule(ctx, rule);
        }
    }

    println!(">>> END OF RULES\n");
}

pub fn render_rule(ctx: &DatalogRenderContext, rule: &Rule) {
    // 1. Formatage de la tête (Head) avec le contexte
    let interner = ctx.problem.interner();
    let head_str = atom::render(ctx, &rule.head());

    // 2. Affichage de la règle
    println!("{} :-", head_str);

    // 3. Formatage du corps (Body)
    let body_len = rule.body().len();
    for (i, atom) in rule.body().iter().enumerate() {
        let atom_str = atom::render(ctx, atom);

        // Style visuel en arborescence
        let connector = if i == body_len - 1 {
            "  └─ "
        } else {
            "  ├─ "
        };

        println!("{}{}", connector, atom_str);
    }
    println!(); // Espace entre chaque règle
}
