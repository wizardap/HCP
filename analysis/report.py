"""Statistical report generation."""

import numpy as np
from scipy.stats import ttest_1samp, chi2_contingency, mannwhitneyu, kruskal, spearmanr
from scipy.spatial.distance import pdist


def hypothesis_test_uniformity(frequencies, alpha=0.01):
    """Chi-squared test for uniform vertex frequency (Exp 1 H0)."""
    expected = np.full_like(frequencies, np.mean(frequencies), dtype=float)
    # Exclude zeros to avoid division issues
    mask = expected > 0
    if np.sum(mask) < 2:
        return {"statistic": float("nan"), "p_value": 1.0, "reject_H0": False}
    stat, p = chi2_contingency([frequencies[mask], expected[mask]])[0:2]
    return {"statistic": float(stat), "p_value": float(p), "reject_H0": bool(p < alpha)}


def hypothesis_test_jaccard(jac_values, baseline=0.1, alpha=0.01):
    """One-sample t-test for Jaccard > baseline (Exp 3 H0)."""
    stat, p = ttest_1samp(jac_values, baseline, alternative="greater")
    n = len(jac_values)
    d = (np.mean(jac_values) - baseline) / (np.std(jac_values, ddof=1) + 1e-10)
    return {"t_statistic": float(stat), "p_value": float(p),
            "cohens_d": float(d), "reject_H0": bool(p < alpha),
            "mean": float(np.mean(jac_values)), "n": n}


def hypothesis_test_edge_asymmetry(transitions, alpha=0.01):
    """Sign test for edge edit asymmetry (Exp 8 H0)."""
    net = np.array([t["added"] - t["removed"] for t in transitions])
    n_pos = np.sum(net > 0)
    n_neg = np.sum(net < 0)
    n_total = n_pos + n_neg
    if n_total == 0:
        return {"p_value": 1.0, "reject_H0": False, "median_net": 0.0}
    from scipy.stats import binom_test
    p = binom_test(min(n_pos, n_neg), n_total, 0.5, alternative="two-sided")
    return {"p_value": float(p), "reject_H0": bool(p < alpha),
            "n_pos": int(n_pos), "n_neg": int(n_neg), "median_net": float(np.median(net))}


def hypothesis_test_correlation(component_sizes, solve_times, alpha=0.01):
    """Spearman correlation between max component size and solve time (Exp 9 H0)."""
    rho, p = spearmanr(component_sizes, solve_times)
    return {"spearman_rho": float(rho), "p_value": float(p),
            "reject_H0": bool(p < alpha and abs(rho) > 0.3)}


def generate_report(rows, graph_name, n_vertices, n_edges, output_path):
    """Generate full statistical report for one graph."""
    from compute_metrics import (
        vertex_frequency, edge_frequency, consecutive_jaccard,
        edge_transitions, solver_trajectory, core_sizes_by_threshold
    )

    lines = []
    lines.append(f"=== Subtour Trajectory Report: {graph_name} ===")
    lines.append(f"Iterations: {len(rows)}")
    lines.append(f"Vertices: {n_vertices}, Edges: {n_edges}")
    lines.append("")

    # Exp 1: Vertex frequency
    vf = vertex_frequency(rows, n_vertices)
    gini = 1 - np.sum((vf / (np.sum(vf) + 1e-10)) ** 2) * 2 / (1 - 1 / len(vf))
    h1 = hypothesis_test_uniformity(vf)
    lines.append(f"Exp 1 - Vertex Frequency: Gini={gini:.3f}, Chi2 reject H0={h1['reject_H0']}")
    lines.append(f"  Top 5 vertices: {np.argsort(vf)[-5:][::-1].tolist()}")

    # Exp 3: Consecutive Jaccard
    jac = consecutive_jaccard(rows)
    h3 = hypothesis_test_jaccard(jac)
    lines.append(f"Exp 3 - Consecutive Jaccard: mean={h3['mean']:.3f}, d={h3['cohens_d']:.2f}, reject H0={h3['reject_H0']}")

    # Exp 5: Core sizes
    core_sizes = core_sizes_by_threshold(rows, n_vertices, max_k=min(10, len(rows)))
    lines.append(f"Exp 5 - Core sizes (k=1..{len(core_sizes)}): {core_sizes.tolist()}")

    # Exp 8: Edge transitions
    trans = edge_transitions(rows)
    if trans:
        h8 = hypothesis_test_edge_asymmetry(trans)
        lines.append(f"Exp 8 - Edge transitions: median_net={h8['median_net']:.1f}, reject H0={h8['reject_H0']}")
        lines.append(f"  Added: {np.mean([t['added'] for t in trans]):.1f}/iter, Removed: {np.mean([t['removed'] for t in trans]):.1f}/iter")

    # Exp 9: Solver trajectory
    traj = solver_trajectory(rows)
    if len(rows) > 2:
        h9 = hypothesis_test_correlation(traj["max_component_size"][1:], traj["solve_times"][1:])
        lines.append(f"Exp 9 - Max component vs solve time: Spearman rho={h9['spearman_rho']:.3f}, reject H0={h9['reject_H0']}")

    lines.append("")
    lines.append("=" * 50)

    with open(output_path, "w") as f:
        f.write("\n".join(lines))
    return "\n".join(lines)
