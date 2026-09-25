use crate::core::graph::Graph;
use crate::solver::cegar_engine;

/// Delegated to unified `cegar_engine::solve_cycle`.
#[inline]
pub fn solve_hamiltonian_cycle(g: &Graph, timeout_secs: f64) -> Result<Vec<i32>, String> {
    cegar_engine::solve_cycle(g, timeout_secs)
}

/// Delegated to unified `cegar_engine::solve_path`.
#[inline]
pub fn solve_hamiltonian_path(
    g: &Graph,
    port_u: i32,
    port_v: i32,
    timeout_secs: f64,
) -> Result<Vec<i32>, String> {
    cegar_engine::solve_path(g, port_u, port_v, timeout_secs)
}
