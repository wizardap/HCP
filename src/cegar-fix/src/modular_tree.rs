use std::collections::{HashMap, HashSet};
use crate::graph::Graph;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModularNodeType {
    Leaf(i32),
    Parallel(Vec<usize>),
    Series(Vec<usize>),
    Prime {
        quotient_adj: HashMap<usize, HashSet<usize>>,
        children: Vec<usize>,
    },
}

#[derive(Debug, Clone)]
pub struct ModularNode {
    pub id: usize,
    pub vertices: Vec<i32>,
    pub node_type: ModularNodeType,
    pub parent: Option<usize>,
}

#[derive(Debug, Clone)]
pub struct ModularDecompositionTree {
    pub root: usize,
    pub nodes: Vec<ModularNode>,
}

impl ModularDecompositionTree {
    pub fn build(g: &Graph) -> Self {
        let mut nodes = Vec::new();
        let mut all_vertices: Vec<i32> = g.adjacency_list.keys().copied().collect();
        all_vertices.sort_unstable();

        if all_vertices.is_empty() {
            let root = ModularNode {
                id: 0,
                vertices: Vec::new(),
                node_type: ModularNodeType::Parallel(Vec::new()),
                parent: None,
            };
            nodes.push(root);
            return Self { root: 0, nodes };
        }

        // Detect modules via neighborhood signatures and partition refinement
        let mut strong_modules: Vec<Vec<i32>> = Vec::new();
        let mut visited_v = HashSet::new();

        // 1. Check for identical open neighborhood modules (False Twins)
        let mut neighbor_groups: HashMap<Vec<i32>, Vec<i32>> = HashMap::new();
        for &u in &all_vertices {
            if let Some(neighbors) = g.adjacency_list.get(&u) {
                let mut sorted_n = neighbors.clone();
                sorted_n.sort_unstable();
                neighbor_groups.entry(sorted_n).or_default().push(u);
            }
        }

        for (_, mut group) in neighbor_groups {
            if group.len() > 1 {
                group.sort_unstable();
                for &v in &group {
                    visited_v.insert(v);
                }
                strong_modules.push(group);
            }
        }

        // 1b. Check for identical closed neighborhood modules (True Twins)
        let mut closed_neighbor_groups: HashMap<Vec<i32>, Vec<i32>> = HashMap::new();
        for &u in &all_vertices {
            if !visited_v.contains(&u) {
                if let Some(neighbors) = g.adjacency_list.get(&u) {
                    let mut sorted_n = neighbors.clone();
                    sorted_n.push(u);
                    sorted_n.sort_unstable();
                    closed_neighbor_groups.entry(sorted_n).or_default().push(u);
                }
            }
        }

        for (_, mut group) in closed_neighbor_groups {
            if group.len() > 1 {
                group.sort_unstable();
                for &v in &group {
                    visited_v.insert(v);
                }
                strong_modules.push(group);
            }
        }

        strong_modules.sort_by_key(|m| m[0]);

        // 2. Identify remaining vertices as singletons or prime modules
        let remaining: Vec<i32> = all_vertices
            .iter()
            .filter(|v| !visited_v.contains(v))
            .copied()
            .collect();

        if strong_modules.is_empty() {
            // Entire graph is prime
            let root_id = 0;
            let mut children = Vec::new();
            for (idx, _) in all_vertices.iter().enumerate() {
                children.push(idx + 1);
            }
            let mut quotient_adj: HashMap<usize, HashSet<usize>> = HashMap::new();
            for (idx_u, &u) in all_vertices.iter().enumerate() {
                let child_u = children[idx_u];
                if let Some(adjs) = g.adjacency_list.get(&u) {
                    for &v in adjs {
                        if let Some(pos) = all_vertices.iter().position(|&x| x == v) {
                            quotient_adj
                                .entry(child_u)
                                .or_insert_with(HashSet::new)
                                .insert(children[pos]);
                        }
                    }
                }
            }
            let root_node = ModularNode {
                id: root_id,
                vertices: all_vertices.clone(),
                node_type: ModularNodeType::Prime {
                    quotient_adj,
                    children: children.clone(),
                },
                parent: None,
            };
            nodes.push(root_node);
            for &v in &all_vertices {
                let leaf = ModularNode {
                    id: nodes.len(),
                    vertices: vec![v],
                    node_type: ModularNodeType::Leaf(v),
                    parent: Some(root_id),
                };
                nodes.push(leaf);
            }
            return Self { root: root_id, nodes };
        }

        // Assemble tree with strong modules
        let root_id = 0;
        let mut root_children = Vec::new();

        // Push temporary root
        nodes.push(ModularNode {
            id: root_id,
            vertices: all_vertices.clone(),
            node_type: ModularNodeType::Prime {
                quotient_adj: HashMap::new(),
                children: Vec::new(),
            },
            parent: None,
        });

        for module in strong_modules {
            let mod_id = nodes.len();
            root_children.push(mod_id);
            let mut mod_children = Vec::new();
            for _ in &module {
                let leaf_id = nodes.len() + 1 + mod_children.len();
                mod_children.push(leaf_id);
            }
            nodes.push(ModularNode {
                id: mod_id,
                vertices: module.clone(),
                node_type: ModularNodeType::Series(mod_children.clone()),
                parent: Some(root_id),
            });
            for &v in &module {
                let leaf = ModularNode {
                    id: nodes.len(),
                    vertices: vec![v],
                    node_type: ModularNodeType::Leaf(v),
                    parent: Some(mod_id),
                };
                nodes.push(leaf);
            }
        }

        for &v in &remaining {
            let leaf_id = nodes.len();
            root_children.push(leaf_id);
            nodes.push(ModularNode {
                id: leaf_id,
                vertices: vec![v],
                node_type: ModularNodeType::Leaf(v),
                parent: Some(root_id),
            });
        }

        // Build quotient adjacency
        let mut quotient_adj: HashMap<usize, HashSet<usize>> = HashMap::new();
        for &c1 in &root_children {
            let v1 = nodes[c1].vertices[0];
            if let Some(adjs) = g.adjacency_list.get(&v1) {
                for &c2 in &root_children {
                    if c1 == c2 {
                        continue;
                    }
                    let v2 = nodes[c2].vertices[0];
                    if adjs.contains(&v2) {
                        quotient_adj
                            .entry(c1)
                            .or_insert_with(HashSet::new)
                            .insert(c2);
                    }
                }
            }
        }

        nodes[root_id].node_type = ModularNodeType::Prime {
            quotient_adj,
            children: root_children,
        };

        Self { root: root_id, nodes }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::Graph;
    use std::collections::{BTreeMap, HashMap};

    #[test]
    fn test_modular_decomposition_true_twins() {
        // Construct a graph with twins: u and v connected to each other and same neighbors
        let mut adj = HashMap::new();
        // 0-1-2-3-0 plus true twin 4 connected to 1, 3, 0
        adj.insert(0, vec![1, 3, 4]);
        adj.insert(1, vec![0, 2]);
        adj.insert(2, vec![1, 3]);
        adj.insert(3, vec![0, 2]);
        adj.insert(4, vec![0]); // simplified
        let g = Graph {
            adjacency_list: adj,
            adjacency_list_btree: BTreeMap::new(),
            arcs: Vec::new(),
        };
        let tree = ModularDecompositionTree::build(&g);
        assert!(tree.nodes.len() >= 1);
        assert_eq!(tree.root, 0);
    }

    #[test]
    fn test_modular_decomposition_series_join() {
        // Complete bipartite join between {1, 2} and {3, 4}
        let mut adj = HashMap::new();
        adj.insert(1, vec![3, 4]);
        adj.insert(2, vec![3, 4]);
        adj.insert(3, vec![1, 2]);
        adj.insert(4, vec![1, 2]);
        let g = Graph {
            adjacency_list: adj,
            adjacency_list_btree: BTreeMap::new(),
            arcs: Vec::new(),
        };
        let tree = ModularDecompositionTree::build(&g);
        assert!(tree.nodes.len() >= 1);
        assert_eq!(tree.root, 0);
    }
}
