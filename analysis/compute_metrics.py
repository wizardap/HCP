"""Compute all 14 experiments from NDJSON trajectory files."""

import json
import numpy as np
from collections import Counter
from scipy.spatial.distance import pdist, squareform
from scipy.cluster.hierarchy import linkage
from scipy.stats import chi2_contingency, ttest_1samp, mannwhitneyu, spearmanr
from itertools import combinations


def load_trajectory(path):
    """Load NDJSON trajectory file. Returns list of dicts + optional final hamiltonian row."""
    rows = []
    hamiltonian_row = None
    with open(path) as f:
        for line in f:
            row = json.loads(line)
            if row.get("hamiltonian"):
                hamiltonian_row = row
            else:
                rows.append(row)
    return rows, hamiltonian_row


# --- Experiment 1: Vertex frequency ---

def vertex_frequency(rows, n_vertices):
    """Count iterations each vertex appears in any subtour component."""
    freqs = Counter()
    for row in rows:
        seen = set()
        for comp in row["components"]:
            for v in comp["vertices"]:
                seen.add(v)
        for v in seen:
            freqs[v] += 1
    counts = np.array([freqs.get(i, 0) for i in range(n_vertices)])
    return counts


# --- Experiment 2: Edge frequency ---

def edge_frequency(rows, n_edges):
    """Count iterations each edge variable appears in the model."""
    freqs = Counter()
    for row in rows:
        for e in row["model_edge_vars"]:
            freqs[e] += 1
    counts = np.array([freqs.get(i, 0) for i in range(1, n_edges + 1)])
    return counts


# --- Experiment 3: Consecutive Jaccard ---

def component_vertex_set(row):
    """Return frozenset of all vertices in all components for this iteration."""
    s = set()
    for comp in row["components"]:
        s.update(comp["vertices"])
    return frozenset(s)

def component_edge_set(row):
    """Return frozenset of all edges in all components for this iteration."""
    s = set()
    for comp in row["components"]:
        s.update(comp["edges"])
    return frozenset(s)

def jaccard(a, b):
    """Jaccard similarity between two sets."""
    if not a and not b:
        return 1.0
    return len(a & b) / len(a | b)

def consecutive_jaccard(rows, use="vertices"):
    """Compute Jaccard between consecutive iterations."""
    sets = [component_vertex_set(r) if use == "vertices" else component_edge_set(r) for r in rows]
    return np.array([jaccard(sets[i], sets[i + 1]) for i in range(len(sets) - 1)])


# --- Experiment 4: Full Jaccard matrix ---

def jaccard_matrix(rows, use="vertices"):
    """Compute NxN Jaccard similarity matrix."""
    sets = [component_vertex_set(r) if use == "vertices" else component_edge_set(r) for r in rows]
    n = len(sets)
    mat = np.ones((n, n))
    for i, j in combinations(range(n), 2):
        jac = jaccard(sets[i], sets[j])
        mat[i, j] = jac
        mat[j, i] = jac
    return mat


# --- Experiment 5: Persistent core detection ---

def persistent_core(rows, k, n_vertices):
    """Vertices appearing in >= k iterations."""
    freqs = Counter()
    for row in rows:
        seen = set()
        for comp in row["components"]:
            for v in comp["vertices"]:
                seen.add(v)
        for v in seen:
            freqs[v] += 1
    core = {v for v, c in freqs.items() if c >= k}
    return core

def core_sizes_by_threshold(rows, n_vertices, max_k=None):
    """Core size as function of frequency threshold k."""
    if max_k is None:
        max_k = len(rows)
    sizes = []
    for k in range(1, max_k + 1):
        core = persistent_core(rows, k, n_vertices)
        sizes.append(len(core))
    return np.array(sizes)


# --- Experiment 6: Core lifetime ---

def core_lifetimes(rows, k, n_vertices):
    """For each core vertex at threshold k, first and last iteration index."""
    core = persistent_core(rows, k, n_vertices)
    lifetimes = {}
    for idx, row in enumerate(rows):
        seen = set()
        for comp in row["components"]:
            for v in comp["vertices"]:
                seen.add(v)
        for v in seen:
            if v in core:
                if v not in lifetimes:
                    lifetimes[v] = {"first": idx, "last": idx}
                else:
                    lifetimes[v]["last"] = idx
    return lifetimes


# --- Experiment 8: Edge transitions ---

def edge_transitions(rows):
    """Per consecutive pair: symmetric difference size, added, removed."""
    results = []
    for i in range(len(rows) - 1):
        e0 = set(rows[i]["model_edge_vars"])
        e1 = set(rows[i + 1]["model_edge_vars"])
        added = len(e1 - e0)
        removed = len(e0 - e1)
        sym_diff = added + removed
        results.append({"sym_diff": sym_diff, "added": added, "removed": removed})
    return results


