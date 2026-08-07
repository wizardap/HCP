use crate::graph::*;
use rustsat::clause;
use rustsat::instances::*;
use rustsat::types::*;
use std::collections::{BTreeMap, BTreeSet, HashMap};

pub struct Encoder {
    pub graph_lit_map: BTreeMap<(i32, i32), Lit>, //キー：有向辺i->j (i,j)、値：s_i_jのリテラル
    pub new_lit_map: BTreeMap<(usize, usize), Lit>, //キー：？？、値：s_i_jのリテラル
    pub edge_map: HashMap<(i32,i32),Lit>,
    pub instance: SatInstance,
}

impl Encoder {
    pub fn new() -> Self {
        Self {
            graph_lit_map: BTreeMap::new(),
            new_lit_map: BTreeMap::new(),
            edge_map: HashMap::new(),
            instance: SatInstance::new(),
        }
    }

    pub fn encode(&mut self, g: &Graph, method: i32, symmetry: i32, loop_prohibition: i32, dearcify: i32, degree_order:i32, arcs_order:i32) -> Cnf {
        let mut cnf = Cnf::new();
        //有向辺に対応するリテラルを作成
        let arcs = match arcs_order{
            1 => {
                let mut sorted_arcs = g.arcs.clone();
                sorted_arcs.sort_by(|a, b| { 
                    let min_a = std::cmp::min(g.adjacency_list.get(&a.0).unwrap().len(), g.adjacency_list.get(&a.1).unwrap().len()); 
                    let min_b = std::cmp::min(g.adjacency_list.get(&b.0).unwrap().len(), g.adjacency_list.get(&b.1).unwrap().len()); 
                    if min_a == min_b { 
                        let other_a = std::cmp::max(g.adjacency_list.get(&a.0).unwrap().len(), g.adjacency_list.get(&a.1).unwrap().len()); 
                        let other_b = std::cmp::max(g.adjacency_list.get(&b.0).unwrap().len(), g.adjacency_list.get(&b.1).unwrap().len()); 
                        other_a.cmp(&other_b) // 昇順 
                    } else {
                        min_a.cmp(&min_b) // 昇順
                    } 
                });
                sorted_arcs
            },
            2 => {
                let mut sorted_arcs = g.arcs.clone();
                sorted_arcs.sort_by(|a, b| { 
                    let max_a = std::cmp::max(g.adjacency_list.get(&a.0).unwrap().len(), g.adjacency_list.get(&a.1).unwrap().len()); 
                    let max_b = std::cmp::max(g.adjacency_list.get(&b.0).unwrap().len(), g.adjacency_list.get(&b.1).unwrap().len()); 
                    if max_a == max_b { 
                        let other_a = std::cmp::min(g.adjacency_list.get(&a.0).unwrap().len(), g.adjacency_list.get(&a.1).unwrap().len()); 
                        let other_b = std::cmp::min(g.adjacency_list.get(&b.0).unwrap().len(), g.adjacency_list.get(&b.1).unwrap().len()); 
                        other_b.cmp(&other_a) // 降順 
                    } else {
                         max_b.cmp(&max_a) // 降順 
                    } 
                });
                sorted_arcs
            }
            _ => g.arcs.clone(),   
        };

        for &(u, v) in arcs.iter() {
            let lit = self.instance.new_lit();
            self.graph_lit_map.insert((u, v), lit);
        }

        let neighbors_zip = match degree_order{
            0 => g.adjacency_list_btree.iter().collect::<Vec<_>>(),
            1 => {
                let mut map_vec = g.adjacency_list_btree.iter().collect::<Vec<_>>();
                //昇順に並べる
                map_vec.sort_by(|a, b| a.1.len().cmp(&b.1.len()));
                map_vec
            },
            2 => {
                let mut map_vec = g.adjacency_list_btree.iter().collect::<Vec<_>>();
                //昇順に並べる
                map_vec.sort_by(|a, b| b.1.len().cmp(&a.1.len()));
                map_vec
            },
            _ => vec!()
        };

        //次数制約をcnfに追加
        for (&u, neighbors) in neighbors_zip {
            let mut out_lits = Vec::new();
            let mut in_lits = Vec::new();
            for &v in neighbors {
                out_lits.push(self.graph_lit_map[&(u, v)]);
                in_lits.push(self.graph_lit_map[&(v, u)]);
            }
            
            if method == 2 || method == 6{
            //入次数と出次数のexact-oneをcnfに追加する
                cnf.extend( self.exact_one(out_lits, method));
                cnf.extend(self.exact_one(in_lits, method));
            }else{
                //入次数と出次数のat-most-oneをcnfに追加する
                cnf.extend(self.at_most_one(out_lits.clone(), method));
                cnf.extend(self.at_most_one(in_lits.clone(), method));

                //入次数と出次数のat-least-oneをcnfに追加する
                cnf.add_clause(Encoder::at_least_one(out_lits));
                cnf.add_clause(Encoder::at_least_one(in_lits));
        }
            }



        if loop_prohibition == 1{
            //二つの頂点だけのループを防ぐ
            cnf.extend(self.two_loop_prohibition(&g));
        }else if loop_prohibition == 2{
            //三つの頂点だけのループを防ぐ
            cnf.extend(self.three_loop_prohibition(&g));
        }else if loop_prohibition == 3{
            //二つの頂点のループと三つの頂点のループを防ぐ
            cnf.extend(self.two_loop_prohibition(&g));
            cnf.extend(self.three_loop_prohibition(&g));
        }

        //対称性の除去
        if symmetry == 1 {
            //次数が一番小さい頂点に対して対称性を禁止する
            let lowest_vertex = g.get_lowest_degree_vertex();
            cnf.extend(self.asymmetry(lowest_vertex, &g.adjacency_list[&lowest_vertex],symmetry));
        } else if symmetry == 2 {
            //次数が一番大きい頂点に対して対称性を禁止する
            let highest_vertex = g.get_highest_degree_vertex();
            cnf.extend(self.asymmetry(highest_vertex, &g.adjacency_list[&highest_vertex],symmetry));
        } else if symmetry == 3{
            //次数が一番小さい頂点に対してサポート節を加える
            let lowest_vertex = g.get_lowest_degree_vertex();
            cnf.extend(self.asymmetry(lowest_vertex, &g.adjacency_list[&lowest_vertex],symmetry))
        }

        if dearcify > 1 {
            cnf.extend(self.dearcify(g, dearcify));
        }

        cnf
    }

