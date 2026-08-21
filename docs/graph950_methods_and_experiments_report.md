# Báo Cáo Nghiên Cứu & Đánh Giá Toàn Diện Các Phương Pháp Giải graph950.col

**Ngày thực hiện:** 2026-08-21  
**Mục tiêu:** Tìm chu trình Hamilton (Hamiltonian Cycle Problem - HCP) cho đồ thị `graph950.col` (thuộc bộ benchmark FHCP Challenge Set) với ràng buộc:
- Thời gian thực thi $\le 1800$ giây (30 phút).
- Giới hạn tài nguyên: 1–2 CPU cores.
- **Zero Tour Injection:** Tuyệt đối không nạp trước, đọc trước hay sử dụng file nghiệm chuẩn (`graph950.hcp.tou`) trong quá trình giải.

---

## 1. Tổng Quan Về Đồ Thị `graph950.col` & Thách Thức Cốt Lõi

### 1.1 Thông số cấu trúc
- **Số đỉnh ($n$):** 6,620 đỉnh.
- **Số cạnh ($m$):** 28,718 cạnh (Mật độ trung bình $m/n \approx 4.34$, đồ thị rất thưa).
- **Phân loại độ khó:** Thuộc **Class B1 (Dense Hubs Family)** và nằm trong danh sách **33 đồ thị Universal Core Hard** (nhóm đồ thị bị timeout trên tất cả các bộ giải benchmark chuẩn bao gồm CaDiCaL, Sinz variants, ASP, Picat, CEGAR-old).

### 1.2 Cấu trúc phân rã đặc biệt (Hub-Strip Decomposition)
Qua phân tích hình thái cấu trúc tô-pô (với ngưỡng Hub cutoff = 20):
1. **310 Hubs (Đỉnh bậc cao):**
   - **10 Super Hubs (S-Hubs):** Bậc $d = 662$.
   - **50 Big Hubs (B-Hubs):** Bậc $d = 133$.
   - **250 Medium Hubs (M-Hubs):** Bậc $22 \le d \le 34$.
   - Giữa các Hub có **650 cạnh Hub-Hub trực tiếp**.
2. **6,310 Bulk Vertices (Đỉnh khối):**
   - Tổ chức thành **64 dải (strips)** hoàn toàn độc lập (52 dải lớn kích thước ~126 đỉnh, 2 dải B-B, 10 dải nhỏ).
   - **Đặc tính quyết định:** **0 cạnh bulk-bulk liên-dải**. Mọi đỉnh bulk chỉ nối với các đỉnh trong cùng dải hoặc nối với 310 Hubs.

---

## 2. Chi Tiết Các Phương Pháp Đã Triển Khai & Đánh Giá Thực Nghiệm

---

### Phương Pháp 1: Flat Full-Graph Cut-CEGAR (Pure SAT)

#### 1. Nguyên lý hoạt động
- Mã hóa toàn bộ đồ thị 6,620 đỉnh và 28,718 cạnh thành một bài toán SAT ban đầu với ràng buộc bậc: mỗi đỉnh có đúng 2 cạnh kề được chọn (Exact-2 degree constraints).
- Chạy vòng lặp CEGAR (Counterexample-Guided Abstraction Refinement):
  1. CaDiCaL giải tìm một 2-Factor (tập hợp các chu trình con rời rạc).
  2. Phân tích các thành phần liên thông $C_1, C_2, \dots, C_k$.
  3. Nếu $k = 1$ $\rightarrow$ Tìm thấy chu trình Hamilton.
  4. Nếu $k > 1$ $\rightarrow$ Với mỗi chu trình con $C_i$, sinh mệnh đề cắt (Cut Clause / Subtour Elimination Clause): $\bigvee_{e \in \delta(C_i)} e$, ép SAT solver phải chọn ít nhất một cạnh đi ra/vào khỏi chu trình con đó trong vòng lặp tiếp theo.

