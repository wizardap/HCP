use rustsat::instances::Cnf;
use rustsat::solvers::{ControlSignal, LimitConflicts, PhaseLit, Solve, SolveIncremental, SolverResult, Terminate};
use rustsat::types::Lit;
use rustsat_cadical::CaDiCaL;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::thread;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PortfolioResult {
    Sat(Vec<Lit>), // Full model (literals representing the satisfying assignment)
    Unsat,
    Interrupted,
}

pub struct ParallelSatPortfolio;

impl ParallelSatPortfolio {
    /// Solves CNF across `num_workers` parallel threads (default 3 for cores 0, 1, 2).
    /// Uses std::thread::scope to borrow CNF directly with zero cloning overhead.
    /// Diversifies seeds and phase heuristics across CEGAR rounds.
    pub fn solve_portfolio(
        cnf: &Cnf,
        assumptions: &[Lit],
        phase_hints: &[Lit],
        num_workers: usize,
        round: usize,
    ) -> PortfolioResult {
        let num_workers = num_workers.max(1);
        let cancelled = AtomicBool::new(false);
        let (tx, rx) = mpsc::channel::<PortfolioResult>();
        let mut result = PortfolioResult::Interrupted;

        thread::scope(|s| {
            for worker_id in 0..num_workers {
                let tx_worker = tx.clone();
                let cancelled_ref = &cancelled;

                s.spawn(move || {
                    let mut solver = CaDiCaL::default();
                    match worker_id {
                        0 => {} // Worker 0: Deterministic default CaDiCaL
                        1 => {
                            let _ = solver.set_option("seed", 42 + (round as i32) * 17);
                        }
                        2 => {
                            let _ = solver.set_option("seed", 1337 + (round as i32) * 31);
                            let _ = solver.set_option("phase", 0);
                        }
                        w => {
                            let _ = solver.set_option("seed", (w as i32) * 1000 + 42 + (round as i32) * 17);
                        }
                    }

                    // Apply phase guidance from backbone edge hints
                    for &hint_lit in phase_hints {
                        let _ = solver.phase_lit(hint_lit);
                    }

                    if solver.add_cnf_ref(cnf).is_err() {
                        return;
                    }

                    solver.attach_terminator(move || {
                        if cancelled_ref.load(Ordering::Relaxed) {
                            ControlSignal::Terminate
                        } else {
                            ControlSignal::Continue
                        }
                    });

                    if !assumptions.is_empty() {
                        if cancelled_ref.load(Ordering::Relaxed) {
                            return;
                        }
                        let _ = solver.limit_conflicts(Some(5000));
                        let assumps_res = solver.solve_assumps(assumptions);
                        match assumps_res {
                            Ok(SolverResult::Sat) => {
                                if let Ok(sol) = solver.full_solution() {
                                    let model: Vec<Lit> = sol.into_iter().collect();
                                    cancelled_ref.store(true, Ordering::Relaxed);
                                    let _ = tx_worker.send(PortfolioResult::Sat(model));
                                    return;
                                }
                            }
                            Ok(SolverResult::Unsat) | Ok(SolverResult::Interrupted) | Err(_) => {
                                if cancelled_ref.load(Ordering::Relaxed) {
                                    return;
                                }
                                let _ = solver.limit_conflicts(None);
                            }
                        }
                    }

                    if cancelled_ref.load(Ordering::Relaxed) {
                        return;
                    }

                    let res = solver.solve();
                    match res {
                        Ok(SolverResult::Sat) => {
                            if let Ok(sol) = solver.full_solution() {
                                let model: Vec<Lit> = sol.into_iter().collect();
                                cancelled_ref.store(true, Ordering::Relaxed);
                                let _ = tx_worker.send(PortfolioResult::Sat(model));
                            }
                        }
                        Ok(SolverResult::Unsat) => {
                            cancelled_ref.store(true, Ordering::Relaxed);
                            let _ = tx_worker.send(PortfolioResult::Unsat);
                        }
                        Ok(SolverResult::Interrupted) | Err(_) => {}
                    }
                });
            }

            drop(tx); // Close parent sender

            if let Ok(msg) = rx.recv() {
                cancelled.store(true, Ordering::Relaxed);
                result = msg;
            } else {
                cancelled.store(true, Ordering::Relaxed);
            }
        });

        result
    }
}
