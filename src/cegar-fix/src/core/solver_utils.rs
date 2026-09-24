use rustsat::solvers::{ControlSignal, Terminate};
use rustsat_cadical::CaDiCaL;
use std::time::Instant;

/// Creates a CaDiCaL solver instance with a terminator callback that
/// interrupts the solver when the deadline is reached.
///
/// Without this, `solver.solve()` is a blocking FFI call into C++ that
/// cannot be interrupted by Rust-side deadline checks between CEGAR
/// iterations. The terminator is polled internally by CaDiCaL during
/// search, ensuring prompt return as `SolverResult::Interrupted`.
pub fn create_solver_with_deadline(deadline: Instant) -> CaDiCaL<'static, 'static> {
    let mut solver = CaDiCaL::default();
    solver.attach_terminator(move || {
        if Instant::now() >= deadline {
            ControlSignal::Terminate
        } else {
            ControlSignal::Continue
        }
    });
    solver
}
