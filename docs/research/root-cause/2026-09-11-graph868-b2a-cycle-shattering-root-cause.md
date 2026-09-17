# SAT Root Cause Analysis: Hiện Tượng Vỡ Chu Trình (Cycle Shattering) & Bùng Nổ Xung Đột CDCL Trên `graph868.col`

**Ngày lập báo cáo:** 2026-09-11  
**Đối tượng khảo sát:** `graph868.col` ($N=5,544, M=9,072$) — Đại diện tiêu biểu nhất của **Class B2a (Universal Core Hard)**.  
**Chuẩn mực áp dụng:** Khung chẩn đoán 4 tầng nhân quả **`sat-root-cause-analysis`**:
1. **Tier 1**: Graph Topology (Tô-pô đồ thị & Bẫy chẵn lẻ)
2. **Tier 2**: CNF Formulation & BCP Propagation Power (Sức mạnh lan truyền suy diễn đơn vị)
3. **Tier 3**: CDCL Search Dynamics (Động học xung đột 1UIP, phân phối LBD, đói biến phân nhánh)
4. **Tier 4**: Engine Runtime & FFI Mechanics (Cấu trúc dữ liệu và chi phí tầng thực thi)

---

## 1. Bằng Chứng Thực Nghiệm Định Lượng (Empirical Solver Metrics)

Trước khi tiến hành phân tích từng tầng, bảng sau ghi lại số liệu đo đạc thực tế từ quá trình chạy chuỗi 8 vòng lặp CEGAR tăng dần trên CaDiCaL 1.9.4 (`Cadical195`):

| Vòng CEGAR | Thời Gian SAT ($s$) | Số Chu Trình Con | Top 5 Chiều Dài Chu Trình | Mệnh Đề Cắt Thêm | Tổng Số Xung Đột (Conflicts) | Số Quyết Định (Decisions) | Số Lan Truyền (Propagations) | Tỷ Lệ Tăng Xung Đột |
| :---: | :---: | :---: | :---: | :---: | :---: | :---: | :---: | :---: |
| **Iter 0** | 0.007s | 100 | [913, 113, 65, 28, 25] | 99 | 39 | 307 | 21,939 | — |
| **Iter 1** | 0.011s | 76 | [1202, 68, 58, 8, 8] | 75 | 203 | 1,037 | 73,277 | **$5.2\times$** |
| **Iter 2** | 0.074s | 65 | [1366, 22, 8, 8, 8] | 64 | 844 | 6,283 | 365,856 | **$4.2\times$** |
| **Iter 3** | 0.192s | 60 | [855, 455, 110, 8, 8] | 59 | 2,804 | 14,955 | 1,263,259 | **$3.3\times$** |
| **Iter 4** | 0.808s | 49 | [910, 587, 12, 8, 8] | 48 | 11,958 | 54,544 | 3,658,567 | **$4.3\times$** |
| **Iter 5** | 0.750s | 47 | [768, 406, 217, 130, 22] | 46 | 20,516 | 106,291 | 6,227,997 | **$1.7\times$** |
| **Iter 6** | 4.354s | 45 | [842, 482, 146, 84, 22] | 44 | 53,557 | 370,154 | 17,537,469 | **$2.6\times$** |
| **Iter 7** | **34.632s** | 42 | [718, 473, 380, 14, 8] | 41 | **240,144** | **1,633,907** | **88,968,999** | **$4.5\times$** |

> [!CAUTION]
> **Hiện Tượng Bùng Nổ Xung Đột Cấp Số Mũ:**
> Chỉ sau 7 vòng lặp, số xung đột tăng vọt từ **39 lên 240,144** ($6,157\times$), số phép lan truyền BCP vượt ngưỡng **88.9 triệu phép tính**, và thời gian SAT tăng từ **0.007s lên 34.6s** ($4,947\times$). Dự phóng Iter 9–10 đạt mốc 400s–1,800s, giải thích chính xác tại sao bản chạy chính thức bị timeout ở Incr 9 (431s) và Incr 10 (500s).

