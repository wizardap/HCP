use crate::parallel_sat_portfolio::PortfolioResult;
use rustsat::instances::Cnf;
use rustsat::solvers::{ControlSignal, LimitConflicts, PhaseLit, Solve, SolveIncremental, SolverResult, Terminate};
use rustsat::types::Lit;
use rustsat_cadical::CaDiCaL;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

/// Persistent incremental CaDiCaL SAT solver for CEGAR solving.
/// Preserves learned CDCL conflict clauses, variable activities (VSIDS),
/// and phase saving across CEGAR rounds, avoiding cold-start re-solving overhead.
pub struct IncrementalSatSolver {
    solver: CaDiCaL<'static, 'static>,
}

impl IncrementalSatSolver {
    /// Creates a new persistent CaDiCaL solver initialized with `base_cnf`.
    pub fn new(base_cnf: &Cnf) -> Self {
        let mut solver = CaDiCaL::default();
        let _ = solver.add_cnf_ref(base_cnf);
        Self { solver }
    }

    /// Adds new cut clauses to the existing CaDiCaL solver incrementally.
    pub fn add_cuts(&mut self, cuts: &Cnf) {
        if !cuts.is_empty() {
            let _ = self.solver.add_cnf_ref(cuts);
        }
    }

    /// Solves the CNF incrementally with assumptions, phase hints, and a soft timeout.
    pub fn solve_with_timeout(
        &mut self,
        assumptions: &[Lit],
        phase_hints: &[Lit],
        timeout_secs: f64,
    ) -> PortfolioResult {
        // Apply phase guidance from backbone edge hints
        for &hint_lit in phase_hints {
            let _ = self.solver.phase_lit(hint_lit);
        }

        let term_flag = Arc::new(AtomicBool::new(false));
        let term_flag_clone = term_flag.clone();
        self.solver.attach_terminator(move || {
            if term_flag_clone.load(Ordering::Relaxed) {
                ControlSignal::Terminate
            } else {
                ControlSignal::Continue
            }
        });

        let timer_flag = term_flag.clone();
        let _ = thread::spawn(move || {
            let step = Duration::from_millis(50);
            let total_steps = (timeout_secs * 20.0) as usize;
            for _ in 0..total_steps {
                if timer_flag.load(Ordering::Relaxed) {
                    return;
                }
                thread::sleep(step);
            }
            timer_flag.store(true, Ordering::Relaxed);
        });

        // 1. Solve with assumptions under bounded conflict limit
        if !assumptions.is_empty() {
            let conflict_limit = (assumptions.len() * 100).clamp(200, 2500) as u32;
            let _ = self.solver.limit_conflicts(Some(conflict_limit));
            let assumps_res = self.solver.solve_assumps(assumptions);
            match assumps_res {
                Ok(SolverResult::Sat) => {
                    term_flag.store(true, Ordering::Relaxed);
                    let _ = self.solver.detach_terminator();
                    if let Ok(sol) = self.solver.full_solution() {
                        let model: Vec<Lit> = sol.into_iter().collect();
                        return PortfolioResult::Sat(model);
                    }
                }
                Ok(SolverResult::Unsat) | Ok(SolverResult::Interrupted) | Err(_) => {
                    let _ = self.solver.limit_conflicts(None);
                }
            }
        }

        if term_flag.load(Ordering::Relaxed) {
            let _ = self.solver.detach_terminator();
            return PortfolioResult::Interrupted;
        }

        // 2. Unconstrained solve
        let res = self.solver.solve();
        term_flag.store(true, Ordering::Relaxed);
        let _ = self.solver.detach_terminator();

        match res {
            Ok(SolverResult::Sat) => {
                if let Ok(sol) = self.solver.full_solution() {
                    let model: Vec<Lit> = sol.into_iter().collect();
                    PortfolioResult::Sat(model)
                } else {
                    PortfolioResult::Interrupted
                }
            }
            Ok(SolverResult::Unsat) => PortfolioResult::Unsat,
            Ok(SolverResult::Interrupted) | Err(_) => PortfolioResult::Interrupted,
        }
    }

    /// Resets the solver with a compacted / subsumed CNF.
    pub fn reset_with_cnf(&mut self, cnf: &Cnf) {
        self.solver = CaDiCaL::default();
        let _ = self.solver.add_cnf_ref(cnf);
    }
}
