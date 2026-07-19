#include <iostream>
#include <cassert>
#include <vector>
#include <algorithm>
#include "Graph.hpp"
#include "SecEncoder.hpp"
#include "SubtourDetector.hpp"

int main() {
    // Construct a 4-node graph: 0-1-2-3-0
    Graph g(4, 8);
    // Directed edge indices matching Solver conventions:
    // 0 -> 1 (edge 1), 1 -> 0 (edge 2)
    // 1 -> 2 (edge 3), 2 -> 1 (edge 4)
    // 2 -> 3 (edge 5), 3 -> 2 (edge 6)
    // 3 -> 0 (edge 7), 0 -> 3 (edge 8)
    g.addEdge(0, 1, 1);
    g.addEdge(1, 0, 2);
    g.addEdge(1, 2, 3);
    g.addEdge(2, 1, 4);
    g.addEdge(2, 3, 5);
    g.addEdge(3, 2, 6);
    g.addEdge(3, 0, 7);
    g.addEdge(0, 3, 8);

    SecEncoder encoder(g);

    // Test 1: Component with 2 vertices {0, 1}
    Component comp1;
    comp1.vertices = {0, 1};

    auto clauses = encoder.encodeSecs({comp1});
    // Expected clauses:
    // 1. Outgoing cut: {8, 3} (from 0->3, 1->2)
    // 2. Incoming cut: {7, 4} (from 3->0, 2->1)
    // 3. Small-cycle DFJ: {-1, -2} (internal edges 0->1 and 1->0)
    assert(clauses.size() == 3);

    auto hasClause = [&](const std::vector<int>& expected) {
        for (const auto& cl : clauses) {
            if (cl.size() == expected.size()) {
                auto c1 = cl;
                auto c2 = expected;
                std::sort(c1.begin(), c1.end());
                std::sort(c2.begin(), c2.end());
                if (c1 == c2) return true;
            }
        }
        return false;
    };

    assert(hasClause({8, 3}));
    assert(hasClause({7, 4}));
    assert(hasClause({-1, -2}));

    // Test 2: Component with 4 vertices {0, 1, 2, 3} (|S| = 4 > 3)
    Component comp2;
    comp2.vertices = {0, 1, 2, 3};
    auto clauses2 = encoder.encodeSecs({comp2});
    // No outgoing, incoming, or DFJ clauses for full graph component
    assert(clauses2.empty());

    // Test 3: findInternalSubcuts behavior
    auto subcuts4 = encoder.findInternalSubcuts(comp2);
    assert(subcuts4.empty()); // size <= 4 returns empty

    Component comp5;
    comp5.vertices = {0, 1, 2, 3, 4, 5};
    Graph g6(6, 12);
    SecEncoder encoder6(g6);
    auto subcuts6 = encoder6.findInternalSubcuts(comp5);
    assert(subcuts6.size() == 2);
    assert(subcuts6[0].vertices.size() == 3);
    assert(subcuts6[1].vertices.size() == 3);

    std::cout << "SecEncoder tests passed successfully!\n";
    return 0;
}
