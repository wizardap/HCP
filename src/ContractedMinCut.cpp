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

// Dinic max-flow wrapper matching maxFlowBFS signature.
// Converts capacity matrix to adjacency list.
int maxFlowDinic(int n, std::vector<std::vector<int>>& cap,
                 int s, int t, std::vector<bool>& minCutSide) {
    Dinic dinic(n);
    for (int u = 0; u < n; ++u) {
        for (int v = 0; v < n; ++v) {
            if (cap[u][v] > 0) {
                dinic.addEdge(u, v, cap[u][v]);
            }
        }
    }
    int flow = dinic.maxFlow(s, t);
    minCutSide = dinic.minCut(s);
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

    auto& localToGlobal = component.vertices;

    // Build sparse capacity representation for edges WITHIN the component
    std::vector<std::vector<std::pair<int, int>>> localAdj(k);
    for (int vi = 0; vi < k; ++vi) {
        int u = localToGlobal[vi];
        if (u < 0 || u >= static_cast<int>(graph.getNodes())) continue;
        for (auto& [v, _] : graph.getNeighbors(u)) {
            int vj = globalToLocal[v];
            if (vj >= 0 && vj != vi) {
                localAdj[vi].push_back({vj, 1});
            }
        }
    }

    // Find boundary vertices
    std::vector<int> boundary;
    for (int vi = 0; vi < k; ++vi) {
        int u = localToGlobal[vi];
        if (u < 0 || u >= static_cast<int>(graph.getNodes())) continue;
        for (auto& [v, _] : graph.getNeighbors(u)) {
            int vj = globalToLocal[v];
            if (vj < 0) {
                boundary.push_back(vi);
                break;
            }
        }
    }

    if (boundary.size() < 2) return {};

    int s = boundary[0];
    MinCutResult best;
    best.cutSize = std::numeric_limits<int>::max();

    // Cap boundary sinks to at most 10
    size_t maxSinks = 10;
    size_t step = std::max<size_t>(1, (boundary.size() - 1) / maxSinks);

    for (size_t ti = 1; ti < boundary.size(); ti += step) {
        int t = boundary[ti];
        
        Dinic dinic(k);
        for (int u = 0; u < k; ++u) {
            for (auto& [v, capVal] : localAdj[u]) {
                dinic.addEdge(u, v, capVal);
            }
        }

        int flowVal = dinic.maxFlow(s, t);
        std::vector<bool> sideA_local = dinic.minCut(s);

        int sideA_count = 0;
        for (int vi = 0; vi < k; ++vi) {
            if (sideA_local[vi]) {
                sideA_count++;
            }
        }

        if (sideA_count > 0 && sideA_count < k) {
            if (flowVal > 0 && flowVal < best.cutSize) {
                best.cutSize = flowVal;
                best.sideA_vertices.clear();
                for (int vi = 0; vi < k; ++vi) {
                    if (sideA_local[vi]) {
                        best.sideA_vertices.push_back(localToGlobal[vi]);
                    }
                }
            }
        }
    }

    if (best.cutSize == std::numeric_limits<int>::max()) return {};
    return best;
}
