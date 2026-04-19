use crate::aiplan4rust::grounding::analysis::reachability::datalog::database::Database;
use crate::aiplan4rust::interner::SymbolInterner;
use crate::aiplan4rust::lang::AtomSkeletonId;
use crate::analysis::reachability::datalog::relation::Relation;
use crate::analysis::reachability::datalog::renderers::context::RenderContext;

/// Affiche l'intégralité de la base de données (Stable et Delta) au format SQL-Style.
pub fn render(ctx: &RenderContext, db: &Database) {
    let interner = ctx.problem.interner();
    let main_separator = "=".repeat(100);

    println!(
        "\n{}\n{:^100}\n{}",
        main_separator, "📊 DATALOG DATABASE STATE (SQL VIEW)", main_separator
    );

    // On utilise les méthodes publiques de Database (stable_relations / delta_relations)
    let sources = [
        ("STABLE STORAGE (Confirmed)", db.stable_relations()),
        ("DELTA BUFFER (Pending)", db.delta_relations()),
    ];

    for (source_name, storage_map) in sources {
        if storage_map.is_empty() {
            continue;
        }
        println!("\n>>> {}", source_name);

        let mut sorted_rels: Vec<_> = storage_map.iter().collect();
        sorted_rels.sort_by_key(|(id, _)| id.as_usize());

        for (sk_id, relation) in sorted_rels {
            if relation.is_empty() {
                continue;
            }
            render_table(ctx, *sk_id, relation, interner);
        }
    }
}

fn render_table(
    ctx: &RenderContext,
    sk_id: AtomSkeletonId,
    relation: &Relation,
    interner: &SymbolInterner,
) {
    let (category, label) = ctx.identify_segment(sk_id);
    let arity = relation.arity();
    let label_upper = label.to_uppercase();

    // --- 1. RÉCUPÉRATION DES NOMS DE COLONNES SÉMANTIQUES ---
    let mut column_names = Vec::new();

    match category {
        "TYPE" => {
            column_names.push("INSTANCE".to_string());
        }
        "FLUENT" => {
            if let Some(pred_def) = ctx.problem.predicate_defs().iter().find(|p| {
                ctx.problem
                    .predicate_symbols()
                    .get_ident(p.symbol())
                    .and_then(|&s| interner.resolve_symbol(s))
                    == Some(&label)
            }) {
                for param in pred_def.parameters() {
                    let type_name = ctx
                        .problem
                        .type_symbols()
                        .get_ident(param.ty().members()[0])
                        .and_then(|&s| interner.resolve_symbol(s))
                        .unwrap_or("any");
                    column_names.push(type_name.to_uppercase());
                }
            }
        }
        "ACTION" => {
            if let Some(act_def) = ctx.problem.action_defs().iter().find(|a| {
                ctx.problem
                    .action_symbols()
                    .get_ident(a.signature().symbol())
                    .and_then(|&s| interner.resolve_symbol(s))
                    == Some(&label)
            }) {
                for param in act_def.parameters() {
                    let type_name = ctx
                        .problem
                        .type_symbols()
                        .get_ident(param.ty().members()[0])
                        .and_then(|&s| interner.resolve_symbol(s))
                        .unwrap_or("any");
                    column_names.push(type_name.to_uppercase());
                }
            }
        }
        _ => {}
    }

    while column_names.len() < arity {
        column_names.push(format!("col_{}", column_names.len()));
    }

    // --- 2. CALCUL DE LA LARGEUR DYNAMIQUE ---
    let mut max_content_len = 10;
    for name in &column_names {
        max_content_len = max_content_len.max(name.chars().count());
    }

    for fact in relation.iter().take(20) {
        for &obj_id in fact.iter() {
            let name_len = ctx
                .problem
                .object_symbols()
                .get_ident(obj_id)
                .and_then(|&s| interner.resolve_symbol(s))
                .map(|s| s.chars().count())
                .unwrap_or(5);
            max_content_len = max_content_len.max(name_len);
        }
    }

    let col_width_base = max_content_len + 2;
    let min_data_width = if arity == 0 {
        25
    } else {
        (col_width_base * arity) + (arity - 1)
    };

    let title_content = format!(
        " TABLE: {} [{}] [ID:{}] [{} rows] ",
        label_upper,
        category,
        sk_id.as_usize(),
        relation.len()
    );
    let title_len = title_content.chars().count();

    let inner_width = std::cmp::max(min_data_width, title_len);

    // --- MODIFICATION : RÉPARTITION ÉQUITABLE DU SURPLUS ---
    let total_surplus = inner_width - min_data_width;
    let base_extra = if arity > 0 { total_surplus / arity } else { 0 };
    let mut remainder = if arity > 0 { total_surplus % arity } else { 0 };

    let get_col_width = |index: usize| {
        let mut w = col_width_base + base_extra;
        if index < remainder {
            w += 1;
        } // On distribue le reste pixel par pixel
        w
    };

    // --- 3. BORDURES ---
    let mut hr_top = String::from("┌");
    hr_top.push_str(&title_content);
    if title_len < inner_width {
        hr_top.push_str(&"─".repeat(inner_width - title_len));
    }
    hr_top.push_str("┐");

    let hr_mid = format!("├{}┤", "─".repeat(inner_width));
    let hr_bottom = format!("└{}┘", "─".repeat(inner_width));

    // --- 4. AFFICHAGE ---
    println!("\n{}", hr_top);

    // Header
    let mut header_line = String::from("│");
    if arity == 0 {
        header_line.push_str(&format!(" {:<w$} ", "(propositional)", w = inner_width - 2));
    } else {
        for (j, name) in column_names.iter().enumerate() {
            let w = get_col_width(j);
            header_line.push_str(&format!(" {:<w$} ", name, w = w - 2));
            if j < arity - 1 {
                header_line.push_str("│");
            }
        }
    }
    header_line.push_str("│");
    println!("{}", header_line);
    println!("{}", hr_mid);

    // Données
    for fact in relation.iter().take(20) {
        let mut row = String::from("│");
        if arity == 0 {
            row.push_str(&format!(" {:<w$} ", "TRUE", w = inner_width - 2));
        } else {
            for (j, &obj_id) in fact.iter().enumerate() {
                let name = ctx
                    .problem
                    .object_symbols()
                    .get_ident(obj_id)
                    .and_then(|&s| interner.resolve_symbol(s))
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| format!("o{}", obj_id.as_usize()));

                let w = get_col_width(j);
                row.push_str(&format!(" {:<w$} ", name, w = w - 2));
                if j < arity - 1 {
                    row.push_str("│");
                }
            }
        }
        row.push_str("│");
        println!("{}", row);
    }

    if relation.len() > 20 {
        let footer_text = format!(" ... and {} more rows", relation.len() - 20);
        println!("│ {:<w$} │", footer_text, w = inner_width - 2);
    }
    println!("{}", hr_bottom);
}

