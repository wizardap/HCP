use cegar_fix::macro_mtz_encoder::MacroMtzEncoder;
use cegar_fix::two_tier_decomposer::DecompositionResult;
use rustsat::clause;
use rustsat::solvers::{Solve, SolverResult};
use rustsat::types::Var;
use rustsat_cadical::CaDiCaL;
use std::collections::{HashMap, HashSet};

#[test]
fn test_macro_mtz_encoding_structure() {
    let mut solver = CaDiCaL::default();
    let mut next_var_id: u32 = 0;

    let mut all_hubs = HashSet::new();
    all_hubs.insert(1);
    all_hubs.insert(2);
    all_hubs.insert(3);
    all_hubs.insert(4);

    let hh_edges = vec![(1, 2), (2, 3), (3, 4), (1, 4)];
    let mut strip_adj_hubs = HashMap::new();
    let mut hub_adj_strips = HashMap::new();

    let mut s0_adj = HashSet::new();
    s0_adj.insert(1);
    s0_adj.insert(3);
    strip_adj_hubs.insert(0, s0_adj);

    let mut s1_adj = HashSet::new();
    s1_adj.insert(2);
    s1_adj.insert(4);
    strip_adj_hubs.insert(1, s1_adj);

    hub_adj_strips.entry(1).or_insert_with(HashSet::new).insert(0);
    hub_adj_strips.entry(3).or_insert_with(HashSet::new).insert(0);
    hub_adj_strips.entry(2).or_insert_with(HashSet::new).insert(1);
    hub_adj_strips.entry(4).or_insert_with(HashSet::new).insert(1);

    let decomp = DecompositionResult {
        s_hubs: vec![],
        b_hubs: vec![],
        m_hubs: vec![1, 2, 3, 4],
        all_hubs,
        hh_edges: hh_edges.clone(),
        strips: vec![vec![101], vec![102]],
        strip_adj_hubs,
        hub_adj_strips,
    };

    let mut var_hh = HashMap::new();
    for &(u, v) in &hh_edges {
        let lit = Var::new(next_var_id).pos_lit();
        next_var_id += 1;
        var_hh.insert((u, v), lit);
        var_hh.insert((v, u), lit);
    }

    let mut var_d1 = HashMap::new();
    for (&si, adj) in &decomp.strip_adj_hubs {
        for &h in adj {
            let lit = Var::new(next_var_id).pos_lit();
            next_var_id += 1;
            var_d1.insert((si, h), lit);
        }
    }

    let encoder = MacroMtzEncoder::encode(
        &mut solver,
        &mut next_var_id,
        &decomp,
        &var_hh,
        &var_d1,
    );

    // Root hub must be the lowest hub (1)
    assert_eq!(encoder.root_hub, 1);

    // Order variables must exist for non-root hubs (2, 3, 4), each with N_H - 1 = 3 literals
    assert_eq!(encoder.hub_order_vars.len(), 3);
    assert!(!encoder.hub_order_vars.contains_key(&1));
    for &h in &[2, 3, 4] {
        let order_lits = encoder.hub_order_vars.get(&h).expect("missing order vars");
        assert_eq!(order_lits.len(), 3);
    }

    // Directed HH variables: 4 undirected edges * 2 directions = 8 directed variables
    assert_eq!(encoder.dir_hh_vars.len(), 8);
    for &(u, v) in &hh_edges {
        assert!(encoder.dir_hh_vars.contains_key(&(u, v)));
        assert!(encoder.dir_hh_vars.contains_key(&(v, u)));
    }

    // Directed strip variables: Strip 0 has (1->3, 3->1), Strip 1 has (2->4, 4->2) = 4 variables
    assert_eq!(encoder.dir_strip_vars.len(), 4);
    assert!(encoder.dir_strip_vars.contains_key(&(0, 1, 3)));
    assert!(encoder.dir_strip_vars.contains_key(&(0, 3, 1)));
    assert!(encoder.dir_strip_vars.contains_key(&(1, 2, 4)));
    assert!(encoder.dir_strip_vars.contains_key(&(1, 4, 2)));
}

