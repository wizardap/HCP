import collections
from typing import Dict, List, Set, Tuple, Any, Optional
from pysat.solvers import Cadical195
from pysat.card import CardEnc, EncType

class GlobalDemandCoordinator:
    def __init__(self, G: Dict[int, Set[int]], decomp: Any):
        self.G = G
        self.decomp = decomp
        self.solver = Cadical195()
        self.nv = 0
        self._build_macro_cnf()
        
    def _build_macro_cnf(self):
        # 1. Variables for Hub-Hub direct edges
        self.var_hh = {}
        for u, v in self.decomp.hh_edges:
            self.nv += 1
            self.var_hh[(u, v)] = self.nv
            self.var_hh[(v, u)] = self.nv
            
        # 2. Variables for Strip-Hub port allocations for all adjacent hubs
        self.var_d1 = {}
        self.var_d2 = {}
        for si, s in enumerate(self.decomp.strips):
            for h in sorted(self.decomp.strip_adj_hubs.get(si, set())):
                self.nv += 1
                self.var_d1[(si, h)] = self.nv
                self.nv += 1
                self.var_d2[(si, h)] = self.nv
                # d2 => d1 (i.e. not d2 or d1)
                self.solver.add_clause([-self.var_d2[(si, h)], self.var_d1[(si, h)]])
                
        # 3. Exact-2 degree constraint on ALL 310 Hubs
        for h in self.decomp.all_hubs:
            inc_hh = [self.var_hh[(h, nbr)] for nbr in self.G[h] if (h, nbr) in self.var_hh]
            inc_strips_1 = [self.var_d1[(si, h)] for si in self.decomp.hub_adj_strips.get(h, set()) if (si, h) in self.var_d1]
            inc_strips_2 = [self.var_d2[(si, h)] for si in self.decomp.hub_adj_strips.get(h, set()) if (si, h) in self.var_d2]
            
            all_lits = inc_hh + inc_strips_1 + inc_strips_2
            self._add_exact_2(all_lits)
            
        # 4. Strict Parity & Endpoint Bounds per Strip
        for si, s in enumerate(self.decomp.strips):
            adj_hubs = sorted(self.decomp.strip_adj_hubs.get(si, set()))
            strip_lits = [self.var_d1[(si, h)] for h in adj_hubs] + [self.var_d2[(si, h)] for h in adj_hubs]
            if len(s) < 10:
                # Small strip: exactly 2 endpoints (K = 1)
                self._add_exact_2(strip_lits)
                # Per-vertex endpoint capacity in small strip: at most 1 hub per bulk vertex
                for u in s:
                    u_hubs = [h for h in self.G[u] if h in self.decomp.all_hubs]
                    u_lits = [self.var_d1[(si, h)] for h in u_hubs if (si, h) in self.var_d1]
                    for i in range(len(u_lits)):
                        for j in range(i + 1, len(u_lits)):
                            self.solver.add_clause([-u_lits[i], -u_lits[j]])
            else:
                # Large strip: exactly one K in {2, 3, 4, 5}
                k_vars = []
                for k in (2, 3, 4, 5):
                    self.nv += 1
                    k_vars.append(self.nv)
                cnf_k = CardEnc.equals(lits=k_vars, bound=1, top_id=self.nv, encoding=EncType.seqcounter)
                self.nv = max(self.nv, cnf_k.nv)
                for cl in cnf_k.clauses:
                    self.solver.add_clause(cl)
                    
                for idx, k in enumerate((2, 3, 4, 5)):
                    target = 2 * k
                    cnf_ge = CardEnc.atleast(lits=strip_lits, bound=target, top_id=self.nv, encoding=EncType.seqcounter)
                    self.nv = max(self.nv, cnf_ge.nv)
                    for cl in cnf_ge.clauses:
                        self.solver.add_clause([-k_vars[idx]] + cl)
                    cnf_le = CardEnc.atmost(lits=strip_lits, bound=target, top_id=self.nv, encoding=EncType.seqcounter)
                    self.nv = max(self.nv, cnf_le.nv)
                    for cl in cnf_le.clauses:
                        self.solver.add_clause([-k_vars[idx]] + cl)
            
    def _add_exact_2(self, lits: List[int]):
        if len(lits) < 2:
            self.solver.add_clause([])
            return
        if len(lits) == 2:
            self.solver.add_clause([lits[0]])
            self.solver.add_clause([lits[1]])
            return
        cnf = CardEnc.equals(lits=lits, bound=2, top_id=self.nv, encoding=EncType.seqcounter)
        self.nv = max(self.nv, cnf.nv)
        for cl in cnf.clauses:
            self.solver.add_clause(cl)
        
    def solve_assignment(self) -> Tuple[bool, List[Tuple[int, int]], Dict[int, Dict[int, int]]]:
        if not self.solver.solve():
            return False, [], {}
            
        model = self.solver.get_model()
        m_bool = {abs(x): x > 0 for x in model}
        
        hh_edges = []
        for (u, v), vi in self.var_hh.items():
            if u < v and m_bool.get(vi, False):
                hh_edges.append((u, v))
                
        strip_demands = {si: {} for si in range(len(self.decomp.strips))}
        for (si, h), v1 in self.var_d1.items():
            d = 0
            if m_bool.get(v1, False):
                d = 2 if m_bool.get(self.var_d2[(si, h)], False) else 1
            strip_demands[si][h] = d
            
        return True, hh_edges, strip_demands
        
    def add_conflict_clause(self, si: int, dem: Dict[int, int], failed_hubs: Optional[List[int]] = None):
        """
        Adds a mathematically exact blocking clause for an unsatisfiable strip demand assignment.
        """
        clause = []
        target_hubs = failed_hubs if (failed_hubs and len(failed_hubs) < len(dem)) else list(dem.keys())
        for h in target_hubs:
            req = dem.get(h, 0)
            v1 = self.var_d1.get((si, h))
            v2 = self.var_d2.get((si, h))
            if req == 0 and v1:
                clause.append(v1)
            elif req == 1 and v1:
                clause.append(-v1)
                if v2:
                    clause.append(v2)
            elif req == 2 and v2:
                clause.append(-v2)
                
        if clause:
            self.solver.add_clause(list(set(clause)))
            
    def add_macro_cut_clause(self, cyc_verts: Set[int], current_hh: Optional[List[Tuple[int, int]]] = None, strip_demands: Optional[Dict[int, Dict[int, int]]] = None):
        """
        Strict Cut-Crossing Subtour Elimination Constraint on Hub Partition:
        Forces at least one crossing HH edge or at least one strip bridging H_inside and H_outside.
        """
        h_inside = set(cyc_verts) & self.decomp.all_hubs
        h_outside = self.decomp.all_hubs - h_inside
        
        if not h_inside or not h_outside:
            return
            
        cut_clause = []
        # 1. Crossing HH edges
        for (u, v), vi in self.var_hh.items():
            if u < v and ((u in h_inside) != (v in h_inside)):
                cut_clause.append(vi)
                
        # 2. Strips bridging H_inside and H_outside
        for si, s in enumerate(self.decomp.strips):
            adj = self.decomp.strip_adj_hubs.get(si, set())
            in_hubs = adj & h_inside
            out_hubs = adj & h_outside
            
            if in_hubs and out_hubs:
                # Indicator y_si represents: strip si allocates >= 1 port to in_hubs AND >= 1 port to out_hubs
                self.nv += 1
                y_si = self.nv
                cut_clause.append(y_si)
                
                # y_si => OR(d1 for in_hubs)
                lits_in = [self.var_d1[(si, h)] for h in in_hubs if (si, h) in self.var_d1]
                self.solver.add_clause([-y_si] + lits_in)
                
                # y_si => OR(d1 for out_hubs)
                lits_out = [self.var_d1[(si, h)] for h in out_hubs if (si, h) in self.var_d1]
                self.solver.add_clause([-y_si] + lits_out)
                
        if cut_clause:
            self.solver.add_clause(list(set(cut_clause)))

    def add_macro_cut(self, cyc_verts: Set[int], current_hh: Optional[List[Tuple[int, int]]] = None, strip_demands: Optional[Dict[int, Dict[int, int]]] = None):
        self.add_macro_cut_clause(cyc_verts, current_hh, strip_demands)
