use std::collections::HashSet;
use cegar_fix::contraction::Degree2Contractor;
use cegar_fix::file_operations;
use cegar_fix::modular_ring_dp_solver::ModularRingDecomposer;

#[test]
fn test_42_module_decomposition_graph868() {
    let graph_path = "FHCPCS-col/graph868.col";
    let alt_graph_path = "../../FHCPCS-col/graph868.col";
    let path = if std::path::Path::new(graph_path).exists() {
        graph_path
    } else if std::path::Path::new(alt_graph_path).exists() {
        alt_graph_path
    } else {
        "/home/ubuntu/HCP/FHCPCS-col/graph868.col"
    };

    let raw_g = file_operations::input_to_graph(path);
    let (g, contractor) = Degree2Contractor::contract(&raw_g);

    assert_eq!(g.adjacency_list.len(), 3696);
    assert_eq!(contractor.chain_map.len() / 2, 1848);

    let modules = ModularRingDecomposer::decompose(&g, &contractor)
        .expect("Decomposition must succeed for graph868");

    assert_eq!(modules.len(), 42, "Must decompose into exactly 42 modules");

    let mut seen_vertices = HashSet::new();
    let mut seen_ve = HashSet::new();

    for (i, m) in modules.iter().enumerate() {
        assert_eq!(m.id, i);
        assert_eq!(m.vertices.len(), 88, "Module {} must have 88 vertices", i);
        assert_eq!(m.virtual_edges.len(), 44, "Module {} must have 44 virtual edges", i);
        assert!(!m.ports_in.is_empty(), "Module {} must have ports_in", i);
        assert!(!m.ports_out.is_empty(), "Module {} must have ports_out", i);

        for &v in &m.vertices {
            assert!(seen_vertices.insert(v), "Duplicate vertex {} in module {}", v, i);
        }
        for &(u, w) in &m.virtual_edges {
            let ve = (u.min(w), u.max(w));
            assert!(seen_ve.insert(ve), "Duplicate VE {:?} in module {}", ve, i);
        }
    }

    assert_eq!(seen_vertices.len(), 3696);
    assert_eq!(seen_ve.len(), 1848);

    // Verify circular connectivity: ports_out of M_i connect to ports_in of M_{(i+1)%42}
    for i in 0..42 {
        let next_i = (i + 1) % 42;
        let next_in: HashSet<i32> = modules[next_i].ports_in.iter().copied().collect();
        let mut has_link = false;
        for &p_out in &modules[i].ports_out {
            if let Some(nbrs) = g.adjacency_list.get(&p_out) {
                for &nxt in nbrs {
                    if next_in.contains(&nxt) {
                        has_link = true;
                        break;
                    }
                }
            }
            if has_link { break; }
        }
        assert!(has_link, "Module {} must connect to module {} along the ring", i, next_i);
    }

    // Required Fix 2: Path Feasibility Smoke Test
    // Verify that Module 0 admits at least one internal Hamiltonian path between ports_in and ports_out
    let m0 = &modules[0];
    let m0_set: HashSet<i32> = m0.vertices.iter().copied().collect();
    let mut v_partner = std::collections::HashMap::new();
    for &(u, w) in &m0.virtual_edges {
        v_partner.insert(u, w);
        v_partner.insert(w, u);
    }
    let mut internal_real = Vec::new();
    for &u in &m0.vertices {
        if let Some(nbrs) = g.adjacency_list.get(&u) {
            for &v in nbrs {
                if u < v && m0_set.contains(&v) && v_partner.get(&u) != Some(&v) {
                    internal_real.push((u, v));
                }
            }
        }
    }

    let edge_vars: std::collections::HashMap<(i32, i32), u32> = internal_real
        .iter()
        .enumerate()
        .map(|(i, &e)| (e, i as u32))
        .collect();

    let mut hp_found = false;
    use rustsat::solvers::{Solve, SolverResult};
    use rustsat::types::{Clause, Lit, TernaryVal};
    use rustsat_cadical::CaDiCaL;

    for &pin in &m0.ports_in {
        for &pout in &m0.ports_out {
            let mut solver = CaDiCaL::default();
            for &u in &m0.vertices {
                let mut inc = Vec::new();
                if let Some(nbrs) = g.adjacency_list.get(&u) {
                    for &v in nbrs {
                        if m0_set.contains(&v) && v_partner.get(&u) != Some(&v) {
                            let e = (u.min(v), u.max(v));
                            inc.push(Lit::new(edge_vars[&e], false));
                        }
                    }
                }
                for i in 0..inc.len() {
                    for j in (i + 1)..inc.len() {
                        solver.add_clause(Clause::from_iter(vec![!inc[i], !inc[j]])).unwrap();
                    }
                }
                if u == pin || u == pout {
                    for var in inc {
                        solver.add_clause(Clause::from_iter(vec![!var])).unwrap();
                    }
                } else {
                    solver.add_clause(Clause::from_iter(inc)).unwrap();
                }
            }

            while solver.solve().unwrap() == SolverResult::Sat {
                let sol = solver.full_solution().unwrap();
                let active_edges: Vec<(i32, i32)> = internal_real
                    .iter()
                    .copied()
                    .filter(|e| sol.lit_value(Lit::new(edge_vars[e], false)) == TernaryVal::True)
                    .collect();

                let mut p_adj: std::collections::HashMap<i32, Vec<i32>> = std::collections::HashMap::new();
                for &u in &m0.vertices {
                    p_adj.entry(u).or_default().push(v_partner[&u]);
                }
                for &(u, v) in &active_edges {
                    p_adj.entry(u).or_default().push(v);
                    p_adj.entry(v).or_default().push(u);
                }

                let mut visited_p = HashSet::new();
                visited_p.insert(pin);
                let mut curr = pin;
                while let Some(nxt) = p_adj.get(&curr).and_then(|nbrs| nbrs.iter().find(|x| !visited_p.contains(x))) {
                    curr = *nxt;
                    visited_p.insert(curr);
                }

                if visited_p.len() == 88 && curr == pout {
                    hp_found = true;
                    break;
                } else {
                    let cut_lits: Vec<Lit> = active_edges
                        .iter()
                        .map(|e| !Lit::new(edge_vars[e], false))
                        .collect();
                    solver.add_clause(Clause::from_iter(cut_lits)).unwrap();
                }
            }
            if hp_found { break; }
        }
        if hp_found { break; }
    }

    assert!(hp_found, "Module 0 must admit at least one spanning Hamiltonian path between ports_in and ports_out");
}
