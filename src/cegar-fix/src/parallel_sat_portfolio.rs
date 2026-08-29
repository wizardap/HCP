use rustsat::instances::Cnf;
use rustsat::solvers::{ControlSignal, LimitConflicts, Solve, SolveIncremental, SolverResult, Terminate};
use rustsat::types::Lit;
use rustsat_cadical::CaDiCaL;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::Arc;
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
    /// If assumptions are provided, workers attempt assumption-based solving first (with conflict limiting),
    /// and if interrupted or UNSAT under assumptions, falls back to unconstrained solving.
    pub fn solve_portfolio(
        cnf: &Cnf,
        assumptions: &[Lit],
        num_workers: usize,
    ) -> PortfolioResult {
        let num_workers = num_workers.max(1);
        let cancelled = Arc::new(AtomicBool::new(false));
        let (tx, rx) = mpsc::channel::<PortfolioResult>();
        let mut handles = Vec::with_capacity(num_workers);

        for worker_id in 0..num_workers {
            let cnf_clone = cnf.clone();
            let assumptions_vec: Vec<Lit> = assumptions.to_vec();
            let cancelled_clone = Arc::clone(&cancelled);
            let tx_clone = tx.clone();

            let handle = thread::spawn(move || {
                let mut solver = CaDiCaL::default();
                match worker_id {
                    0 => {} // Worker 0: Deterministic default CaDiCaL
                    1 => {
                        let _ = solver.set_option("seed", 42);
                    }
                    2 => {
                        let _ = solver.set_option("seed", 1337);
                    }
                    w => {
                        let _ = solver.set_option("seed", (w as i32) * 1000 + 42);
                    }
                }

                if solver.add_cnf_ref(&cnf_clone).is_err() {
                    return;
                }

                let cancel_cb = Arc::clone(&cancelled_clone);
                solver.attach_terminator(move || {
                    if cancel_cb.load(Ordering::Relaxed) {
                        ControlSignal::Terminate
                    } else {
                        ControlSignal::Continue
                    }
                });

                if !assumptions_vec.is_empty() {
                    if cancelled_clone.load(Ordering::Relaxed) {
                        return;
                    }
                    let _ = solver.limit_conflicts(Some(5000));
                    let assumps_res = solver.solve_assumps(&assumptions_vec);
                    match assumps_res {
                        Ok(SolverResult::Sat) => {
                            if let Ok(sol) = solver.full_solution() {
                                let model: Vec<Lit> = sol.into_iter().collect();
                                cancelled_clone.store(true, Ordering::Relaxed);
                                let _ = tx_clone.send(PortfolioResult::Sat(model));
                                return;
                            }
                        }
                        Ok(SolverResult::Unsat) | Ok(SolverResult::Interrupted) => {
                            if cancelled_clone.load(Ordering::Relaxed) {
                                return;
                            }
                            let _ = solver.limit_conflicts(None);
                        }
                        Err(_) => {
                            if cancelled_clone.load(Ordering::Relaxed) {
                                return;
                            }
                            let _ = solver.limit_conflicts(None);
                        }
                    }
                }

                if cancelled_clone.load(Ordering::Relaxed) {
                    return;
                }

                let res = solver.solve();
                match res {
                    Ok(SolverResult::Sat) => {
                        if let Ok(sol) = solver.full_solution() {
                            let model: Vec<Lit> = sol.into_iter().collect();
                            cancelled_clone.store(true, Ordering::Relaxed);
                            let _ = tx_clone.send(PortfolioResult::Sat(model));
                        }
                    }
                    Ok(SolverResult::Unsat) => {
                        cancelled_clone.store(true, Ordering::Relaxed);
                        let _ = tx_clone.send(PortfolioResult::Unsat);
                    }
                    Ok(SolverResult::Interrupted) | Err(_) => {
                        // Worker terminated or errored
                    }
                }
            });

            handles.push(handle);
        }

        drop(tx); // Close parent sender

        let mut result = PortfolioResult::Interrupted;
        if let Ok(msg) = rx.recv() {
            result = msg;
        }

        cancelled.store(true, Ordering::Relaxed);
        for handle in handles {
            let _ = handle.join();
        }

        result
    }
}