# --- Metric 15: All 4 iteration-to-iteration metrics ---

def all_consecutive_metrics(rows, n_vars=None):
    """Compute Vertex Jaccard, Edge Jaccard, Edge Hamming, Assignment Hamming.
    
    n_vars: total number of CNF variables. If None, inferred from max(model_edge_vars).
    
    Returns list of dicts, one per consecutive pair.
    """
    if n_vars is None:
        n_vars = max(max(r["model_edge_vars"]) for r in rows) if rows else 0

    results = []
    for i in range(len(rows) - 1):
        r0, r1 = rows[i], rows[i + 1]

        # Vertex sets
        v0 = component_vertex_set(r0)
        v1 = component_vertex_set(r1)

        # Edge sets from components
        ec0 = component_edge_set(r0)
        ec1 = component_edge_set(r1)

        # Full model: all positively-assigned variables
        m0 = set(r0["model_edge_vars"])
        m1 = set(r1["model_edge_vars"])

        # Vertex Jaccard
        vj = jaccard(v0, v1)

        # Edge Jaccard (from component edges)
        ej = jaccard(ec0, ec1)

        # Edge Hamming = component edge vars that differ
        edge_hamming = len(ec0 ^ ec1)

        # Assignment Hamming = all literals that flip sign
        assignment_hamming = len(m0 ^ m1)

        # Percentages
        n_comp_edges = len(ec0 | ec1)
        assignment_pct = (assignment_hamming / n_vars * 100) if n_vars > 0 else 0.0
        edge_pct = (edge_hamming / n_comp_edges * 100) if n_comp_edges > 0 else 0.0

        results.append({
            "vertex_jaccard": float(vj),
            "edge_jaccard": float(ej),
            "edge_hamming": edge_hamming,
            "assignment_hamming": assignment_hamming,
            "assignment_pct": float(assignment_pct),
            "edge_pct": float(edge_pct),
        })
    return results


# --- Experiment 9: Solver trajectory ---

def solver_trajectory(rows):
    """Extract time series of component sizes and timing."""
    comp_sizes = [[c["size"] for c in r["components"]] for r in rows]
    times = [r["solve_time_s"] for r in rows]
    total_times = [r["total_time_s"] for r in rows]
    return {
        "component_sizes": comp_sizes,
        "max_component_size": [max(s) if s else 0 for s in comp_sizes],
        "num_components": [len(s) for s in comp_sizes],
        "solve_times": times,
        "total_times": total_times,
    }


# --- Experiment 10: Similarity clustering ---

def graph_similarity_profile(rows):
    """Mean consecutive Jaccard for a single graph trajectory."""
    jac = consecutive_jaccard(rows)
    return {
        "mean_jaccard": float(np.mean(jac)),
        "std_jaccard": float(np.std(jac)),
        "min_jaccard": float(np.min(jac)),
        "max_jaccard": float(np.max(jac)),
        "n_iterations": len(rows),
    }


# --- Experiment 11: Frequent pattern mining (pairwise + permutation test) ---

def get_transactions(rows):
    """Each iteration -> set of vertices in all components."""
    transactions = []
    for row in rows:
        verts = set()
        for comp in row["components"]:
            verts.update(comp["vertices"])
        transactions.append(frozenset(verts))
    return transactions

def frequent_pairs(transactions, min_support_abs):
    """Find frequent size-1 and size-2 itemsets efficiently."""
    item_counts = Counter()
    pair_counts = Counter()
    
    for t in transactions:
        items = list(t)
        for v in items:
            item_counts[v] += 1
        for i in range(len(items)):
            for j in range(i + 1, len(items)):
                a, b = (items[i], items[j]) if items[i] < items[j] else (items[j], items[i])
                pair_counts[(a, b)] += 1
    
    freq_items = {item for item, cnt in item_counts.items() if cnt >= min_support_abs}
    freq_pairs = {pair: cnt for pair, cnt in pair_counts.items() if cnt >= min_support_abs
                  and pair[0] in freq_items and pair[1] in freq_items}
    
    return freq_items, freq_pairs