    fn at_most_one(&mut self, lits: Vec<Lit>, method: i32) -> Vec<Clause> {
        let clauses = if method == 0 {
            Encoder::binominal(lits)
        } else if method == 1 {
            self.sinz_k(lits, 1)
        }else if method == 3{
            self.sinz_advance(lits, 1)
        }else if method == 4 || method == 5{
            self.product(lits,method)
        }else{
            panic!("out_of_number for encoding method\nbinominal: -e 0\nsinz: -e 1 \nadder: -e 2 \nsinz advance: -e 3")
        };
        clauses
    }

    fn exact_one(&mut self,lits:Vec<Lit>,method:i32) -> Vec<Clause>{
        let clauses = match method
        {
            2 => self.bailleux_k(lits, 1),
            6 => self.ladder(lits),
            _ => panic!(),
        };
        clauses
    }

    //at-most-oneのbinominal符号化
    fn binominal(lits: Vec<Lit>) -> Vec<Clause> {
        let mut clauses = Vec::new();

        for i in 0..lits.len() {
            for j in i + 1..lits.len() {
                let clause = clause!(!lits[i].clone(), !lits[j].clone());
                clauses.push(clause);
            }
        }
        clauses
    }

    //at-most-kのsinz符号化
    fn sinz_k(&mut self, lits: Vec<Lit>, k: usize) -> Vec<Clause> {
        let n = lits.len();
        let mut clauses = Vec::new();
        for i in 1..n {
            for j in 1..=k {
                let lit = self.instance.new_lit();
                self.new_lit_map.insert((i, j), lit);
            }
        }
        // -x1 v s1,1
        clauses.push(clause!(!lits[0], self.s(1, 1)));

        // -s1,j
        for j in 2..=k {
            clauses.push(clause!(!self.s(1, j)));
        }

        for i in 2..n {
            // -xi v si,1
            clauses.push(clause!(!lits[i - 1], self.s(i, 1)));
            // -si-1,1 v si,1
            clauses.push(clause!(!self.s(i - 1, 1), self.s(i, 1)));

            for j in 2..=k {
                // -xi v -si-1,j-1 v si,j
                clauses.push(clause!(!lits[i - 1], !self.s(i - 1, j - 1), self.s(i, j)));
                // -si-1,j v si,j
                clauses.push(clause!(!self.s(i - 1, j), self.s(i, j)));
            }

            // -xi v -si-1,k
            clauses.push(clause!(!lits[i - 1], !self.s(i - 1, k)));
        }

        // -xn v s_n-1,k
        clauses.push(clause!(!lits[n - 1], !self.s(n - 1, k)));
        clauses
    }

