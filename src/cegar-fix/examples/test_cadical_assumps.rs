use rustsat::solvers::{Solve, SolveIncremental};
use rustsat::types::{Clause, Lit};
use rustsat_cadical::CaDiCaL;

fn main() {
    let mut solver = CaDiCaL::default();
    let l1 = Lit::positive(0);
    let l2 = Lit::positive(1);
    solver.add_clause(Clause::from_iter(vec![l1, l2])).unwrap();
    solver.add_clause(Clause::from_iter(vec![!l1, l2])).unwrap();

    let res1 = solver.solve_assumps(&[!l2]);
    println!("Solve with assumption !l2: {:?}", res1);

    let res2 = solver.solve();
    println!("Solve unconstrained after assumps UNSAT: {:?}", res2);
}
