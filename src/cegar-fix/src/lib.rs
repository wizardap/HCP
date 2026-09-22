pub mod core;
pub use core::encoder;
pub use core::file_operations;
pub use core::graph;
pub use core::graph::Graph;
pub use core::encoder::Encoder;
pub use core::tour_verifier;
pub use core::tour_verifier::TourVerifier;

pub mod fallback;
pub use fallback::contraction;
pub use fallback::contraction::Degree2Contractor;
pub use fallback::cycle_merge;
pub use fallback::cycle_merge::safe_2opt_merge;
pub use fallback::fallback_cegar;

pub mod macro_decomp;
pub use macro_decomp::bipartite as macro_bipartite;
pub use macro_decomp::corridor as macro_corridor;
pub use macro_decomp::portfolio_788 as macro_788;

pub mod pipeline;
pub use pipeline::options;
pub use pipeline::options::Options;
pub use pipeline::solver_pipeline;
pub use pipeline::solver_pipeline::{
    find_graph_file, run_batch, run_batch_range, save_checkpoint_atomic, solve_single_graph,
    verify_and_export, BatchItemResult, SolverPipelineError,
};

