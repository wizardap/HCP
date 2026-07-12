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

MinCutResult computeInternalMinCut(
    const Component& component,
    const Graph& graph,
    int maxFlowVertLimit
) {
    int k = static_cast<int>(component.vertices.size());
    if (k < 4 || k > maxFlowVertLimit) return {};

    // Map component vertices to local indices 0..k-1
    std::vector<int> globalToLocal(graph.getNodes(), -1);
    for (int i = 0; i < k; ++i) {
        int v = component.vertices[i];
        if (v >= 0 && v < static_cast<int>(graph.getNodes())) {
            globalToLocal[v] = i;
        }
    }

    // Local vertex -> global vertex
    auto& localToGlobal = component.vertices;

    // Build undirected capacity matrix for edges WITHIN the component
    std::vector<std::vector<int>> cap(k, std::vector<int>(k, 0));
    for (int vi = 0; vi < k; ++vi) {
        int u = localToGlobal[vi];
        if (u < 0) continue;
        for (auto& [v, _] : graph.getNeighbors(u)) {
            int vj = globalToLocal[v];
            if (vj >= 0 && vj != vi) {
                cap[vi][vj] = 1;  // unit capacity per directed edge
            }
        }
    }

    // Find boundary vertices: those with at least one neighbor OUTSIDE the component
    std::vector<int> boundary;
    for (int vi = 0; vi < k; ++vi) {
        int u = localToGlobal[vi];
        for (auto& [v, _] : graph.getNeighbors(u)) {
            int vj = globalToLocal[v];
            if (vj < 0) {  // neighbor is outside component
                boundary.push_back(vi);
                break;
            }
        }
    }

    if (boundary.size() < 2) return {};

    // Use the smallest boundary vertex as source, try each other as sink
    // (smallest = will be on one side of a good cut)
    int s = boundary[0];
    MinCutResult best;
    best.cutSize = std::numeric_limits<int>::max();

    for (size_t ti = 1; ti < boundary.size(); ++ti) {
        int t = boundary[ti];
        auto capCopy = cap;
        std::vector<bool> sideA_local;
        int flowVal = maxFlowBFS(k, capCopy, s, t, sideA_local);

        if (flowVal > 0 && flowVal < best.cutSize) {
            best.cutSize = flowVal;

            // Map sideA from local indices back to global vertex IDs
            best.sideA_vertices.clear();
            std::vector<bool> inComponent(graph.getNodes(), false);
            for (int vi = 0; vi < k; ++vi) {
                if (sideA_local[vi]) {
                    int gv = localToGlobal[vi];
                    best.sideA_vertices.push_back(gv);
                    inComponent[gv] = true;
                }
            }

            if (best.sideA_vertices.empty() ||
                static_cast<int>(best.sideA_vertices.size()) >= k) {
                best.cutSize = std::numeric_limits<int>::max();
                best.sideA_vertices.clear();
            }
        }
    }

    if (best.cutSize == std::numeric_limits<int>::max()) return {};
    return best;
}
