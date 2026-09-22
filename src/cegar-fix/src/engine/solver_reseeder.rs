use rustsat::instances::Cnf;
use rustsat::solvers::Solve;
use rustsat_cadical::{CaDiCaL, Config};

#[derive(Debug, Clone)]
pub struct ReseederOptions {
    pub max_sat_time_threshold_secs: f64, // Default: 15.0s
    pub periodic_interval_rounds: usize,  // Default: 10 rounds
    pub enable_reseeding: bool,           // Default: true
}

impl Default for ReseederOptions {
    fn default() -> Self {
        Self {
            max_sat_time_threshold_secs: 15.0,
            periodic_interval_rounds: 10,
            enable_reseeding: true,
        }
    }
}

pub struct SolverReseeder;

impl SolverReseeder {
    pub fn should_reseed(
        last_sat_time_secs: f64,
        current_round: usize,
        options: &ReseederOptions,
    ) -> bool {
        if !options.enable_reseeding || current_round == 0 {
            return false;
        }
        last_sat_time_secs >= options.max_sat_time_threshold_secs
            || (options.periodic_interval_rounds > 0 && current_round % options.periodic_interval_rounds == 0)
    }

    pub fn reseed_solver(
        base_cnf: &Cnf,
        accumulated_cuts: &[Cnf],
        cadical_config: i32,
    ) -> CaDiCaL<'static, 'static> {
        let mut solver = CaDiCaL::default();
        match cadical_config {
            1 => { let _ = solver.set_configuration(Config::Sat); }
            2 => { let _ = solver.set_configuration(Config::Plain); }
            3 => { let _ = solver.set_configuration(Config::Default); }
            4 => { let _ = solver.set_configuration(Config::Unsat); }
            _ => {}
        }

        let _ = solver.add_cnf(base_cnf.clone());
        for cut_cnf in accumulated_cuts {
            let _ = solver.add_cnf(cut_cnf.clone());
        }
        solver
    }
}