    //sinz符号化の際のs_i_jを表すリテラルを返す関数
    fn s(&self, i: usize, j: usize) -> Lit {
        *self.new_lit_map.get(&(i, j)).unwrap()
    }

    //sinz符号化の改良
    fn sinz_advance(&mut self, lits: Vec<Lit>, r: usize) -> Vec<Clause> {
        let n = lits.len();
        let mut clauses = Vec::new();


        for j in 1..=(n-r) {
            for k in 1..=r {
                let lit = self.instance.new_lit();
                //sj,k
                self.new_lit_map.insert((j,k), lit);
            }
        }
        
        //k=0
        // -xj v sj,1
        for j in 1..=(n-r) {
            clauses.push(clause!(!lits[j-1],self.s(j,1)));
        }
        
        //j!=n-r
        for j in 1..(n-r) {
            //k!=0,r
            for k in 1..r{
                //-sj,k v sj+1,k
                clauses.push(clause!(!self.s(j,k),self.s(j+1,k)));
                //-xj+k v -sj,k v sj,k+1
                clauses.push(clause!(!lits[j+k-1],!self.s(j,k),self.s(j,k+1)));
            }
            //k=r
            //-sj,r v sj+1,r
            clauses.push(clause!(!self.s(j,r),self.s(j+1,r)));
            //-xj+k v -sj,k v sj,k+1
            clauses.push(clause!(!lits[j+r-1],!self.s(j,r)));
        }

        //j=n-r
        for k in 1..r{
            //-xn-r+k v -sn-r,k v sn-r,k+1
            clauses.push(clause!(!lits[n-r+k-1],!self.s(n-r,k),self.s(n-r,k+1)));
        }
        //k=r
        //-xn-r+k v -sn-r,k v sn-r,k+1
        clauses.push(clause!(!lits[n-r+r-1],!self.s(n-r,r)));

        clauses
    }

    fn product(&mut self,lits: Vec<Lit>,method:i32) -> Vec<Clause> {
        let mut clauses = Vec::new();
        let mut p_lit_map = HashMap::new();
        let mut q_lit_map = HashMap::new();
        let n = lits.len();
        let p: usize = (n as f64).sqrt().ceil() as usize;
        let q: usize = ((n as f64)/(p as f64)).ceil() as usize;
        for i in 1..=p{
            let lit = self.instance.new_lit();
            // self.new_lit_map.insert((i,1),lit);
            p_lit_map.insert(i, lit);
        }

        for i in 1..=q{
            let lit = self.instance.new_lit();
            // self.new_lit_map.insert((i,2),lit);
            q_lit_map.insert(i,lit);
        }
        
        if method == 4{
            for i in 1..=p {
                for j in i + 1..=p {
                    // clauses.push(clause!(!self.s(i, 1),!self.s(j,1)));
                    clauses.push(clause!(!*p_lit_map.get(&i).unwrap(),!*p_lit_map.get(&j).unwrap()));

                }
            }
        
            for i in 1..=q {
                for j in i + 1..=q {
                    // clauses.push(clause!(!self.s(i, 2),!self.s(j,2)));
                    clauses.push(clause!(!*q_lit_map.get(&i).unwrap(),!*q_lit_map.get(&j).unwrap()));
                }
            }
        }else if method == 5{
            match p{
                1 => (),
                2 => clauses.push(clause!(!*p_lit_map.get(&1).unwrap(),!*p_lit_map.get(&2).unwrap())),
                _ => {
                    let p_lits:Vec<Lit> = (1..=p).map(|i| *p_lit_map.get(&i).unwrap()).collect();
                    let p_clauses = self.product(p_lits.clone(), method);
                    clauses.extend(p_clauses);
                },
            }
            match q{
                1 => (),
                2 => clauses.push(clause!(!*q_lit_map.get(&1).unwrap(),!*q_lit_map.get(&2).unwrap())),
                _ => {
                    let q_lits:Vec<Lit> = (1..=q).map(|i| *q_lit_map.get(&i).unwrap()).collect();
                    let q_clauses = self.product(q_lits.clone(), method);
                    clauses.extend(q_clauses);
                },
            }
            
        }
    
        for k in 1..=n {
            for i in 1..=p {
                for j in 1..=q {
                    if k == ((i - 1) * q + j) as usize{
                        // clauses.push(clause!(!lits[k-1], self.s(i, 1)));
                        // clauses.push(clause!(!lits[k-1], self.s(j, 2)));
                        clauses.push(clause!(!lits[k-1],*p_lit_map.get(&i).unwrap()));
                        clauses.push(clause!(!lits[k-1],*q_lit_map.get(&j).unwrap()));
                    }
                }
            }
        }
        // println!("{clauses:?}");
    
        clauses
    }

