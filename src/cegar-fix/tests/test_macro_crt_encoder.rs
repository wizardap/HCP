use std::collections::{HashMap, HashSet};
use cegar_fix::two_tier_decomposer::DecompositionResult;
use cegar_fix::macro_crt_encoder::MacroCrtEncoder;
use rustsat::solvers::{Solve, SolverResult};
use rustsat::types::{Lit, Var};
use rustsat_cadical::CaDiCaL;

#[test]
fn test_macro_crt_encoder_basic() {
    let mut solver = CaDiCaL::default();
    let mut next_var_id: u32 = 0;

    let decomp = DecompositionResult {
        s_hubs: vec![1],
        b_hubs: vec![2, 3],
        m_hubs: vec![4, 5],
        all_hubs: HashSet::from([1, 2, 3, 4, 5]),
        strips: vec![vec![10, 11]],
        strip_adj_hubs: HashMap::from([(0, HashSet::from([1, 2, 3, 4, 5]))]),
        hub_adj_strips: HashMap::from([
            (1, HashSet::from([0])),
            (2, HashSet::from([0])),
            (3, HashSet::from([0])),
            (4, HashSet::from([0])),
            (5, HashSet::from([0])),
        ]),
        hh_edges: vec![(1, 2), (2, 3), (3, 4), (4, 5), (5, 1)],
    };

    let mut var_hh: HashMap<(i32, i32), Lit> = HashMap::new();
    for &(u, v) in &decomp.hh_edges {
        let lit = Var::new(next_var_id).pos_lit();
        next_var_id += 1;
        var_hh.insert((u, v), lit);
        var_hh.insert((v, u), lit);
    }

    let mut var_d1: HashMap<(usize, i32), Lit> = HashMap::new();
    for &h in &decomp.all_hubs {
        let lit = Var::new(next_var_id).pos_lit();
        next_var_id += 1;
        var_d1.insert((0, h), lit);
    }

    let encoder = MacroCrtEncoder::encode(
        &mut solver,
        &mut next_var_id,
        &decomp,
        &var_hh,
        &var_d1,
    );

    assert_eq!(encoder.moduli, vec![2, 3, 7]);
    assert_eq!(encoder.root_hub, 1);
    assert_eq!(encoder.residue_vars.len(), 5 * 3); // 5 hubs * 3 moduli

    let res = solver.solve();
    assert!(matches!(res, Ok(SolverResult::Sat)));
}
