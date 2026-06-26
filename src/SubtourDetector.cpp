#include "SubtourDetector.hpp"
#include "Graph.hpp"
#include <vector>
#include <numeric>
#include <algorithm>

namespace {
    // Union-Find helper structure
    struct UnionFind {
        std::vector<int> parent;
        std::vector<int> rank;

        UnionFind(int n) : parent(n), rank(n, 0) {
            std::iota(parent.begin(), parent.end(), 0);
        }

        int find(int i) {
            if (parent[i] == i)
                return i;
            return parent[i] = find(parent[i]);
        }

        void unite(int i, int j) {
            int root_i = find(i);
            int root_j = find(j);
            if (root_i != root_j) {
                if (rank[root_i] < rank[root_j]) {
                    parent[root_i] = root_j;
                } else if (rank[root_i] > rank[root_j]) {
                    parent[root_j] = root_i;
                } else {
                    parent[root_j] = root_i;
                    rank[root_i]++;
                }
            }
        }
    };
} // namespace

std::vector<Component> SubtourDetector::detect(
    const std::vector<int>& model,
    const Graph& graph
) {
    int n = graph.getNodes();
    if (n <= 0) {
        return {};
    }

    UnionFind uf(n);
    for (int u = 0; u < n; ++u) {
        for (auto& [v, edgeVar] : graph.getNeighbors(u)) {
            if (edgeVar > 0 && edgeVar < static_cast<int>(model.size()) && model[edgeVar] > 0) {
                uf.unite(u, v);
            }
        }
    }

    // Group vertices by their root representative
    std::vector<std::vector<int>> groups(n);
    for (int i = 0; i < n; ++i) {
        groups[uf.find(i)].push_back(i);
    }

    // Create components and filter
    std::vector<Component> components;
    for (int i = 0; i < n; ++i) {
        if (!groups[i].empty()) {
            // Trivial component filtering: only keep components of size < graph.getNodes()
            if (groups[i].size() < static_cast<size_t>(n)) {
                components.push_back({std::move(groups[i])});
            }
        }
    }

    // Sort components by size (smallest first)
    std::sort(components.begin(), components.end());

    return components;
}