    fn ladder(&mut self,lits:Vec<Lit>) -> Vec<Clause> {
        let mut clauses = Vec::new();
        let n = lits.len();
        let ys:Vec<Lit> = (0..n-1).map(|_|self.instance.new_lit()).collect();

        for i in 0..n-2{
            clauses.push(clause!(ys[i],!ys[i+1]));
        }

        for i in 0..n{
            if i == 0{
                clauses.push(clause!(ys[i],lits[i]));
                clauses.push(clause!(!lits[i],!ys[i]));
            }else if i == (n-1) {
                clauses.push(clause!(!ys[i-1],lits[i]));
                clauses.push(clause!(!lits[i],ys[i-1]));
            }else{
                clauses.push(clause!(!ys[i-1],ys[i],lits[i]));
                clauses.push(clause!(!lits[i],ys[i-1]));
                clauses.push(clause!(!lits[i],!ys[i]));
            }
        }

        clauses
    }

    //adder encoding
    //exact one制約を符号化する
    //節集合を返す.
    fn bailleux_k(&mut self,lits: Vec<Lit>, k: usize) -> Vec<Clause> {
        let n = lits.len();
        let mut clauses = Vec::new();

        let (totalizer_clauses, s) = self.bailleux_tortalize(lits,&vec!());
        
        clauses.extend(totalizer_clauses);
        
        // The Comparator.
        for i in 0..n{
            if i < k{
                clauses.push(clause!(s[i]));
            }else{
                clauses.push(clause!(!s[i]));
            }
        }

        clauses
    }

