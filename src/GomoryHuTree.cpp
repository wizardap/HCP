#include "GomoryHuTree.hpp"
#include "Graph.hpp"
#include "ContractedMinCut.hpp"
#include <algorithm>
#include <numeric>
#include <queue>

GomoryHuTree computeGomoryHuTree(
    const std::vector<Component>& components,
    const Graph& graph)
{
    GomoryHuTree result;
    int C = static_cast<int>(components.size());
    if (C < 2) return result;

    int n = graph.getNodes();

    // Map each vertex to its component index
    std::vector<int> vertToComp(n, -1);
    for (int ci = 0; ci < C; ++ci) {
        for (int v : components[ci].vertices) {
            if (v >= 0 && v < n) vertToComp[v] = ci;
        }
    }

    // Build contracted graph capacity matrix (C x C)
    std::vector<std::vector<int>> baseCap(C, std::vector<int>(C, 0));
    for (int u = 0; u < n; ++u) {
        int cu = vertToComp[u];
        if (cu < 0) continue;
        for (auto& [v, _] : graph.getNeighbors(u)) {
            int cv = vertToComp[v];
            if (cv < 0 || cv == cu) continue;
            baseCap[cu][cv]++;
        }
    }

    // Gomory-Hu tree algorithm:
    // tree[i] = parent of node i in the GH tree (tree[0] = -1, root)
    // treeWeight[i] = weight of edge from i to tree[i]
    std::vector<int> tree(C, 0);  // initially all nodes connected to node 0
    std::vector<int> treeWeight(C, 0);

    for (int i = 1; i < C; ++i) {
        int t = tree[i];  // current tree-neighbor of i

        // Run max-flow from i to t on contracted graph
        Dinic dinic(C);
        for (int u = 0; u < C; ++u) {
            for (int v = 0; v < C; ++v) {
                if (baseCap[u][v] > 0) {
                    dinic.addEdge(u, v, baseCap[u][v]);
                }
            }
        }

        int flowVal = dinic.maxFlow(i, t);
        auto sideI = dinic.minCut(i);  // vertices reachable from i in residual

        treeWeight[i] = flowVal;

        // Update tree pointers: for each j > i that is currently connected to t
        // and is on the same side as i, redirect j to point to i instead
        for (int j = i + 1; j < C; ++j) {
            if (tree[j] == t && sideI[j]) {
                tree[j] = i;
            }
        }
    }

    // Convert tree[] to GHEdge list, computing sideA for each edge
    // For each tree edge (i, tree[i]), sideA = subtree rooted at i
    // We compute this by BFS/DFS on the tree structure
    
    // Build tree adjacency
    std::vector<std::vector<int>> treeAdj(C);
    for (int i = 1; i < C; ++i) {
        treeAdj[i].push_back(tree[i]);
        treeAdj[tree[i]].push_back(i);
    }

    for (int i = 1; i < C; ++i) {
        GHEdge edge;
        edge.u = i;
        edge.v = tree[i];
        edge.cutWeight = treeWeight[i];

        // Find sideA = all nodes in the subtree rooted at i (when edge i-tree[i] is removed)
        std::vector<bool> visited(C, false);
        std::queue<int> q;
        q.push(i);
        visited[i] = true;
        while (!q.empty()) {
            int cur = q.front(); q.pop();
            edge.sideA.push_back(cur);
            for (int nb : treeAdj[cur]) {
                if (!visited[nb] && !(cur == i && nb == tree[i]) && !(cur == tree[i] && nb == i)) {
                    visited[nb] = true;
                    q.push(nb);
                }
            }
        }

        result.edges.push_back(std::move(edge));
    }

    // Sort by cutWeight ascending (weakest cuts first = highest priority)
    std::sort(result.edges.begin(), result.edges.end(),
              [](const GHEdge& a, const GHEdge& b) { return a.cutWeight < b.cutWeight; });

    return result;
}
