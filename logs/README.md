# Benchmark Execution Logs (Compressed)

Toàn bộ các file log từ các phiên chạy benchmark trên bộ đồ thị FHCP Challenge Set đã được nén bằng `gzip -9` (giảm hơn 94% dung lượng từ 1.8GB xuống còn ~104MB).

Bạn có thể đọc hoặc tìm kiếm trực tiếp trong các file này mà không cần giải nén bằng `zgrep`, `zless`, hoặc `zcat`.

| Tên File Nén | Kích thước Nén | Kích thước Gốc | Nguồn gốc | Mô tả nội dung |
|---|---|---|---|---|
| `results_no_sym_official.log.gz` | **53 MB** | 840 MB | `SAT-based-CEGAR` | Toàn bộ log chạy chính thức (Official Run) của bộ giải Rust `cegar-fix` không dùng symmetry breaking trên 1,001 testcase FHCPCS. |
| `results_official_readme_all.log.gz` | **49 MB** | 893 MB | `SAT-based-CEGAR` | Log chạy tổng hợp đầy đủ theo cấu hình README chính thức trên toàn bộ 1,001 đồ thị. |
| `results_official_readme_y3.log.gz` | **1.9 MB** | 28 MB | `HCP` | Log chạy với cờ `-y 3` (symmetry blocking by support) trên tập đồ thị FHCPCS. |
| `no_sym_results.log.gz` | **9.5 KB** | 82 KB | `SAT-based-CEGAR` | Log tóm tắt kết quả chạy no-symmetry. |
| `results_stem_cycle_patcher_full.log.gz` | **1.1 KB** | 4.4 KB | `HCP` | Log thực nghiệm của bộ giải Stem Cycle Patcher (k-opt splice / unvisited-vertex absorption). |

### Hướng dẫn tra cứu nhanh
- Xem nội dung bằng `zless`:
  ```bash
  zless logs/results_no_sym_official.log.gz
  ```
- Tìm kiếm kết quả đồ thị cụ thể (ví dụ `graph950`):
  ```bash
  zgrep -n "graph950" logs/results_no_sym_official.log.gz
  ```