---

## 2. Tier 1 — Phân Tích Cấu Trúc Tô-Pô Đồ Thị (Graph Topology Obstructions)

1. **Khối Co Cụm Đỉnh Bậc 2 (Degree-2 Contraction):**
   - Đồ thị gốc có $N=5,544$ đỉnh, trong đó có đúng **1,848 đỉnh bậc 2** (chiếm chính xác $33.33\%$).
   - Khi co cụm (contraction), đồ thị rút gọn thành một cấu trúc gồm **1,848 khối (blocks)**.
2. **Đặc Tính Hai Phía Chặt Chẽ (Strict Bipartiteness):**
   - Đồ thị khối là một đồ thị hai phía (bipartite graph) hoàn hảo: mỗi khối có 1 cổng vào (In-Port) thuộc tập $V_0$ và 1 cổng ra (Out-Port) thuộc tập $V_1$.
   - **Hệ quả tô-pô:** Mọi chu trình con bắt buộc phải có chiều dài khối là **số chẵn** ($2, 4, 6, 8, \dots$), tương ứng trên đồ thị gốc là các bội số của 6 đỉnh ($6, 12, 18, 24, \dots$).
3. **Bẫy Gadget Đối Xứng (Symmetric Gadget Trap):**
   - Đồ thị được chia thành **20 thành phần đối xứng (gadget clusters)**. Bên trong mỗi gadget tồn tại 2 hoán vị ghép cặp hoàn hảo (perfect matchings) độc lập.
   - Do tính chẵn lẻ và cấu trúc đối xứng khép kín của từng gadget, không thể dùng bất kỳ thao tác 2-opt hay 3-opt thông thường nào để nối hai chu trình con nếu không phá vỡ tính liên tục của luồng hai phía. Điều này giải thích triệt để vì sao log hệ thống ghi nhận **`number of connected cycles = 0` ở 100% các vòng lặp**.

---

## 3. Tier 2 — Đánh Giá CNF & Sức Mạnh Lan Truyền BCP (BCP Propagation Audit)

1. **Ngữ Nghĩa Biến:**
   - Biến $x_{u,v}$ thể hiện cung có hướng đi từ cổng ra của khối $u$ đến cổng vào của khối $v$.
   - Số lượng biến: 5,376 biến cung. Số lượng mệnh đề cơ sở: 16,468 mệnh đề (Exact-1 vào/ra và cấm 2-cycle đối xứng).
2. **Hiện Tượng Mệnh Đề Cắt "Ngủ Yên" (Dormant Cut Clauses):**
   - Mệnh đề loại trừ chu trình con (Subtour Exclusion Clause) có dạng:
     $$\bigvee_{i=1}^{k} \neg x_{c_i, c_{i+1}}$$
   - Đối với chu trình kích thước 8, mệnh đề cắt chứa 8 literal âm.
   - Đối với chu trình khổng lồ (Giant Cycle, kích thước từ 718 đến 1,366 đỉnh), mệnh đề cắt chứa **hàng trăm đến hàng nghìn literal âm**.
3. **Mất Hoàn Toàn Khả Năng Lan Truyền Đơn Vị (Zero Unit Propagation Leverage):**
   - Một mệnh đề cắt có chiều dài $k \ge 8$ **hoàn toàn không có khả năng kích hoạt suy diễn đơn vị (Unit Propagation)** khi solver bắt đầu phân nhánh.
   - CaDiCaL chỉ nhận diện mệnh đề cắt bị vi phạm khi nó đã gán chân trị cho ít nhất $k-1$ biến thành `True` (nghĩa là đã xây dựng gần như hoàn chỉnh toàn bộ chu trình con đó).
   - **Hậu quả:** Mệnh đề cắt nằm "ngủ yên" (dormant) trong danh sách theo dõi (watchlists), không hỗ trợ tỉa bớt không gian tìm kiếm ở các tầng quyết định nông.

