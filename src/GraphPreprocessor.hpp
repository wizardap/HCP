#pragma once
#include <vector>
#include <utility>
#include "Graph.hpp"

struct EdgePair {
    int u1, v1; // first undirected edge endpoints
    int u2, v2; // second undirected edge endpoints
    // side A contains u1 and u2 after removing both edges
};

class GraphPreprocessor {
public:
    explicit GraphPreprocessor(const Graph& g);

    // Returns true if graph has a bridge (no HC possible).
    bool hasBridge() const;

    // Returns all 2-edge-cuts found. Empty if none.
    const std::vector<EdgePair>& getTwoEdgeCuts() const;

    // Returns all degree-2 vertices.
    const std::vector<int>& getDegree2Vertices() const;

private:
    const Graph& graph_;
    bool hasBridge_ = false;
    std::vector<EdgePair> twoEdgeCuts_;
    std::vector<int> degree2Vertices_;

    void compute();
    // Tarjan bridge-finding DFS on graph with one undirected edge (skipU,skipV) disabled.
    // Appends discovered bridges to out_bridges.
    void findBridges(int skipU, int skipV,
                     std::vector<std::pair<int,int>>& out_bridges) const;
};
