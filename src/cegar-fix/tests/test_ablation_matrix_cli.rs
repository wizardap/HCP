use cegar_fix::hybrid_orchestrator::HybridOptions;

#[test]
fn test_ablation_flags_mapping() {
    for mode in 0..=3 {
        let mut opts = HybridOptions::default();
        opts.apply_ablation_mode(mode);
        match mode {
            0 => {
                assert!(!opts.macro_gadget, "C0 must have macro_gadget disabled");
                assert!(!opts.bounded_freezer, "C0 must have bounded_freezer disabled");
            }
            1 => {
                assert!(!opts.macro_gadget, "C1 must have macro_gadget disabled");
                assert!(opts.bounded_freezer, "C1 must have bounded_freezer enabled");
            }
            2 => {
                assert!(opts.macro_gadget, "C2 must have macro_gadget enabled");
                assert!(!opts.bounded_freezer, "C2 must have bounded_freezer disabled");
            }
            3 => {
                assert!(opts.macro_gadget, "C3 must have macro_gadget enabled");
                assert!(opts.bounded_freezer, "C3 must have bounded_freezer enabled");
            }
            _ => unreachable!(),
        }
    }
}

#[test]
fn test_ablation_default_is_c0_equivalent() {
    let opts = HybridOptions::default();
    assert!(!opts.macro_gadget);
    assert!(!opts.bounded_freezer);
}
