// tests/test_gadget_path_absorber.rs
use cegar_fix::gadget_path_absorber::GadgetPathAbsorber;
use cegar_fix::graph::Graph;
use std::collections::HashSet;

#[test]
fn test_already_single_cycle() {
    let mut g = Graph::new();
    let n = 20;
    for i in 0..n {
        g.add_edge(i, (i + 1) % n);
    }
    let cycles = vec![(0..n).collect()];
    let protected = HashSet::new();

    let res = GadgetPathAbsorber::try_absorb_gadgets(&cycles, &g, &protected);
    assert_eq!(res.len(), 1);
    assert_eq!(res[0].len(), n as usize);

    // Also single small cycle
    let mut g_small = Graph::new();
    for i in 0..6 {
        g_small.add_edge(i, (i + 1) % 6);
    }
    let cycles_small = vec![(0..6).collect()];
    let res_small = GadgetPathAbsorber::try_absorb_gadgets(&cycles_small, &g_small, &protected);
    assert_eq!(res_small.len(), 1);
    assert_eq!(res_small[0].len(), 6);
}

#[test]
fn test_single_gadget_absorption() {
    // 20-vertex large cycle + 8-vertex satellite gadget
    let mut g = Graph::new();
    // Large cycle: 0..20
    for i in 0..20 {
        g.add_edge(i, (i + 1) % 20);
    }
    // Gadget: 20..28
    for i in 20..28 {
        let nxt = if i == 27 { 20 } else { i + 1 };
        g.add_edge(i, nxt);
    }
    // Connecting edges to large cycle at edge (0, 1):
    // (0, 20) and (27, 1)
    g.add_edge(0, 20);
    g.add_edge(27, 1);

    let cycles = vec![
        (0..20).collect::<Vec<i32>>(),
        (20..28).collect::<Vec<i32>>(),
    ];
    let protected = HashSet::new();

    let res = GadgetPathAbsorber::try_absorb_gadgets(&cycles, &g, &protected);
    assert_eq!(res.len(), 1, "Should absorb gadget into single cycle");
    assert_eq!(res[0].len(), 28, "Resulting cycle should contain 28 vertices");

    // Verify all vertices are unique and all consecutive pairs are edges in G
    let cycle = &res[0];
    let set: HashSet<i32> = cycle.iter().cloned().collect();
    assert_eq!(set.len(), 28);
    for i in 0..28 {
        let u = cycle[i];
        let v = cycle[(i + 1) % 28];
        assert!(g.adjacency_list.get(&u).unwrap().contains(&v), "Edge ({}, {}) must exist in G", u, v);
    }
}

#[test]
fn test_multiple_gadget_absorption() {
    // Large cycle: 20 vertices (0..20)
    // 3 satellite gadgets:
    // Gadget 1: 4 vertices (20..24), connecting to edge (2, 3) via (2, 20) and (23, 3)
    // Gadget 2: 4 vertices (24..28), connecting to edge (7, 8) via (7, 24) and (27, 8)
    // Gadget 3: 4 vertices (28..32), connecting to edge (12, 13) via (12, 28) and (31, 13)
    let mut g = Graph::new();
    for i in 0..20 {
        g.add_edge(i, (i + 1) % 20);
    }
    for &base in &[20, 24, 28] {
        for i in 0..4 {
            let u = base + i;
            let v = base + (i + 1) % 4;
            g.add_edge(u, v);
        }
    }
    // Gadget 1 connections:
    g.add_edge(2, 20);
    g.add_edge(23, 3);
    // Gadget 2 connections:
    g.add_edge(7, 24);
    g.add_edge(27, 8);
    // Gadget 3 connections:
    g.add_edge(12, 28);
    g.add_edge(31, 13);

    let cycles = vec![
        (0..20).collect::<Vec<i32>>(),
        (20..24).collect::<Vec<i32>>(),
        (24..28).collect::<Vec<i32>>(),
        (28..32).collect::<Vec<i32>>(),
    ];
    let protected = HashSet::new();

    let res = GadgetPathAbsorber::try_absorb_gadgets(&cycles, &g, &protected);
    assert_eq!(res.len(), 1, "All 3 gadgets should be absorbed into 1 cycle");
    assert_eq!(res[0].len(), 32, "Total cycle length should be 32");

    let cycle = &res[0];
    let set: HashSet<i32> = cycle.iter().cloned().collect();
    assert_eq!(set.len(), 32);
    for i in 0..32 {
        let u = cycle[i];
        let v = cycle[(i + 1) % 32];
        assert!(g.adjacency_list.get(&u).unwrap().contains(&v), "Edge ({}, {}) must exist in G", u, v);
    }
}

#[test]
fn test_protected_edge_preservation() {
    let mut g = Graph::new();
    // Large cycle: 20 vertices (0..20)
    for i in 0..20 {
        g.add_edge(i, (i + 1) % 20);
    }
    // Gadget: 8 vertices (20..28)
    for i in 20..28 {
        let nxt = if i == 27 { 20 } else { i + 1 };
        g.add_edge(i, nxt);
    }

    // Connect gadget to edge (0, 1) and also edge (5, 6)
    g.add_edge(0, 20);
    g.add_edge(27, 1);

    g.add_edge(5, 20);
    g.add_edge(27, 6);

    let cycles = vec![
        (0..20).collect::<Vec<i32>>(),
        (20..28).collect::<Vec<i32>>(),
    ];

    // Protect edge (0, 1)
    let mut protected = HashSet::new();
    protected.insert((0, 1));
    protected.insert((1, 0));

    let res = GadgetPathAbsorber::try_absorb_gadgets(&cycles, &g, &protected);
    assert_eq!(res.len(), 1, "Should absorb via edge (5, 6) since (0, 1) is protected");
    assert_eq!(res[0].len(), 28);

    // Verify (0, 1) is still an edge in the resulting cycle!
    let cycle = &res[0];
    let mut has_protected_edge = false;
    for i in 0..28 {
        let u = cycle[i];
        let v = cycle[(i + 1) % 28];
        if (u == 0 && v == 1) || (u == 1 && v == 0) {
            has_protected_edge = true;
            break;
        }
    }
    assert!(has_protected_edge, "Protected edge (0, 1) must be preserved in the cycle");

    // If both (0, 1) and (5, 6) are protected, absorption must fail and keep 2 cycles
    protected.insert((5, 6));
    protected.insert((6, 5));
    let res2 = GadgetPathAbsorber::try_absorb_gadgets(&cycles, &g, &protected);
    assert_eq!(res2.len(), 2, "Should NOT absorb when all candidate edges are protected");
}
