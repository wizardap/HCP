use clap::{App, Arg};

pub fn get_options() -> clap::ArgMatches {
    return App::new("HPC Solver")
        .version("1.0")
        .author("Me <me@example.com>")
        .about("Solves Hamiltonian circuits")
        .arg(
            Arg::with_name("input")
                .short('i')
                .long("input")
                .value_name("FILE NAME")
                .help("Input file (Required)")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("blocking")
                .short('b')
                .long("block")
                .value_name("n")
                .help("Blocking method:
    0: exiting CEGAR (default)
    1: Add outgoing and incoming cut-arcs to the same clause
    2: Add existing block clauses and option 1 clause
    3: (proposed) Add cut-arcs to separate clauses
    4: Add only outgoing cut-arcs
    5: Add cut-arcs to separate clauses by only highest vertex
    6: Use exiting methods only when vertices are three or fewer
    7: Use exiting methods only when vertices are four or fewer
    8: Use exiting methods only when vertices are five or fewer
    9: Adopt the shorter between the exiting and proposed
    10: proposed and add the exiting only three vertices")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("symmetry")
                .short('y')
                .long("symmetry")
                .value_name("n")
                .help("Symmetry blocking method:
    0: No Block Symmetry option (default)
    1: Block symmetry for smallest degree vertex
    2: Block symmetry for largest degree vertex
    3: Block symmetry for smallest degree vertex by support (can't use)
    4: Block symmetry for smallest degree vertex by cardinality constraint")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("2-opt")
                .short('t')
                .long("two-opt")
                .value_name("n")
                .help("2-opt method:
    0: No 2-opt option (default)
    1: Add block clauses to subcycles found by SAT solver and to each merged subcycle
    2: Add block clauses to subcycles found by SAT solver and to most merged subcycles
    3: Add block clauses only to the most merged subcycles
    4: If even one cannot be merged, terminate and add the block sections up to that point
    5: If even one cannot be merged, terminate")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("three-opt")
                .short('x')
                .long("three-opt")
                .value_name("n")
                .help("Restricted 3-opt method:\n    0: Disabled (default)\n    1: Enabled — fallback inside 2-opt loop when 2-opt is stuck")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("loop-prohibition")
                .short('l')
                .long("loop")
                .value_name("n")
                .help("Loop prohibition:
    0: No Loop prohibition option (default)
    1: Prohibit loops with only two vertices")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("de-arcify")
                .short('d')
                .long("de-arcify")
                .value_name("n")
                .help("Remove redundant arcs:
    0: No de-arcify option (default)
    1: Remove redundant arcs before encoding")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("degree-order")
                .short('r')
                .long("degree_order")
                .value_name("n")
                .help("clauses order:
    0: (default)
    1: ascending order by degree
    2: descending order by degree")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("arcs-order")
                .short('a')
                .long("arc_order")
                .value_name("n")
                .help("literal number order:
    0: (default)
    1: ascending order by degree
    2: descending order by degree")
                .takes_value(true),
        )
        .get_matches();
}


//実行方法
// cargo run -- --solver 1 --encoding 2
// cargo run -- -s 1 -e 2
// cargo run
