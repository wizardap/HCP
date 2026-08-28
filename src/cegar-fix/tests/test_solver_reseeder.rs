use cegar_fix::solver_reseeder::{ReseederOptions, SolverReseeder};
use rustsat::instances::Cnf;
use rustsat::solvers::{Solve, SolverResult};
use rustsat::types::{Clause, Lit};

#[test]
fn test_should_reseed_trigger() {
    let default_opts = ReseederOptions::default();
    assert_eq!(default_opts.max_sat_time_threshold_secs, 15.0);
    assert_eq!(default_opts.periodic_interval_rounds, 10);
    assert!(default_opts.enable_reseeding);

    // Round 0 should never trigger re-seeding regardless of time or periodic round
    assert!(!SolverReseeder::should_reseed(0.0, 0, &default_opts));
    assert!(!SolverReseeder::should_reseed(100.0, 0, &default_opts));

    // When reseeding is disabled, never trigger
    let disabled_opts = ReseederOptions {
        enable_reseeding: false,
        ..default_opts
    };
    assert!(!SolverReseeder::should_reseed(20.0, 10, &disabled_opts));
    assert!(!SolverReseeder::should_reseed(5.0, 10, &disabled_opts));
    assert!(!SolverReseeder::should_reseed(20.0, 3, &disabled_opts));

    // Under default options:
    // 1. Time threshold trigger (>= 15.0s)
    assert!(SolverReseeder::should_reseed(15.0, 1, &default_opts));
    assert!(SolverReseeder::should_reseed(16.5, 3, &default_opts));
    assert!(!SolverReseeder::should_reseed(14.9, 3, &default_opts));

    // 2. Periodic round trigger (every 10 rounds)
    assert!(SolverReseeder::should_reseed(0.1, 10, &default_opts));
    assert!(SolverReseeder::should_reseed(0.0, 20, &default_opts));
    assert!(SolverReseeder::should_reseed(1.5, 30, &default_opts));
    assert!(!SolverReseeder::should_reseed(0.1, 9, &default_opts));
    assert!(!SolverReseeder::should_reseed(0.1, 11, &default_opts));

    // Custom options
    let custom_opts = ReseederOptions {
        max_sat_time_threshold_secs: 5.0,
        periodic_interval_rounds: 3,
        enable_reseeding: true,
    };
    assert!(SolverReseeder::should_reseed(5.0, 1, &custom_opts));
    assert!(SolverReseeder::should_reseed(0.1, 3, &custom_opts));
    assert!(SolverReseeder::should_reseed(0.1, 6, &custom_opts));
    assert!(!SolverReseeder::should_reseed(4.9, 2, &custom_opts));
}

#[test]
fn test_reseed_solver_preserves_clauses() {
    let lit_x = Lit::positive(0); // variable 0 positive
    let lit_not_x = Lit::negative(0); // variable 0 negative

    // Base CNF: clause [x]
    let mut base_cnf = Cnf::new();
    base_cnf.add_clause(Clause::from_iter([lit_x]));

    // Accumulated cut CNF: clause [not x]
    let mut cut_cnf = Cnf::new();
    cut_cnf.add_clause(Clause::from_iter([lit_not_x]));

    // 1. Reseed solver with only base CNF -> Should be SAT
    let mut solver_sat = SolverReseeder::reseed_solver(&base_cnf, &[], 0);
    let res_sat = solver_sat.solve();
    assert_eq!(res_sat.unwrap(), SolverResult::Sat);

    // 2. Reseed solver with base CNF + cut CNF -> Should be UNSAT
    let mut solver_unsat = SolverReseeder::reseed_solver(&base_cnf, &[cut_cnf.clone()], 0);
    let res_unsat = solver_unsat.solve();
    assert_eq!(res_unsat.unwrap(), SolverResult::Unsat);

    // 3. Test different cadical configs (0..=4)
    for cfg in 0..=4 {
        let mut solver = SolverReseeder::reseed_solver(&base_cnf, &[cut_cnf.clone()], cfg);
        let res = solver.solve();
        assert_eq!(res.unwrap(), SolverResult::Unsat, "Config {} should result in UNSAT", cfg);
    }
}
