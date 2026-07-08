#include "ContractedMinCut.hpp"
#include <algorithm>
#include <limits>
#include <queue>
#include <vector>

// Edmonds-Karp max-flow on a small adjacency matrix (up to 64 nodes).
// capacity[u][v] = capacity of edge u->v.
// Returns max flow from s to t and fills minCutSide (true = side containing s).
static int maxFlowBFS(int n, std::vector<std::vector<int>>& cap,
                      int s, int t, std::vector<bool>& minCutSide) {
    int flow = 0;
    while (true) {
        // BFS to find augmenting path
        std::vector<int> parent(n, -1);
        std::queue<int> q;
        q.push(s);
        parent[s] = s;
        while (!q.empty() && parent[t] == -1) {
            int u = q.front(); q.pop();
            for (int v = 0; v < n; ++v) {
                if (parent[v] == -1 && cap[u][v] > 0) {
                    parent[v] = u;
                    q.push(v);
                }
            }
        }
        if (parent[t] == -1) break; // no augmenting path

        // Find bottleneck
        int bottleneck = std::numeric_limits<int>::max();
        for (int v = t; v != s; v = parent[v]) {
            int u = parent[v];
            bottleneck = std::min(bottleneck, cap[u][v]);
        }
        // Update capacities
        for (int v = t; v != s; v = parent[v]) {
            int u = parent[v];
            cap[u][v] -= bottleneck;
            cap[v][u] += bottleneck;
        }
        flow += bottleneck;
    }
    // Min-cut: BFS from s on residual graph
    minCutSide.assign(n, false);
    std::queue<int> q;
    q.push(s);
    minCutSide[s] = true;
    while (!q.empty()) {
        int u = q.front(); q.pop();
        for (int v = 0; v < n; ++v) {
            if (!minCutSide[v] && cap[u][v] > 0) {
                minCutSide[v] = true;
                q.push(v);
            }
        }
    }
    return flow;
}

MinCutResult computeComponentMinCut(
    const std::vector<Component>& components,
    const Graph& graph
) {
    int m = static_cast<int>(components.size());
    if (m < 2) return {};

    // Map each vertex to its component index
    int n = graph.getNodes();
    std::vector<int> vertToComp(n, -1);
    for (int ci = 0; ci < m; ++ci) {
        for (int v : components[ci].vertices) {
            if (v >= 0 && v < n) vertToComp[v] = ci;
        }
    }

    // Build contracted graph capacity matrix (m x m)
    std::vector<std::vector<int>> baseCap(m, std::vector<int>(m, 0));
    for (int u = 0; u < n; ++u) {
        int cu = vertToComp[u];
        if (cu < 0) continue;
        for (auto& [v, _] : graph.getNeighbors(u)) {
            int cv = vertToComp[v];
            if (cv < 0 || cv == cu) continue;
            baseCap[cu][cv]++;  // directed: count both directions for undirected
        }
    }
    // Make symmetric (undirected): already symmetric since we count both u->v and v->u

    // Find smallest component (source = s)
    int s = 0;
    for (int i = 1; i < m; ++i) {
        if (components[i].vertices.size() < components[s].vertices.size()) s = i;
    }

    MinCutResult best;
    best.cutSize = std::numeric_limits<int>::max();

    // Try each neighbor of s as sink t
    for (int t = 0; t < m; ++t) {
        if (t == s || baseCap[s][t] == 0) continue;

        // Copy capacity matrix (max-flow is destructive)
        auto cap = baseCap;
        std::vector<bool> sideA;
        int flowVal = maxFlowBFS(m, cap, s, t, sideA);

        if (flowVal < best.cutSize) {
            best.cutSize = flowVal;
            best.sideA_vertices.clear();
            for (int ci = 0; ci < m; ++ci) {
                if (sideA[ci]) {
                    for (int v : components[ci].vertices) {
                        best.sideA_vertices.push_back(v);
                    }
                }
            }
        }
    }

    if (best.cutSize == std::numeric_limits<int>::max()) return {};
    return best;
}
