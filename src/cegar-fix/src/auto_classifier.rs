use crate::graph::Graph;
use std::collections::HashSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetTrack {
    B1LadderTwoTier,
    B2SinzChainSMT,
    GeneralCaDiCaL,
}

#[derive(Debug, Clone)]
pub struct TopologyFeatures {
    pub n: usize,
    pub m: usize,
    pub density: f64,
    pub max_degree: usize,
    pub hub_count: usize,
    pub degree2_count: usize,
}

pub struct AutoTopologyClassifier;

impl AutoTopologyClassifier {
    pub fn extract_features(g: &Graph) -> TopologyFeatures {
        let n = g.adjacency_list.len();
        let mut edges_set: HashSet<(i32, i32)> = HashSet::new();
        let mut max_degree = 0;
        let mut hub_count = 0;
        let mut degree2_count = 0;

        for (&u, nbrs) in &g.adjacency_list {
            let deg = nbrs.len();
            if deg > max_degree {
                max_degree = deg;
            }
            if deg >= 10 {
                hub_count += 1;
            }
            if deg == 2 {
                degree2_count += 1;
            }
            for &v in nbrs {
                let pair = if u < v { (u, v) } else { (v, u) };
                edges_set.insert(pair);
            }
        }

        let m = edges_set.len();
        let density = if n > 0 { m as f64 / n as f64 } else { 0.0 };

        TopologyFeatures {
            n,
            m,
            density,
            max_degree,
            hub_count,
            degree2_count,
        }
    }

    pub fn classify(features: &TopologyFeatures) -> TargetTrack {
        if features.hub_count >= 50 && features.density >= 2.8 {
            TargetTrack::B1LadderTwoTier
        } else if features.density <= 2.2 && features.n >= 1000 {
            TargetTrack::B2SinzChainSMT
        } else {
            TargetTrack::GeneralCaDiCaL
        }
    }
}
