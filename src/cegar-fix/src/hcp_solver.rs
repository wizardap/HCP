use rustsat::instances::*;
use rustsat::types::*;
use rustsat::solvers::*;
use rustsat::clause;
use rustsat_cadical::Config;
use std::collections::{BTreeMap,HashMap,HashSet};
use std::time::{Instant,Duration};
// use crate::encoder;
use crate::graph::*;
use crate::encoder::*;
use crate::file_operations;
use crate::contraction::Degree2Contractor;
use crate::hub_registry::HubRegistry;


pub fn solve_hamilton(g:Graph, contractor: &Degree2Contractor, hub_registry: &HubRegistry, _s:i32, encode_method:i32, block_method: i32,symmetry: i32 ,opt:i32,loop_prohibition: i32,cnf_normalize:i32,balanced:i32,dearcify:i32, cadical_config:i32, degree_order:i32, arcs_order:i32, three_opt:i32, _cegar_fallback:i32, _mtz_stall:i32, _adaptive_escalation:i32, _sub_hcp_timeout: u64, _max_cluster_size: usize, instant:Instant,output_folder:&str) {
    let now = instant.elapsed();
    let mut encoder = Encoder::new();
    // グラフをcnf形式に変形し、cnfへ格納
    let mut cnf = encoder.encode(&g,encode_method,symmetry,loop_prohibition,dearcify,degree_order,arcs_order);
    // Add mandatory edge constraints for contracted degree-2 paths
    for (&(u, w), _) in &contractor.chain_map {
        if u < w {
            if let (Some(&lit_uw), Some(&lit_wu)) = (encoder.graph_lit_map.get(&(u, w)), encoder.graph_lit_map.get(&(w, u))) {
                cnf.add_clause(clause!(lit_uw, lit_wu));
            }
        }
    }
    let current_cnf = if output_folder != "default" {
        //フォルダーの作成
        let _ = file_operations::create_folder_if_not_exists(output_folder);
        //cnfをファイルに出力する
        let output_file = format!("{}/increment0.cnf",output_folder);
        let _ = file_operations::write_dimacs(cnf.clone(),&output_file);
        //出力のために複製
        cnf.clone()
    }else{
        Cnf::new()
    };

    // 標準入力で -s の後の数字により、minisat,kissat,cadicalを選択する
    println!("encodhing time = {:?}",instant.elapsed()-now);
    // let mut solver: Box<MySolver> =
    // if s == 0 {
    //     Box::new(MySolver::Minisat(rustsat_minisat::core::Minisat::default()))
    // } else if s == 1 {
    //     Box::new(MySolver::Kissat(rustsat_kissat::Kissat::default())) 
    // } else if s == 2 {
    //     Box::new(MySolver::CaDiCaL(rustsat_cadical::CaDiCaL::default()))
    // } else {
    //     panic!("out of number for solver\nminisat: -s 0\nkissat: -s 1\ncadical: -s 2");
    // };
    let mut solver = rustsat_cadical::CaDiCaL::default();
    if cadical_config == 1{
        _ = solver.set_configuration(Config::Sat);
    }
    // println!("encodhing clauses number = {}",cnf.len());
    println!();
    //　ソルバーにcnfを入れる
    if cnf_normalize == 1{
        let normalized_cnf = cnf.normalize();
        println!("encodhing clauses number = {}",normalized_cnf.len());
        let _ = solver.add_cnf(normalized_cnf);
    }else{
        println!("encodhing clauses number = {}",cnf.len());
        let _ = solver.add_cnf(cnf);
    }
    // cegar関数により、解を求め、increment数と追加したblock節の合計を返す
    let (increment, block) = cegar(
        &mut encoder,
        solver,
        0,
        0,
        g,
        contractor,
        hub_registry,
        block_method,
        opt,
        three_opt,
        instant,
        cnf_normalize,
        balanced,
        instant.elapsed(),
        current_cnf,
        output_folder,
    );
    println!("overall incremented number = {}", increment);
    println!("overall number of added block clauses = {}", block);
}

fn cegar(
    encoder: &mut Encoder,
    mut solver: rustsat_cadical::CaDiCaL<'_, '_>,
    mut count: i32,
    mut clause_count: i32,
    g: Graph,
    contractor: &Degree2Contractor,
    hub_registry: &HubRegistry,
    block_method: i32,
    opt: i32,
    three_opt: i32,
    instant: Instant,
    cnf_normalize: i32,
    balanced: i32,
    mut previous_time: Duration,
    mut previous_cnf: Cnf,
    output_folder: &str,
) -> (i32, i32) {
    loop {
        // SATソルバーで解を求める
        let res = solver.solve().unwrap();
        let now = instant.elapsed();
        let sat_solving_time = now - previous_time;

        println!();
        println!("Increment...");
        println!("incremented number = {}", count);
        println!("sat solving time = {:?}", sat_solving_time);

        // 解がSATならば、ハミルトン閉路になっているかを調べる
        if res == SolverResult::Sat {
            // 変数の値割り当て
            let sol = solver.full_solution().unwrap();
            // どの辺が選択されたかの解析
            let sol_arcs = get_solution_arcs(sol, &encoder.graph_lit_map);
            // 閉路
            let sol_cycles = get_solution_cycles(sol_arcs);

            // 閉路が一つであれば、ハミルトン閉路なので解を出力
            if sol_cycles.len() == 1 {
                let flat: Vec<i32> = sol_cycles.into_iter().flatten().collect();
                let full_cycle = contractor.uncontract_cycle(&flat);
                let line = full_cycle.iter().map(|i| i.to_string()).collect::<Vec<String>>().join(" ");
                println!();
                println!("solution: ");
                println!("{}\n", line);
                println!("s SATISFIABLE");
                return (count, clause_count);
            } else {
                println!("number of subcycles found = {}", sol_cycles.len());
                println!("sat solution cycle lengths map (length:number) = {:?}", map_cycle_lengths(&sol_cycles));

                // 2-opt / 3-opt solution constructor
                let (block_clauses, _active_cycles) = if opt == 0 {
                    (get_blocking_clauses(&sol_cycles, encoder, &g, block_method, balanced), sol_cycles.clone())
                } else if opt >= 1 {
                    let (clauses, cycles) = two_opt(&sol_cycles, encoder, &g, contractor, hub_registry, block_method, balanced, opt, three_opt);
                    if cycles.len() == 1 && cycles[0].len() == g.adjacency_list.len() {
                        let flat: Vec<i32> = cycles.into_iter().flatten().collect();
                        let full_cycle = contractor.uncontract_cycle(&flat);
                        let line = full_cycle.iter().map(|i| i.to_string()).collect::<Vec<String>>().join(" ");
                        let now = instant.elapsed();
                        let time = now - previous_time;
                        let add_block_clauses_time = now - previous_time - sat_solving_time;
                        println!("number of added block clauses = {}", clause_count);
                        println!("add block clauses time = {:?}", add_block_clauses_time);
                        println!("increment time = {:?}", time);
                        println!();
                        println!("hamiltonian cycle found by 2-opt/3-opt");
                        println!("solution: ");
                        println!("{}\n", line);
                        println!("s SATISFIABLE");
                        return (count, clause_count);
                    }
                    (clauses, cycles)
                } else {
                    panic!("2-opt option \n-t 0:2-opt off\n-t 1,2,3:2-opt on");
                };

                let mut cnf = Cnf::new();
                cnf.extend(block_clauses);
                count += 1;

                previous_cnf = if output_folder != "default" {
                    let mut write_cnf = previous_cnf;
                    write_cnf.extend(cnf.clone());
                    let output_file = format!("{}/increment{}.cnf", output_folder, count);
                    let _ = file_operations::write_dimacs(write_cnf.clone(), &output_file);
                    write_cnf
                } else {
                    Cnf::new()
                };

                if cnf_normalize == 1 {
                    let normalized_cnf = cnf.normalize();
                    clause_count += normalized_cnf.len() as i32;
                    let _ = solver.add_cnf(normalized_cnf);
                } else {
                    clause_count += cnf.len() as i32;
                    let _ = solver.add_cnf(cnf);
                }

                let time = now - previous_time;
                let add_block_clauses_time = now - previous_time - sat_solving_time;
                previous_time = now;
                println!("number of added block clauses = {}", clause_count);
                println!("add block clauses time = {:?}", add_block_clauses_time);
                println!("increment time = {:?}", time);
            }
        } else {
            println!("s UNSATISFIABLE");
            return (count, clause_count);
        }
    }
}