///// Rendu d'une table unique.
/*fn render_table(
    ctx: &RenderContext,
    sk_id: AtomSkeletonId,
    relation: &Relation,
    interner: &SymbolInterner,
) {
    let (category, label) = ctx.identify_segment(sk_id);
    let arity = relation.arity();
    let label_upper = label.to_uppercase();

    // 1. Déterminer la largeur nécessaire pour les données
    // On parcourt la table pour trouver le nom d'objet le plus long
    let mut max_obj_len = 10; // Minimum par défaut (ex: "col_0" fait 5)

    // On vérifie les 20 premières lignes (celles qu'on va afficher)
    for fact in relation.iter().take(20) {
        for &obj_id in fact.iter() {
            let name_len = ctx
                .problem
                .object_symbols()
                .get_ident(obj_id)
                .and_then(|&s| interner.resolve_symbol(s))
                .map(|s| s.chars().count())
                .unwrap_or(5); // o123
            if name_len > max_obj_len {
                max_obj_len = name_len;
            }
        }
    }

    // On ajoute des marges (1 espace de chaque côté)
    let col_width_base = max_obj_len + 2;

    // 2. Largeur minimale basée sur les colonnes
    let min_data_width = if arity == 0 {
        25 // Largeur fixe pour les propositions
    } else {
        (col_width_base * arity) + (arity - 1)
    };

    // 3. Mesure du titre
    let title_content = format!(
        " TABLE: {} [{}] [ID:{}] [{} rows] ",
        label_upper,
        category,
        sk_id.as_usize(),
        relation.len()
    );
    let title_len = title_content.chars().count();

    // 4. Largeur finale (Le max entre titre et données)
    let inner_width = std::cmp::max(min_data_width, title_len);
    let last_col_adjustment = inner_width - min_data_width;

    // --- CONSTRUCTION DES BORDURES ---
    let mut hr_top = String::from("┌");
    hr_top.push_str(&title_content);
    if title_len < inner_width {
        hr_top.push_str(&"─".repeat(inner_width - title_len));
    }
    hr_top.push_str("┐");

    let hr_mid = format!("├{}┤", "─".repeat(inner_width));
    let hr_bottom = format!("└{}┘", "─".repeat(inner_width));

    // --- AFFICHAGE ---
    println!("\n{}", hr_top);

    // Header
    let mut header = String::from("│");
    if arity == 0 {
        header.push_str(&format!(" {:<w$} ", "(propositional)", w = inner_width - 2));
    } else {
        for j in 0..arity {
            let w = if j == arity - 1 {
                col_width_base + last_col_adjustment
            } else {
                col_width_base
            };
            header.push_str(&format!(" {:<w$} ", format!("col_{}", j), w = w - 2));
            if j < arity - 1 {
                header.push_str("│");
            }
        }
    }
    header.push_str("│");
    println!("{}", header);
    println!("{}", hr_mid);

    // Données
    for fact in relation.iter().take(20) {
        let mut row = String::from("│");
        for (j, &obj_id) in fact.iter().enumerate() {
            let name = ctx
                .problem
                .object_symbols()
                .get_ident(obj_id)
                .and_then(|&s| interner.resolve_symbol(s))
                .map(|s| s.to_string())
                .unwrap_or_else(|| format!("o{}", obj_id.as_usize()));

            let w = if j == arity - 1 {
                col_width_base + last_col_adjustment
            } else {
                col_width_base
            };
            row.push_str(&format!(" {:<w$} ", name, w = w - 2));
            if j < arity - 1 {
                row.push_str("│");
            }
        }
        row.push_str("│");
        println!("{}", row);
    }

    if relation.len() > 20 {
        println!(
            "│ {:<w$} │",
            format!(" ... and {} more rows", relation.len() - 20),
            w = inner_width - 2
        );
    }
    println!("{}", hr_bottom);
}*/
