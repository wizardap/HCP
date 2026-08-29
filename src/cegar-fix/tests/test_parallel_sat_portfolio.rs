use cegar_fix::parallel_sat_portfolio::{ParallelSatPortfolio, PortfolioResult};
use rustsat::instances::Cnf;
use rustsat::types::{Clause, Lit};
use std::collections::HashSet;

fn verify_model_satisfies_cnf(cnf: &Cnf, model: &[Lit]) -> bool {
    let model_set: HashSet<Lit> = model.iter().copied().collect();
    for clause in cnf.iter() {
        let mut satisfied = false;
        for lit in clause.iter() {
            if model_set.contains(lit) {
                satisfied = true;
                break;
            }
        }
        if !satisfied {
            return false;
        }
    }
    true
}

#[test]
fn test_portfolio_solves_simple_sat() {
    let lit_a = Lit::positive(0);
    let lit_not_a = Lit::negative(0);
    let lit_b = Lit::positive(1);
    let lit_not_b = Lit::negative(1);
    let lit_c = Lit::positive(2);

    // Formula: (a or b) and (~a or c) and (~b or c)
    let mut cnf = Cnf::new();
    cnf.add_clause(Clause::from_iter([lit_a, lit_b]));
    cnf.add_clause(Clause::from_iter([lit_not_a, lit_c]));
    cnf.add_clause(Clause::from_iter([lit_not_b, lit_c]));

    let result = ParallelSatPortfolio::solve_portfolio(&cnf, &[], 3, 0);
    match result {
        PortfolioResult::Sat(model) => {
            assert!(
                verify_model_satisfies_cnf(&cnf, &model),
                "Returned model does not satisfy the formula: {:?}",
                model
            );
        }
        other => panic!("Expected Sat, got {:?}", other),
    }
}

#[test]
fn test_portfolio_solves_unsat() {
    let lit_a = Lit::positive(0);
    let lit_not_a = Lit::negative(0);

    // Formula: (a) and (~a)
    let mut cnf = Cnf::new();
    cnf.add_clause(Clause::from_iter([lit_a]));
    cnf.add_clause(Clause::from_iter([lit_not_a]));

    let result = ParallelSatPortfolio::solve_portfolio(&cnf, &[], 3, 0);
    match result {
        PortfolioResult::Unsat => {}
        other => panic!("Expected Unsat, got {:?}", other),
    }
}

#[test]
fn test_portfolio_with_assumptions() {
    let lit_a = Lit::positive(0);
    let lit_not_a = Lit::negative(0);
    let lit_b = Lit::positive(1);
    let lit_not_b = Lit::negative(1);

    // 1. Formula: (a or b) and (~a or b)  => forces b = true, a can be true or false.
    let mut cnf = Cnf::new();
    cnf.add_clause(Clause::from_iter([lit_a, lit_b]));
    cnf.add_clause(Clause::from_iter([lit_not_a, lit_b]));

    // Assumption: a = true (lit_a)
    let assumptions = vec![lit_a];
    let result = ParallelSatPortfolio::solve_portfolio(&cnf, &assumptions, 3, 0);
    match result {
        PortfolioResult::Sat(model) => {
            assert!(verify_model_satisfies_cnf(&cnf, &model));
            let model_set: HashSet<Lit> = model.into_iter().collect();
            assert!(model_set.contains(&lit_a), "Model should satisfy assumption lit_a");
            assert!(model_set.contains(&lit_b), "Model should satisfy entailed lit_b");
        }
        other => panic!("Expected Sat under assumptions, got {:?}", other),
    }

    // 2. Fast-fail fallback: Assumptions contradict formula, but unconstrained is SAT
    // Formula: (a or b)
    // Assumptions: ~a and ~b (contradicts formula)
    let mut cnf2 = Cnf::new();
    cnf2.add_clause(Clause::from_iter([lit_a, lit_b]));
    let bad_assumptions = vec![lit_not_a, lit_not_b];

    let result2 = ParallelSatPortfolio::solve_portfolio(&cnf2, &bad_assumptions, 3, 0);
    match result2 {
        PortfolioResult::Sat(model) => {
            assert!(
                verify_model_satisfies_cnf(&cnf2, &model),
                "Fallback should find unconstrained satisfying model"
            );
        }
        other => panic!("Expected fallback to find Sat, got {:?}", other),
    }
}
