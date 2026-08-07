use std::collections::{BTreeMap, HashMap};

#[derive(Debug)]
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
}
