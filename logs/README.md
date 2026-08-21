# Benchmark Execution Logs

Thư mục này lưu trữ toàn bộ các file log từ các phiên chạy benchmark trên bộ đồ thị FHCP Challenge Set:

| Tên File | Kích thước | Nguồn gốc / Phiên chạy | Mô tả nội dung |
|---|---|---|---|
| `results_no_sym_official.log` | ~840 MB | `/home/ubuntu/SAT-based-CEGAR` | Toàn bộ log chạy chính thức (Official Run) của bộ giải Rust `cegar-fix` không dùng symmetry breaking trên 1,001 testcase FHCPCS. |
| `results_official_readme_all.log` | ~893 MB | `/home/ubuntu/SAT-based-CEGAR` | Log chạy tổng hợp đầy đủ theo cấu hình README chính thức trên toàn bộ 1,001 đồ thị. |
| `results_official_readme_y3.log` | ~28 MB | `/home/ubuntu/HCP` | Log chạy với cờ `-y 3` (symmetry blocking by support) trên tập đồ thị FHCPCS. |
| `no_sym_results.log` | ~82 KB | `/home/ubuntu/SAT-based-CEGAR` | Log tóm tắt kết quả chạy no-symmetry. |
| `results_stem_cycle_patcher_full.log` | ~4.4 KB | `/home/ubuntu/HCP` | Log thực nghiệm của bộ giải Stem Cycle Patcher (k-opt splice / unvisited-vertex absorption). |

*Lưu ý: Các file `*.log` dung lượng lớn được loại trừ trong `.gitignore` để không làm phình dung lượng git repository.*
