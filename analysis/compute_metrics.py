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


# --- Experiment 11: Frequent pattern mining (simplified Apriori) ---

def frequent_vertex_patterns(rows, min_support=0.3):
    """Simple frequent pattern mining: find vertices co-occurring above threshold."""
    n = len(rows)
    threshold = min_support * n
    vertex_iterations = {}
    for idx, row in enumerate(rows):
        for comp in row["components"]:
            for v in comp["vertices"]:
                if v not in vertex_iterations:
                    vertex_iterations[v] = set()
                vertex_iterations[v].add(idx)

    # Pairs that co-occur above threshold
    vertices = list(vertex_iterations.keys())
    frequent_pairs = []
    for i, j in combinations(range(len(vertices)), 2):
        intersection = vertex_iterations[vertices[i]] & vertex_iterations[vertices[j]]
        if len(intersection) >= threshold:
            frequent_pairs.append((vertices[i], vertices[j], len(intersection)))
    return {"frequent_vertices": {v: len(s) for v, s in vertex_iterations.items() if len(s) >= threshold},
            "frequent_pairs": frequent_pairs}


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
