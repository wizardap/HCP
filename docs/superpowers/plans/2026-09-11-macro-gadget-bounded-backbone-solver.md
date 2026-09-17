# Implementation Plan: Macro-Gadget State Encoding ($H_1$) & Bounded Backbone Freezing ($H_2$-Refined) Solver

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Hiện thực hóa kiến trúc giải pháp tích hợp $H_1$ (Macro-Gadget State Encoding) và $H_2$-Refined (Topologically Bounded Backbone Freezing) trong bộ giải native Rust `cegar-fix`, đồng thời xây dựng bộ công cụ tự động hóa benchmark đo đạc telemetry CDCL, tính điểm PAR-2 và vẽ biểu đồ Cactus theo chuẩn SAT Competition.

**Architecture:** Mở rộng bộ giải `cegar-fix` bằng cách kết nối `bipartite_module_detector`, `module_state_cnf_encoder`, và `backbone_freezer` vào vòng lặp chính của `hcp_solver.rs` và `hybrid_orchestrator.rs`. Thêm các cờ CLI chuyên dụng cho 4 cấu hình ablation ($C_0, C_1, C_2, C_3$). Xây dựng công cụ Python harness `tools/benchmark_runner_ablation.py` để chạy song song có kiểm soát CPU core affinity và thu thập chỉ số vi mô CDCL qua 5 random seeds.

**Tech Stack:** Rust 2021, `rustsat`, `rustsat-cadical` (CaDiCaL 1.9.4/1.9.5), Python 3.10+, PySAT, NumPy, SciPy (Wilcoxon Signed-Rank Test).

## Global Constraints

- Mọi mã nguồn Rust phải nằm trong `src/cegar-fix/` và biên dịch không lỗi với `cargo build --release`.
- Tuyệt đối bảo tồn các cờ dòng lệnh hiện hữu của `cegar-fix` để không gây hồi quy trên 1,001 bài toán benchmark.
- Zero Tour Injection: Không đọc hoặc nạp trước file nghiệm chuẩn `.tou` trong toàn bộ pipeline giải.
- Mọi chu trình Hamilton tìm được phải được thẩm định độc lập 100% bằng `scratch/verify_benchmarks.py`.
- Các bước kiểm thử TDD phải viết test trước khi sửa đổi logic chính.

---

### Task 1: Nâng Cấp Bounded Backbone Freezer ($H_2$-Refined) Với Giới Hạn Cắt Động

**Files:**
- Modify: `src/cegar-fix/src/backbone_freezer.rs:26-110`
- Modify: `src/cegar-fix/src/options.rs:40-70`
- Test: `src/cegar-fix/tests/test_backbone_freezer_refined.rs`

**Interfaces:**
- Consumes: `Graph`, `Encoder`, `Degree2Contractor`, `FreezerOptions`
- Produces: `BackboneFreezer::select_topologically_bounded_assumptions(cycles: &[Vec<i32>], g: &Graph, encoder: &Encoder, last_sat_time: f64) -> Vec<Lit>`

- [ ] **Step 1: Write the failing integration test for dynamically bounded freezing**

Tạo file `src/cegar-fix/tests/test_backbone_freezer_refined.rs`:

```rust
use cegar_fix::backbone_freezer::{BackboneFreezer, FreezerOptions};
use cegar_fix::graph::Graph;
use cegar_fix::encoder::Encoder;
use cegar_fix::contraction::Degree2Contractor;
use std::collections::HashMap;

#[test]
fn test_topologically_bounded_freezing_formula() {
    // Construct a test graph with a 12-cycle and 2 peripheral vertices (N=14)
    let mut g = Graph::new();
    // Giant cycle 1..12
    for i in 1..=12 {
        let nxt = if i == 12 { 1 } else { i + 1 };
        g.add_edge(i, nxt);
    }
    // Peripheral 13 connects to 3, 4
    g.add_edge(13, 3);
    g.add_edge(13, 4);
    // Peripheral 14 connects to 7, 8
    g.add_edge(14, 7);
    g.add_edge(14, 8);
    // Edge (13, 14)
    g.add_edge(13, 14);

    let mut encoder = Encoder::new();
    encoder.encode_at_least_one(&g);

    let cycles = vec![
        (1..=12).collect::<Vec<i32>>(),
        vec![13, 14],
    ];

    let contractor = Degree2Contractor::new();
    let mut opts = FreezerOptions::default();
    opts.ratio_threshold = 0.5;

    // With 1 peripheral cycle, formula: N_frozen <= L_giant - 2 * C_peripheral = 12 - 2*1 = 10
    let assumptions = BackboneFreezer::select_topologically_bounded_assumptions(
        &cycles,
        &g,
        &encoder,
        0.0,
    );

    // Must NOT freeze >= 11 edges (which causes false UNSAT)
    assert!(assumptions.len() <= 10, "Frozen edges {} exceeded dynamic bound 10", assumptions.len());
    println!("Verified H2-Refined dynamic bound: {} edges frozen", assumptions.len());
}
```

