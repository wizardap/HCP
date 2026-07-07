"""Visualization functions for all 14 experiments."""

import numpy as np
import matplotlib
matplotlib.use("Agg")
import matplotlib.pyplot as plt
import seaborn as sns
from scipy.cluster.hierarchy import dendrogram
import os


def plot_vertex_frequency(counts, n_vertices, output_path):
    fig, ax = plt.subplots(figsize=(12, 4))
    ax.bar(range(n_vertices), counts, width=0.8)
    ax.set_xlabel("Vertex ID")
    ax.set_ylabel("Frequency (iterations)")
    ax.set_title("Vertex Frequency in Subtour Components")
    fig.tight_layout()
    fig.savefig(output_path, dpi=150)
    plt.close(fig)


def plot_edge_frequency(counts, n_edges, output_path):
    fig, ax = plt.subplots(figsize=(12, 4))
    ax.bar(range(1, n_edges + 1), counts, width=0.8)
    ax.set_xlabel("Edge Variable ID")
    ax.set_ylabel("Frequency (iterations)")
    ax.set_title("Edge Variable Frequency in SAT Model")
    fig.tight_layout()
    fig.savefig(output_path, dpi=150)
    plt.close(fig)


def plot_consecutive_jaccard(jac_values, output_path):
    fig, ax = plt.subplots(figsize=(10, 4))
    ax.plot(range(1, len(jac_values) + 1), jac_values, marker=".", linestyle="-", alpha=0.7)
    ax.axhline(y=np.mean(jac_values), color="r", linestyle="--", label=f"Mean: {np.mean(jac_values):.3f}")
    ax.set_xlabel("Iteration pair (i, i+1)")
    ax.set_ylabel("Jaccard Similarity")
    ax.set_title("Consecutive Subtour Jaccard Similarity")
    ax.legend()
    fig.tight_layout()
    fig.savefig(output_path, dpi=150)
    plt.close(fig)


def plot_jaccard_matrix(mat, output_path):
    fig, ax = plt.subplots(figsize=(8, 7))
    im = ax.imshow(mat, cmap="viridis", vmin=0, vmax=1)
    ax.set_xlabel("Iteration")
    ax.set_ylabel("Iteration")
    ax.set_title("Pairwise Jaccard Similarity Matrix")
    plt.colorbar(im, ax=ax, shrink=0.8)
    fig.tight_layout()
    fig.savefig(output_path, dpi=150)
    plt.close(fig)


def plot_core_sizes(sizes, output_path):
    fig, ax = plt.subplots(figsize=(8, 4))
    ks = np.arange(1, len(sizes) + 1)
    ax.plot(ks, sizes, marker=".", linestyle="-")
    ax.set_xlabel("Frequency threshold k")
    ax.set_ylabel("Core size (vertices)")
    ax.set_title("Persistent Core Size vs. Frequency Threshold")
    ax.set_yscale("log" if max(sizes) > 50 else "linear")
    fig.tight_layout()
    fig.savefig(output_path, dpi=150)
    plt.close(fig)


def plot_solver_trajectory(traj, output_path):
    fig, axes = plt.subplots(3, 1, figsize=(10, 8), sharex=True)
    iters = np.arange(1, len(traj["solve_times"]) + 1)

    axes[0].plot(iters, traj["max_component_size"], marker=".", linestyle="-")
    axes[0].set_ylabel("Max component size")
    axes[0].set_title("Solver Trajectory")

    axes[1].plot(iters, traj["num_components"], marker=".", linestyle="-", color="orange")
    axes[1].set_ylabel("Num components")

    axes[2].plot(iters, traj["solve_times"], marker=".", linestyle="-", color="green")
    axes[2].set_xlabel("Iteration")
    axes[2].set_ylabel("Solve time (s)")

    fig.tight_layout()
    fig.savefig(output_path, dpi=150)
    plt.close(fig)


def plot_family_comparison(family_data, output_path):
    names = list(family_data.keys())
    means = [family_data[n]["mean_jaccard"] for n in names]
    stds = [family_data[n]["std_jaccard"] for n in names]

    fig, ax = plt.subplots(figsize=(10, 5))
    ax.bar(names, means, yerr=stds, capsize=5)
    ax.set_xlabel("Graph Family")
    ax.set_ylabel("Mean Consecutive Jaccard")
    ax.set_title("Similarity Across Graph Families")
    ax.tick_params(axis="x", rotation=45)
    fig.tight_layout()
    fig.savefig(output_path, dpi=150)
    plt.close(fig)
