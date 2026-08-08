# Phân Tích Chi Tiết: Hybrid CEGAR + Partial MTZ (Hướng C)

**Date**: 2026-08-08
**Status**: Deep Analysis / Reference

---

## 1. Tại Sao Partial MTZ Mạnh Hơn Cut-Set Constraints?

### 1.1 Cut-set constraint (hiện tại, `-b 3`)

Khi CEGAR tìm được subtour trên tập đỉnh $S = \{v_1, ..., v_k\}$:

$$x(\delta(S)) \geq 2$$

**Phủ 1 tập $S$ cụ thể.** Nếu SAT solver chỉ dời 1 đỉnh (ví dụ $S' = \{v_1, ..., v_{k-1}, v_{k+1}\}$), constraint trên $S$ KHÔNG ngăn subtour trên $S'$.

→ Mỗi vòng CEGAR loại bỏ **1** partition. Cần tới $O(2^n)$ vòng lặp trong trường hợp xấu nhất.

### 1.2 Partial MTZ trên tập $K$

Thêm biến vị trí $p_v$ cho mỗi $v \in K$, với ràng buộc:

$$x_{u,v} = 1 \land u, v \in K \setminus \{s\} \implies p_v \geq p_u + 1$$

(Với $s \in K$ là đỉnh nguồn cố định.)

**Phủ $2^{|K|}$ tập con cùng lúc.** Constraint này ngăn ĐỒNG THỜI:
- Subtour trên $\{v_2, v_3\}$
- Subtour trên $\{v_2, v_3, v_4\}$
- Subtour trên $\{v_5, v_{10}, v_{20}\}$
- **Mọi** tập con $T \subseteq K \setminus \{s\}$

...chỉ bằng $O(|K| \log n)$ biến phụ và $O(|E_K| \log^2 n)$ clause.

**Đây là sự khác biệt cốt lõi**: 1 lần inject MTZ = loại bỏ exponentially nhiều partition, thay vì 1 partition mỗi vòng CEGAR.

---

## 2. Partial MTZ KHÔNG ngăn được gì?

- **Subtour chứa đỉnh ngoài $K$**: Nếu subtour gồm cả đỉnh trong $K$ và ngoài $K$, MTZ trên $K$ KHÔNG ngăn (vì đỉnh ngoài $K$ không có biến $p_v$).
- **Subtour chứa source $s$**: MTZ convention cho phép source $s$ ở vị trí 0 và quay vòng.

→ Partial MTZ **bổ sung** cho CEGAR, không thay thế hoàn toàn.

---

## 3. Thiết Kế Cụ Thể Cho Codebase Hiện Tại

### 3.1 Khi nào inject MTZ? (Adaptive Detection)

Thêm bộ đếm `stall_count` trong vòng CEGAR (`hcp_solver.rs`):

```
prev_subcycle_count ← ∞
stall_count ← 0

Mỗi vòng CEGAR:
  subcycle_count ← sol_cycles.len() sau 2-opt/3-opt
  if subcycle_count >= prev_subcycle_count:
      stall_count += 1
  else:
      stall_count = 0
  prev_subcycle_count = subcycle_count

  if stall_count >= STALL_THRESHOLD (ví dụ 3-5):
      → Kích hoạt inject Partial MTZ
      → Reset stall_count
```

### 3.2 Chọn tập $K$ như thế nào?

Khi phát hiện bị stall, ta có tập subcycles $C_1, C_2, ..., C_m$ từ vòng hiện tại.

**Chiến lược đề xuất**: Chọn $K$ = tập đỉnh của **subcycle nhỏ nhất** $C_{min}$.

Lý do:
- Subcycle nhỏ → ít biến phụ hơn ($|K| \log n$)
- Subcycle nhỏ → ít clause hơn
- Subcycle nhỏ thường là "bẫy cứng" (rigid trap) dễ tái xuất hiện nhất
- Nếu subcycle nhỏ nhất quá nhỏ ($|C_{min}| \leq 3$), có thể chọn 2-3 subcycles nhỏ nhất gộp lại

**Chiến lược nâng cao (nếu cần)**: Nếu vẫn bị stall sau lần inject đầu, mở rộng $K$ bằng cách thêm đỉnh từ subcycle nhỏ tiếp theo.

### 3.3 Encoding MTZ vào SAT

Cho tập $K = \{v_1, v_2, ..., v_k\}$ với source $s = v_1$.

**Biến phụ**: Với mỗi $v \in K$, encode vị trí $p_v \in \{0, 1, ..., n-1\}$ bằng $B = \lceil \log_2 n \rceil$ biến boolean:

$$p_v = \sum_{j=0}^{B-1} b_{v,j} \cdot 2^j$$

**Ràng buộc MTZ**: Với mỗi cạnh có hướng $(u, v)$ trong đồ thị mà $u, v \in K \setminus \{s\}$:

$$x_{u,v} = 1 \implies p_v \geq p_u + 1$$

Encode qua: $\neg x_{u,v} \lor (p_v \geq p_u + 1)$

Phép so sánh $p_v \geq p_u + 1$ encode bằng binary comparator circuit:
- Tính $d = p_v - p_u$ bằng binary subtractor
- Ràng buộc $d \geq 1$ (bit thấp nhất hoặc bất kỳ bit cao nào bằng 1)

### 3.4 Chi phí encoding

Với $k = |K|$, $n = |V|$, $B = \lceil \log_2 n \rceil$:

| Thành phần | Biến phụ | Clause phụ |
|---|---|---|
| Position variables | $k \cdot B$ | 0 |
| Binary comparator (mỗi cạnh trong $K$) | $O(B)$ carry vars | $O(B)$ clauses |
| Tổng (cho $|E_K|$ cạnh trong $K$) | $O(k \cdot B + |E_K| \cdot B)$ | $O(|E_K| \cdot B)$ |

**Ví dụ cụ thể cho `graph339.col`** ($n = 132$, $B = 8$):
- Nếu $K$ = subcycle nhỏ nhất (63 đỉnh, avg degree $d \approx 4$):
  - Position vars: $63 \times 8 = 504$
  - $|E_K| \approx 63 \times 4 / 2 = 126$ cạnh nội bộ
  - Comparator vars: $\sim 126 \times 8 = 1008$
  - Comparator clauses: $\sim 126 \times 8 = 1008$
  - **Tổng thêm: ~1512 biến, ~1008 clause** — rất nhỏ so với CNF ban đầu

---

## 4. Luồng Hoạt Động Tổng Thể

```
CEGAR Loop (hiện tại):
  1. SAT solve → subcycles
  2. 2-opt + 3-opt merge
  3. Cut-set blocking (ASP -b 3)
  4. Repeat

Hybrid CEGAR + Partial MTZ (đề xuất):
  1. SAT solve → subcycles
  2. 2-opt + 3-opt merge
  3. Cut-set blocking (ASP -b 3)
  4. Kiểm tra stall:
     - Nếu stall_count >= THRESHOLD:
       a. Chọn K = vertex set của subcycle(s) nhỏ nhất
       b. Inject Partial MTZ(K) vào solver
       c. Log: "MTZ injected for |K| vertices"
  5. Repeat
```

---

## 5. Rủi Ro & Mitigation

| Rủi ro | Mô tả | Giải pháp |
|---|---|---|
| Formula phình to nếu inject nhiều lần | Mỗi lần inject thêm $O(k \cdot B)$ biến | Giới hạn tổng tổng biến thêm $\leq n \cdot B$ |
| SAT solver chậm hơn do clause nhiều hơn | CaDiCaL phải xử lý thêm clause | Chi phí rất nhỏ cho $k$ nhỏ (xem ví dụ ở mục 3.4) |
| MTZ trên K không đủ → vẫn bị stall | Subtour mới chứa đỉnh ngoài K | Adaptive: mở rộng K mỗi lần stall thêm |
| Binary arithmetic encoding phức tạp | Cài đặt comparator circuit trong Rust | Có thể dùng thư viện rustsat hoặc tự encode |

---

## 6. So Sánh Separation Power

| Phương pháp | Subtour sets bị loại mỗi lần | Chi phí mỗi lần |
|---|---|---|
| Subtour blocking (`-b 0`) | 1 cấu hình cạnh cụ thể | $O(k)$ clause |
| Cut-set (`-b 3`) | 1 vertex set cụ thể (mọi cấu hình bên trong) | $O(\|\delta(S)\|)$ clause |
| **Partial MTZ trên $K$** | **$2^{|K|}$ vertex subsets** (mọi tập con $\subseteq K$) | $O(\|K\| \cdot \log n)$ biến + clause |
| Full MTZ | $2^n$ (tất cả) | $O(n \log n)$ biến |
