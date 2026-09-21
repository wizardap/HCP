use std::collections::{BTreeMap, HashMap};

#[derive(Debug, Clone)]
pub struct Graph {
    pub adjacency_list: HashMap<i32, Vec<i32>>, //キーにノード、値に接続されているノードの集合
    pub adjacency_list_btree: BTreeMap<i32,Vec<i32>>,
    pub arcs: Vec<(i32, i32)>,                  //
}

impl Graph {
    pub fn new() -> Self {
        Self {
            adjacency_list: HashMap::new(),
            adjacency_list_btree: BTreeMap::new(),
            arcs: Vec::new(),
        }
    }

    pub fn add_edge(&mut self, u: i32, v: i32) {
        self.adjacency_list
            .entry(u)
            .or_insert_with(Vec::new)
            .push(v);
        self.adjacency_list
            .entry(v)
            .or_insert_with(Vec::new)
            .push(u);
        self.arcs.push((u, v));
        self.arcs.push((v, u));

        self.adjacency_list_btree
        .entry(u)
        .or_insert_with(Vec::new)
        .push(v);
        self.adjacency_list_btree
            .entry(v)
            .or_insert_with(Vec::new)
            .push(u);
    }

    pub fn get_highest_degree_vertex(&self) -> i32 {
        let mut max_degree = 0;
        let mut vertex = 0;

        for (v, neighbors) in &self.adjacency_list_btree {
            if neighbors.len() > max_degree {
                max_degree = neighbors.len();
                vertex = *v;
            }
        }

        vertex
    }

    pub fn get_lowest_degree_vertex(&self) -> i32 {
        let mut min_degree = std::usize::MAX;
        let mut vertex = 0;

        for (v, neighbors) in &self.adjacency_list_btree {
            if neighbors.len() < min_degree {
                min_degree = neighbors.len();
                vertex = *v;
            }
        }

        vertex
    }

    pub fn remove_redundant_arcs(&mut self) {
        let mut count = 0;
        loop {
            let mut to_remove = None;
    
            for (v, adjs) in self.adjacency_list_btree.iter() {
                if let Some((n1, n2)) = self.is_vertex_with_redundant_arcs(&adjs) {
                    to_remove = Some((*v, n1, n2));
                    count += 1;                    
                    break;
                }
            }
            
            match to_remove{
                Some((v,n1,n2)) => self.remove_arcs(&v, n1, n2),
                None => break
            }
        }
        println!("Number of vertices changed to degree two = {count}");
        
    }

    fn is_vertex_with_redundant_arcs(&self,adjs:&Vec<i32>) -> Option<(i32,i32)>{
        if adjs.len() == 2{
            return None
        }
        let mut two_degree_verties = Vec::new();
        for u in adjs.iter(){
            let u_adjs = self.adjacency_list.get(u);
            if u_adjs?.len() == 2{
                two_degree_verties.push(*u);
                if two_degree_verties.len() == 2{
                    return Some((two_degree_verties[0],two_degree_verties[1]))
                }
            }
        }
        return None
    }

    fn remove_arcs(&mut self,v:&i32,n1:i32,n2:i32){
        if let Some(v_adj) = self.adjacency_list.get_mut(v) {
            let mut new_adj = Vec::new();
            let mut to_remove = Vec::new();

            for u in v_adj {
                if *u == n1 || *u == n2 {
                    new_adj.push(*u);
                } else {
                    to_remove.push(*u);
                }
            }

            // 更新されたadjを追加
            self.adjacency_list.insert(*v, new_adj.clone());
            self.adjacency_list_btree.insert(*v, new_adj);

            for &u in &to_remove {
                if let Some(another_adj) = self.adjacency_list.get_mut(&u) {
                    another_adj.retain(|&x| x != *v);
                }

                if let Some(another_adj) = self.adjacency_list_btree.get_mut(&u) {
                    another_adj.retain(|&x| x != *v);
                }

            }
        }



        // Remove arcs containing v but not containing n1 or n2
        self.arcs.retain(|&(a, b)| !(a == *v && b != n1 && b != n2) && !(b == *v && a != n1 && a != n2));
    }

    pub fn induced_subgraph(&self, vertices: &std::collections::HashSet<i32>) -> Graph {
        let mut new_adjacency = std::collections::BTreeMap::new();
        let mut new_arcs = Vec::new();

        for &u in vertices {
            if let Some(adjs) = self.adjacency_list.get(&u) {
                let filtered_adjs: Vec<i32> = adjs.iter().filter(|v| vertices.contains(v)).cloned().collect();
                new_adjacency.insert(u, filtered_adjs.clone());
                for v in filtered_adjs {
                    new_arcs.push((u, v));
                }
            }
        }

        let new_adjacency_hash: std::collections::HashMap<i32, Vec<i32>> = new_adjacency
            .iter()
            .map(|(&k, v)| (k, v.clone()))
            .collect();

        Graph {
            adjacency_list: new_adjacency_hash,
            adjacency_list_btree: new_adjacency,
            arcs: new_arcs,
        }
    }

