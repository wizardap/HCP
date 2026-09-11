#![allow(unused_variables, unused_imports, unused_mut, unused_assignments)]

use cegar_fix::backbone_freezer::{BackboneFreezer, FreezerOptions};
use cegar_fix::graph::Graph;
use cegar_fix::encoder::Encoder;
use cegar_fix::contraction::Degree2Contractor;
use std::collections::HashMap;

trait EncoderExt {
    fn encode_at_least_one(&mut self, g: &Graph);
}

impl EncoderExt for Encoder {
    fn encode_at_least_one(&mut self, g: &Graph) {
        let _ = self.encode(g, 0, 0, 0, 0, 0, 0);
    }
}

#[test]
fn test_topologically_bounded_freezing_formula() {
    // Construct a test graph with a 12-cycle and 2 peripheral vertices (N=14)
    let mut g = Graph::new();
    // Giant cycle 1..12
    for i in 1..=12 {
        let nxt = if i == 12 { 1 } else { i + 1 };
        g.add_edge(i, nxt);
    }
    // Peripheral 13 connects to 3, 4
    g.add_edge(13, 3);
    g.add_edge(13, 4);
    // Peripheral 14 connects to 7, 8
    g.add_edge(14, 7);
    g.add_edge(14, 8);
    // Edge (13, 14)
    g.add_edge(13, 14);

    let mut encoder = Encoder::new();
    encoder.encode_at_least_one(&g);

    let cycles = vec![
        (1..=12).collect::<Vec<i32>>(),
        vec![13, 14],
    ];

    let contractor = Degree2Contractor::new();
    let mut opts = FreezerOptions::default();
    opts.ratio_threshold = 0.5;

    // With 1 peripheral cycle, formula: N_frozen <= L_giant - 2 * C_peripheral = 12 - 2*1 = 10
    let assumptions = BackboneFreezer::select_topologically_bounded_assumptions(
        &cycles,
        &g,
        &encoder,
        0.0,
    );

    // Must NOT freeze >= 11 edges (which causes false UNSAT)
    assert!(assumptions.len() <= 10, "Frozen edges {} exceeded dynamic bound 10", assumptions.len());
    println!("Verified H2-Refined dynamic bound: {} edges frozen", assumptions.len());
}
