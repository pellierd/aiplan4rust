use crate::aiplan4rust::lir::store::builder::ExprBuilder;
use crate::aiplan4rust::lir::store::iter::Scratchpad;
use crate::aiplan4rust::lir::store::ops::error::ExprOpErrorHC;
use crate::aiplan4rust::lir::store::{ExprEntryKind, ExprId};
use smallvec::SmallVec;
use std::mem;

pub fn to_fnf(
    expr: ExprId,
    builder: &mut ExprBuilder,
    scratch: &mut Scratchpad,
    recursive: bool,
) -> Result<ExprId, ExprOpErrorHC> {
    // --- 1. PRÉPARATION ---
    // On extrait les enfants de l'expression racine (le OR) dans une SmallVec locale.
    // Cela évite d'utiliser un buffer partagé ou d'allouer sur le tas (si < 16 enfants).
    let mut input_ids: SmallVec<[ExprId; 16]> = SmallVec::new();
    {
        let entry = builder.fetch(expr)?;
        // Si c'est déjà un OR, on prend ses enfants, sinon on traite l'expr comme enfant unique.
        if matches!(entry.kind(), ExprEntryKind::Or) {
            input_ids.extend_from_slice(entry.children());
        } else {
            input_ids.push(expr);
        }
    }

    // On nettoie le pad pour préparer les buffers de travail internes (recyclage de mémoire)
    scratch.group_buffer_mut().clear();
    scratch.other_kids_mut().clear();

    // Buffers de travail locaux
    let mut groups_with_f: SmallVec<[ExprId; 32]> = SmallVec::new();
    let mut still_to_process: Vec<Vec<ExprId>> = Vec::new();
    let mut extraction_buf: SmallVec<[ExprId; 32]> = SmallVec::new();

    // Initialisation : répartition initiale
    for child_id in input_ids {
        let entry = builder.fetch(child_id)?;
        if matches!(entry.kind(), ExprEntryKind::And) {
            let mut sub_kids = entry.children().to_vec();
            sub_kids.sort_unstable();
            scratch.group_buffer_mut().push(sub_kids);
        } else {
            scratch.other_kids_mut().push(child_id);
        }
    }

    // --- 2. BOUCLE DE FACTORISATION RECYCLÉE (Ton Algo) ---
    loop {
        if scratch.group_buffer().len() < 2 {
            break;
        }

        scratch.compute_frequencies();

        if let Some(f) = scratch.find_best_factor() {
            groups_with_f.clear();
            still_to_process.clear();

            let mut old_groups = mem::take(scratch.group_buffer_mut());

            for mut group in old_groups.drain(..) {
                if group.binary_search(&f).is_ok() {
                    group.retain(|&x| x != f);
                    groups_with_f.push(builder.and(&group));
                } else {
                    still_to_process.push(group);
                }
            }

            let inner_or = builder.or(&groups_with_f);
            let factored_and = builder.and(&[f, inner_or]);

            extraction_buf.clear();
            {
                let entry = builder.fetch(factored_and)?;
                extraction_buf.extend_from_slice(entry.children());
            }

            let mut new_sub_kids = extraction_buf.to_vec();
            new_sub_kids.sort_unstable();

            let current_groups = scratch.group_buffer_mut();
            *current_groups = still_to_process;
            current_groups.push(new_sub_kids);

            still_to_process = old_groups;

            if !recursive {
                break;
            }
        } else {
            break;
        }
    }

    // --- 3. ASSEMBLAGE FINAL ---
    let mut final_or_args: SmallVec<[ExprId; 32]> = SmallVec::new();

    // On récupère les éléments qui n'étaient pas des AND ou non factorisés
    final_or_args.extend(mem::take(scratch.other_kids_mut()));

    for g in mem::take(scratch.group_buffer_mut()) {
        final_or_args.push(builder.and(&g));
    }

    Ok(builder.or(&final_or_args))
}