#[test]
fn test_macro_mtz_prevents_subcycles() {
    // 6 Hubs: Triangle 1 (1, 2, 3) and Triangle 2 (4, 5, 6)
    // Cross edges: (3, 4) and (1, 6)
    let mut all_hubs = HashSet::new();
    for h in 1..=6 {
        all_hubs.insert(h);
    }

    let hh_edges = vec![
        (1, 2), (2, 3), (1, 3), // Triangle 1
        (4, 5), (5, 6), (4, 6), // Triangle 2
        (3, 4), (1, 6),         // Bridges
    ];

    let decomp = DecompositionResult {
        s_hubs: vec![],
        b_hubs: vec![],
        m_hubs: vec![1, 2, 3, 4, 5, 6],
        all_hubs,
        hh_edges: hh_edges.clone(),
        strips: vec![],
        strip_adj_hubs: HashMap::new(),
        hub_adj_strips: HashMap::new(),
    };

    // PART A: Assert disconnected 2-subcycle state -> MTZ must make it UNSAT
    {
        let mut solver = CaDiCaL::default();
        let mut next_var_id: u32 = 0;

        let mut var_hh = HashMap::new();
        for &(u, v) in &hh_edges {
            let lit = Var::new(next_var_id).pos_lit();
            next_var_id += 1;
            var_hh.insert((u, v), lit);
            var_hh.insert((v, u), lit);
        }
        let var_d1 = HashMap::new();

        // Encode MTZ
        let _encoder = MacroMtzEncoder::encode(
            &mut solver,
            &mut next_var_id,
            &decomp,
            &var_hh,
            &var_d1,
        );

        // Force Triangle 1: (1,2), (2,3), (1,3) = TRUE
        let _ = solver.add_clause(clause![var_hh[&(1, 2)]]);
        let _ = solver.add_clause(clause![var_hh[&(2, 3)]]);
        let _ = solver.add_clause(clause![var_hh[&(1, 3)]]);

        // Force Triangle 2: (4,5), (5,6), (4,6) = TRUE
        let _ = solver.add_clause(clause![var_hh[&(4, 5)]]);
        let _ = solver.add_clause(clause![var_hh[&(5, 6)]]);
        let _ = solver.add_clause(clause![var_hh[&(4, 6)]]);

        // Force Bridges = FALSE
        let _ = solver.add_clause(clause![!var_hh[&(3, 4)]]);
        let _ = solver.add_clause(clause![!var_hh[&(1, 6)]]);

        let res = solver.solve().expect("solve failed");
        assert_eq!(res, SolverResult::Unsat, "Disconnected 2-subcycle state must be UNSAT under MTZ");
    }

    // PART B: Assert connected 6-cycle state -> MTZ must be SAT
    {
        let mut solver = CaDiCaL::default();
        let mut next_var_id: u32 = 0;

        let mut var_hh = HashMap::new();
        for &(u, v) in &hh_edges {
            let lit = Var::new(next_var_id).pos_lit();
            next_var_id += 1;
            var_hh.insert((u, v), lit);
            var_hh.insert((v, u), lit);
        }
        let var_d1 = HashMap::new();

        // Encode MTZ
        let _encoder = MacroMtzEncoder::encode(
            &mut solver,
            &mut next_var_id,
            &decomp,
            &var_hh,
            &var_d1,
        );

        // Force 6-cycle: 1-2-3-4-5-6-1 = TRUE
        let _ = solver.add_clause(clause![var_hh[&(1, 2)]]);
        let _ = solver.add_clause(clause![var_hh[&(2, 3)]]);
        let _ = solver.add_clause(clause![var_hh[&(3, 4)]]);
        let _ = solver.add_clause(clause![var_hh[&(4, 5)]]);
        let _ = solver.add_clause(clause![var_hh[&(5, 6)]]);
        let _ = solver.add_clause(clause![var_hh[&(1, 6)]]);

        // Force non-cycle edges = FALSE
        let _ = solver.add_clause(clause![!var_hh[&(1, 3)]]);
        let _ = solver.add_clause(clause![!var_hh[&(4, 6)]]);

        let res = solver.solve().expect("solve failed");
        assert_eq!(res, SolverResult::Sat, "Connected 6-cycle state must be SAT under MTZ");
    }
}