fn get_solution_arcs(sol:Assignment,lit_map:&BTreeMap<(i32,i32),Lit>) -> Vec<(i32,i32)>{
    let sol_arcs: Vec<(i32,i32)> = lit_map.iter().filter_map(|((u,v), lit)| if sol[lit.var()] == TernaryVal::True { Some((*u,*v)) } else { None }).collect();
    sol_arcs

}

fn get_solution_cycles(sol_arcs: Vec<(i32, i32)>) -> Vec<Vec<i32>> {
    let mut arcs: BTreeMap<i32,i32> = std::collections::BTreeMap::new();
    let mut cycles = Vec::new();
    let mut visited = std::collections::BTreeSet::new();

    for arc in sol_arcs{
        arcs.insert(arc.0,arc.1);
    }
    
    for node in arcs.keys() {
        if visited.contains(node) {
            continue;
        }
        let mut cycle = Vec::new();
        let mut current_node = node;
        loop{
            visited.insert(current_node);
            cycle.push(*current_node);
            current_node = match arcs.get(current_node) {
                Some(node) => node,
                None => break,
            };
            if visited.contains(current_node) {
                break;
            }
        }
        cycles.push(cycle);
    }

    cycles
}


fn cycle_hub_score(cycle: &[i32], hub_registry: &HubRegistry, g: &Graph) -> (usize, usize) {
    if hub_registry.hub_vertices.is_empty() {
        return (0, 0);
    }
    let mut contains_hub_count = 0;
    let mut incident_hub_edges = 0;

    for &v in cycle {
        if hub_registry.is_hub_vertex(v) {
            contains_hub_count += 1;
        }
        if let Some(adjs) = g.adjacency_list.get(&v) {
            for &u in adjs {
                if hub_registry.is_hub_vertex(u) {
                    incident_hub_edges += 1;
                }
            }
        }
    }

    let tier = if contains_hub_count > 0 {
        2
    } else if incident_hub_edges > 0 {
        1
    } else {
        0
    };
    (tier, contains_hub_count * 1000 + incident_hub_edges)
}

// 2-opt and Candidate 3-opt Solution Constructor
// Attempts to merge subcycles into a single Hamiltonian Cycle.
// If complete merger fails, returns standard blocking clauses for active subcycles.
fn two_opt(
    sol_cycles: &Vec<Vec<i32>>,
    encoder: &mut Encoder,
    g: &Graph,
    contractor: &Degree2Contractor,
    hub_registry: &HubRegistry,
    block_method: i32,
    balanced: i32,
    opt: i32,
    three_opt: i32,
) -> (Vec<Clause>, Vec<Vec<i32>>) {
    let mut cycles = sol_cycles.to_vec();
    let mut merged = true;
    let mut cache_vertex: HashSet<usize> = HashSet::new();
    let mut active_cycles_number: Vec<usize> = (0..cycles.len()).collect();

    if !hub_registry.hub_vertices.is_empty() {
        active_cycles_number.sort_by(|&a, &b| {
            cycle_hub_score(&cycles[b], hub_registry, g).cmp(&cycle_hub_score(&cycles[a], hub_registry, g))
        });
    }

    while merged {
        let (_new_block_clauses, new_merged, merged_numbers, new_cycle) =
            merge_cycles(&cycles, encoder, g, contractor, hub_registry, block_method, balanced, &mut cache_vertex, &active_cycles_number, opt);
        merged = new_merged;

        if merged {
            cycles.push(new_cycle);
            let mut remove_indices = [merged_numbers.0, merged_numbers.1];
            remove_indices.sort_unstable_by(|a, b| b.cmp(a));
            for &idx in &remove_indices {
                active_cycles_number.remove(idx);
            }
            active_cycles_number.push(cycles.len() - 1);
            if !hub_registry.hub_vertices.is_empty() {
                active_cycles_number.sort_by(|&a, &b| {
                    cycle_hub_score(&cycles[b], hub_registry, g).cmp(&cycle_hub_score(&cycles[a], hub_registry, g))
                });
            }
        }

        // Try candidate 3-opt merge when 2-opt cannot merge further
        if !merged && three_opt == 1 && active_cycles_number.len() >= 3 {
            let (_three_block_clauses, three_merged, three_indices, three_cycle) =
                merge_three_cycles(&cycles, encoder, g, contractor, hub_registry, block_method, balanced, &active_cycles_number);
            if three_merged {
                cycles.push(three_cycle);
                let (ia, ib, ic) = three_indices;
                let mut remove_indices = [ia, ib, ic];
                remove_indices.sort_unstable_by(|a, b| b.cmp(a));
                for &idx in &remove_indices {
                    active_cycles_number.remove(idx);
                }
                active_cycles_number.push(cycles.len() - 1);
                merged = true;
                cache_vertex.clear();
                if !hub_registry.hub_vertices.is_empty() {
                    active_cycles_number.sort_by(|&a, &b| {
                        cycle_hub_score(&cycles[b], hub_registry, g).cmp(&cycle_hub_score(&cycles[a], hub_registry, g))
                    });
                }
                continue;
            }
        }

        if active_cycles_number.len() == 1 || !merged {
            break;
        }
    }

    let active_cycles = get_active_cycles(&cycles, &active_cycles_number);
    let block_clauses = if active_cycles.len() == 1 && active_cycles[0].len() == g.adjacency_list.len() {
        Vec::new()
    } else {
        get_blocking_clauses(sol_cycles, encoder, g, block_method, balanced)
    };

    println!("number of connected cycles = {}", cycles.len() - sol_cycles.len());
    println!("number of merged cycles = {}", active_cycles.len());
    println!("merged cycle lengths map (length:number) = {:?}", map_cycle_lengths(&active_cycles));

    (block_clauses, active_cycles)
}

