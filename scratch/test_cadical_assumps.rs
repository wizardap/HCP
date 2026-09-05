use rustsat::instances::Cnf;
use rustsat::solvers::{Solve, SolveIncremental, SolverResult};
use rustsat::types::{Clause, Lit};
use rustsat_cadical::CaDiCaL;

fn main() {
    let mut solver = CaDiCaL::default();
    let l1 = Lit::positive(0);
    let l2 = Lit::positive(1);
    // Formula: (l1 or l2), (!l1 or l2) => l2 must be true.
    solver.add_clause(Clause::from_iter(vec![l1, l2])).unwrap();
    solver.add_clause(Clause::from_iter(vec![!l1, l2])).unwrap();

    // Now assume !l2 (which is false, formula is UNSAT under assumption !l2)
    let res1 = solver.solve_assumps(&[!l2]);
    println!("Solve with assumption !l2: {:?}", res1);

    // Now solve unconstrained (should be SAT: l2 = true)
    let res2 = solver.solve();
    println!("Solve unconstrained after assumps UNSAT: {:?}", res2);
}