#[test]
fn test_macro_mtz_prevents_subcycles_with_strips() {
    // Hubs 1, 2, 3, 4
    // Strip 0 between 1 and 2
    // Strip 1 between 3 and 4
    // HH edge (1, 2) and HH edge (3, 4)
    // Connecting HH edges (2, 3) and (4, 1)
    let mut all_hubs = HashSet::new();
    for h in 1..=4 {
        all_hubs.insert(h);
    }

    let hh_edges = vec![(1, 2), (3, 4), (2, 3), (4, 1)];

    let mut strip_adj_hubs = HashMap::new();
    let mut s0 = HashSet::new();
    s0.insert(1);
    s0.insert(2);
    strip_adj_hubs.insert(0, s0);

    let mut s1 = HashSet::new();
    s1.insert(3);
    s1.insert(4);
    strip_adj_hubs.insert(1, s1);

    let decomp = DecompositionResult {
        s_hubs: vec![],
        b_hubs: vec![],
        m_hubs: vec![1, 2, 3, 4],
        all_hubs,
        hh_edges: hh_edges.clone(),
        strips: vec![vec![101], vec![102]],
        strip_adj_hubs,
        hub_adj_strips: HashMap::new(),
    };

    // Subcycle test: Subcycle A: Hub 1 - Strip 0 - Hub 2 - HH(1,2) - Hub 1
    //                Subcycle B: Hub 3 - Strip 1 - Hub 4 - HH(3,4) - Hub 3
    {
        let mut solver = CaDiCaL::default();
        let mut next_var_id: u32 = 0;

        let mut var_hh = HashMap::new();
        for &(u, v) in &hh_edges {
            let lit = Var::new(next_var_id).pos_lit();
            next_var_id += 1;
            var_hh.insert((u, v), lit);
            var_hh.insert((v, u), lit);
        }

        let mut var_d1 = HashMap::new();
        for (&si, adj) in &decomp.strip_adj_hubs {
            for &h in adj {
                let lit = Var::new(next_var_id).pos_lit();
                next_var_id += 1;
                var_d1.insert((si, h), lit);
            }
        }

        let encoder = MacroMtzEncoder::encode(
            &mut solver,
            &mut next_var_id,
            &decomp,
            &var_hh,
            &var_d1,
        );

        // Activate Subcycle A:
        // HH(1,2) = true, Strip 0 active on 1 and 2
        let _ = solver.add_clause(clause![var_hh[&(1, 2)]]);
        let _ = solver.add_clause(clause![var_d1[&(0, 1)]]);
        let _ = solver.add_clause(clause![var_d1[&(0, 2)]]);

        // Activate Subcycle B:
        // HH(3,4) = true, Strip 1 active on 3 and 4
        let _ = solver.add_clause(clause![var_hh[&(3, 4)]]);
        let _ = solver.add_clause(clause![var_d1[&(1, 3)]]);
        let _ = solver.add_clause(clause![var_d1[&(1, 4)]]);

        // Deactivate connecting edges
        let _ = solver.add_clause(clause![!var_hh[&(2, 3)]]);
        let _ = solver.add_clause(clause![!var_hh[&(4, 1)]]);

        let res = solver.solve().expect("solve failed");
        assert_eq!(res, SolverResult::Unsat, "Disconnected strip subcycles must be UNSAT");
    }
}

