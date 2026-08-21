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
            
        # 2. Variables for Strip-M-Hub port allocations: var_d[(si, h, count)] for count in {1, 2}
        self.var_d1 = {}
        self.var_d2 = {}
        for si, s in enumerate(self.decomp.strips):
            adj_m = self.decomp.strip_adj_hubs.get(si, set()) & set(self.decomp.m_hubs)
            for h in sorted(adj_m):
                self.nv += 1
                self.var_d1[(si, h)] = self.nv
                self.nv += 1
                self.var_d2[(si, h)] = self.nv
                # d2 => d1  (i.e. not d2 or d1)
                self.solver.add_clause([-self.var_d2[(si, h)], self.var_d1[(si, h)]])
                
        # 3. Exact-2 degree constraint on all M-Hubs
        for h in self.decomp.m_hubs:
            inc_hh = [self.var_hh[(h, nbr)] for nbr in self.G[h] if (h, nbr) in self.var_hh]
            inc_strips_1 = [self.var_d1[(si, h)] for si in self.decomp.hub_adj_strips.get(h, set()) if (si, h) in self.var_d1]
            inc_strips_2 = [self.var_d2[(si, h)] for si in self.decomp.hub_adj_strips.get(h, set()) if (si, h) in self.var_d2]
            
            all_lits = inc_hh + inc_strips_1 + inc_strips_2
            self._add_exact_2(all_lits)
            
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
        
    def add_conflict_clause(self, si: int, failed_hubs: Any):
        # Clause: NOT (all failed_hubs active simultaneously in strip si)
        clause = []
        for h in failed_hubs:
            if (si, h) in self.var_d1:
                clause.append(-self.var_d1[(si, h)])
        if clause:
            self.solver.add_clause(clause)
            
    def add_macro_cut_clause(self, cut_hubs: Set[int]):
        # Cut-crossing clause for Hub-Hub and Strip connectors across cut_hubs
        clause = []
        for (u, v), vi in self.var_hh.items():
            if u < v and ((u in cut_hubs) != (v in cut_hubs)):
                clause.append(vi)
                
        # Strips that interface with both inside and outside the cut
        for si, s in enumerate(self.decomp.strips):
            adj_m = self.decomp.strip_adj_hubs.get(si, set()) & set(self.decomp.m_hubs)
            has_inside = any(h in cut_hubs for h in adj_m)
            has_outside = any(h not in cut_hubs for h in adj_m)
            if has_inside and has_outside:
                for h in adj_m:
                    if (si, h) in self.var_d1:
                        clause.append(self.var_d1[(si, h)])
                        
        if clause:
            self.solver.add_clause(list(set(clause)))

    def add_macro_cut(self, cut_hubs: Set[int]):
        self.add_macro_cut_clause(cut_hubs)
