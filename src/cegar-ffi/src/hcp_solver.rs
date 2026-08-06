extern crate libc;

use libc::c_int;
// use rustsat::instances::*;
// use rustsat::types::*;
// use rustsat::clause;

use std::collections::{BTreeMap,HashSet};
use std::time::{Instant,Duration};
use std::vec;
// use crate::encoder;
use crate::graph::*;
use crate::encoder::*;
// use crate::file_operations;


pub fn solve_hamilton(g:Graph, _s:i32, block_method: i32,symmetry: i32 ,opt:i32,loop_prohibition: i32, degree_order:i32, arcs_order:i32, instant:Instant) {
    let now = instant.elapsed();
    let mut encoder = Encoder::new();
    unsafe{
        let solver = Solver_new();
        // グラフをcnf形式に変形し、cnfへ格納
        encoder.encode(solver,&g,symmetry,loop_prohibition,degree_order,arcs_order);

        // 標準入力で -s の後の数字により、minisat,kissat,cadicalを選択する
        println!("encodhing time = {:?}",instant.elapsed()-now);
        
        // cegar関数により、解を求め、increment数と追加したblock節の合計を返す
        let (increment,block) = cegar(&mut encoder,solver,0,0, g,block_method, opt,instant,instant.elapsed());
        Solver_delete(solver);
        println!("overall incremented number = {}",increment);
        // println!("overall number of added block clauses = {}",block);
    }
    
}

fn cegar(encoder: &mut Encoder,solver:*mut Solver,mut count: i32,block_count:i32, g:Graph,block_method: i32, opt:i32, instant:Instant,previous_time:Duration) ->(i32,i32) {
    //SATソルバーで解を求める
    unsafe{
        let res = Solver_solve(solver);
        let now = instant.elapsed();
        let sat_solving_time = now - previous_time;

        println!();
        println!("Increment...");
        println!("incremented number = {}",count);
        println!("sat solving time = {:?}",sat_solving_time);
        // println!("res {}",res);
        //解がSATならば、ハミルトン閉路になっているかを調べる
        if res == 10{
            //どの辺が選択されたかの解析
            let sol_arcs = get_solution_arcs(solver,&encoder.graph_lit_map);
            //閉路
            let sol_cycles = get_solution_cycles(sol_arcs);
            //閉路が一つであれば、ハミルトン閉路なので解を出力
            if sol_cycles.len() == 1{
                let flat: Vec<i32> = sol_cycles.into_iter().flatten().collect();
                let line = flat.iter().map(|i| i.to_string()).collect::<Vec<String>>().join(" ");
                println!();
                println!("solution: ");
                println!("{}\n", line);
                println!("s SATISFIABLE");
                return (count, block_count);
            }else{
                println!("number of subcycles found = {}",sol_cycles.len());
                println!("sat solution cycle lengths map (length:number) = {:?}",map_cycle_lengths(&sol_cycles));
            //閉路が二つ以上であれば、ソルバーにブロック節を加えて、もう一度解を求める
                if opt == 0{
                    get_blocking_clauses(&sol_cycles,solver,encoder,&g,block_method);
                }else if opt >= 1{
                    let cycles = two_opt(&sol_cycles,solver,encoder,&g,block_method,opt);
                    if cycles.len() == 1{
                        let flat: Vec<i32> = cycles.into_iter().flatten().collect();
                        let line = flat.iter().map(|i| i.to_string()).collect::<Vec<String>>().join(" ");
                        let now = instant.elapsed();
                        let time = now-previous_time;
                        let add_block_clauses_time = now-previous_time-sat_solving_time;
                        // println!("number of added block clauses = {}",block_count);
                        println!("add block clauses time = {:?}",add_block_clauses_time);
                        println!("increment time = {:?}", time);
                        println!();
                        println!("hamiltonian cycle found by 2-opt");
                        println!("solution: ");
                        println!("{}\n", line);
                        println!("s SATISFIABLE");
                        return (count, block_count);
                    }
                }else{
                    panic!("2-opt option \n-t 0:2-opt off\n-t 1,2,3:2-opt on");
                };
                // block_count += block_clauses.len() as i32;
                // cnf.extend(block_clauses);
                count += 1;


                let now = instant.elapsed();
                let time = now-previous_time;
                let add_block_clauses_time = now-previous_time-sat_solving_time;
                println!("number of added block clauses = {}",block_count);
                println!("add block clauses time = {:?}", add_block_clauses_time);
                println!("increment time = {:?}", time);
                
                return cegar(encoder,solver, count,block_count, g, block_method,opt,instant,now);
            }
        }else{
            println!("s UNSATISFIABLE");
            return (count, block_count);
        }
    }
}