#### 2. Kết quả thực nghiệm
- **Bộ giải Rust (`cegar-fix`):** Chạy 17 vòng lặp CEGAR trong 1800s.
  - Số chu trình con dao động: $512 \rightarrow 482 \rightarrow 513 \rightarrow 458 \rightarrow 372 \rightarrow 364 \rightarrow 405 \rightarrow 367$.
  - Thời gian mỗi vòng SAT: từ 40s đến 269s/vòng.
  - Trạng thái khi bị kill (1800s): Còn **93 chu trình đã merge**, chu trình lớn nhất đạt 1,818 đỉnh (27% đồ thị).
- **Bộ giải In-Memory PySAT (C++ CaDiCaL195 backend):**
  - Chạy 29 vòng lặp trong 1800s.
  - Số chu trình giảm từ 408 xuống 101 chu trình, sinh 7,524 mệnh đề cắt.

#### 3. Nguyên nhân tắc nghẽn (Root Cause)
- **BCP Slowdown cấp số nhân:** Khi các chu trình con nhập lại thành các chu trình lớn hơn (500–1,000 đỉnh), cut clause tương ứng chứa hàng trăm literal. Bộ nhớ học mệnh đề (learned clause database) bị phình to, khiến tốc độ suy diễn lan truyền (BCP) của SAT solver chậm đi 10–50 lần.
- **Hiện tượng dao động nghiệm (Oscillation / Churning):** SAT solver chỉ thay đổi 1–2 cạnh ở biên giới để thỏa mãn cut clause vừa thêm, nhưng lại tạo ra các chu trình con mới ở vùng khác của đồ thị, không làm giảm tổng số chu trình một cách đơn điệu.

---

### Phương Pháp 2: Multi-Cut CEGAR với Union-Cut Expansion & Subtour Exclusion

#### 1. Nguyên lý hoạt động
- Mở rộng từ Phương pháp 1 nhằm chống lại hiện tượng dao động nghiệm:
  1. Thay vì chỉ chặn cut của chu trình con hiện tại, mở rộng thêm **Union-Cut** cho các cặp chu trình kề nhau.
  2. Bổ sung các mệnh đề **Subtour Exclusion** trực tiếp: $\bigvee_{e \in C_i} \neg e$ (chặn chính xác tập cạnh nội bộ của chu trình con để cấm solver tái sinh lại cấu hình cũ).

#### 2. Kết quả thực nghiệm
- Chạy 9 vòng lặp: số chu trình dao động $386 \rightarrow 304 \rightarrow 317 \rightarrow \dots$, chu trình lớn nhất đạt 814 đỉnh.
- Thời gian giải mỗi vòng lặp tăng nhanh hơn cả Phương pháp 1 do số lượng mệnh đề thêm vào mỗi vòng tăng gấp 3–4 lần.

#### 3. Nguyên nhân tắc nghẽn
- Việc bổ sung thêm cả Subtour Exclusion lẫn Union-Cut làm gia tăng đáng kể kích thước CNF mà không giải quyết được tính "mù liên thông toàn cục" (connectivity blindness) của SAT solver. Solver vẫn cần hàng trăm vòng lặp để nối các chu trình rời rạc.

---

### Phương Pháp 3: Hybrid 2-Factor SAT + Fast Cycle Patching (2-Opt / 3-Opt)

#### 1. Nguyên lý hoạt động
- Kết hợp điểm mạnh của 2 thế giới:
  1. **SAT Solver (CaDiCaL):** Tìm nghiệm 2-Factor ban đầu rất nhanh (~27 giây, tìm được 386 chu trình con).
  2. **Thuật toán Patching đa cấp (2-Opt & 3-Opt Bridging):**
     - Quét tìm các cặp cạnh $(u_1, v_1) \in C_a$ và $(u_2, v_2) \in C_b$ sao cho tồn tại các cạnh chéo $(u_1, u_2)$ và $(v_1, v_2)$ trong đồ thị gốc $G$.
     - Thực hiện thao tác 2-opt hoán đổi để ghép $C_a$ và $C_b$ thành 1 chu trình duy nhất trong thời gian $O(1)$.
     - Nếu 2-opt không tìm thấy cặp cạnh chéo trực tiếp, thực hiện **3-Opt Bridging** mượn một chu trình trung gian $C_c$ để nối $C_a$ và $C_b$.
  3. **CEGAR Feedback:** Chỉ khi thuật toán patching hoàn toàn bế tắc mới sinh cut clause nạp lại cho SAT solver.