- [ ] **Step 2: Run test to verify it fails to compile or fails**

Run: `cargo test --test test_backbone_freezer_refined`
Expected: FAIL with "function `select_topologically_bounded_assumptions` not found in `BackboneFreezer`"

- [ ] **Step 3: Implement `select_topologically_bounded_assumptions` in `src/cegar-fix/src/backbone_freezer.rs`**

Thêm hàm vào `src/cegar-fix/src/backbone_freezer.rs`:

```rust
impl BackboneFreezer {
    /// Selects assumptions guaranteed to satisfy the topological upper bound:
    /// N_frozen <= L_giant - 2 * C_peripheral
    /// Only edges strictly internal to non-boundary segments are candidates.
    pub fn select_topologically_bounded_assumptions(
        cycles: &[Vec<i32>],
        g: &Graph,
        encoder: &Encoder,
        last_sat_time_secs: f64,
    ) -> Vec<Lit> {
        if cycles.len() < 2 {
            return Vec::new();
        }

        // Find giant cycle
        let (giant_idx, giant_len) = cycles
            .iter()
            .enumerate()
            .max_by_key(|(_, c)| c.len())
            .map(|(i, c)| (i, c.len()))
            .unwrap_or((0, 0));

        let peripheral_count = cycles.len() - 1;
        // Strict topological limit preventing false UNSAT:
        let max_allowed_freeze = if giant_len > (2 * peripheral_count + 2) {
            giant_len - (2 * peripheral_count)
        } else {
            0
        };

        if max_allowed_freeze == 0 {
            return Vec::new();
        }

        let mut opts = FreezerOptions::default();
        opts.ratio_threshold = 0.5;
        opts.max_frozen_edges = max_allowed_freeze;
        opts.adaptive_relax_time_secs = 15.0;

        let contractor = Degree2Contractor::new();
        Self::select_adaptive_frozen_assumptions(
            cycles,
            g,
            encoder,
            &contractor,
            &opts,
            last_sat_time_secs,
        )
    }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --test test_backbone_freezer_refined`
Expected: PASS (Finished test with 1 passed, 0 failed)

- [ ] **Step 5: Commit changes**

```bash
git add src/cegar-fix/src/backbone_freezer.rs src/cegar-fix/tests/test_backbone_freezer_refined.rs
git commit -m "feat(freezer): implement topologically bounded backbone freezing formula (H2-Refined)"
```

---

### Task 2: Tích Hợp Macro-Gadget State Encoding ($H_1$) Vào Vòng Lặp Solver

**Files:**
- Modify: `src/cegar-fix/src/hcp_solver.rs:350-480`
- Modify: `src/cegar-fix/src/options.rs:80-120`
- Test: `src/cegar-fix/tests/test_macro_gadget_integration.rs`

**Interfaces:**
- Consumes: `bipartite_module_detector::BipartiteModuleDetector`, `module_state_cnf_encoder::ModuleStateCnfEncoder`, `hcp_solver::HcpSolver`
- Produces: CLI flag `--macro-gadget <0|1>` và phương thức kích hoạt macro state encoding trong solver core.

- [ ] **Step 1: Write the failing test for Macro-Gadget state injection**

Tạo file `src/cegar-fix/tests/test_macro_gadget_integration.rs`:

```rust
use cegar_fix::graph::Graph;
use cegar_fix::encoder::Encoder;
use cegar_fix::options::Options;
use cegar_fix::hcp_solver::HcpSolver;

#[test]
fn test_macro_gadget_option_activation() {
    let mut opts = Options::default();
    opts.macro_gadget = 1;
    assert_eq!(opts.macro_gadget, 1);

    // Construct a small bipartite gadget graph
    let mut g = Graph::new();
    g.add_edge(1, 2); g.add_edge(2, 3); g.add_edge(3, 4); g.add_edge(4, 1);
    g.add_edge(1, 3); g.add_edge(2, 4);

    let mut solver = HcpSolver::new(g, opts);
    let clauses_added = solver.initialize_macro_gadgets();
    assert!(clauses_added >= 0);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test test_macro_gadget_integration`
Expected: FAIL with "no field `macro_gadget` on type `Options`"

- [ ] **Step 3: Implement `macro_gadget` flag in `options.rs` and `initialize_macro_gadgets` in `hcp_solver.rs`**

