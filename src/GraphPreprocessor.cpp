#include "GraphPreprocessor.hpp"
#include <algorithm>
#include <vector>
#include <queue>
#include <functional>
#include <tuple>

GraphPreprocessor::GraphPreprocessor(const Graph& g) : graph_(g) {
    compute();
}

bool GraphPreprocessor::hasBridge() const {
    return hasBridge_;
}

const std::vector<EdgePair>& GraphPreprocessor::getTwoEdgeCuts() const {
    return twoEdgeCuts_;
}

const std::vector<int>& GraphPreprocessor::getDegree2Vertices() const {
    return degree2Vertices_;
}

void GraphPreprocessor::compute() {
    int n = graph_.getNodes();

    // --- Degree-2 vertices ---
    for (int u = 0; u < n; ++u) {
        if (graph_.getNeighbors(u).size() == 2) {
            degree2Vertices_.push_back(u);
        }
    }

    // --- Base bridge detection (no edge removed) ---
    {
        std::vector<std::pair<int, int>> bridges;
        findBridges(-1, -1, bridges);
        if (!bridges.empty()) {
            hasBridge_ = true;
            return; // No HC possible; skip 2-edge-cut detection
        }
    }

    // --- 2-edge-cut detection ---
    // O(E) per edge — bail out for large graphs
    if (graph_.getEdges() > 10000) {
        std::cerr << "c Preprocessing: graph has " << graph_.getEdges()
                  << " edges, skipping expensive 2-edge-cut detection\n";
        return;
    }
    for (int u = 0; u < n; ++u) {
        for (auto& [v, _] : graph_.getNeighbors(u)) {
            if (v <= u) continue; // undirected: process each edge once

            // Remove {u,v} temporarily and find bridges in remaining graph
            std::vector<std::pair<int, int>> bridges;
            findBridges(u, v, bridges);
            for (auto& [bu, bv] : bridges) {
                if ((bu == u && bv == v) || (bu == v && bv == u)) continue; // skip self

                // {u,v} and {bu,bv} form a 2-edge-cut.
                // Determine sides A and B by BFS from u without both edges.
                std::vector<bool> visited(n, false);
                std::queue<int> q;
                q.push(u);
                visited[u] = true;
                while (!q.empty()) {
                    int curr = q.front();
                    q.pop();
                    for (auto& [neigh, _] : graph_.getNeighbors(curr)) {
                        // Skip removed edges
                        if ((curr == u && neigh == v) || (curr == v && neigh == u)) continue;
                        if ((curr == bu && neigh == bv) || (curr == bv && neigh == bu)) continue;
                        if (!visited[neigh]) {
                            visited[neigh] = true;
                            q.push(neigh);
                        }
                    }
                }

                EdgePair ep;
                ep.u1 = u;
                ep.v1 = v;
                if (visited[bu]) {
                    ep.u2 = bu;
                    ep.v2 = bv;
                } else {
                    ep.u2 = bv;
                    ep.v2 = bu;
                }
                twoEdgeCuts_.push_back(ep);
            }
        }
    }

    // Canonicalize each EdgePair to ensure unique representative
    for (auto& ep : twoEdgeCuts_) {
        EdgePair candidates[4];
        candidates[0] = ep;

        candidates[1] = ep;
        std::swap(candidates[1].u1, candidates[1].v1);
        std::swap(candidates[1].u2, candidates[1].v2);

        candidates[2] = ep;
        std::swap(candidates[2].u1, candidates[2].u2);
        std::swap(candidates[2].v1, candidates[2].v2);

        candidates[3] = ep;
        std::swap(candidates[3].u1, candidates[3].v1);
        std::swap(candidates[3].u2, candidates[3].v2);
        std::swap(candidates[3].u1, candidates[3].u2);
        std::swap(candidates[3].v1, candidates[3].v2);

        auto cmp = [](const EdgePair& a, const EdgePair& b) {
            return std::tie(a.u1, a.v1, a.u2, a.v2) < std::tie(b.u1, b.v1, b.u2, b.v2);
        };
        ep = *std::min_element(candidates, candidates + 4, cmp);
    }

    // Deduplicate
    std::sort(twoEdgeCuts_.begin(), twoEdgeCuts_.end(), [](const EdgePair& a, const EdgePair& b) {
        return std::tie(a.u1, a.v1, a.u2, a.v2) < std::tie(b.u1, b.v1, b.u2, b.v2);
    });
    twoEdgeCuts_.erase(std::unique(twoEdgeCuts_.begin(), twoEdgeCuts_.end(), [](const EdgePair& a, const EdgePair& b) {
        return a.u1 == b.u1 && a.v1 == b.v1 && a.u2 == b.u2 && a.v2 == b.v2;
    }), twoEdgeCuts_.end());
}

void GraphPreprocessor::findBridges(int skipU, int skipV,
                                    std::vector<std::pair<int, int>>& out_bridges) const {
    int n = graph_.getNodes();
    std::vector<int> disc(n, -1), low(n, -1), parent(n, -1);
    int timer = 0;

    std::function<void(int)> dfs = [&](int u) {
        disc[u] = low[u] = timer++;
        for (auto& [v, _] : graph_.getNeighbors(u)) {
            // Skip the removed edge {skipU, skipV} in both directions
            if ((u == skipU && v == skipV) || (u == skipV && v == skipU)) continue;
            if (disc[v] == -1) {
                parent[v] = u;
                dfs(v);
                low[u] = std::min(low[u], low[v]);
                if (low[v] > disc[u]) {
                    // {u,v} is a bridge
                    int bu = std::min(u, v), bv = std::max(u, v);
                    out_bridges.push_back({bu, bv});
                }
            } else if (v != parent[u]) {
                low[u] = std::min(low[u], disc[v]);
            }
        }
    };

    for (int i = 0; i < n; ++i) {
        if (disc[i] == -1) {
            dfs(i);
        }
    }
}