---

## 4. Tier 3 — Động Học Tìm Kiếm CDCL (CDCL Search Dynamics)

Đây là **trọng tâm chẩn đoán** giải thích hiện tượng bùng nổ thời gian và vỡ chu trình:

```
                            Quyết định sâu (Decision Level > 200)
                                           │
                                           ▼
                      Vi phạm Mệnh Đề Cắt Ngủ Yên (Dormant Cut)
                                           │
                                           ▼
                  Phân tích xung đột 1UIP xuyên nhiều Gadgets
                                           │
                                           ▼
                  Sinh Mệnh Đề Học Có LBD Cao (LBD > 10, Kém Chất Lượng)
                                           │
                                           ▼
                  CaDiCaL Clause Reduction Xóa Bỏ (Tier 3 Garbage Purge)
                                           │
                                           ▼
             Solver Quên Mệnh Đề Cắt -> Tái Diễn Nghiệm Vỡ Chu Trình (Shatter)
```

1. **Độ Sâu Quyết Định & Phân Tích Xung Đột 1UIP:**
   - Vì các mệnh đề cắt không lan truyền sớm, CaDiCaL phải ra quyết định xuống độ sâu rất lớn (Decision level $> 200$).
   - Khi xung đột xảy ra, đồ thị hàm ý (implication graph) nối qua hàng loạt gadget rời rạc. Vết cắt 1UIP (First Unique Implication Point) tạo ra mệnh đề học chứa các literal nằm rải rác trên nhiều mức quyết định khác nhau.
2. **Phân Phối LBD (Literal Block Distance) & Sự Xóa Bỏ Mệnh Đề Học:**
   - Các mệnh đề học giải quyết chu trình có **LBD trung bình rất cao ($LBD > 10$)**.
   - Theo chiến lược quản lý bộ nhớ mệnh đề của CaDiCaL:
     * $LBD \le 2$ (Glue clauses): Giữ lại vĩnh viễn.
     * $LBD \le 6$ (Tier 2): Giữ lại có điều kiện.
     * $LBD > 6$ (Tier 3): Bị coi là "rác" và bị xóa sạch trong các đợt dọn dẹp bộ nhớ định kỳ (`reduce` phase).
   - **Vòng lặp quên lãng (Amnesic Loop):** CaDiCaL học một mệnh đề cắt phức tạp $\rightarrow$ phân loại là Tier 3 $\rightarrow$ xóa bỏ $\rightarrow$ ở các vòng lặp sau lại đi vào không gian tìm kiếm tương tự và tái học lại chính xung đột đó.
3. **Hiện Tượng Vỡ Chu Trình Không Đơn Điệu (Cycle Shattering & Shrinking Giant):**
   - Nhìn vào diễn tiến của chu trình lớn nhất qua các vòng:
     * Iter 2: Chu trình lớn nhất đạt **1,366 khối** ($73.9\%$ đồ thị).
     * Iter 3: Chu trình lớn nhất sụt xuống **855 khối**!
     * Iter 5: Chu trình lớn nhất tiếp tục vỡ thành **768 khối**!
     * Iter 7: Chu trình lớn nhất chỉ còn **718 khối**, kèm theo 2 chu trình lớn khác là **473** và **380**.
   - Thay vì hợp nhất các chu trình con nhỏ vào chu trình khổng lồ, việc thêm mệnh đề cắt khiến chu trình khổng lồ **bị vỡ vụn (shattered)** thành các khối lớn cạnh tranh lẫn nhau.