#[test]
fn test_macro_mtz_mixed_strips_and_hh_connected() {
    // 4 Hubs: 1, 2, 3, 4 forming a single cycle:
    // 1 -> 2 (HH), 2 -> 3 (Strip 0), 3 -> 4 (HH), 4 -> 1 (Strip 1)
    let mut all_hubs = HashSet::new();
    for h in 1..=4 {
        all_hubs.insert(h);
    }

    let hh_edges = vec![(1, 2), (3, 4)];

    let mut strip_adj_hubs = HashMap::new();
    let mut s0 = HashSet::new();
    s0.insert(2);
    s0.insert(3);
    strip_adj_hubs.insert(0, s0);

    let mut s1 = HashSet::new();
    s1.insert(4);
    s1.insert(1);
    strip_adj_hubs.insert(1, s1);

    let decomp = DecompositionResult {
        s_hubs: vec![],
        b_hubs: vec![],
        m_hubs: vec![1, 2, 3, 4],
        all_hubs,
        hh_edges: hh_edges.clone(),
        strips: vec![vec![101], vec![102]],
        strip_adj_hubs,
        hub_adj_strips: HashMap::new(),
    };

    let mut solver = CaDiCaL::default();
    let mut next_var_id: u32 = 0;

    let mut var_hh = HashMap::new();
    for &(u, v) in &hh_edges {
        let lit = Var::new(next_var_id).pos_lit();
        next_var_id += 1;
        var_hh.insert((u, v), lit);
        var_hh.insert((v, u), lit);
    }

    let mut var_d1 = HashMap::new();
    for (&si, adj) in &decomp.strip_adj_hubs {
        for &h in adj {
            let lit = Var::new(next_var_id).pos_lit();
            next_var_id += 1;
            var_d1.insert((si, h), lit);
        }
    }

    let encoder = MacroMtzEncoder::encode(
        &mut solver,
        &mut next_var_id,
        &decomp,
        &var_hh,
        &var_d1,
    );

    // Assert active single cycle:
    // HH(1,2) = true
    let _ = solver.add_clause(clause![var_hh[&(1, 2)]]);
    // Strip 0 active on (2,3)
    let _ = solver.add_clause(clause![var_d1[&(0, 2)]]);
    let _ = solver.add_clause(clause![var_d1[&(0, 3)]]);
    let s0_23 = encoder.dir_strip_vars[&(0, 2, 3)];
    let _ = solver.add_clause(clause![s0_23]);

    // HH(3,4) = true
    let _ = solver.add_clause(clause![var_hh[&(3, 4)]]);
    // Strip 1 active on (4,1)
    let _ = solver.add_clause(clause![var_d1[&(1, 4)]]);
    let _ = solver.add_clause(clause![var_d1[&(1, 1)]]);
    let s1_41 = encoder.dir_strip_vars[&(1, 4, 1)];
    let _ = solver.add_clause(clause![s1_41]);

    let res = solver.solve().expect("solve failed");
    assert_eq!(res, SolverResult::Sat, "Connected mixed HH + Strip 4-cycle must be SAT");
}

#[test]
fn test_macro_mtz_empty_and_single_hub() {
    // Empty hubs
    {
        let mut solver = CaDiCaL::default();
        let mut next_var_id: u32 = 0;
        let decomp = DecompositionResult {
            s_hubs: vec![],
            b_hubs: vec![],
            m_hubs: vec![],
            all_hubs: HashSet::new(),
            hh_edges: vec![],
            strips: vec![],
            strip_adj_hubs: HashMap::new(),
            hub_adj_strips: HashMap::new(),
        };
        let encoder = MacroMtzEncoder::encode(
            &mut solver,
            &mut next_var_id,
            &decomp,
            &HashMap::new(),
            &HashMap::new(),
        );
        assert_eq!(encoder.root_hub, -1);
        assert!(encoder.hub_order_vars.is_empty());
    }

    // Single hub
    {
        let mut solver = CaDiCaL::default();
        let mut next_var_id: u32 = 0;
        let mut all_hubs = HashSet::new();
        all_hubs.insert(42);
        let decomp = DecompositionResult {
            s_hubs: vec![],
            b_hubs: vec![],
            m_hubs: vec![42],
            all_hubs,
            hh_edges: vec![],
            strips: vec![],
            strip_adj_hubs: HashMap::new(),
            hub_adj_strips: HashMap::new(),
        };
        let encoder = MacroMtzEncoder::encode(
            &mut solver,
            &mut next_var_id,
            &decomp,
            &HashMap::new(),
            &HashMap::new(),
        );
        assert_eq!(encoder.root_hub, 42);
        assert!(encoder.hub_order_vars.is_empty());
    }
}
