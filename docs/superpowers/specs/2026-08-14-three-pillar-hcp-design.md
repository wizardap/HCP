# Thiết Kế Chi Tiết: Tối Ưu Hóa Thuật Toán Giải Hamiltonian Cycle Bằng SAT-based CEGAR (Kiến Trúc 3 Trụ Cột)

- **Ngày tạo**: 2026-08-14
- **Dự án**: SAT-based CEGAR HCP Solver
- **Vị trí tệp thiết kế**: `docs/superpowers/specs/2026-08-14-three-pillar-hcp-design.md`

---

## 1. Mục Tiêu Thiết Kế

1. **Bảo tồn toàn bộ tỷ lệ giải thành công**: Không gây ra bất kỳ hiện tượng thoái lui (regression) nào trên 926+ đồ thị mà bản gốc giải được.
2. **Tăng tốc độ giải trung bình**: Giảm thời gian tìm chu trình Hamilton trên các đồ thị quy mô lớn và đồ thị có mật độ cạnh phức tạp.
3. **Mở rộng khả năng giải đồ thị khó**: Giải quyết thêm các đồ thị bị bế tắc ở 2-opt truyền thống thông qua việc kết hợp tiền xử lý cấu trúc và 3-opt heuristic sạch.

---

## 2. Kiến Trúc 3 Trụ Cột (Three-Pillar Architecture)

```
[Đồ thị đầu vào G]
        │
        ▼
[Trụ cột 1: Tiền Xử Lý Đồ Thị (Graph Preprocessing)]
  - Khử cạnh tam giác cô lập (Degree-2 Invariant Pruning)
  - Phát hiện đỉnh khớp (Tarjan Cut-Vertex / Articulation Point)
        │
        ▼
[Mã hóa SAT Ban Đầu (Sinz Encoding -e 1, -b 3, -y 0, -t 3, -l 1)]
        │
        ▼
   ┌────┴──────────────────────────────┐
   │                                   ▼
   │                         [CaDiCaL SAT Solver]
   │                                   │
   │                                   ▼
   │                        [Tập chu trình con {C_i}]
   │                                   │
   │                                   ▼
   │              [Trụ cột 2: Ghép Chu Trình Thuần Túy]
   │                - 2-opt Solution Constructor
   │                - 3-opt Candidate Solution Constructor
   │                                   │
   │              ┌────────────────────┴───────────────────┐
   │              ▼                                        ▼
   │       [Gộp thành 1 chu trình]                 [Còn >= 2 chu trình]
   │              │                                        │
   │              ▼                                        ▼
   │    [s SATISFIABLE (XONG)]            [Trụ cột 3: Strong Minimal Cuts]
   │                                        - |C| <= 4: Cut-set + Exclusion Cut
   │                                        - |C| > 4 : Minimal ASP Cut-set
   │                                                       │
   └───────────────────────────────────────────────────────┘
```

---

## 3. Chi Tiết Từng Trụ Cột

### Trụ Cột 1: Tiền Xử Lý Đồ Thị (Graph Preprocessing & Invariant Pruning)

Tệp ảnh hưởng: `src/graph.rs`

1. **Khử cạnh tam giác cô lập (Degree-2 Triangle Pruning)**:
   - **Định lý**: Cho đồ thị $G = (V, E)$. Nếu $v \in V$ có bậc $d(v) = 2$ với hai đỉnh kề $u, w$, thì hai cạnh $(u, v)$ và $(v, w)$ bắt buộc phải thuộc mọi chu trình Hamilton của $G$. Nếu tồn tại cạnh $(u, w) \in E$ và $|V| > 3$, thì việc chọn cạnh $(u, w)$ cùng với $(u, v)$ và $(v, w)$ sẽ tạo ra tam giác con độc lập $u-v-w-u$ và cô lập phần còn lại của $G$.
   - **Hành động**: Xóa vĩnh viễn cạnh $(u, w)$ khỏi danh sách kề và danh sách cung có hướng trước khi sinh biến Boolean.
   - **Lợi ích**: Giảm bớt số biến Boolean $x_{u,v}, x_{v,u}$, giảm kích thước CNF ban đầu và triệt tiêu các nhánh tìm kiếm vô nghiệm.