    pub fn bailleux_tortalize(&mut self,lits: Vec<Lit>, s:&Vec<Lit>) -> (Vec<Clause>,Vec<Lit>){
        let mut clauses = Vec::new();
        if lits.len() == 1{
            return (clauses,lits);
        }

        //子ノードの作成
        let (left_clauses,left_l) = self.bailleux_tortalize(lits[..lits.len()/2].to_vec(),&vec!());
        let (right_clauses,right_l) = self.bailleux_tortalize(lits[lits.len()/2..].to_vec(),&vec!());

        clauses.extend(left_clauses);
        clauses.extend(right_clauses);

        let m = left_l.len() + right_l.len();

        let l = 
        if s.len() == 0 {
            //linking variablesの生成
            let mut add_l:Vec<Lit> = Vec::new();
            for _ in 0..m{
                let lit = self.instance.new_lit();
                add_l.push(lit);
            }
            add_l

        }else{
            s.to_vec()
        };

        //節の生成
        for sigma in 0..=m{
            for i in 0..=left_l.len(){
                for j in 0..=right_l.len(){
                    if i+j == sigma{
                        if sigma == 0 {
                            //C2の節を加える
                            //a_1,b_1,!r_1
                            clauses.push(clause!(left_l[i],right_l[j],!l[sigma]));
                        }else if sigma == m{
                            //C1の節を加える
                            //a_max,b_max,r_max
                            clauses.push(clause!(!left_l[i-1],!right_l[j-1],l[sigma-1]));
                        }else{
                            //C1の節を加える
                            if i == 0{
                                // !b_j,r_sigma
                                clauses.push(clause!(!right_l[j-1],l[sigma-1]));
                            }else if j == 0{
                                // !a_i,r_sigma
                                clauses.push(clause!(!left_l[i-1],l[sigma-1]));
                            }else{
                                // !a_i,!b_j,r_sigma
                                clauses.push(clause!(!left_l[i-1],!right_l[j-1],l[sigma-1]));
                            }

                            //C2の節を加える
                            if i == left_l.len(){
                                // !b_j+1,r_sigma+1
                                clauses.push(clause!(right_l[j],!l[sigma]));
                            }else if j == right_l.len(){
                                // !a_i+1,r_sigma+1
                                clauses.push(clause!(left_l[i],!l[sigma]));
                            }else{
                                // !a_i+1,!b_j+1,r_sigma+1
                                clauses.push(clause!(left_l[i],right_l[j],!l[sigma]));
                            }
                        }
                    }
                }
            }
        }

        (clauses,l)
    }

    //at-least-one節
    fn at_least_one(lits: Vec<Lit>) -> Clause {
        let mut clause = Clause::new();
        clause.extend(lits);
        clause
    }

    fn asymmetry(&self, u: i32, adjs: &Vec<i32>,symmetry: i32) -> Vec<Clause> {
        let mut clauses: Vec<Clause> = Vec::new();
        let mut adjs_sort = adjs.clone();
        adjs_sort.sort();
        for (i, v_out) in adjs_sort.iter().enumerate() {
            if symmetry < 3{
                if i == 0 {
                    continue;
                } else if i == adjs_sort.len() - 1 {
                    //隣接する頂点の中で、頂点番号が一番大きい頂点へ出ていく辺を禁止する
                    let lit = self.graph_lit_map[&(u, *v_out)];
                    clauses.push(clause!(!lit));
                } else {
                    //出ていく辺よりも入ってくる辺の頂点番号のほうが小さいものを禁止する
                    for v_in in &adjs_sort[..i] {
                        let lit1 = self.graph_lit_map[&(u, *v_out)];
                        let lit2 = self.graph_lit_map[&(*v_in, u)];
                        clauses.push(clause!(!lit1, !lit2));
                    }
                }
            }else{
                if i == 0{
                    let mut clause = Clause::new();
                    for v_in in &adjs_sort[1..]{
                        let lit = self.graph_lit_map[&(*v_in,u)];
                        clause.add(lit);
                    }
                    clauses.push(clause);
                }else if i == adjs_sort.len()-1{
                    let lit = self.graph_lit_map[&(u, *v_out)];
                    clauses.push(clause!(!lit));
                }else{
                    let mut clause = Clause::new();
                    for v_in in &adjs_sort[i+1..]{
                        let lit = self.graph_lit_map[&(*v_in,u)];
                        clause.add(lit);
                    }
                    let lit = self.graph_lit_map[&(u, *v_out)];
                    clause.add(!lit);
                    clauses.push(clause);
                }
            }
        }

        clauses
    }

    fn two_loop_prohibition(&self, g:&Graph) -> Vec<Clause>{
        let mut clauses = Vec::new();
        let mut memo: BTreeSet<(i32, i32)> = BTreeSet::new();
            for &(u, v) in g.arcs.iter() {
                if memo.contains(&(u, v)) {
                    continue;
                } else {
                    let lit1 = self.graph_lit_map[&(u, v)];
                    let lit2 = self.graph_lit_map[&(v, u)];
                    clauses.push(clause!(!lit1, !lit2));
                    memo.insert((v, u));
                }
            }
        clauses
    }

