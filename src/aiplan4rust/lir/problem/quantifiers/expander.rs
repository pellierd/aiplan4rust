use crate::aiplan4rust::lir::LirError;
use crate::aiplan4rust::lir::problem::quantifiers::analyzer;
use crate::aiplan4rust::lir::problem::LiftedProblem;


pub fn expand_quantifiers(problem: &mut LiftedProblem) -> Result<(), LirError> {
    let interner = problem.interner();

    

    let inertia = analyzer::analyze_inertia(problem)?;


    println!("inertia: {}", inertia);

    Ok(())
}