    /// If vertex v has degree 2 with neighbors u and w, and edge (u, w) exists (with |V| > 3),
    /// edge (u, w) cannot be part of any Hamiltonian cycle because choosing (u, w) isolates u-v-w-u.
    pub fn prune_degree2_triangles(&mut self) -> usize {
        let n = self.adjacency_list_btree.len();
        if n <= 3 {
            return 0;
        }
        let mut total_pruned = 0;
        loop {
            let mut edges_to_remove = Vec::new();
            for (_, neighbors) in self.adjacency_list_btree.iter() {
                if neighbors.len() == 2 {
                    let u = neighbors[0];
                    let w = neighbors[1];
                    if let Some(u_neighbors) = self.adjacency_list.get(&u) {
                        if u_neighbors.contains(&w) {
                            edges_to_remove.push((u, w));
                        }
                    }
                }
            }
            if edges_to_remove.is_empty() {
                break;
            }
            let mut count = 0;
            for (u, w) in edges_to_remove {
                if self.remove_edge_if_exists(u, w) {
                    count += 1;
                }
            }
            if count == 0 {
                break;
            }
            total_pruned += count;
        }
        total_pruned
    }

    pub fn remove_edge_if_exists(&mut self, u: i32, w: i32) -> bool {
        let mut removed = false;
        if let Some(u_list) = self.adjacency_list.get_mut(&u) {
            if let Some(pos) = u_list.iter().position(|&x| x == w) {
                u_list.remove(pos);
                removed = true;
            }
        }
        if let Some(w_list) = self.adjacency_list.get_mut(&w) {
            if let Some(pos) = w_list.iter().position(|&x| x == u) {
                w_list.remove(pos);
            }
        }
        if let Some(u_list) = self.adjacency_list_btree.get_mut(&u) {
            if let Some(pos) = u_list.iter().position(|&x| x == w) {
                u_list.remove(pos);
            }
        }
        if let Some(w_list) = self.adjacency_list_btree.get_mut(&w) {
            if let Some(pos) = w_list.iter().position(|&x| x == u) {
                w_list.remove(pos);
            }
        }
        self.arcs.retain(|&(a, b)| !((a == u && b == w) || (a == w && b == u)));
        removed
    }

    /// Returns true if removing any single vertex disconnects the graph (instant UNSAT for HCP)
    /// or if the graph is already disconnected.
    pub fn has_articulation_points(&self) -> bool {
        let vertices: Vec<i32> = self.adjacency_list_btree.keys().copied().collect();
        let n = vertices.len();
        if n <= 2 {
            return false;
        }

        let mut tin: HashMap<i32, usize> = HashMap::new();
        let mut low: HashMap<i32, usize> = HashMap::new();
        let mut timer = 0;
        let mut is_cut = false;

        fn dfs(
            v: i32,
            p: i32,
            adj: &HashMap<i32, Vec<i32>>,
            tin: &mut HashMap<i32, usize>,
            low: &mut HashMap<i32, usize>,
            timer: &mut usize,
            is_cut: &mut bool,
        ) {
            *timer += 1;
            tin.insert(v, *timer);
            low.insert(v, *timer);
            let mut children = 0;
            if let Some(neighbors) = adj.get(&v) {
                for &to in neighbors {
                    if to == p {
                        continue;
                    }
                    if tin.contains_key(&to) {
                        let to_tin = *tin.get(&to).unwrap();
                        let v_low = low.get_mut(&v).unwrap();
                        *v_low = std::cmp::min(*v_low, to_tin);
                    } else {
                        dfs(to, v, adj, tin, low, timer, is_cut);
                        let to_low = *low.get(&to).unwrap();
                        let v_low = low.get_mut(&v).unwrap();
                        *v_low = std::cmp::min(*v_low, to_low);
                        if to_low >= *tin.get(&v).unwrap() && p != -1 {
                            *is_cut = true;
                        }
                        children += 1;
                    }
                }
            }
            if p == -1 && children > 1 {
                *is_cut = true;
            }
        }

        dfs(
            vertices[0],
            -1,
            &self.adjacency_list,
            &mut tin,
            &mut low,
            &mut timer,
            &mut is_cut,
        );

        // Also check if graph is disconnected
        if tin.len() < n {
            return true;
        }
        is_cut
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prune_degree2_triangles() {
        let mut g = Graph::new();
        // Graph with 4 nodes: (1,2), (2,3), (1,3), (1,4), (3,4)
        // Node 2 has degree 2 with neighbors 1 and 3. Edge (1, 3) is a shortcut in a degree-2 triangle.
        g.add_edge(1, 2);
        g.add_edge(2, 3);
        g.add_edge(1, 3);
        g.add_edge(1, 4);
        g.add_edge(3, 4);

        let pruned = g.prune_degree2_triangles();
        assert_eq!(pruned, 1);
        assert!(!g.adjacency_list.get(&1).unwrap().contains(&3));
        assert!(!g.adjacency_list.get(&3).unwrap().contains(&1));
        assert!(!g.arcs.contains(&(1, 3)));
        assert!(!g.arcs.contains(&(3, 1)));
    }

    #[test]
    fn test_has_articulation_points_cycle() {
        let mut g = Graph::new();
        // 4-cycle: 1-2-3-4-1 (no cut vertices)
        g.add_edge(1, 2);
        g.add_edge(2, 3);
        g.add_edge(3, 4);
        g.add_edge(4, 1);

        assert!(!g.has_articulation_points());
    }

    #[test]
    fn test_has_articulation_points_cut_vertex() {
        let mut g = Graph::new();
        // Two triangles joined at node 3: (1,2,3) and (3,4,5)
        g.add_edge(1, 2);
        g.add_edge(2, 3);
        g.add_edge(3, 1);
        g.add_edge(3, 4);
        g.add_edge(4, 5);
        g.add_edge(5, 3);

        assert!(g.has_articulation_points());
    }

    #[test]
    fn test_has_articulation_points_disconnected() {
        let mut g = Graph::new();
        // Disconnected graph: 1-2 and 3-4
        g.add_edge(1, 2);
        g.add_edge(3, 4);

        assert!(g.has_articulation_points());
    }
}