    fn three_loop_prohibition(&self, g:&Graph) -> Vec<Clause>{
        let mut clauses = Vec::new();
        let mut memo: Vec<BTreeSet<i32>> = Vec::new();
        // let mut two_memo: BTreeSet<BTreeSet<i32>> = BTreeSet::new();
        // let mut one_memo: BTreeSet<i32> = BTreeSet::new()

        for x in g.adjacency_list.keys() {
            for ys in g.adjacency_list.get(&x).iter(){
                for y in ys.iter(){
                    let bool1 = memo.contains(&BTreeSet::from([*y]));
                    let bool2 = memo.contains(&BTreeSet::from([*x,*y]));
                    if  !bool1 && !bool2 {
                        for zs in g.adjacency_list.get(&y).iter(){
                            for z in zs.iter(){
                                if ys.contains(z){
                                    let bool3 = x == z;
                                    let bool4 = memo.contains(&BTreeSet::from([*x,*y,*z]));
                                    let bool5 = memo.contains(&BTreeSet::from([*y,*z]));
                                    let bool6 = memo.contains(&BTreeSet::from([*x,*z]));
                                    let bool7 = memo.contains(&BTreeSet::from([*z]));
                                    
                                    if  !bool3 && !bool4 && !bool5 && !bool6 && !bool7{
                                        //順方向
                                        let lit1 = self.graph_lit_map[&(*x, *y)];
                                        let lit2 = self.graph_lit_map[&(*y, *z)];
                                        let lit3 = self.graph_lit_map[&(*z, *x)];
                                        clauses.push(clause!(!lit1,!lit2,!lit3));

                                        //逆方向
                                        let lit4 = self.graph_lit_map[&(*x, *z)];
                                        let lit5 = self.graph_lit_map[&(*y, *x)];
                                        let lit6 = self.graph_lit_map[&(*z, *y)];
                                        clauses.push(clause!(!lit4,!lit5,!lit6));
                                        
                                        memo.push(BTreeSet::from([*x,*y,*z]));
                                    }
                                }
                            }
                        }
                    }
                    memo.push(BTreeSet::from([*x,*y]));
                }
            }
            memo.push(BTreeSet::from([*x]));
        }

        clauses


    }

    fn dearcify(&mut self,g:&Graph,method: i32) -> Vec<Clause>{
        let mut clauses = Vec::new();
        let degrees = match method{
            2 => vec![2,3,4],
            3 => vec![2,3],
            _ => vec![2],
        };

        for (u,adj) in g.adjacency_list_btree.iter(){
            match Encoder::is_adjacent_to_degree(adj,g,&degrees){
                Some(low_verticies) => {
                    clauses.extend(self.dearcify_encode(g, *u, low_verticies))
                },
                None => ()
            }
        }
        clauses
    }

    fn is_adjacent_to_degree(adjs:&Vec<i32>,g:&Graph,degrees:&Vec<i32>) -> Option<Vec<i32>>{

        if adjs.len() == 2{
            return None
        }
        
        let mut specified_degree_verticies = Vec::new();
        for adj in adjs.iter(){
            let adj_degree = g.adjacency_list.get(adj).unwrap().len() as i32;
            if degrees.contains(&adj_degree){
                specified_degree_verticies.push(*adj);
            }
        }

        if specified_degree_verticies.len() < 2{
            None
        }else{
            Some(specified_degree_verticies)
        }
    }
    