2. **Kiểm tra đỉnh khớp (Cut-Vertex / Articulation Point Detection)**:
   - **Định lý**: Chu trình Hamilton đi qua mỗi đỉnh đúng một lần. Nếu loại bỏ đỉnh $v$ làm đồ thị bị phân rã thành $\ge 2$ thành phần liên thông, đồ thị không thể chứa chu trình Hamilton.
   - **Hành động**: Chạy thuật toán Tarjan tìm khớp/cầu trong $O(V + E)$ ngay sau khi đọc đồ thị. Nếu phát hiện đỉnh khớp, xuất ngay `s UNSATISFIABLE` và kết thúc mà không cần gọi SAT solver.

---

### Trụ Cột 2: Ghép Chu Trình Thuần Túy (Non-Polluting 2-opt / 3-opt Constructor)

Tệp ảnh hưởng: `src/hcp_solver.rs`

- **Nguyên lý thiết kế**: Ghép chu trình đóng vai trò là một **Bộ sinh nghiệm (Solution Constructor)** đa thức, KHÔNG đóng vai trò sinh mệnh đề chặn nhân tạo (synthetic blocking clauses).
- **Quy trình thực thi**:
  1. SAT Solver trả về tập chu trình con $\{C_1, C_2, \dots, C_k\}$.
  2. Nếu $k == 1 \rightarrow$ Chu trình Hamilton hoàn chỉnh $\rightarrow$ Trả về nghiệm `s SATISFIABLE`.
  3. Nếu $k > 1$:
     - Thực hiện **2-opt merge** lặp: Tìm cặp $(C_i, C_j)$ có thể nối qua 2 cạnh chéo.
     - Nếu 2-opt đưa số chu trình về 1 $\rightarrow$ Trả về nghiệm `s SATISFIABLE`.
     - Nếu 2-opt dừng lại ở $m \ge 3$ chu trình: Thử tiếp **3-opt candidate merge** (chỉ trên các ứng viên có cạnh nối).
     - Nếu 3-opt đưa số chu trình về 1 $\rightarrow$ Trả về nghiệm `s SATISFIABLE`.
  4. Nếu cả 2-opt và 3-opt không thể gộp thành 1 chu trình duy nhất:
     - **Không sinh bất kỳ mệnh đề nào cho các chu trình ghép dở dang**.
     - Trả về danh sách chu trình con còn lại cho Trụ cột 3 để sinh Cut chuẩn.

---

### Trụ Cột 3: Sinh Mệnh Đề Chặn Chuẩn Hóa (Strong Minimal Subcycle Cuts)

Tệp ảnh hưởng: `src/hcp_solver.rs`, `src/encoder.rs`

- **Quy tắc phân loại chu trình con**:
  1. **Chu trình con rất nhỏ ($|C| \le 4$)**:
     - Sinh đồng thời:
       - **Cut-set clause** qua tập cung ra $\delta^+(C)$: $\bigvee_{(u, v) \in \delta^+(C)} x_{u, v}$ (buộc phải có ít nhất 1 cạnh thoát khỏi $C$).
       - **Vertex-exclusion clause**: $\bigvee_{(u, v) \in C} \neg x_{u, v}$ (loại bỏ trực tiếp tổ hợp cạnh tạo nên $C$).
     - Do $|C| \le 4$, clause này rất ngắn ($\le 4$ literals), giúp CaDiCaL kích hoạt Unit Propagation ngay lập tức.
  2. **Chu trình con thông thường ($|C| > 4$)**:
     - Chỉ sinh **Cut-set clause** qua $\delta^+(C)$ và $\delta^-(C)$ theo định dạng ASP Cut (`block_method = 3`).
     - **Tuyệt đối không** inject MTZ clauses hay CEGAR fallback clauses tràn lan làm ô nhiễm không gian VSIDS của SAT solver.

---

## 4. Kế Hoạch Kiểm Thử & Tiêu Chí Thành Công

1. **Kiểm tra hồi quy (Regression Test)**:
   - Chạy 24 đồ thị từng bị timeout (`graph161`, `graph178`, `graph248`, `graph313`...).
   - **Yêu cầu**: 100% các đồ thị này phải giải thành công mà không bị Timeout.
2. **Kiểm tra đồ thị khó (Hard Benchmark Test)**:
   - Kiểm tra `graph339.col` và các đồ thị có kích thước $> 1000$ đỉnh.
   - **Yêu cầu**: Duy trì khả năng giải đồ thị khó nhờ 3-opt Solution Constructor.
3. **Toàn bộ bộ dữ liệu FHCPCS-col (1001 đồ thị)**:
   - **Yêu cầu**: Tỷ lệ giải $\ge 926/1001$ đồ thị và tổng thời gian giải nhanh hơn bản gốc.