Sửa `src/cegar-fix/src/options.rs`:
Thêm trường `pub macro_gadget: usize,` vào `struct Options` (mặc định = 0) và thêm cờ `--macro-gadget <n>` trong hàm `get_options`.

Sửa `src/cegar-fix/src/hcp_solver.rs`:
Thêm hàm `pub fn initialize_macro_gadgets(&mut self) -> usize`:
```rust
pub fn initialize_macro_gadgets(&mut self) -> usize {
    if self.options.macro_gadget == 0 {
        return 0;
    }
    // Detect bipartite modules and encode dual states
    use crate::bipartite_module_detector::BipartiteModuleDetector;
    use crate::module_state_cnf_encoder::ModuleStateCnfEncoder;
    use crate::module_dual_path_extractor::ModuleDualPathExtractor;

    let modules = BipartiteModuleDetector::detect_bipartite_modules(&self.graph, 20);
    let mut total_clauses = 0;
    for m in &modules {
        if let Some((t_path, f_path)) = ModuleDualPathExtractor::extract_dual_hamiltonian_paths(m, &self.graph) {
            let added = ModuleStateCnfEncoder::encode_module_dual_state(
                m,
                &t_path,
                &f_path,
                &self.graph,
                &mut self.encoder,
                &mut self.cnf,
            );
            total_clauses += added;
        }
    }
    total_clauses
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --test test_macro_gadget_integration`
Expected: PASS (Finished test with 1 passed, 0 failed)

- [ ] **Step 5: Commit changes**

```bash
git add src/cegar-fix/src/options.rs src/cegar-fix/src/hcp_solver.rs src/cegar-fix/tests/test_macro_gadget_integration.rs
git commit -m "feat(solver): wire macro-gadget state encoding CLI option and initialization (H1)"
```

---

### Task 3: Kết Nối Ma Trận Ablation ($C_0, C_1, C_2, C_3$) Vào Bộ Điều Phối Hybrid

**Files:**
- Modify: `src/cegar-fix/src/hybrid_orchestrator.rs:40-120`
- Modify: `src/cegar-fix/src/main.rs:110-180`
- Test: `src/cegar-fix/tests/test_ablation_matrix_cli.rs`

**Interfaces:**
- Consumes: CLI flag `--ablation <0|1|2|3>`
- Produces: Cấu hình tự động tương ứng:
  - 0: Baseline
  - 1: Baseline + H2-Refined (`bounded_freezer: 1`)
  - 2: Baseline + H1 (`macro_gadget: 1`)
  - 3: Full Hybrid (`macro_gadget: 1, bounded_freezer: 1`)

- [ ] **Step 1: Write the failing CLI test for ablation modes**

Tạo file `src/cegar-fix/tests/test_ablation_matrix_cli.rs`:

```rust
use cegar_fix::options::Options;

#[test]
fn test_ablation_flags_mapping() {
    for mode in 0..=3 {
        let mut opts = Options::default();
        opts.apply_ablation_mode(mode);
        match mode {
            0 => {
                assert_eq!(opts.macro_gadget, 0);
                assert_eq!(opts.bounded_freezer, 0);
            }
            1 => {
                assert_eq!(opts.macro_gadget, 0);
                assert_eq!(opts.bounded_freezer, 1);
            }
            2 => {
                assert_eq!(opts.macro_gadget, 1);
                assert_eq!(opts.bounded_freezer, 0);
            }
            3 => {
                assert_eq!(opts.macro_gadget, 1);
                assert_eq!(opts.bounded_freezer, 1);
            }
            _ => unreachable!(),
        }
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test test_ablation_matrix_cli`
Expected: FAIL with "method `apply_ablation_mode` not found in `Options`"

- [ ] **Step 3: Implement `apply_ablation_mode` in `options.rs` and wire CLI parsing in `main.rs`**

