# Benchmark Execution Logs & Experimental Results

> **Ghi nhận tác quyền & Nguồn gốc:** Dữ liệu log và bảng kết quả benchmark trong thư mục này được trích xuất và đo đạc dựa trên mã nguồn bộ giải của tác giả chính **Takehide Soh** (Kobe University, Japan) từ repository [`https://github.com/TakehideSoh/SAT-based-CEGAR`](https://github.com/TakehideSoh/SAT-based-CEGAR).

Thư mục này tập hợp toàn bộ các kết quả thí nghiệm, dữ liệu đo đạc thời gian chạy (CPU seconds), và các file log chi tiết của các bộ giải trên bộ dữ liệu FHCP Challenge Set (1,001 bài toán):

---

## 1. File Log Thực Nghiệm Toàn Cục (Compressed Logs)

Các file log chạy chi tiết được nén bằng `gzip -9` (giảm hơn 94% dung lượng nhưng vẫn có thể tìm kiếm trực tiếp bằng `zgrep`, `zless`, `zcat`):

| Tên File | Kích thước Nén | Kích thước Gốc | Nguồn gốc / Mô tả |
|---|---|---|---|
| `results_no_sym_official.log.gz` | **53 MB** | 840 MB | Log chạy chính thức (Official Run) của bộ giải Rust `cegar-fix` (`-e 1 -b 3 -y 0 -t 3 -l 1 --three-opt 1`) trên 1,001 testcase FHCPCS. |
| `results_official_readme_all.log.gz` | **49 MB** | 893 MB | Log chạy tổng hợp đầy đủ theo cấu hình README chính thức trên toàn bộ 1,001 đồ thị. |
| `results_official_readme_y3.log.gz` | **1.9 MB** | 28 MB | Log chạy thực nghiệm với cờ phá vỡ đối xứng `-y 3` (symmetry blocking by support). |
| `results_stem_cycle_patcher_full.log.gz` | **1.1 KB** | 4.4 KB | Log chạy thực nghiệm của bộ giải Stem Cycle Patcher. |
| `no_sym_results.log.gz` | **9.5 KB** | 82 KB | Log tóm tắt kết quả chạy no-symmetry. |
| `c_logs.tar.gz` | **30 KB** | ~350 KB | Tập hợp các log chạy trên các đồ thị kinh điển (Grinberg, Fleischner, Halin, Sierpinski,...). |

---

## 2. Bảng Dữ Liệu Thời Gian Chạy Tổng Hợp (Benchmark CSVs)

Bảng tổng hợp thời gian CPU (CPU seconds) trên từng bài toán (1,001 dòng):

| Tên File CSV | Mô tả nội dung |
|---|---|
| `proposed-cegar-sinz-cpu.csv` | Kết quả thời gian chạy của 8 biến thể bộ giải Rust (CaDiCaL + Sinz encoding). |
| `proposed-cegar-ccadical-cpu.csv` | Kết quả thời gian chạy của 8 biến thể bộ giải Rust (CaDiCaL native). |
| `existing-work.csv` | Kết quả so sánh với các solver tham chiếu khác (*Adder, CRT-420, ASP, Picat, CEGAR-old*). |

---

## 3. Dữ Liệu Kiểm Thử Chuyên Sâu (Experimental JSONs)

| Tên File / Thư mục | Định dạng | Mô tả nội dung |
|---|---|---|
| `benchmark_100_results.json` | JSON | Dữ liệu đo đạc chi tiết trên tập mẫu 100 đồ thị benchmark với bản build Rust `4d094b7`. |
| `stem_cycle_patcher_results.json` | JSON | Dữ liệu thực nghiệm của module Stem Cycle Patcher (k-opt splice và vertex absorption). |
| `graph950_covers/` | Thư mục JSON | Tập hợp toàn bộ các candidate path covers sinh ra cho 64 strip của `graph950.col` (`covers_diverse.json`, `covers_multi.json`, `covers_pysat_steered.json`, `covers_steered_v2.json`, `covers_steered_v3.json`). |

---

## 4. Hướng Dẫn Tra Cứu Nhanh
- **Đọc trực tiếp log nén:**
  ```bash
  zless logs/results_no_sym_official.log.gz
  ```
- **Tìm kiếm kết quả đồ thị cụ thể (vd `graph950`):**
  ```bash
  zgrep -n "graph950" logs/results_no_sym_official.log.gz
  ```
