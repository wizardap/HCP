mod encoder;
mod file_operations;
mod graph;
mod hcp_solver;
mod options;
use std::time::Instant;
use log::info;

fn main() {
    env_logger::init();
    info!("プログラム開始");
    let instant = Instant::now();

    let matches = options::get_options();

    // solver,encodingのオプションをintで受け取る
    let solver = matches.value_of_t::<i32>("solver").unwrap_or(0);
    // let _encoding = matches.value_of_t::<i32>("encoding").unwrap_or(0);
    let blocking = matches.value_of_t::<i32>("blocking").unwrap_or(0);
    let symmetry = matches.value_of_t::<i32>("symmetry").unwrap_or(0);
    let two_opt = matches.value_of_t::<i32>("2-opt").unwrap_or(0);
    let three_opt = matches.value_of_t::<i32>("three-opt").unwrap_or(0);
    let loop_prohibition = matches.value_of_t::<i32>("loop-prohibition").unwrap_or(0);
    // let _cnf_normalize = matches.value_of_t::<i32>("cnf-normalize").unwrap_or(0);
    // let _balanced = matches.value_of_t::<i32>("balanced").unwrap_or(0);
    let de_arcify = matches.value_of_t::<i32>("de-arcify").unwrap_or(0);
    // let _config = matches.value_of_t::<i32>("set-configration").unwrap_or(0);
    let degree_order = matches.value_of_t::<i32>("degree-order").unwrap_or(0);
    let arcs_order = matches.value_of_t::<i32>("arcs-order").unwrap_or(0);
    // solver,encodingのオプションを&strで受け取る
    let input_filename = matches.value_of("input").unwrap_or("default");
    // let _output_foldername = matches.value_of("output").unwrap_or("default");

    println!("solve {}", input_filename);
    // let g = instance();
    let mut g = file_operations::input_to_graph(input_filename);
    if de_arcify != 0{
        g.remove_redundant_arcs();
    }
    let time1 = instant.elapsed();
    // println!("encodhing time = {:?} sec",instant.elapsed().as_secs());
    // let instant2 = Instant::now();

    // println!("solver={},encoding={}",solver,encoding);
    // println!("{:?}",g);
    println!("file input time = {:?}", time1);
    hcp_solver::solve_hamilton(g, solver, blocking, symmetry, two_opt, loop_prohibition,degree_order,arcs_order,instant);
    let time2 = instant.elapsed() - time1;

    // println!("solving time = {:?} sec",instant2.elapsed().as_secs());
    println!("solving time = {:?}", time2);
    println!("overall time = {:?}", instant.elapsed());
    info!("プログラム終了");
}
