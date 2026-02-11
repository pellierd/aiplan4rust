use crate::aiplan4rust::lir::analysis::inertia::analyzer;
use crate::aiplan4rust::lir::LirError;
use crate::aiplan4rust::lir::problem::LiftedProblem;


pub fn expand_quantifiers(problem: &mut LiftedProblem) -> Result<(), LirError> {
    let _interner = problem.interner();

    

    let inertia = analyzer::analyze(problem)?;


    println!("inertia: {}", inertia);

    Ok(())
}
