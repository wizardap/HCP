use cegar_fix::cnf_subsumer::CnfSubsumer;
use rustsat::instances::Cnf;
use rustsat::types::{Clause, Lit};

#[test]
fn test_empty_and_single_clause() {
    // Empty CNFs
    let empty_cnfs: Vec<Cnf> = vec![];
    let result = CnfSubsumer::prune_and_subsume_cuts(&empty_cnfs);
    assert_eq!(result.len(), 0);

    let empty_cnf = vec![Cnf::new()];
    let result = CnfSubsumer::prune_and_subsume_cuts(&empty_cnf);
    assert_eq!(result.len(), 0);

    // Single clause
    let mut cnf = Cnf::new();
    let lit_a = Lit::positive(0);
    let lit_b = Lit::positive(1);
    cnf.add_clause(Clause::from_iter([lit_a, lit_b]));

    let result = CnfSubsumer::prune_and_subsume_cuts(&[cnf]);
    assert_eq!(result.len(), 1);
    let clause = result.into_iter().next().unwrap();
    let lits: Vec<Lit> = clause.into_iter().collect();
    assert_eq!(lits, vec![lit_a, lit_b]);
}

#[test]
fn test_exact_deduplication() {
    let lit_a = Lit::positive(0);
    let lit_b = Lit::positive(1);

    let mut cnf1 = Cnf::new();
    cnf1.add_clause(Clause::from_iter([lit_a, lit_b]));
    // Clause with reversed literals order should be normalized and deduplicated
    cnf1.add_clause(Clause::from_iter([lit_b, lit_a]));
    // Duplicate literals in a single clause (e.g. A | B | A)
    cnf1.add_clause(Clause::from_iter([lit_a, lit_b, lit_a]));

    let mut cnf2 = Cnf::new();
    cnf2.add_clause(Clause::from_iter([lit_a, lit_b]));

    let result = CnfSubsumer::prune_and_subsume_cuts(&[cnf1, cnf2]);
    assert_eq!(result.len(), 1);
}

#[test]
fn test_proper_subsumption() {
    let lit_a = Lit::positive(0);
    let lit_b = Lit::positive(1);
    let lit_c = Lit::positive(2);
    let lit_d = Lit::negative(3);

    // cnf with (A or B) and (A or B or C) and (A or B or D)
    let mut cnf1 = Cnf::new();
    cnf1.add_clause(Clause::from_iter([lit_a, lit_b, lit_c]));
    cnf1.add_clause(Clause::from_iter([lit_a, lit_b]));

    let mut cnf2 = Cnf::new();
    cnf2.add_clause(Clause::from_iter([lit_a, lit_b, lit_d]));
    cnf2.add_clause(Clause::from_iter([lit_c, lit_d]));

    let result = CnfSubsumer::prune_and_subsume_cuts(&[cnf1, cnf2]);
    // (A or B) subsumes (A or B or C) and (A or B or D).
    // (C or D) is independent, so kept.
    // Total remaining: 2 clauses: (A or B) and (C or D).
    assert_eq!(result.len(), 2);
}

#[test]
fn test_tautology_elimination() {
    let lit_a = Lit::positive(0);
    let lit_not_a = Lit::negative(0);
    let lit_b = Lit::positive(1);

    let mut cnf = Cnf::new();
    // Tautology: A or ~A
    cnf.add_clause(Clause::from_iter([lit_a, lit_not_a]));
    // Tautology with other literal: A or ~A or B
    cnf.add_clause(Clause::from_iter([lit_a, lit_not_a, lit_b]));
    // Non-tautology: A or B
    cnf.add_clause(Clause::from_iter([lit_a, lit_b]));

    let result = CnfSubsumer::prune_and_subsume_cuts(&[cnf]);
    assert_eq!(result.len(), 1);
    let clause = result.into_iter().next().unwrap();
    let lits: Vec<Lit> = clause.into_iter().collect();
    assert_eq!(lits, vec![lit_a, lit_b]);
}
