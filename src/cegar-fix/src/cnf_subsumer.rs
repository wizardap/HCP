use rustsat::instances::Cnf;
use rustsat::types::{Clause, Lit};
use std::collections::HashSet;

pub struct CnfSubsumer;

impl CnfSubsumer {
    /// Normalizes, deduplicates, and eliminates subsumed cut clauses across all input CNFs.
    pub fn prune_and_subsume_cuts(cnfs: &[Cnf]) -> Cnf {
        // Step 1: Normalize clauses across all CNFs
        let mut normalized_clauses: Vec<Vec<Lit>> = Vec::new();

        for cnf in cnfs {
            for clause in cnf.iter() {
                // Collect literals into a set to eliminate duplicate literals within the clause
                let mut lit_set: HashSet<Lit> = HashSet::new();
                let mut is_tautology = false;

                for &lit in clause.iter() {
                    if lit_set.contains(&!lit) {
                        is_tautology = true;
                        break;
                    }
                    lit_set.insert(lit);
                }

                if is_tautology {
                    continue;
                }

                // Convert to sorted vector ordered by variable index
                let mut lits: Vec<Lit> = lit_set.into_iter().collect();
                lits.sort_by_key(|l| l.var());

                normalized_clauses.push(lits);
            }
        }

        // Step 2: Sort clauses by length (shortest first)
        normalized_clauses.sort_by_key(|c| c.len());

        // Step 3: Deduplication & Subsumption Check
        let mut kept_clauses: Vec<Vec<Lit>> = Vec::new();

        for cand in normalized_clauses {
            let cand_set: HashSet<Lit> = cand.iter().copied().collect();
            let mut is_subsumed = false;

            for kept_vec in &kept_clauses {
                // If the kept clause is longer than candidate, it cannot be a subset
                if kept_vec.len() > cand.len() {
                    break;
                }
                // Check if kept_vec is a subset of cand (K <= C)
                if kept_vec.iter().all(|l| cand_set.contains(l)) {
                    is_subsumed = true;
                    break;
                }
            }

            if !is_subsumed {
                kept_clauses.push(cand);
            }
        }

        // Step 4: Build compact result Cnf
        let mut result = Cnf::new();
        for clause_lits in kept_clauses {
            result.add_clause(Clause::from_iter(clause_lits));
        }

        result
    }
}
