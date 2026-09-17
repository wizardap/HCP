import sys, os, time
from scratch.test_splice_14v import test_splice
from scratch.engine.assembler import assemble_full_tour, export_hcp_tour
from scratch.verify_benchmarks import verify_tour

def run_full_assembly():
    print("=" * 70)
    print("RUNNING FULL ASSEMBLY & CERTIFICATION FOR graph944.col")
    print("=" * 70)

    t0 = time.time()
    res = test_splice()
    if not res:
        print("Failed to get merged cycle!")
        sys.exit(1)

    comp0_contracted_cyc, edge_chains, comp0_nodes, corridors, G = res
    print(f"[*] Stage 3 Complete: Comp 0 Contracted cycle: {len(comp0_contracted_cyc)} vertices.")

    # Expand degree-2 contracted chains
    print("[*] Expanding degree-2 contracted chains...")
    expanded = []
    n = len(comp0_contracted_cyc)
    for i in range(n):
        u = comp0_contracted_cyc[i]
        v = comp0_contracted_cyc[(i + 1) % n]
        e = tuple(sorted([u, v]))
        if e in edge_chains:
            chain = edge_chains[e]
            if chain[0] != u:
                chain = list(reversed(chain))
            expanded.extend(chain[:-1])
        else:
            expanded.append(u)

    print(f"[*] Comp 0 Expanded: {len(expanded)} vertices (expected {len(comp0_nodes)}).")
    assert len(expanded) == len(comp0_nodes)
    assert len(set(expanded)) == len(comp0_nodes)

    # Solve corridor paths
    from scratch.engine.corridor_solver import solve_corridor_path
    print("[*] Solving corridors via local SAT HP...")
    for idx, c in enumerate(corridors):
        u, v = c['ports']
        t_c = time.time()
        path = solve_corridor_path(G, c['nodes'], src=u, dst=v)
        c['path'] = path
        print(f"    Corridor {idx+1} ({len(path)}v) solved in {time.time()-t_c:.2f}s.")

    # Assemble full tour
    print("[*] Splicing corridors into Comp 0...")
    tour = assemble_full_tour(expanded, corridors)
    N = len(G)
    print(f"[*] Full tour assembled: {len(tour)} vertices (expected {N}).")
    assert len(tour) == N
    assert len(set(tour)) == N

    output_path = "scratch/engine/found_tour_graph944.hcp"
    export_hcp_tour(tour, "graph944.col", output_path)
    print(f"[*] Exported tour to {output_path}")

    # Verify tour with verify_benchmarks
    print("=" * 70)
    print("RUNNING OFFICIAL BENCHMARK VERIFIER...")
    print("=" * 70)
    ok = verify_tour("FHCPCS-col/graph944.col", output_path)
    if ok:
        print(f"[✓✓✓] graph944.col CERTIFIED 100% SOUND IN {time.time()-t0:.2f}s!")
    else:
        print("[X] VERIFICATION FAILED!")
        sys.exit(1)

if __name__ == '__main__':
    run_full_assembly()