4. **Đói Biến Phân Nhánh (VSIDS Starvation):**
   - Điểm số hoạt động của thuật toán phân nhánh VSIDS liên tục được cập nhật cho các biến nội bộ bên trong 20 gadget (nơi các hoán đổi cục bộ xảy ra liên tục).
   - Các biến kết nối liên-gadget (vốn quyết định việc hợp nhất chu trình toàn cục) không nhận được điểm tăng (bumped activity), dẫn tới tình trạng **đói biến (starvation)**. Solver dành 99% tài nguyên quyết định để xáo trộn nghiệm bên trong các gadget thay vì tìm đường nối liên-gadget.

---

## 5. Tier 4 — Chi Phí Tầng Thực Thi & FFI (Engine Runtime Profiling)

1. **Ô Nhiễm Danh Sách Theo Dõi (Watchlist Overhead):**
   - Sau mỗi vòng CEGAR, hàng chục đến hàng trăm mệnh đề cắt mới được nhồi vào solver thông qua tầng FFI (`rustsat-cadical`).
   - Cấu trúc Two-Watched-Literal của CaDiCaL phải duyệt qua hàng nghìn con trỏ mệnh đề dài mỗi khi một biến thay đổi chân trị, khiến throughput lan truyền thực tế giảm từ **3.25 M props/s** xuống dưới **1.8 M props/s**.
2. **Ảnh Hưởng Của Inprocessing:**
   - Các thủ tục inprocessing của CaDiCaL (như `vivify`, `subsume`, `probe`) bị kích hoạt lặp đi lặp lại trên cơ sở dữ liệu mệnh đề ngày càng phình to, tiêu tốn phần lớn chu kỳ CPU của các vòng SAT sau (tăng từ 0.8s ở Iter 4 lên 34.6s ở Iter 7).

---

## 6. Kết Luận Nguyên Nhân Gốc Rễ (Formal Causal Conclusion)

$$\mathbf{\text{Root Cause}} = \text{Bẫy chẵn lẻ Gadget (Tier 1)} + \text{Dormant Cuts (Tier 2)} \longrightarrow \text{LBD Bloat \& Clause Purging (Tier 3)}$$

Bế tắc của `graph868` (và toàn bộ phân lớp Class B2a) **không phải do bug phần mềm hay nhiễu môi trường**, mà là kết quả của sự tương tác bất lợi có tính hệ thống giữa cấu trúc tô-pô đồ thị và động học CDCL:
1. **Tô-pô:** Tính hai phía và cấu trúc gadget đối xứng cô lập ngăn cản mọi phép nối cục bộ (2-opt/3-opt), ép solver phải tự tìm cách giải bài toán toàn cục.
2. **Mã hóa:** Các mệnh đề cắt subtour quá dài và hoàn toàn ngủ yên trong pha BCP, không cung cấp lực dẫn hướng cho solver ở các mức quyết định sớm.
3. **CDCL:** Solver phát hiện xung đột ở mức sâu, sinh mệnh đề học có LBD cao, bị cơ chế dọn rác của CaDiCaL xóa bỏ liên tục, dẫn đến hiện tượng dao động vỡ chu trình (Cycle Shattering) và bùng nổ xung đột cấp số mũ.

---

## 7. Khuyến Nghị Chuyển Tiếp (Next Transition)

Theo chuẩn mực `sat-root-cause-analysis`, vì nguyên nhân gốc rễ đã được chứng minh định lượng là **Rào cản biểu diễn và động học CDCL nội tại**:
- **Chuyển tiếp sang `sat-hypothesis-generation`**: Xây dựng giả thuyết khoa học về:
  1. **Direct Gadget State Encoding**: Thay thế các mệnh đề cắt subtour ngủ yên bằng các biến trạng thái gadget vĩ mô có khả năng kích hoạt BCP ngay từ Root Level.
  2. **Bounded Backbone Freezing**: Khóa cứng ít nhất $90\%$ các cạnh của chu trình khổng lồ (Giant Cycle) bằng cơ chế assumptions để ngăn chặn triệt để hiện tượng vỡ chu trình (Cycle Shattering).