fn merge_cycles(
    cycles: &Vec<Vec<i32>>,
    encoder: &mut Encoder,
    g: &Graph,
    contractor: &Degree2Contractor,
    hub_registry: &HubRegistry,
    block_method: i32,
    balanced: i32,
    cache_vertex: &mut HashSet<usize>,
    active_cycles_number: &Vec<usize>,
    opt: i32,
) -> (Vec<Clause>, bool, (usize, usize), Vec<i32>) {
    //(block_clauses,merged,(merged_number1,merged_number2),new_cycle)
    
    for i in 0..active_cycles_number.len(){
        let left = active_cycles_number[i];
        if !cache_vertex.contains(&left){
            for j in i+1..active_cycles_number.len(){
                let right = active_cycles_number[j];

                match swap_node(&cycles[left],&cycles[right],&g, contractor, hub_registry){
                    Some(new_cycle) =>{
                    let new_block_clauses = get_blocking_clauses(&vec!(new_cycle.clone()), encoder, g, block_method, balanced);
                    return (new_block_clauses,true,(i,j),new_cycle)
                    }
                    None =>{
                        continue
                    }
                }
                
            }
            cache_vertex.insert(left);
        }
        if opt == 4 || opt == 5{
            return (vec!(),false,(0,0),vec!())
        }
    }
    
    (vec!(),false,(0,0),vec!())
}


fn swap_node(
    cycle1: &Vec<i32>,
    cycle2: &Vec<i32>,
    g: &Graph,
    contractor: &Degree2Contractor,
    hub_registry: &HubRegistry,
) -> Option<Vec<i32>> {
    for i in 0..cycle1.len() {
        let u1 = cycle1[i];
        let v1 = cycle1[(i + 1) % cycle1.len()];
        if contractor.chain_map.contains_key(&(u1, v1)) || contractor.chain_map.contains_key(&(v1, u1)) {
            continue;
        }

        let u1_hub_set = hub_registry.hub_neighbors.get(&u1);
        let v1_hub_set = hub_registry.hub_neighbors.get(&v1);

        let adjs_of_left_head = g.adjacency_list.get(&u1).unwrap();
        let adjs_of_left_tail = g.adjacency_list.get(&v1).unwrap();

        for j in 0..cycle2.len() {
            let u2 = cycle2[j];
            let v2_fwd = cycle2[(j + 1) % cycle2.len()];
            let v2_rev = cycle2[(j + cycle2.len() - 1) % cycle2.len()];

            let u1_connected_u2 = if let Some(hset) = u1_hub_set {
                hset.contains(&u2)
            } else {
                adjs_of_left_head.contains(&u2)
            };

            if u1_connected_u2 {
                let v1_connected_fwd = if let Some(hset) = v1_hub_set {
                    hset.contains(&v2_fwd)
                } else {
                    adjs_of_left_tail.contains(&v2_fwd)
                };

                if v1_connected_fwd {
                    if !contractor.chain_map.contains_key(&(u2, v2_fwd)) && !contractor.chain_map.contains_key(&(v2_fwd, u2)) {
                        return cycle_join(&cycle1, &cycle2, i, j, true);
                    }
                }

                let v1_connected_rev = if let Some(hset) = v1_hub_set {
                    hset.contains(&v2_rev)
                } else {
                    adjs_of_left_tail.contains(&v2_rev)
                };

                if v1_connected_rev {
                    if !contractor.chain_map.contains_key(&(u2, v2_rev)) && !contractor.chain_map.contains_key(&(v2_rev, u2)) {
                        return cycle_join(&cycle1, &cycle2, i, j, false);
                    }
                }
            }
        }
    }
    None
}

fn cycle_join(cycle1:&Vec<i32>,cycle2:&Vec<i32>,i:usize,j:usize,reverse:bool) -> Option<Vec<i32>>{
    let mut new_cycle = Vec::new();

    if reverse{
        // cycle1のindex iまでを追加
        new_cycle.extend(&cycle1[0..=i]);

        // cycle2のindex jから逆順にindex 0までの要素を追加
        new_cycle.extend(cycle2[0..=j].iter().rev());
        if j != cycle2.len()-1{
        // cycle2のindexの最後から、j+1までの要素を逆順に追加
            new_cycle.extend(cycle2[j+1..].iter().rev());
        }
        
        if i != cycle1.len()-1{
        // cycle1のindex i+1から最後までをnew_cycleに追加
            new_cycle.extend(&cycle1[i+1..]);
        }
    }else{
        new_cycle.extend(&cycle1[0..=i]);
        new_cycle.extend(&cycle2[j..]);
        if j != 0{
            new_cycle.extend(&cycle2[0..=j-1]);
        }
        if i != cycle1.len()-1{
            new_cycle.extend(&cycle1[i+1..]);
        }
    }

    Some(new_cycle)
}