fn get_solution_arcs(solver:*mut Solver,lit_map:&BTreeMap<(i32,i32),Lit>) -> Vec<(i32,i32)>{
    unsafe{
        let sol_arcs: Vec<(i32,i32)> = lit_map.iter().filter_map(|((u,v), lit)| if Solver_val(solver, lit.var()) > 0 { Some((*u,*v)) } else { None }).collect();
        sol_arcs
    }
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


//2-optアルゴリズム
//ブロック節と、つながって新たに見つかった閉路を返す
fn two_opt(sol_cycles:&Vec<Vec<i32>>,solver:*mut Solver,encoder: &mut Encoder,g:&Graph,block_method:i32,opt:i32) -> Vec<Vec<i32>>{
    // let mut block_clauses = Vec::new();
    let mut cycles = sol_cycles.to_vec();
    let mut merged = true;
    let mut maximam_cycles = Vec::new();
    // let mut cache_set: HashSet<(usize,usize)> = HashSet::new();
    let mut cache_vertex: HashSet<usize> = HashSet::new();
    let mut active_cycles_number = Vec::new();

    if opt !=3{
        // block_clauses.extend(get_blocking_clauses(&sol_cycles,solver,encoder,&g,block_method));
        get_blocking_clauses(&sol_cycles,solver,encoder,&g,block_method);
    // }else if block_method >= 10{
    //     let subclauses = get_blocking_clauses(&sol_cycles,solver,encoder,&g,block_method+100);
    //     if subclauses.len() != 0{
    //         block_clauses.extend(subclauses);
    //     }
    }


    for i in 0..cycles.len(){
        active_cycles_number.push(i);
    }

    while merged{
 
        let (new_merged,merged_numbers,new_cycle) = merge_cycles(&cycles,g,&mut cache_vertex,&active_cycles_number,opt);
        merged = new_merged;
        
        if merged{
            cycles.push(new_cycle.clone());
            active_cycles_number.swap_remove(merged_numbers.1);
            active_cycles_number.swap_remove(merged_numbers.0);
            active_cycles_number.push(cycles.len()-1);
        }

        if active_cycles_number.len() == 1 || !merged{
            break
        }
        if opt==1 || opt == 4{
            get_blocking_clauses(&vec!(new_cycle), solver, encoder, g, block_method);
        }else{
            // if block_method >= 10{
            //     let active_cycles = get_active_cycles(&cycles, &active_cycles_number);
            //     let subclauses = get_blocking_clauses(&active_cycles,solver,encoder,&g,block_method+100);
            //     if subclauses.len() != 0{
            //         block_clauses.extend(subclauses);
            //     }
            // }
            maximam_cycles = new_cycle;
        }
    }
    if opt==2 && maximam_cycles.len() != 0{
        get_blocking_clauses(&vec!(maximam_cycles), solver, encoder, g, block_method);
    }
    if opt==3{
        let active_cycles = get_active_cycles(&cycles, &active_cycles_number);
        get_blocking_clauses(&active_cycles,solver, encoder, g, block_method);
    }

    println!("number of connected cycles = {}",cycles.len()-sol_cycles.len());
    println!("number of merged cycles = {}",active_cycles_number.len());
    println!("merged cycle lengths map (length:number) = {:?}",map_cycle_lengths(&get_active_cycles(&cycles, &active_cycles_number)));

    get_active_cycles(&cycles, &active_cycles_number).to_vec()    

}

fn merge_cycles(cycles:&Vec<Vec<i32>>,g:&Graph,cache_vertex:&mut HashSet<usize>,active_cycles_number:&Vec<usize>,opt:i32) -> (bool,(usize,usize),Vec<i32>){
    //(block_clauses,merged,(merged_number1,merged_number2),new_cycle)
    
    for i in 0..active_cycles_number.len(){
        let left = active_cycles_number[i];
        if !cache_vertex.contains(&left){
            for j in i+1..active_cycles_number.len(){
                let right = active_cycles_number[j];

                // if !contains_in_set(cache_set, left, right) && !cache_vertex.contains(&right){
                match swap_node(&cycles[left],&cycles[right],&g){
                    Some(new_cycle) =>{
                    return (true,(i,j),new_cycle)
                    }
                    None =>{
                        // add_to_set(cache_set,left,right);
                        continue
                    }
                }
                
            }
            cache_vertex.insert(left);
        }
        if opt == 4 || opt == 5{
            // println!("break");
            return (false,(0,0),vec!())
        }
    }
    
    (false,(0,0),vec!())
}


fn swap_node(cycle1:&Vec<i32>,cycle2:&Vec<i32>,g:&Graph) -> Option<Vec<i32>>{

    for i in 0..cycle1.len(){
        let adjs_of_left_head = g.adjacency_list.get(&cycle1[i]).unwrap();
        let adjs_of_left_tail = g.adjacency_list.get(&cycle1[(i+1)%cycle1.len()]).unwrap();

        for j in 0..cycle2.len(){
            if adjs_of_left_head.contains(&cycle2[j]){

                if adjs_of_left_tail.contains(&cycle2[(j+1)%cycle2.len()]){
                    return cycle_join(&cycle1,&cycle2,i,j,true)
                }

                if adjs_of_left_tail.contains(&cycle2[(j+cycle2.len()-1)%cycle2.len()]){
                    return cycle_join(&cycle1,&cycle2,i,j,false)
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



fn get_blocking_clauses(sol_cycles:&Vec<Vec<i32>>,solver:*mut Solver,encoder: &mut Encoder,g:&Graph, block_method:i32){

    // let mut cut_arcs_map = BTreeMap::new();

    for sol_cycle in sol_cycles.iter(){
        match block_method{
            0 => cegar_blocking_clauses(&sol_cycle,solver,&encoder.graph_lit_map),
            1 => asp_blocking_clauses(&sol_cycle,solver,encoder,&g,1),//閉路から出ていく辺と閉路へと入っていく辺両方を同じ節へ追加する
            // 2 => [cegar_blocking_clauses(&sol_cycle.clone(),solver,&encoder.graph_lit_map.clone()),asp_blocking_clauses(sol_cycle,encoder,&g,1)].concat(),//既存ブロック節を追加し、閉路から出ていく辺と閉路へと入っていく辺両方を同じ節へ追加する
            3 => {
                // let clauses1 = asp_blocking_clauses(&sol_cycle,solver,encoder,&g,2);
                // *cut_arcs_map.entry(clauses1[0].len()).or_insert(0) += 1;
                // clauses1
                asp_blocking_clauses(&sol_cycle,solver,encoder,&g,2)
            },//閉路から出ていく辺と閉路へと入っていく辺を別々の節へ追加する
            4 => asp_blocking_clauses(&sol_cycle,solver,encoder,&g,3),//閉路から出ていく辺のみを節へ追加する
            5 => asp_blocking_clauses(&sol_cycle,solver,encoder,&g,4),//次数が一番高い頂点が含まれてる閉路のみブロック節を追加する
            6 => {
                //3頂点以下の場合にだけ従来手法を採用
                if sol_cycle.len() > 3{
                    asp_blocking_clauses(&sol_cycle,solver,encoder,&g,2)
                }else{
                    cegar_blocking_clauses(&sol_cycle,solver,&encoder.graph_lit_map)
                }
            },
            7 => {
                //4頂点以下の場合にだけ従来手法を採用
                if sol_cycle.len() > 4{
                    asp_blocking_clauses(&sol_cycle,solver,encoder,&g,2)
                }else{
                    cegar_blocking_clauses(&sol_cycle,solver,&encoder.graph_lit_map)
                }
            },
            8 => {
                //5頂点以下の場合にだけ従来手法を採用
                if sol_cycle.len() > 5{
                    asp_blocking_clauses(&sol_cycle,solver,encoder,&g,2)
                }else{
                    cegar_blocking_clauses(&sol_cycle,solver,&encoder.graph_lit_map)
                }
            },
            // 9 => {
            //     //従来手法と提案手法の長さを比較して短い方を採用
            //     let clauses1 = asp_blocking_clauses(&sol_cycle.clone(),encoder,&g,2,balanced);
            //     let clauses2 = cegar_blocking_clauses(&sol_cycle,&encoder.graph_lit_map);
            //     *cut_arcs_map.entry(clauses1[0].len()).or_insert(0) += 1;
            //     if clauses1[0].len() > clauses2[0].len(){
            //         clauses2
            //     }else{
            //         clauses1
            //     }
            // },
            // 10 => {
            //     let mut subsubclauses = Vec::new();
            //     if sol_cycle.len() == 3{
            //         subsubclauses.extend(cegar_blocking_clauses(&sol_cycle.clone(),&encoder.graph_lit_map));
            //     }
            //     let clauses1 = asp_blocking_clauses(&sol_cycle.clone(),encoder,&g,2,balanced);
            //     *cut_arcs_map.entry(clauses1[0].len()).or_insert(0) += 1;
            //     subsubclauses.extend(clauses1);

            //     subsubclauses
            // },
            // 11 => {
            //     let mut subsubclauses = Vec::new();
            //     if sol_cycle.len() >= 3 && sol_cycle.len() <= 6{
            //         subsubclauses.extend(cegar_blocking_clauses(&sol_cycle.clone(),&encoder.graph_lit_map));
            //     }
            //     let clauses1 = asp_blocking_clauses(&sol_cycle.clone(),encoder,&g,2,balanced);
            //     *cut_arcs_map.entry(clauses1[0].len()).or_insert(0) += 1;
            //     subsubclauses.extend(clauses1);

            //     subsubclauses
            // },
            // 110 => {
            //     let mut subsubclauses = Vec::new();
            //     if sol_cycle.len() == 3{
            //         subsubclauses.extend(cegar_blocking_clauses(&sol_cycle.clone(),&encoder.graph_lit_map));
            //     }

            //     subsubclauses
            // }
            // 111 => {
            //     let mut subsubclauses = Vec::new();
            //     if sol_cycle.len() >= 3 && sol_cycle.len() <= 6{
            //         subsubclauses.extend(cegar_blocking_clauses(&sol_cycle.clone(),&encoder.graph_lit_map));
            //     }

            //     subsubclauses
            // }
            _ => panic!("out of number by blocking method")
        };
        // clauses.extend(subclause);
    }
    // if block_method == 3 || (block_method >= 9 && block_method <= 11){
    //     println!("cut arcs number = {cut_arcs_map:?}");
    // }
    // let clauses:Vec<Clause> =
    //     if block_method == 0{
    //         cegar_blocking_clauses(&sol_cycles,&encoder.graph_lit_map)
    //     }else if block_method == 1{
    //         //閉路から出ていく辺と閉路へと入っていく辺両方を同じ節へ追加する
    //         asp_blocking_clauses(&sol_cycles,encoder,&g,1,balanced)
    //     }else if block_method == 2{
    //         //既存ブロック節を追加し、閉路から出ていく辺と閉路へと入っていく辺両方を同じ節へ追加する
    //         [cegar_blocking_clauses(&sol_cycles.clone(),&encoder.graph_lit_map.clone()),asp_blocking_clauses(sol_cycles,encoder,&g,1,balanced)].concat()
    //     }else if block_method == 3{
    //         //閉路から出ていく辺と閉路へと入っていく辺を別々の節へ追加する
    //         asp_blocking_clauses(&sol_cycles,encoder,&g,2,balanced)
    //     }else if block_method == 4{
    //         //閉路から出ていく辺のみを節へ追加する
    //         asp_blocking_clauses(&sol_cycles,encoder,&g,3,balanced)
    //     }else if block_method == 5{
    //         //閉路から出ていく辺のみを節へ追加する
    //         asp_blocking_clauses(&sol_cycles,encoder,&g,4,balanced)
    //     }else{
    //         panic!("cegarのみ:-b 0\naspのみ:-b 1\n両方:-b 2");
    //     };
    // clauses

    
}

fn cegar_blocking_clauses(cycle:&Vec<i32>,solver:*mut Solver,lit_map:&BTreeMap<(i32,i32),Lit>){
    // let mut clauses =  Vec::new();
    // for cycle in sol_cycles.iter() {
    let len = cycle.len();
    // 順方向
    let mut lits1 = Vec::new();
    for i in 0..len {
        let lit = lit_map.get(&(cycle[i], cycle[(i+1)%len])).unwrap();
        lits1.push(*lit);
    }
    Encoder::at_most_n(solver, &lits1, (lits1.len()-1) as i32);
    

    // 逆方向
    if len != 2{
        let mut lits2 = Vec::new();
        for i in (0..len).rev() {
            let lit = lit_map.get(&(cycle[i], cycle[(i+len-1)%len])).unwrap();
            lits2.push(*lit);
        }
        Encoder::at_most_n(solver, &lits2, (lits2.len()-1) as i32);
    }
    // }

}

fn asp_blocking_clauses(cycle:&Vec<i32>,solver:*mut Solver,encoder: &mut Encoder,g:&Graph, method: i32){
    // let mut clauses = Vec::new();
    // if method != 4{
        // for cycle in sol_cycles {
        // cycleごとに節を作る
        let mut lits1 = Vec::new();
        let mut lits2 = Vec::new();
        for u in cycle.iter() {
            for adjs in g.adjacency_list.get(u).iter(){
                // cycleの中の頂点と、その頂点に接続している頂点のなかで、cycleに入っていないものとの辺を節の中に加える
                for v in adjs.iter(){
                    if !cycle.contains(v){
                        //閉路から出ていく辺と閉路へと入っていく辺両方を同じ節へ追加する
                        if method == 1{
                            let lit1 = encoder.graph_lit_map.get(&(*u,*v)).unwrap();
                            let lit2 = encoder.graph_lit_map.get(&(*v,*u)).unwrap();
                            lits1.extend([*lit1,*lit2]);
                        //閉路から出ていく辺と閉路へと入っていく辺を別々の節へ追加する
                        }else if method == 2{
                            let lit1 = encoder.graph_lit_map.get(&(*u,*v)).unwrap();
                            let lit2 = encoder.graph_lit_map.get(&(*v,*u)).unwrap();
                            lits1.push(*lit1);
                            lits2.push(*lit2);
                        //閉路から出ていく辺のみを節へ追加する
                        }else if method == 3{
                            let lit = encoder.graph_lit_map.get(&(*u,*v)).unwrap();
                            lits1.push(*lit);
                        }
                    }

                }
            }
        }
        // if balanced == 0 {
            // clauses.push(clause1);
            Encoder::at_least_n(solver, &lits1, 1);
            if lits2.len() != 0{
                Encoder::at_least_n(solver, &lits2, 1);
            }
        // }else if balanced == 1 {
        //     let lits1:Vec<Lit> = clause1.iter().cloned().collect();
        //     let lits2:Vec<Lit> = clause2.iter().cloned().collect();
        //     let (adder_clause1,s) = encoder.bailleux_tortalize(lits1.to_vec(),&vec!());
        //     let (adder_clause2,_) = encoder.bailleux_tortalize(lits2.to_vec(),&s);
        //     clauses.extend(adder_clause1);
        //     clauses.extend(adder_clause2);
        //     clauses.push(clause!(s[0]));

        // }else{
        //     let lits1:Vec<Lit> = clause1.iter().cloned().collect();
        //     let lits2:Vec<Lit> = clause2.iter().cloned().collect();
        //     let (adder_clause1,s) = encoder.bailleux_tortalize(lits1.to_vec(),&vec!());
        //     let (adder_clause2,_) = encoder.bailleux_tortalize(lits2.to_vec(),&s);
        //     clauses.extend(adder_clause1);
        //     clauses.extend(adder_clause2);
        //     clauses.push(clause!(s[0]));
        //     clauses.push(clause1);
        //     clauses.push(clause2);
        // }
        // }
    // }else{
    //     let highest_v = g.get_highest_degree_vertex();
    //     // for cycle in sol_cycles {
    //     //次数が一番高い頂点が含まれてる閉路のみブロック節を追加する
    //     if cycle.contains(&highest_v){
    //         let mut clause1 = rustsat::types::Clause::new();
    //         let mut clause2 = rustsat::types::Clause::new();
    //         for u in cycle.iter() {
    //             for adjs in g.adjacency_list.get(u).iter(){
    //                 // cycleの中の頂点と、その頂点に接続している頂点のなかで、cycleに入っていないものとの辺を節の中に加える
    //                 for v in adjs.iter(){
    //                     if !cycle.contains(v){
    //                     //閉路から出ていく辺と閉路へと入っていく辺を別々の節へ追加する
    //                     let lit1 = encoder.graph_lit_map.get(&(*u,*v)).unwrap();
    //                     let lit2 = encoder.graph_lit_map.get(&(*v,*u)).unwrap();
    //                     clause1.add(*lit1);
    //                     clause2.add(*lit2);
    //                     }
    //                 }
    //             }
    //         }
    //         clauses.push(clause1);
    //         clauses.push(clause2);
    //     }
    //     // }
    // }
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

#[repr(C)]
pub struct Solver {
    _private: [u8; 0], // サイズが0のバイト配列（ダミー）
}

extern "C" {
    fn Solver_new() -> *mut Solver;
    fn Solver_delete(solver: *mut Solver);
    pub fn Solver_add(solver: *mut Solver, lit: c_int);
    // fn Solver_assume(solver: *mut Solver, lit: c_int);
    fn Solver_solve(solver: *mut Solver) -> c_int;
    fn Solver_val(solver: *mut Solver, lit: c_int) -> c_int;
    // fn Solver_failed(solver: *mut Solver, lit: c_int) -> c_int;
    pub fn Solver_CARadd(solver: *mut Solver, lit: c_int, encoding: bool);
}

