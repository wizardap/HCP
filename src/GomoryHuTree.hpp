#pragma once
#include <vector>
#include "SubtourDetector.hpp"

class Graph;

struct GHEdge {
    int u, v;           // component indices
    int cutWeight;       // min s-t cut value in contracted graph
    std::vector<int> sideA;  // component indices on the source side of this cut
};

struct GomoryHuTree {
    std::vector<GHEdge> edges;  // C-1 edges, sorted by cutWeight ascending
};

// Compute Gomory-Hu tree on the contracted component graph.
// Each component becomes a super-node; edge weight = number of directed edges
// between components in the original graph.
// Returns tree with C-1 edges sorted by cutWeight ascending.
// Returns empty tree if components.size() < 2.
GomoryHuTree computeGomoryHuTree(
    const std::vector<Component>& components,
    const Graph& graph);
