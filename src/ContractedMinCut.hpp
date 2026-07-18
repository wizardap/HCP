#pragma once
#include <vector>
#include <queue>
#include <limits>
#include "Graph.hpp"
#include "SubtourDetector.hpp"

struct MinCutResult {
    std::vector<int> sideA_vertices; // original vertices on the source side of the min-cut
    int cutSize = 0;
};

// Dinic max-flow on adjacency list (sparse). Converted from n x n capacity matrix.
// O(E * sqrt(V)) on unit-capacity networks.
struct Dinic {
    struct Edge { int to, rev; int cap; };
    std::vector<std::vector<Edge>> g;
    std::vector<int> level, iter;
    Dinic(int n) : g(n), level(n), iter(n) {}
    void addEdge(int from, int to, int cap) {
        g[from].push_back({to, (int)g[to].size(), cap});
        g[to].push_back({from, (int)g[from].size() - 1, 0});
    }
    void bfs(int s) {
        level.assign(g.size(), -1);
        std::queue<int> q;
        level[s] = 0;
        q.push(s);
        while (!q.empty()) {
            int v = q.front(); q.pop();
            for (auto& e : g[v]) {
                if (e.cap > 0 && level[e.to] < 0) {
                    level[e.to] = level[v] + 1;
                    q.push(e.to);
                }
            }
        }
    }
    int dfs(int v, int t, int f) {
        if (v == t) return f;
        for (int& i = iter[v]; i < (int)g[v].size(); ++i) {
            Edge& e = g[v][i];
            if (e.cap > 0 && level[v] < level[e.to]) {
                int d = dfs(e.to, t, std::min(f, e.cap));
                if (d > 0) {
                    e.cap -= d;
                    g[e.to][e.rev].cap += d;
                    return d;
                }
            }
        }
        return 0;
    }
    int maxFlow(int s, int t) {
        int flow = 0;
        while (true) {
            bfs(s);
            if (level[t] < 0) break;
            iter.assign(g.size(), 0);
            int f;
            while ((f = dfs(s, t, std::numeric_limits<int>::max())) > 0) {
                flow += f;
            }
        }
        return flow;
    }
    // Returns vertices reachable from s in residual graph
    std::vector<bool> minCut(int s) {
        std::vector<bool> visited(g.size(), false);
        std::queue<int> q;
        q.push(s);
        visited[s] = true;
        while (!q.empty()) {
            int v = q.front(); q.pop();
            for (auto& e : g[v]) {
                if (e.cap > 0 && !visited[e.to]) {
                    visited[e.to] = true;
                    q.push(e.to);
                }
            }
        }
        return visited;
    }
};

// Given current subtour components and the original graph,
// build the contracted graph and find the minimum s-t cut
// between the smallest component and each neighbor.
// Returns the cut with the smallest cutSize across all neighbor pairs.
// Returns MinCutResult with empty sideA_vertices if no cut < total boundary found.
MinCutResult computeComponentMinCut(
    const std::vector<Component>& components,
    const Graph& graph
);

// Find the minimum edge cut within a single component.
// Used to split a large component when stagnation occurs at moderate count.
// Returns the cut edges (global edge indices) and the vertices on one side.
// Returns empty vectors if no useful cut found.
MinCutResult computeInternalMinCut(
    const Component& component,
    const Graph& graph,
    int maxFlowVertLimit = 2000
);

// Dinic max-flow wrapper — same signature as maxFlowBFS but uses Dinic's algorithm
// (O(E * sqrt(V)) on unit capacities) instead of Edmonds-Karp (O(V * E^2)).
int maxFlowDinic(int n, std::vector<std::vector<int>>& cap,
                 int s, int t, std::vector<bool>& minCutSide);