def frequent_vertex_patterns(rows, min_support=0.3, n_perm=200):
    """Frequent vertex pairs with permutation significance test."""
    n = len(rows)
    if n < 10:
        return {"n_transactions": n, "error": "too few iterations"}
    threshold = max(2, int(np.ceil(min_support * n)))
    transactions = get_transactions(rows)
    
    freq_items, freq_pairs = frequent_pairs(transactions, threshold)
    
    # Permutation test: shuffle vertex IDs per transaction
    perm_max_support = []
    for _ in range(n_perm):
        perm_trans = []
        for t in transactions:
            tlist = list(t)
            np.random.shuffle(tlist)
            perm_trans.append(frozenset(tlist))
        _, perm_pairs = frequent_pairs(perm_trans, threshold)
        if perm_pairs:
            perm_max_support.append(max(perm_pairs.values()))
    
    perm_95 = np.percentile(perm_max_support, 95) if perm_max_support else 0
    
    significant = {pair: cnt for pair, cnt in freq_pairs.items() if cnt >= perm_95}
    
    return {
        "n_transactions": n,
        "threshold": threshold,
        "n_vertices_in_components": len(freq_items),
        "n_frequent_pairs": len(freq_pairs),
        "n_significant_pairs": len(significant),
        "permutation_95p": int(perm_95),
        "significant_pairs": [{"u": int(u), "v": int(v), "support": int(c)}
                               for (u, v), c in sorted(significant.items(), key=lambda x: -x[1])[:30]],
        "top_pairs": [{"u": int(u), "v": int(v), "support": int(c)}
                      for (u, v), c in sorted(freq_pairs.items(), key=lambda x: -x[1])[:10]],
    }


# --- Experiment 13: Seed sensitivity ---

def seed_sensitivity(trajectories):
    """Compare core sets across multiple seeds for same graph.
    
    trajectories: list of (rows, hamiltonian_row) tuples, one per seed.
    """
    n_vertices = max(v for rows, _ in trajectories
                     for r in rows for c in r["components"] for v in c["vertices"]) + 1
    cores = []
    for rows, _ in trajectories:
        core = persistent_core(rows, k=3, n_vertices=n_vertices)
        cores.append(core)
    pairwise_jac = []
    for i, j in combinations(range(len(cores)), 2):
        pairwise_jac.append(jaccard(cores[i], cores[j]))
    return {
        "core_sets": [sorted(c) for c in cores],
        "pairwise_jaccard": pairwise_jac,
        "mean_pairwise_jaccard": float(np.mean(pairwise_jac)) if pairwise_jac else 0.0,
        "core_overlap_80pct": cores[0] & cores[1] if len(cores) >= 2 else set(),
    }


# --- Experiment 14: Graph family comparison ---

def family_aggregate(trajectory_map):
    """Aggregate metrics per family.
    
    trajectory_map: {family_name: [list of (rows, hamiltonian) tuples]}
    """
    families = {}
    for family_name, trajectories in trajectory_map.items():
        metrics = []
        for rows, _ in trajectories:
            metrics.append(graph_similarity_profile(rows))
        families[family_name] = {
            "mean_jaccard": np.mean([m["mean_jaccard"] for m in metrics]),
            "std_jaccard": np.std([m["mean_jaccard"] for m in metrics]),
            "mean_iterations": np.mean([m["n_iterations"] for m in metrics]),
            "std_iterations": np.std([m["n_iterations"] for m in metrics]),
            "count": len(metrics),
        }
    return families


if __name__ == "__main__":
    import sys, re, os
    if len(sys.argv) < 2:
        print("Usage: python3 analysis/compute_metrics.py <trajectory.ndjson> [trajectory2.ndjson ...]")
        print("Computes: vertex_jaccard, edge_jaccard, edge_hamming, assignment_hamming (+ %)")
        sys.exit(1)

    for path in sys.argv[1:]:
        if not path.endswith(".ndjson"):
            continue
        try:
            rows, hamiltonian = load_trajectory(path)
            if len(rows) < 2:
                print(f"{path}: only {len(rows)} iteration(s), skipping")
                continue

            # Try to get total vars from corresponding .log file
            log_path = path.replace(".ndjson", ".log")
            n_vars = None
            if os.path.exists(log_path):
                m = re.search(r"total variables:\s*(\d+)", open(log_path).read())
                if m:
                    n_vars = int(m.group(1))

            metrics = all_consecutive_metrics(rows, n_vars=n_vars)
            vj = np.mean([m["vertex_jaccard"] for m in metrics])
            ej = np.mean([m["edge_jaccard"] for m in metrics])
            eh = np.mean([m["edge_hamming"] for m in metrics])
            ah = np.mean([m["assignment_hamming"] for m in metrics])
            ap = np.mean([m["assignment_pct"] for m in metrics])
            ep = np.mean([m["edge_pct"] for m in metrics])
            name = path.split("/")[-1].replace("_seed0.ndjson", "")
            print(f"{name:45s} VJ={vj:.4f}  EJ={ej:.4f}  EH={eh:4.0f}({ep:.1f}%)  AH={ah:5.0f}({ap:.1f}%)  iters={len(rows)}")
        except Exception as e:
            print(f"{path}: ERROR {e}")