/// Try to merge three directed cycles by a 3-edge swap.
/// Config A (0): C1 -> C2 -> C3 -> C1  (u1->v2, u2->v3, u3->v1)
/// Config B (1): C1 -> C3 -> C2 -> C1  (u1->v3, u3->v2, u2->v1)
fn swap_three_nodes(
    c1: &Vec<i32>,
    c2: &Vec<i32>,
    c3: &Vec<i32>,
    g: &Graph,
    contractor: &Degree2Contractor,
    hub_registry: &HubRegistry,
) -> Option<Vec<i32>> {
    for i in 0..c1.len() {
        let u1 = c1[i];
        let v1 = c1[(i + 1) % c1.len()];
        if contractor.chain_map.contains_key(&(u1, v1)) || contractor.chain_map.contains_key(&(v1, u1)) {
            continue;
        }
        let u1_hub = hub_registry.hub_neighbors.get(&u1);
        let adjs_u1 = g.adjacency_list.get(&u1).unwrap();

        for j in 0..c2.len() {
            let u2 = c2[j];
            let v2 = c2[(j + 1) % c2.len()];
            if contractor.chain_map.contains_key(&(u2, v2)) || contractor.chain_map.contains_key(&(v2, u2)) {
                continue;
            }
            let u2_hub = hub_registry.hub_neighbors.get(&u2);
            let adjs_u2 = g.adjacency_list.get(&u2).unwrap();

            for k in 0..c3.len() {
                let u3 = c3[k];
                let v3 = c3[(k + 1) % c3.len()];
                if contractor.chain_map.contains_key(&(u3, v3)) || contractor.chain_map.contains_key(&(v3, u3)) {
                    continue;
                }
                let u3_hub = hub_registry.hub_neighbors.get(&u3);
                let adjs_u3 = g.adjacency_list.get(&u3).unwrap();

                let u1_has_v2 = if let Some(h) = u1_hub { h.contains(&v2) } else { adjs_u1.contains(&v2) };
                let u1_has_v3 = if let Some(h) = u1_hub { h.contains(&v3) } else { adjs_u1.contains(&v3) };
                let u2_has_v3 = if let Some(h) = u2_hub { h.contains(&v3) } else { adjs_u2.contains(&v3) };
                let u2_has_v1 = if let Some(h) = u2_hub { h.contains(&v1) } else { adjs_u2.contains(&v1) };
                let u3_has_v1 = if let Some(h) = u3_hub { h.contains(&v1) } else { adjs_u3.contains(&v1) };
                let u3_has_v2 = if let Some(h) = u3_hub { h.contains(&v2) } else { adjs_u3.contains(&v2) };

                // Config A: u1->v2, u2->v3, u3->v1
                if u1_has_v2 && u2_has_v3 && u3_has_v1 {
                    return cycle_join_three(c1, c2, c3, i, j, k, 0);
                }
                // Config B: u1->v3, u3->v2, u2->v1
                if u1_has_v3 && u3_has_v2 && u2_has_v1 {
                    return cycle_join_three(c1, c2, c3, i, j, k, 1);
                }
            }
        }
    }
    None
}

/// Reconstruct a single merged cycle from three directed cycles given cut positions.
fn cycle_join_three(
    c1: &Vec<i32>, c2: &Vec<i32>, c3: &Vec<i32>,
    i: usize, j: usize, k: usize,
    config: u8,
) -> Option<Vec<i32>> {
    let mut new_cycle = Vec::new();
    if config == 0 {
        new_cycle.extend(&c1[0..=i]);
        new_cycle.extend(&c2[j+1..]);
        new_cycle.extend(&c2[..=j]);
        new_cycle.extend(&c3[k+1..]);
        new_cycle.extend(&c3[..=k]);
        if i + 1 < c1.len() {
            new_cycle.extend(&c1[i+1..]);
        }
    } else {
        new_cycle.extend(&c1[0..=i]);
        new_cycle.extend(&c3[k+1..]);
        new_cycle.extend(&c3[..=k]);
        new_cycle.extend(&c2[j+1..]);
        new_cycle.extend(&c2[..=j]);
        if i + 1 < c1.len() {
            new_cycle.extend(&c1[i+1..]);
        }
    }
    Some(new_cycle)
}

/// Try to merge a triplet of active cycles using candidate graph filtering.
fn merge_three_cycles(
    cycles: &Vec<Vec<i32>>,
    encoder: &mut Encoder,
    g: &Graph,
    contractor: &Degree2Contractor,
    hub_registry: &HubRegistry,
    block_method: i32,
    balanced: i32,
    active_cycles_number: &Vec<usize>,
) -> (Vec<Clause>, bool, (usize, usize, usize), Vec<i32>) {
    let n = active_cycles_number.len();

    let mut vertex_to_active: HashMap<i32, usize> = HashMap::new();
    for (active_idx, &cycle_idx) in active_cycles_number.iter().enumerate() {
        for &v in &cycles[cycle_idx] {
            vertex_to_active.insert(v, active_idx);
        }
    }
    let mut cycle_neighbors: Vec<HashSet<usize>> = vec![HashSet::new(); n];
    for (active_idx, &cycle_idx) in active_cycles_number.iter().enumerate() {
        for &u in &cycles[cycle_idx] {
            if let Some(adjs) = g.adjacency_list.get(&u) {
                for &v in adjs {
                    if let Some(&neighbor_active) = vertex_to_active.get(&v) {
                        if neighbor_active != active_idx {
                            cycle_neighbors[active_idx].insert(neighbor_active);
                        }
                    }
                }
            }
        }
    }

    for a in 0..n {
        let neighbors_a: Vec<usize> = cycle_neighbors[a].iter()
            .filter(|&&b| b > a)
            .cloned()
            .collect();
        for &b in &neighbors_a {
            for &c in &cycle_neighbors[b] {
                if c <= b { continue; }
                if !cycle_neighbors[a].contains(&c) { continue; }
                let ci = active_cycles_number[a];
                let cj = active_cycles_number[b];
                let ck = active_cycles_number[c];
                if let Some(new_cycle) = swap_three_nodes(&cycles[ci], &cycles[cj], &cycles[ck], g, contractor, hub_registry) {
                    let new_block_clauses = get_blocking_clauses(&vec!(new_cycle.clone()), encoder, g, block_method, balanced);
                    return (new_block_clauses, true, (a, b, c), new_cycle);
                }
            }
        }
    }
    (vec![], false, (0, 0, 0), vec![])
}

