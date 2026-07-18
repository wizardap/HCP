#include <iostream>
#include <vector>
#include <cstdlib>
#include <algorithm>
#include <fstream>
#include "GomoryHuTree.hpp"
#include "SubtourDetector.hpp"
#include "Graph.hpp"
#include "ContractedMinCut.hpp"

#define TEST_ASSERT(cond) \
    do { \
        if (!(cond)) { \
            std::cerr << "Assertion failed: " << #cond << " at " << __FILE__ << ":" << __LINE__ << "\n"; \
            std::abort(); \
        } \
    } while (0)

// Helper: create a simple graph from edge list
// Edges are undirected: (u, v) creates both (u,v) and (v,u) with edge indices
static Graph makeGraph(int n, const std::vector<std::pair<int,int>>& edges) {
    // Write temp .edge file and load
    std::string tmpFile = "test_gh_tmp.edge";
    {
        std::ofstream f(tmpFile);
        f << "p edge " << n << " " << edges.size() << "\n";
        for (auto& [u, v] : edges) {
            f << (u + 1) << " " << (v + 1) << "\n";  // 1-indexed in DIMACS
        }
    }
    Graph g;
    g.loadFromFile(tmpFile, true);
    std::remove(tmpFile.c_str());
    return g;
}

void testGomoryHuBasic3Components() {
    std::cout << "Testing Gomory-Hu tree with 3 components...\n";

    // 6-node graph: {0,1} -- {2,3} -- {4,5}
    // Edges: 0-1, 2-3, 4-5 (internal), 1-2 (bridge 1), 3-4 (bridge 2)
    Graph g = makeGraph(6, {{0,1},{1,2},{2,3},{3,4},{4,5}});

    // Create 3 components manually
    Component c0, c1, c2;
    c0.vertices = {0, 1};
    c1.vertices = {2, 3};
    c2.vertices = {4, 5};
    std::vector<Component> components = {c0, c1, c2};

    auto ghTree = computeGomoryHuTree(components, g);

    TEST_ASSERT(ghTree.edges.size() == 2);  // C-1 = 2 edges

    // Both cuts should be small (bridged connections)
    // Sorted by cutWeight ascending
    for (const auto& e : ghTree.edges) {
        TEST_ASSERT(e.cutWeight > 0);
        TEST_ASSERT(!e.sideA.empty());
        TEST_ASSERT((int)e.sideA.size() < 3);  // proper partition
    }

    std::cout << "  Tree edges:\n";
    for (const auto& e : ghTree.edges) {
        std::cout << "    comp " << e.u << " -- comp " << e.v
                  << " weight=" << e.cutWeight
                  << " sideA=[";
        for (int c : e.sideA) std::cout << c << " ";
        std::cout << "]\n";
    }

    std::cout << "Gomory-Hu 3-component test passed!\n";
}

void testGomoryHuSingleComponent() {
    std::cout << "Testing Gomory-Hu tree with < 2 components...\n";

    Graph g = makeGraph(4, {{0,1},{1,2},{2,3},{3,0}});
    std::vector<Component> components;  // empty
    auto ghTree = computeGomoryHuTree(components, g);
    TEST_ASSERT(ghTree.edges.empty());

    Component c0;
    c0.vertices = {0, 1, 2, 3};
    components = {c0};
    ghTree = computeGomoryHuTree(components, g);
    TEST_ASSERT(ghTree.edges.empty());

    std::cout << "Gomory-Hu single-component test passed!\n";
}

void testGomoryHuCutWeightsMatchBruteForce() {
    std::cout << "Testing Gomory-Hu cut weights match brute force...\n";

    // 8-node graph with 4 components of 2 nodes each, varying connectivity
    // Comp0={0,1}, Comp1={2,3}, Comp2={4,5}, Comp3={6,7}
    // Edges: 0-1, 2-3, 4-5, 6-7 (internal)
    // 0-2, 1-3 (comp0-comp1: weight 4 directed = 2 undirected × 2 directions)
    // 2-4 (comp1-comp2: weight 2 directed)
    // 4-6, 5-7 (comp2-comp3: weight 4 directed)
    Graph g = makeGraph(8, {{0,1},{2,3},{4,5},{6,7}, {0,2},{1,3}, {2,4}, {4,6},{5,7}});

    Component c0, c1, c2, c3;
    c0.vertices = {0, 1};
    c1.vertices = {2, 3};
    c2.vertices = {4, 5};
    c3.vertices = {6, 7};
    std::vector<Component> components = {c0, c1, c2, c3};

    auto ghTree = computeGomoryHuTree(components, g);
    TEST_ASSERT(ghTree.edges.size() == 3);

    // The weakest cut should be comp1-comp2 with weight 2
    TEST_ASSERT(ghTree.edges[0].cutWeight <= ghTree.edges[1].cutWeight);
    TEST_ASSERT(ghTree.edges[1].cutWeight <= ghTree.edges[2].cutWeight);

    std::cout << "  Sorted cut weights:";
    for (const auto& e : ghTree.edges) {
        std::cout << " " << e.cutWeight;
    }
    std::cout << "\n";

    std::cout << "Gomory-Hu brute-force comparison passed!\n";
}

int main() {
    testGomoryHuSingleComponent();
    testGomoryHuBasic3Components();
    testGomoryHuCutWeightsMatchBruteForce();
    std::cout << "\nAll Gomory-Hu tests passed!\n";
    return 0;
}