    fn dearcify_encode(&mut self,g:&Graph,u:i32,low_verticies:Vec<i32>) -> Vec<Clause>{
        let mut clauses = Vec::new();
        let mut adjs_lits = Vec::new();
        let mut verticies_of_degree_two = Vec::new();

        for low_vertex in low_verticies.iter(){
            let low_adjs = g.adjacency_list.get(low_vertex).unwrap();
            if low_adjs.len() == 2{
                verticies_of_degree_two.push(*low_vertex);
            }else{
                let (clause,lit) = self.activate_on_degree_two(u, *low_vertex, low_adjs);
                clauses.extend(clause);
                adjs_lits.push((*low_vertex,lit));
            }
        }
        
        match verticies_of_degree_two.len(){
            2 => {
                let u_x = *self.graph_lit_map.get(&(u,verticies_of_degree_two[0])).unwrap();
                let u_y = *self.graph_lit_map.get(&(u,verticies_of_degree_two[1])).unwrap();
                let x_u = *self.graph_lit_map.get(&(verticies_of_degree_two[0],u)).unwrap();
                let y_u = *self.graph_lit_map.get(&(verticies_of_degree_two[1],u)).unwrap();
                clauses.push(clause!(u_x,u_y));
                clauses.push(clause!(x_u,y_u));
            },
            1 => {
                let u_x = *self.graph_lit_map.get(&(u,verticies_of_degree_two[0])).unwrap();
                let x_u = *self.graph_lit_map.get(&(verticies_of_degree_two[0],u)).unwrap();
                for (y,lit) in adjs_lits.iter() {
                    let u_y = *self.graph_lit_map.get(&(u,*y)).unwrap();
                    let y_u = *self.graph_lit_map.get(&(*y,u)).unwrap();
                    clauses.push(clause!(!*lit,u_x,u_y));
                    clauses.push(clause!(!*lit,x_u,y_u));
                }
            },
            _ => {
                for i in 0..adjs_lits.len(){
                    let (x,x_lit) = adjs_lits[i];
                    let u_x = *self.graph_lit_map.get(&(u,x)).unwrap();
                    let x_u = *self.graph_lit_map.get(&(x,u)).unwrap();
                    for j in i+1..adjs_lits.len(){
                        let (y,y_lit) = adjs_lits[j];
                        let u_y = *self.graph_lit_map.get(&(u,y)).unwrap();
                        let y_u = *self.graph_lit_map.get(&(y,u)).unwrap();
                        clauses.push(clause!(!x_lit,!y_lit,u_x,u_y));
                        clauses.push(clause!(!x_lit,!y_lit,x_u,y_u));
                        
                    }
                }
            }
        }
        

        clauses
    }

    fn activate_on_degree_two(&mut self,u:i32,v:i32,adjs:&Vec<i32>) -> (Vec<Clause>,Lit) {
        let mut clauses = Vec::new();
        let new_lit = self.instance.new_lit(); 
        let mut lits = Vec::new();
        let new_adjs:Vec<i32> = adjs.iter().filter(|&&x| x != u).cloned().collect();
        for i in 0..new_adjs.len(){
            let is_insert = self.insert_edge(v,new_adjs[i]);
            match is_insert{
                Some(clause) => clauses.extend(clause),
                None => ()
            }
        }

        for i in 0..new_adjs.len(){
            for j in i+1..new_adjs.len(){
                let x = new_adjs[i];
                let y = new_adjs[j];
                let lit = self.instance.new_lit();
                // !e1 and !e2 -> s1
                clauses.push(clause!(*self.edge(v,x).unwrap(),*self.edge(v,y).unwrap(),lit));

                // s1 -> !e1 and !e2
                clauses.push(clause!(!lit,!*self.edge(v,x).unwrap()));
                clauses.push(clause!(!lit,!*self.edge(v,y).unwrap()));

                lits.push(lit);
            }
        }

        let mut clause = Clause::new();
        for lit in lits.iter(){
            clauses.push(clause!(!*lit,new_lit));
            clause.add(*lit);
        }
        clause.add(!new_lit);
        clauses.push(clause);


        (clauses,new_lit)
    }


    fn insert_edge(&mut self,u:i32,v:i32) -> Option<Vec<Clause>> {
        let mut clauses = Vec::new();
        
        match self.edge(u,v){
            Some(_) => return None,
            None =>{
                let lit = self.instance.new_lit();
                let u_v = *self.graph_lit_map.get(&(u,v)).unwrap();
                let v_u = *self.graph_lit_map.get(&(v,u)).unwrap();
                // u_v or v_u -> edge_u_v
                clauses.push(clause!(!u_v,lit));
                clauses.push(clause!(!v_u,lit));
                
                // !u_v and !v_u -> !edge_u_v
                clauses.push(clause!(u_v,v_u,!lit));

                if u < v {
                    self.edge_map.insert((u,v), lit);
                }else{
                    self.edge_map.insert((v,u),lit);
                }

            }
        }
        

        Some(clauses)
    }

    fn edge(&self,u:i32,v:i32) -> Option<&Lit> {
        if u < v{
            self.edge_map.get(&(u,v))
        }else{
            self.edge_map.get(&(v,u))
        }
    }
}