/// Inject Partial MTZ ordering constraints for a set of vertices K.
/// Uses ORDER encoding: for each v in K, creates n-1 boolean variables
/// o[v][t] where o[v][t] = 1 iff position(v) >= t+1.
///
/// Prevents ANY subtour entirely within K\{source}.
fn inject_partial_mtz(
    k_vertices: &Vec<i32>,
    g: &Graph,
    encoder: &mut Encoder,
    _n: usize,
) -> Vec<Clause> {
    
    let mut clauses: Vec<Clause> = Vec::new();
    let source = k_vertices[0];  // Pick first vertex as source
    let k_set: HashSet<i32> = k_vertices.iter().cloned().collect();
    
    let mut order_vars: HashMap<i32, Vec<Lit>> = HashMap::new();
    let k_len = k_vertices.len();
    for &v in k_vertices.iter() {
        let mut vars = Vec::new();
        // Allocate order variables corresponding to positions within K (1..k_len-1)
        for _t in 0..k_len-1 {
            let lit = encoder.instance.new_lit();
            vars.push(lit);
        }
        order_vars.insert(v, vars);
    }
    
    // 1. Monotonicity: o[v][t+1] → o[v][t]
    //    Clause: ¬o[v][t+1] ∨ o[v][t]
    for &v in k_vertices.iter() {
        let vars = &order_vars[&v];
        for t in 0..vars.len()-1 {
            clauses.push(clause!(!vars[t+1], vars[t]));
        }
    }
    
    // 3. Source constraint: position(source) = 0 → o[source][t] = 0 for all t
    for lit in order_vars[&source].iter() {
        clauses.push(clause!(!*lit));
    }
    
    // 4. MTZ constraints: for each directed edge (u,v) where u,v ∈ K\{source}
    //    x_{u,v} = 1 → position(v) >= position(u) + 1
    //    Clause: ¬x_{u,v} ∨ ¬o[u][t] ∨ o[v][t+1]  for t = 0..n-3
    //    Also:   ¬x_{u,v} ∨ o[v][0]  (position(v) >= 1 when arc is selected)
    for &u in k_vertices.iter() {
        if let Some(adjs) = g.adjacency_list.get(&u) {
            for &v in adjs.iter() {
                if !k_set.contains(&v) { continue; }  // Only edges within K
                if v == source && u == source { continue; } // Skip self-loops
                
                let x_uv = match encoder.graph_lit_map.get(&(u, v)) {
                    Some(lit) => *lit,
                    None => continue,  // Edge doesn't exist in encoding
                };
                
                if u == source {
                    // Edge from source: x_{s,v} → position(v) >= 1
                    let o_v = &order_vars[&v];
                    clauses.push(clause!(!x_uv, o_v[0]));
                } else if v == source {
                    // Edge TO source: no MTZ constraint needed
                    // (source can be at position 0 and receive edge from position n-1)
                    continue;
                } else {
                    // Edge between non-source K vertices
                    let o_u = &order_vars[&u];
                    let o_v = &order_vars[&v];
                    
                    // x_{u,v} → position(v) >= 1
                    clauses.push(clause!(!x_uv, o_v[0]));
                    
                    // x_{u,v} ∧ o[u][t] → o[v][t+1] for t = 0..n-3
                    for t in 0..o_u.len()-1 {
                        clauses.push(clause!(!x_uv, !o_u[t], o_v[t+1]));
                    }
                }
            }
        }
    }
    
    clauses
}

fn get_blocking_clauses(
    sol_cycles: &Vec<Vec<i32>>,
    encoder: &mut Encoder,
    g: &Graph,
    block_method: i32,
    balanced: i32,
) -> Vec<Clause> {
    let mut clauses = Vec::new();
    let total_v = g.adjacency_list.len();

    for sol_cycle in sol_cycles.iter() {
        match block_method {
            3 => {
                // Technique A1 & A3: Boundary Minimal Cut & Complementary Cut with Totalizer support
                let cut_clauses = get_boundary_cut_clauses(sol_cycle, encoder, g, total_v, balanced);
                clauses.extend(cut_clauses);

                // Technique A2: Induced Subgraph SECs for |C| <= 4
                if sol_cycle.len() <= 4 {
                    let sec_clauses = get_induced_subgraph_sec_clauses(sol_cycle, encoder, g);
                    clauses.extend(sec_clauses);
                }
            }
            0 => clauses.extend(cegar_blocking_clauses(sol_cycle, &encoder.graph_lit_map)),
            1 => clauses.extend(asp_blocking_clauses(sol_cycle, encoder, g, 1, balanced)),
            _ => clauses.extend(asp_blocking_clauses(sol_cycle, encoder, g, 2, balanced)),
        }
    }
    clauses
}

fn cegar_blocking_clauses(cycle:&Vec<i32>,lit_map:&BTreeMap<(i32,i32),Lit>)-> Vec<Clause>{
    let mut clauses =  Vec::new();
    // for cycle in sol_cycles.iter() {
    let len = cycle.len();
    // 順方向
    let mut clause = rustsat::types::Clause::new();
    for i in 0..len {
        let lit = lit_map.get(&(cycle[i], cycle[(i+1)%len])).unwrap();
        clause.add(!*lit);
    }
    clauses.push(clause);

    // 逆方向
    if len != 2{
        let mut clause = rustsat::types::Clause::new();
        for i in (0..len).rev() {
            let lit = lit_map.get(&(cycle[i], cycle[(i+len-1)%len])).unwrap();
            clause.add(!*lit);
        }
        clauses.push(clause);
    }
    // }
    clauses

}