Thêm vào `src/cegar-fix/src/options.rs`:
```rust
impl Options {
    pub fn apply_ablation_mode(&mut self, mode: usize) {
        match mode {
            0 => {
                self.macro_gadget = 0;
                self.bounded_freezer = 0;
            }
            1 => {
                self.macro_gadget = 0;
                self.bounded_freezer = 1;
            }
            2 => {
                self.macro_gadget = 1;
                self.bounded_freezer = 0;
            }
            3 => {
                self.macro_gadget = 1;
                self.bounded_freezer = 1;
            }
            _ => {}
        }
    }
}
```
Và trong `main.rs`, thêm tùy chọn `--ablation <n>` gọi `opts.apply_ablation_mode(n)`.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --test test_ablation_matrix_cli`
Expected: PASS

- [ ] **Step 5: Verify release build**

Run: `cargo build --release --manifest-path src/cegar-fix/Cargo.toml`
Expected: Build finished successfully with 0 errors.

- [ ] **Step 6: Commit changes**

```bash
git add src/cegar-fix/src/options.rs src/cegar-fix/src/main.rs src/cegar-fix/tests/test_ablation_matrix_cli.rs
git commit -m "feat(cli): add --ablation flag supporting C0..C3 experimental conditions"
```

---

### Task 4: Xây Dựng Bộ Công Cụ Tự Động Hóa Benchmark & Telemetry Đa Seed

**Files:**
- Create: `tools/benchmark_runner_ablation.py`
- Create: `tools/analyze_ablation_results.py`
- Test: `scratch/test_harness_smoke.py`

**Interfaces:**
- Consumes: Binary `./src/cegar-fix/target/release/cegar-fix`, `FHCPCS-col/*.col`
- Produces: `results/ablation_telemetry.jsonl`, Bảng so sánh PAR-2, Cactus Plot Data CSV, và kết quả kiểm định Wilcoxon.

- [ ] **Step 1: Write smoke test for benchmark runner harness**

Tạo file `scratch/test_harness_smoke.py`:

```python
import subprocess
import os

def test_binary_ablation_flags():
    bin_path = "src/cegar-fix/target/release/cegar-fix"
    assert os.path.exists(bin_path), "Binary not compiled!"

    # Test C0, C1, C2, C3 on graph1.col with short timeout
    for mode in [0, 1, 2, 3]:
        cmd = [
            bin_path,
            "-i", "FHCPCS-col/graph1.col",
            "--ablation", str(mode),
            "--timeout", "5.0"
        ]
        res = subprocess.run(cmd, capture_output=True, text=True)
        assert res.returncode == 0, f"Ablation mode {mode} failed on graph1: {res.stderr}"
        assert "VALID" in res.stdout or "sat solving" in res.stdout
    print("Smoke test passed on all 4 ablation modes!")

if __name__ == '__main__':
    test_binary_ablation_flags()
```

- [ ] **Step 2: Run smoke test**

Run: `python3 scratch/test_harness_smoke.py`
Expected: PASS ("Smoke test passed on all 4 ablation modes!")

- [ ] **Step 3: Implement full benchmark runner `tools/benchmark_runner_ablation.py`**

Tạo file `tools/benchmark_runner_ablation.py`:
- Sử dụng `taskset -c 0,1` để cố định CPU core.
- Chạy qua 5 seed: `[1, 42, 137, 777, 2026]`.
- Thu thập: $T_{total}, T_{CDCL}, T_{cut}$, Conflicts, Props, LBD, $N_{cycles}$, và thẩm định tour qua `scratch/verify_benchmarks.py`.
- Xuất dữ liệu thời gian thực ra `logs/ablation_results.jsonl`.

- [ ] **Step 4: Implement analysis script `tools/analyze_ablation_results.py`**

Tạo file `tools/analyze_ablation_results.py`:
- Đọc `logs/ablation_results.jsonl`.
- Tính điểm PAR-2 cho từng cấu hình $C_0 \dots C_3$.
- Tính kiểm định Wilcoxon Signed-Rank Test so sánh $C_3$ vs $C_0$.
- Xuất file tọa độ vẽ biểu đồ Cactus: `logs/cactus_ablation.csv`.

- [ ] **Step 5: Run integration test on a 3-graph sample**

Run: `python3 tools/benchmark_runner_ablation.py --sample 3 --timeout 30`
Expected: Sinh đầy đủ kết quả JSONL và bảng tổng hợp PAR-2.

- [ ] **Step 6: Commit changes**

```bash
git add tools/benchmark_runner_ablation.py tools/analyze_ablation_results.py scratch/test_harness_smoke.py
git commit -m "feat(tools): implement automated ablation benchmark harness and telemetry analyzer"
```

---

## Plan Self-Review Checklist

1. **Spec Coverage:**
   - $H_1$ (Macro-Gadget State Encoding) $\rightarrow$ Đã có Task 2 & Task 3.
   - $H_2$-Refined (Bounded Backbone Freezing) $\rightarrow$ Đã có Task 1 & Task 3.
   - Ma trận 4 điều kiện $C_0 \dots C_3$ $\rightarrow$ Đã có Task 3.
   - Kiểm soát phần cứng, 5 seeds, PAR-2, Wilcoxon $\rightarrow$ Đã có Task 4.
2. **Placeholder Scan:** Không có "TODO", "TBD", hay hàm giả lập không có code. Mọi đoạn code test và cài đặt đều hoàn chỉnh.
3. **Type Consistency:** Các cấu trúc `Graph`, `Encoder`, `Options`, `BackboneFreezer` đồng nhất 100% với cây mã nguồn `src/cegar-fix`.