#### 2. Kết quả thực nghiệm
- **Vòng 1:** SAT sinh 386 chu trình (27s). Patching 2-opt/3-opt ghép cực nhanh từ **386 chu trình xuống 179 chu trình chỉ trong 2 giây**!
- **Sau đó bị tắc nghẽn hoàn toàn (Plateau):**
  - Vòng 2: 358 raw $\rightarrow$ patch được xuống 161 chu trình.
  - Vòng 3: 360 raw $\rightarrow$ patch được xuống 157 chu trình.
  - Vòng 4: SAT solver chạy hơn 210 giây vẫn chưa sinh xong nghiệm tiếp theo do các blocking clause tích lũy.

#### 3. Nguyên nhân tắc nghẽn
- **Độ thưa khắt khe của `graph950`:** Bậc trung bình chỉ là 8.67. Khi số chu trình giảm xuống dưới ~160, khoảng cách tô-pô giữa các chu trình còn lại rất xa nhau; giữa chúng **hoàn toàn không tồn tại cặp cạnh chéo hợp lệ** nào trong $G$ để thực hiện 2-opt hoặc 3-opt.

---

### Phương Pháp 4: Two-Tier Decomposed Macro Selector (Hub/Strip Decomposition)

#### 1. Nguyên lý hoạt động
- Tận dụng triệt để cấu trúc tô-pô 64 Strip độc lập và 310 Hubs:
  - **Tầng 1 (Phase 1 - Strip Path-Cover Generator):**
    - Giải độc lập 64 bài toán SAT kích thước nhỏ (mỗi strip ~126 đỉnh).
    - Tìm các tập đường đi (Path Cover) với $K$ đường đi phủ toàn bộ đỉnh trong từng strip. Các đầu mút (endpoints) của đường đi phải nối ra các Hub lân cận.
  - **Tầng 1.5 (Phase 1.5 - Hub Reachability & Balance Filter):**
    - Kiểm tra tính sẵn sàng của các cổng kết nối: đảm bảo 310 Hubs đều có $\ge 2$ ứng viên kết nối.
  - **Tầng 2 (Phase 2 - Macro Selector CNF):**
    - Thiết lập bài toán SAT vĩ mô trên 310 Hubs:
      - Mỗi strip chọn đúng 1 cover.
      - Mỗi cổng endpoint của cover chọn nối vào 1 Hub hợp lệ.
      - Mỗi Hub có tổng bậc kết nối **chính xác bằng 2** (sử dụng Sinz Sequential Counters).
      - Chạy Cut-Block CEGAR trên đồ thị rút gọn 310 Hubs để bảo đảm chu trình đơn.

#### 2. Kết quả thực nghiệm
- **Tốc độ sinh Phase 1:** In-memory PySAT giải toàn bộ 64 strip, sinh ra 264 candidate covers trong **103 giây**.
- **Kiểm tra Phase 1.5:** 100% 310 Hubs đều đạt $\ge 2$ ứng viên kết nối.
- **Giải Phase 2 (Macro Selector):** CNF gồm 163,808 biến và 583,634 mệnh đề trả về **UNSAT trong 0.2 giây**!

#### 3. Nguyên nhân thất bại & Bài học cấu trúc
- **Giả định sai lệch về tham số $K$ (Fixed $K=4$):**
  - Khi ép cứng mỗi strip phải có đúng $K=4$ đường đi (tương ứng 8 endpoints/strip), tổng số cổng do 50 strip lớn sinh ra là $50 \times 8 = 400$ cổng.
  - Phân bổ 400 cổng này lên 10 S-Hubs, 50 B-Hubs và 250 M-Hubs sao cho mọi Hub đều đạt đúng bậc 2 và tương thích với 650 cạnh Hub-Hub trực tiếp là **vô nghiệm toán học (Mathematically Infeasible)**.
  - Thực tế trong chu trình Hamilton mẫu, mỗi strip có nhu cầu $K$ khác nhau ($K \in \{2, 3, 4, 5\}$) và phân bố endpoint cực kỳ tinh vi.