fn asp_blocking_clauses(cycle:&Vec<i32>,encoder: &mut Encoder,g:&Graph, method: i32,balanced:i32) -> Vec<Clause>{
    let mut clauses = Vec::new();
    if method != 4{
        // for cycle in sol_cycles {
        // cycleごとに節を作る
        let mut clause1 = rustsat::types::Clause::new();
        let mut clause2 = rustsat::types::Clause::new();
        for u in cycle.iter() {
            for adjs in g.adjacency_list.get(u).iter(){
                // cycleの中の頂点と、その頂点に接続している頂点のなかで、cycleに入っていないものとの辺を節の中に加える
                for v in adjs.iter(){
                    if !cycle.contains(v){
                        //閉路から出ていく辺と閉路へと入っていく辺両方を同じ節へ追加する
                        if method == 1{
                            let lit1 = encoder.graph_lit_map.get(&(*u,*v)).unwrap();
                            let lit2 = encoder.graph_lit_map.get(&(*v,*u)).unwrap();
                            clause1.extend([*lit1,*lit2]);
                        //閉路から出ていく辺と閉路へと入っていく辺を別々の節へ追加する
                        }else if method == 2{
                            let lit1 = encoder.graph_lit_map.get(&(*u,*v)).unwrap();
                            let lit2 = encoder.graph_lit_map.get(&(*v,*u)).unwrap();
                            clause1.add(*lit1);
                            clause2.add(*lit2);
                        //閉路から出ていく辺のみを節へ追加する
                        }else if method == 3{
                            let lit = encoder.graph_lit_map.get(&(*u,*v)).unwrap();
                            clause1.add(*lit);
                        }
                    }

                }
            }
        }
        if balanced == 0 {
            clauses.push(clause1);
            if clause2.len() != 0{
                clauses.push(clause2);
            }
        }else if balanced == 1 {
            let lits1:Vec<Lit> = clause1.iter().cloned().collect();
            let lits2:Vec<Lit> = clause2.iter().cloned().collect();
            let (adder_clause1,s) = encoder.bailleux_tortalize(lits1.to_vec(),&vec!());
            let (adder_clause2,_) = encoder.bailleux_tortalize(lits2.to_vec(),&s);
            clauses.extend(adder_clause1);
            clauses.extend(adder_clause2);
            clauses.push(clause!(s[0]));

        }else{
            let lits1:Vec<Lit> = clause1.iter().cloned().collect();
            let lits2:Vec<Lit> = clause2.iter().cloned().collect();
            let (adder_clause1,s) = encoder.bailleux_tortalize(lits1.to_vec(),&vec!());
            let (adder_clause2,_) = encoder.bailleux_tortalize(lits2.to_vec(),&s);
            clauses.extend(adder_clause1);
            clauses.extend(adder_clause2);
            clauses.push(clause!(s[0]));
            clauses.push(clause1);
            clauses.push(clause2);
        }
        // }
    }else{
        let highest_v = g.get_highest_degree_vertex();
        // for cycle in sol_cycles {
        //次数が一番高い頂点が含まれてる閉路のみブロック節を追加する
        if cycle.contains(&highest_v){
            let mut clause1 = rustsat::types::Clause::new();
            let mut clause2 = rustsat::types::Clause::new();
            for u in cycle.iter() {
                for adjs in g.adjacency_list.get(u).iter(){
                    // cycleの中の頂点と、その頂点に接続している頂点のなかで、cycleに入っていないものとの辺を節の中に加える
                    for v in adjs.iter(){
                        if !cycle.contains(v){
                        //閉路から出ていく辺と閉路へと入っていく辺を別々の節へ追加する
                        let lit1 = encoder.graph_lit_map.get(&(*u,*v)).unwrap();
                        let lit2 = encoder.graph_lit_map.get(&(*v,*u)).unwrap();
                        clause1.add(*lit1);
                        clause2.add(*lit2);
                        }
                    }
                }
            }
            clauses.push(clause1);
            clauses.push(clause2);
        }
        // }
    }
    clauses
}

    // 要素を追加する関数
fn _add_to_set(set: &mut HashSet<(usize, usize)>, a: usize, b: usize) {
    let pair = if a < b { (a, b) } else { (b, a) };
    set.insert(pair);
}

fn _contains_in_set(set: &HashSet<(usize, usize)>, a: usize, b: usize) -> bool {
    let pair = if a < b { (a, b) } else { (b, a) };
    set.contains(&pair)
}

fn get_active_cycles(cycles: &Vec<Vec<i32>>, active_cycles_number: &Vec<usize>) -> Vec<Vec<i32>> {
    active_cycles_number.iter()
        .map(|&index| cycles[index].clone())
        .collect()
}

fn map_cycle_lengths(cycles: &Vec<Vec<i32>>) -> BTreeMap<usize, i32> {
    let mut length_map = BTreeMap::new();

    for cycle in cycles {
        let length = cycle.len();
        *length_map.entry(length).or_insert(0) += 1;
    }

    length_map
}

pub fn get_boundary_cut_clauses(
    cycle: &[i32],
    encoder: &mut Encoder,
    g: &Graph,
    total_vertices: usize,
    balanced: i32,
) -> Vec<Clause> {
    let mut clauses = Vec::new();
    let cycle_set: HashSet<i32> = cycle.iter().cloned().collect();

    // Technique A3: If |C| > |V| / 2, use complementary set S = V \ C
    let (target_set, is_complementary) = if cycle.len() > total_vertices / 2 && total_vertices > 0 {
        let all_v: HashSet<i32> = g.adjacency_list.keys().cloned().collect();
        let comp_set: HashSet<i32> = all_v.difference(&cycle_set).cloned().collect();
        (comp_set, true)
    } else {
        (cycle_set, false)
    };

    let mut clause_out = rustsat::types::Clause::new();
    let mut clause_in = rustsat::types::Clause::new();

    // Technique A1: Iterate over vertices in target_set and only collect boundary cut edges
    for &u in &target_set {
        if let Some(adjs) = g.adjacency_list.get(&u) {
            for &v in adjs {
                if !target_set.contains(&v) {
                    if let Some(lit_out) = encoder.graph_lit_map.get(&(u, v)) {
                        clause_out.add(*lit_out);
                    }
                    if let Some(lit_in) = encoder.graph_lit_map.get(&(v, u)) {
                        clause_in.add(*lit_in);
                    }
                }
            }
        }
    }

    let (c1, c2) = if is_complementary {
        // By duality: delta^+(V \ C) = delta^-(C) and delta^-(V \ C) = delta^+(C)
        (clause_in, clause_out)
    } else {
        (clause_out, clause_in)
    };

    if balanced == 0 {
        if !c1.is_empty() {
            clauses.push(c1);
        }
        if !c2.is_empty() {
            clauses.push(c2);
        }
    } else if balanced == 1 {
        let lits1: Vec<Lit> = c1.iter().cloned().collect();
        let lits2: Vec<Lit> = c2.iter().cloned().collect();
        let (adder_clause1, s) = encoder.bailleux_tortalize(lits1.to_vec(), &vec![]);
        let (adder_clause2, _) = encoder.bailleux_tortalize(lits2.to_vec(), &s);
        clauses.extend(adder_clause1);
        clauses.extend(adder_clause2);
        if !s.is_empty() {
            clauses.push(clause!(s[0]));
        }
    } else {
        let lits1: Vec<Lit> = c1.iter().cloned().collect();
        let lits2: Vec<Lit> = c2.iter().cloned().collect();
        let (adder_clause1, s) = encoder.bailleux_tortalize(lits1.to_vec(), &vec![]);
        let (adder_clause2, _) = encoder.bailleux_tortalize(lits2.to_vec(), &s);
        clauses.extend(adder_clause1);
        clauses.extend(adder_clause2);
        if !s.is_empty() {
            clauses.push(clause!(s[0]));
        }
        if !c1.is_empty() {
            clauses.push(c1);
        }
        if !c2.is_empty() {
            clauses.push(c2);
        }
    }

    clauses
}

pub fn get_induced_subgraph_sec_clauses(
    cycle: &[i32],
    encoder: &Encoder,
    g: &Graph,
) -> Vec<Clause> {
    let mut clauses = Vec::new();
    let len = cycle.len();
    if len < 3 || len > 6 {
        return clauses;
    }

    let cycle_set: HashSet<i32> = cycle.iter().cloned().collect();

    // 1. Standard forward & reverse exclusion for active cycle C
    let mut fwd_clause = rustsat::types::Clause::new();
    for i in 0..len {
        if let Some(lit) = encoder.graph_lit_map.get(&(cycle[i], cycle[(i + 1) % len])) {
            fwd_clause.add(!*lit);
        }
    }
    if !fwd_clause.is_empty() {
        clauses.push(fwd_clause);
    }

    if len != 2 {
        let mut rev_clause = rustsat::types::Clause::new();
        for i in (0..len).rev() {
            if let Some(lit) = encoder.graph_lit_map.get(&(cycle[i], cycle[(i + len - 1) % len])) {
                rev_clause.add(!*lit);
            }
        }
        if !rev_clause.is_empty() {
            clauses.push(rev_clause);
        }
    }

    // 2. Search for internal chords in G[C] to forbid chord subtours
    // For small |C| <= 6, enumerate chord paths
    for i in 0..len {
        let u = cycle[i];
        if let Some(adjs) = g.adjacency_list.get(&u) {
            for &v in adjs {
                if cycle_set.contains(&v) {
                    let next_u = cycle[(i + 1) % len];
                    let prev_u = cycle[(i + len - 1) % len];
                    // If (u, v) is a chord (not consecutive in C)
                    if v != next_u && v != prev_u {
                        // Forbid shortcut triangle (u, next_u, v) if (next_u, v) is an edge
                        if let Some(next_adjs) = g.adjacency_list.get(&next_u) {
                            if next_adjs.contains(&v) {
                                if let (Some(l1), Some(l2), Some(l3)) = (
                                    encoder.graph_lit_map.get(&(u, next_u)),
                                    encoder.graph_lit_map.get(&(next_u, v)),
                                    encoder.graph_lit_map.get(&(v, u)),
                                ) {
                                    clauses.push(clause!(!*l1, !*l2, !*l3));
                                }
                                if let (Some(l1), Some(l2), Some(l3)) = (
                                    encoder.graph_lit_map.get(&(u, v)),
                                    encoder.graph_lit_map.get(&(v, next_u)),
                                    encoder.graph_lit_map.get(&(next_u, u)),
                                ) {
                                    clauses.push(clause!(!*l1, !*l2, !*l3));
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    clauses
}

// enum MySolver<'a> {
//     Minisat(rustsat_minisat::core::Minisat),
//     Kissat(rustsat_kissat::Kissat<'a>), 
//     CaDiCaL(rustsat_cadical::CaDiCaL<'a, 'a>),
// }


// impl Solve for MySolver<'_> {
//     fn solve(&mut self) -> Result<SolverResult, SolverError>{
//         match self {
//             MySolver::Minisat(solver) => solver.solve(),
//             MySolver::Kissat(solver) => solver.solve(), 
//             MySolver::CaDiCaL(solver) => solver.solve(),
//         }
//     }

//     fn full_solution(&self) -> Result<Assignment, SolverError>{
//         match self {
//             MySolver::Minisat(solver) => solver.full_solution(),
//             MySolver::Kissat(solver) => solver.full_solution(), 
//             MySolver::CaDiCaL(solver) => solver.full_solution(),
//         }
//     }

//     fn add_cnf(&mut self, cnf: Cnf) -> SolveMightFail{
//         match self {
//             MySolver::Minisat(solver) => solver.add_cnf(cnf),
//             MySolver::Kissat(solver) => solver.add_cnf(cnf),
//             MySolver::CaDiCaL(solver) => solver.add_cnf(cnf),
//         }
//     }

//     fn signature(&self) -> &'static str {
//         match self {
//             MySolver::Minisat(solver) => solver.signature(),
//             MySolver::Kissat(solver) => solver.signature(),
//             MySolver::CaDiCaL(solver) => solver.signature(),
//         }
//     }

//     fn lit_val(&self, lit: rustsat::types::Lit) -> Result<rustsat::types::TernaryVal, rustsat::solvers::SolverError> { 
//         match self {
//             MySolver::Minisat(solver) => solver.lit_val(lit),
//             MySolver::Kissat(solver) => solver.lit_val(lit),
//             MySolver::CaDiCaL(solver) => solver.lit_val(lit),
//         }
//     }

//     fn add_clause(&mut self, clause: rustsat::types::Clause) -> Result<(), rustsat::solvers::SolverError>{
//         match self {
//             MySolver::Minisat(solver) => solver.add_clause(clause),
//             MySolver::Kissat(solver) => solver.add_clause(clause),
//             MySolver::CaDiCaL(solver) => solver.add_clause(clause),
//         }
//     }

//     // Implement other required methods in a similar way...
// }

// impl<'a> Extend<rustsat::types::Clause> for MySolver<'a> {
//     fn extend<T: IntoIterator<Item = rustsat::types::Clause>>(&mut self, iter: T) {
//         match self {
//             MySolver::Minisat(solver) => solver.extend(iter),
//             MySolver::Kissat(solver) => solver.extend(iter),
//             MySolver::CaDiCaL(solver) => solver.extend(iter),
//         }
//     }
// }

// impl SolveStats for MySolver<'_>{
//     fn stats(&self) -> SolverStats{
//         match self {
//             MySolver::Minisat(solver) => solver.stats(),
//             MySolver::Kissat(solver) => solver.stats(),
//             MySolver::CaDiCaL(solver) => solver.stats(),
//         }
//     }
// }

// impl<'a> MySolver<'a>{
//     fn set_configuration(&mut self, config:Config) -> SolveMightFail{
//         match self{
//             MySolver::CaDiCaL(solver) => solver.set_configuration(config),
//             _ => Err(SolverError::Api("このソルバーではset_configrationは使用できません。".to_string()))
//         }
//     }
// }

#[cfg(test)]
mod tests_blocking_enhancements {
    use super::*;

    #[test]
    fn test_boundary_cut_complementary_equivalence() {
        // Build a 6-vertex cycle graph 1-2-3-4-5-6-1 with a chord 1-4
        let mut g = Graph::new();
        g.add_edge(1, 2);
        g.add_edge(2, 3);
        g.add_edge(3, 4);
        g.add_edge(4, 5);
        g.add_edge(5, 6);
        g.add_edge(6, 1);
        g.add_edge(1, 4); // chord

        let mut encoder = Encoder::new();
        encoder.encode(&g, 1, 0, 0, 0, 0, 0);

        // Subcycle C = [1, 2, 3, 4] (|C| = 4 > 6/2 = 3) -> Complementary S = [5, 6]
        let c = vec![1, 2, 3, 4];
        let clauses = get_boundary_cut_clauses(&c, &mut encoder, &g, 6, 0);
        assert!(!clauses.is_empty());
        // Verify both out-cut and in-cut clauses exist
        assert_eq!(clauses.len(), 2);

        // Compare with direct calculation (total_vertices = 0 disables complementation)
        let clauses_direct = get_boundary_cut_clauses(&c, &mut encoder, &g, 0, 0);
        assert_eq!(clauses_direct.len(), 2);

        let set_comp_0: HashSet<Lit> = clauses[0].iter().cloned().collect();
        let set_dir_0: HashSet<Lit> = clauses_direct[0].iter().cloned().collect();
        assert_eq!(set_comp_0, set_dir_0);

        let set_comp_1: HashSet<Lit> = clauses[1].iter().cloned().collect();
        let set_dir_1: HashSet<Lit> = clauses_direct[1].iter().cloned().collect();
        assert_eq!(set_comp_1, set_dir_1);
    }

    #[test]
    fn test_induced_subgraph_chord_cycles() {
        // 4-vertex graph 1-2-3-4 with chord (1,3)
        let mut g = Graph::new();
        g.add_edge(1, 2);
        g.add_edge(2, 3);
        g.add_edge(3, 4);
        g.add_edge(4, 1);
        g.add_edge(1, 3); // chord (1, 3)

        let mut encoder = Encoder::new();
        encoder.encode(&g, 1, 0, 0, 0, 0, 0);

        let c = vec![1, 2, 3, 4];
        let clauses = get_induced_subgraph_sec_clauses(&c, &encoder, &g);
        // Must generate at least the main cycle exclusion + chord triangle exclusion
        assert!(!clauses.is_empty());
        // Forward (1) + Reverse (1) for 4-cycle, plus chord triangles (forward & reverse for each of 2 triangles = 4 clauses) -> 6 clauses
        assert_eq!(clauses.len(), 6);

        // Out of bounds cycles (|C| < 3 or |C| > 6) should return empty
        let c_small = vec![1, 2];
        assert!(get_induced_subgraph_sec_clauses(&c_small, &encoder, &g).is_empty());

        let c_large = vec![1, 2, 3, 4, 5, 6, 7];
        assert!(get_induced_subgraph_sec_clauses(&c_large, &encoder, &g).is_empty());
    }

    #[test]
    fn test_hub_aware_swap_node() {
        // Build graph with hub 1 connected to vertices 2..=31
        let mut g = Graph::new();
        for v in 2..=31 {
            g.add_edge(1, v);
            let next_v = if v == 31 { 2 } else { v + 1 };
            g.add_edge(v, next_v);
        }
        let (contracted_g, contractor) = Degree2Contractor::contract(&g);
        let hub_registry = HubRegistry::new(&contracted_g);
        assert!(hub_registry.is_hub_vertex(1));

        let c1 = vec![1, 2, 3];
        let c2 = vec![4, 5, 6];
        let merged = swap_node(&c1, &c2, &contracted_g, &contractor, &hub_registry);
        assert!(merged.is_some());
    }

    #[test]
    fn test_two_opt_hub_prioritization() {
        let mut g = Graph::new();
        for v in 2..=31 {
            g.add_edge(1, v);
            let next_v = if v == 31 { 2 } else { v + 1 };
            g.add_edge(v, next_v);
        }
        g.add_edge(6, 4);
        g.add_edge(9, 7);
        g.add_edge(5, 8);
        g.add_edge(5, 9);
        let (contracted_g, contractor) = Degree2Contractor::contract(&g);
        let hub_registry = HubRegistry::new(&contracted_g);

        let sol_cycles = vec![
            vec![7, 8, 9],
            vec![1, 2, 3],
            vec![4, 5, 6],
        ];
        let mut encoder = Encoder::new();
        encoder.encode(&contracted_g, 1, 0, 0, 0, 0, 0);

        let (_clauses, merged_cycles) = two_opt(
            &sol_cycles,
            &mut encoder,
            &contracted_g,
            &contractor,
            &hub_registry,
            3,
            0,
            1,
            0,
        );
        // All 9 vertices should be merged into a single cycle
        assert_eq!(merged_cycles.len(), 1);
        assert_eq!(merged_cycles[0].len(), 9);
    }
}