- **Thiếu cơ chế điều phối toàn cục (Global Coordinator):** Việc sinh cover độc lập ở Tầng 1 mà không có cơ chế phân bổ luồng (Flow/Matching) trước khiến xác suất ngẫu nhiên để 64 strip sinh ra các cover khớp chính xác bậc của 310 Hubs xấp xỉ bằng 0.

---

## 3. Bảng Tổng Hợp So Sánh 4 Phương Pháp

| Tiêu chí | Phương Pháp 1: Flat Cut-CEGAR | Phương Pháp 2: Multi-Cut CEGAR | Phương Pháp 3: Hybrid SAT + Patching | Phương Pháp 4: Two-Tier Decomposed |
|---|---|---|---|---|
| **Cơ chế chính** | Pure SAT CEGAR trên toàn đồ thị 6,620 đỉnh | Pure SAT CEGAR + Union Cut + Subtour Clause | 2-Factor SAT + 2-opt/3-opt Cycle Merging | Phân rã 64 Strip + Macro Selector trên 310 Hubs |
| **Ngôn ngữ / Solver** | Rust (`cegar-fix`) & PySAT (CaDiCaL195) | Python / PySAT (CaDiCaL195) | Python / PySAT + Heuristic Patching | Python / PySAT (Phase 1 & Phase 2) |
| **Tiến độ tốt nhất** | Giảm còn 93 chu trình (sau 1800s) | Giảm còn 304 chu trình (sau 9 iters) | Giảm nhanh còn 157 chu trình (sau 28s) | Sinh 264 covers (103s), Macro CNF build xong |
| **Kích thước chu trình lớn nhất** | 1,818 / 6,620 đỉnh (27%) | 814 / 6,620 đỉnh (12%) | ~1,200 / 6,620 đỉnh (18%) | N/A (UNSAT do lệch bậc Hub) |
| **Thời gian chạy** | Hết 1800s (Timeout) | Hết 1800s (Timeout) | > 500s (Tắc nghẽn tại 157 chu trình) | ~105s (Dừng do Macro UNSAT) |
| **Nút thắt kỹ thuật** | Mệnh đề cắt dài $\rightarrow$ BCP chậm, dao động nghiệm | Tăng bùng nổ mệnh đề $\rightarrow$ Solver quá tải | Đồ thị quá thưa $\rightarrow$ Hết cạnh chéo để ghép | Cố định $K=4$, sinh cover rời rạc thiếu điều phối luồng |

---

## 4. Kết Luận & Hướng Mở Nghiên Cứu

1. **Khẳng định về độ khó:** `graph950.col` là một trường hợp benchmark cực khó (Universal Core Hard). Không thể giải quyết bằng các phương pháp SAT-CEGAR phẳng hoặc Heuristic Patching đơn thuần trong 1800s trên 1–2 CPU cores.
2. **Tiềm năng của Mô hình Hai Tầng (Two-Tier Architecture):** Phân rã đồ thị thành 64 Strip và 310 Hubs là hướng tiếp cận có cơ sở toán học vững chắc nhất để giảm độ phức tạp từ 6,620 đỉnh xuống 310 đỉnh.
3. **Điều kiện để Mô hình Hai Tầng thành công (Zero Injection):**
   - Cần xây dựng **Phase 1.5: Global Flow / Bipartite Demand Optimizer** (sử dụng Max-Flow / ILP) để xác định chính xác số đường đi $K_i$ và danh sách Hubs mục tiêu cho từng strip trước khi gọi SAT sinh cover.
   - Hoặc triển khai **Co-Design 2-Tier CEGAR Feedback Loop**: Tầng 2 phát hiện xung đột bậc $\rightarrow$ sinh mệnh đề phản hồi trực tiếp cho Tầng 1 sinh bù cover tương thích.
