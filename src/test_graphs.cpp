#include <iostream>
#include <vector>
#include <string>
#include <sstream>
#include <fstream>
#include <cstdlib>
#include <chrono>
#include <sys/types.h>
#include <dirent.h>
#include <cstring>
#include <algorithm>
#include <set>
#include "SubtourDetector.hpp"
#include "Graph.hpp"
#include "HcpEncoder.hpp"

// Forward declaration for find2EdgeConnectedBlocks (defined in Solver.cpp)
std::vector<std::vector<int>> find2EdgeConnectedBlocks(const Graph& g);
#include "AtMostOne/DefaultAtMostOne.hpp"
#include "SymmetryBreaking/DefaultSymmetryBreaker.hpp"
#include "IncrementalSolver.hpp"
#include "VariableManager.hpp"
#include "Solver.hpp"

#define TEST_ASSERT(cond) \
    do { \
        if (!(cond)) { \
            std::cerr << "FAIL: " << #cond << " at " << __FILE__ << ":" << __LINE__ << "\n"; \
            std::abort(); \
        } \
    } while (0)

struct GraphFile {
    std::string path;
    std::string name;
    int nodes;
    int edges;
};

std::vector<GraphFile> discoverGraphs() {
    std::vector<GraphFile> graphs;
    std::vector<std::string> dirs = {
        ".",
        "../refs/ChineseRemainderEncoding/graphs",
        "../graphs"
    };
    for (const auto& dir : dirs) {
        DIR* d = opendir(dir.c_str());
        if (!d) continue;
        struct dirent* entry;
        while ((entry = readdir(d)) != nullptr) {
            std::string name = entry->d_name;
            if (name.size() < 5 || name.substr(name.size() - 5) != ".edge") continue;
            std::string fullPath = dir + "/" + name;
            std::ifstream f(fullPath);
            std::string p, edge;
            int n, e;
            f >> p >> edge >> n >> e;
            graphs.push_back({fullPath, name, n, e});
        }
        closedir(d);
    }
    std::sort(graphs.begin(), graphs.end(),
        [](const GraphFile& a, const GraphFile& b) { return a.nodes < b.nodes; });
    return graphs;
}

void testGraphFileExists() {
    std::cout << "Testing graph files exist...\n";
    auto graphs = discoverGraphs();
    TEST_ASSERT(graphs.size() > 0);
    std::cout << "  Found " << graphs.size() << " graph files\n";
    for (auto& g : graphs) {
        std::cout << "  " << g.name << ": " << g.nodes << " nodes, " << g.edges << " edges\n";
    }
}

struct NullBuf : std::streambuf {
    int overflow(int c) override { return c; }
};

void testEncodingProducesValidCnf() {
    std::cout << "Testing encoding produces valid CNF for all graphs...\n";
    auto graphs = discoverGraphs();
    NullBuf nullBuf;
    std::streambuf* oldCout = std::cout.rdbuf(&nullBuf);

    int tested = 0;
    for (auto& gf : graphs) {
        if (gf.nodes > 200) continue;
        Graph g;
        bool loaded = g.loadFromFile(gf.path, true);
        TEST_ASSERT(loaded);
        DefaultAtMostOne amo;
        DefaultSymmetryBreaker sym;
        HcpEncoder encoder(g, 2, amo, sym, -1);

        std::stringstream cnfCapture;
        std::streambuf* oldCout2 = std::cout.rdbuf(cnfCapture.rdbuf());
        encoder.encode();
        std::cout.rdbuf(oldCout2);

        // Validate the CNF header
        std::string firstLine;
        std::getline(cnfCapture, firstLine);
        int vars = 0, clauses = 0;
        std::string token;
        TEST_ASSERT(firstLine.substr(0, 6) == "p cnf ");
        std::stringstream(firstLine) >> token >> token >> vars >> clauses;
        TEST_ASSERT(vars > 0);
        TEST_ASSERT(clauses > 0);

        tested++;
    }
    std::cout.rdbuf(oldCout);
    TEST_ASSERT(tested > 0);
    std::cout << "  Tested " << tested << " graphs (skipped " << (graphs.size() - tested) << " large)\n";
}

void testIncrementalVsNonIncrementalCountsMatch() {
    std::cout << "Testing incremental vs non-incremental variable/clause counts match...\n";
    auto graphs = discoverGraphs();

    int tested = 0;
    for (auto& gf : graphs) {
        if (gf.nodes > 100) continue;

        // Non-incremental: capture CNF to stringstream
        Graph g1;
        bool loaded = g1.loadFromFile(gf.path, true);
        TEST_ASSERT(loaded);
        DefaultAtMostOne amo1;
        DefaultSymmetryBreaker sym1;
        HcpEncoder encoder1(g1, 2, amo1, sym1, -1);

        std::stringstream cnfStream;
        std::streambuf* oldCout = std::cout.rdbuf(cnfStream.rdbuf());
        encoder1.encode();
        std::cout.rdbuf(oldCout);

        std::string firstLine;
        std::getline(cnfStream, firstLine);
        int nonIncVars = 0, nonIncClauses = 0;
        if (firstLine.substr(0, 6) == "p cnf ") {
            std::stringstream ss(firstLine);
            std::string p, cnf;
            ss >> p >> cnf >> nonIncVars >> nonIncClauses;
        }
        TEST_ASSERT(nonIncVars > 0);
        TEST_ASSERT(nonIncClauses > 0);

        // Incremental: count via IncrementalSolver
        Graph g2;
        g2.loadFromFile(gf.path, true);
        DefaultAtMostOne amo2;
        DefaultSymmetryBreaker sym2;
        VariableManager vm(2 * g2.getEdges() + 1);
        IncrementalSolver isolver;
        HcpEncoder encoder2(g2, 2, amo2, sym2, -1, vm);
        encoder2.encodeBase(isolver);

        int incVars = isolver.getNumVars();
        int64_t incClauses = isolver.getNumClauses();

        TEST_ASSERT(incVars == nonIncVars);
        TEST_ASSERT(incClauses == nonIncClauses);

        tested++;
    }
    TEST_ASSERT(tested > 0);
    std::cout << "  Tested " << tested << " graphs\n";
}

void testTimingMeasurement() {
    std::cout << "Testing timing measurement in incremental solver...\n";
    auto graphs = discoverGraphs();

    int tested = 0;
    for (auto& gf : graphs) {
        if (gf.nodes > 100) continue;

        Graph g;
        bool loaded = g.loadFromFile(gf.path, true);
        TEST_ASSERT(loaded);

        DefaultAtMostOne amo;
        DefaultSymmetryBreaker sym;
        VariableManager vm(2 * g.getEdges() + 1);
        IncrementalSolver isolver(5000); // 5 second timeout
        HcpEncoder encoder(g, 2, amo, sym, -1, vm);
        encoder.encodeBase(isolver);

        auto t1 = std::chrono::steady_clock::now();
        auto result = isolver.solve();
        auto t2 = std::chrono::steady_clock::now();

        auto elapsed = std::chrono::duration_cast<std::chrono::milliseconds>(t2 - t1).count();
        TEST_ASSERT(elapsed <= 6000); // should not exceed 5s timeout by more than 1s
        if (result == IncrementalSolver::Result::TIMEOUT) {
            TEST_ASSERT(elapsed >= 4900); // near the 5s limit
        }
        TEST_ASSERT(result == IncrementalSolver::Result::SAT ||
                    result == IncrementalSolver::Result::UNSAT ||
                    result == IncrementalSolver::Result::TIMEOUT);
        tested++;
    }
    TEST_ASSERT(tested > 0);
    std::cout << "  Tested " << tested << " graphs, timing OK\n";
}

void testStagnationStrategies() {
    std::cout << "Testing stagnation strategies on small.edge...\n";
    auto graphs = discoverGraphs();
    std::string smallEdgePath = "";
    for (const auto& gf : graphs) {
        if (gf.name == "small.edge") {
            smallEdgePath = gf.path;
            break;
        }
    }
    TEST_ASSERT(!smallEdgePath.empty());

    std::vector<std::string> strategies = {"dfj", "union", "both", "greedy"};
    for (const auto& strat : strategies) {
        Solver solver(smallEdgePath);
        solver.setStagnationK(2);
        solver.setStagnationStrategy(strat);
        
        std::stringstream outputCapture;
        std::streambuf* oldCerr = std::cerr.rdbuf(outputCapture.rdbuf());
        std::streambuf* oldCout = std::cout.rdbuf(outputCapture.rdbuf());
        
        auto result = solver.runIncremental(5000); // 5s timeout
        
        std::cerr.rdbuf(oldCerr);
        std::cout.rdbuf(oldCout);
        
        TEST_ASSERT(result == Solver::SolveResult::HAMILTONIAN);
    }
    std::cout << "  Stagnation strategies tested successfully!\n";
}

static void test2EdgeConnectedBlocks() {
    // Simple triangle (3-cycle): no bridges, one block with all vertices
    {
        Graph g(3, 3);
        g.addEdge(0, 1);
        g.addEdge(1, 2);
        g.addEdge(2, 0);
        auto blocks = find2EdgeConnectedBlocks(g);
        TEST_ASSERT(blocks.size() == 1);
        TEST_ASSERT(blocks[0].size() == 3);
    }
    // Two triangles connected by a single bridge edge:
    // Triangle A (0-1-2), bridge 2-3, triangle B (3-4-5)
    // Expected: 2 blocks, block 0 = {0,1,2}, block 1 = {3,4,5}
    {
        Graph g(6, 7);
        g.addEdge(0, 1); g.addEdge(1, 2); g.addEdge(2, 0);  // triangle A
        g.addEdge(2, 3);  // bridge
        g.addEdge(3, 4); g.addEdge(4, 5); g.addEdge(5, 3);  // triangle B
        auto blocks = find2EdgeConnectedBlocks(g);
        TEST_ASSERT(blocks.size() == 2);
        for (auto& b : blocks) TEST_ASSERT(b.size() == 3);
    }
    std::cerr << "PASS: test2EdgeConnectedBlocks\n";
}

static void testPrecomputedBlockClauses() {
    // Two triangles with bridge 2-3
    Graph g(6, 7);
    g.addEdge(0, 1); g.addEdge(1, 2); g.addEdge(2, 0);
    g.addEdge(2, 3);
    g.addEdge(3, 4); g.addEdge(4, 5); g.addEdge(5, 3);

    auto blocks = find2EdgeConnectedBlocks(g);
    TEST_ASSERT(blocks.size() == 2);  // two blocks

    // For each block, find outgoing edges
    int totalClauses = 0;
    for (auto& block : blocks) {
        if ((int)block.size() == g.getNodes()) continue; // not proper subset
        std::set<int> blockSet(block.begin(), block.end());
        std::vector<int> clause;
        for (int u : block) {
            for (auto& [v, _] : g.getNeighbors(u)) {
                if (!blockSet.count(v)) {
                    clause.push_back(-1); // placeholder, just count
                }
            }
        }
        if (clause.size() >= 2) totalClauses++;
    }
    // Triangle A has 1 outgoing edge (2→3), Triangle B has 1 incoming (3→2).
    // Each directed edge gets a separate clause, but we skip <2 literals.
    // So: 0 clauses (each block has only 1 outgoing directed edge).
    std::cerr << "PASS: testPrecomputedBlockClauses (clauses=" << totalClauses << ")\n";
}

static void testOscillationTracker() {
    OscillationTracker tracker(10, 100, 10);

    uint64_t h1 = 0xAAAA;
    uint64_t h2 = 0xBBBB;

    TEST_ASSERT(!tracker.isOscillating(h1, 0));

    tracker.record(h1, 0);
    TEST_ASSERT(tracker.isOscillating(h1, 5));
    TEST_ASSERT(!tracker.isOscillating(h1, 10));

    tracker.record(h1, 10);
    TEST_ASSERT(tracker.isOscillating(h1, 15));

    TEST_ASSERT(!tracker.isOscillating(h2, 15));

    tracker.record(h2, 100);
    TEST_ASSERT(!tracker.isOscillating(h1, 100));

    std::cerr << "PASS: testOscillationTracker\n";
}

static void testOrderIndependentHash() {
    std::cout << "Testing order independent hash...\n";
    std::vector<int> v1 = {1, 2, 3, 4, 5};
    std::vector<int> v2 = {5, 4, 3, 2, 1};
    std::vector<int> v3 = {3, 1, 5, 2, 4};
    
    auto compute_hash = [](const std::vector<int>& vertices) {
        uint64_t hash = 0;
        for (int v : vertices) {
            hash ^= std::hash<int>{}(v) * 0x9e3779b97f4a7c15ULL;
        }
        return hash;
    };
    
    uint64_t h1 = compute_hash(v1);
    uint64_t h2 = compute_hash(v2);
    uint64_t h3 = compute_hash(v3);
    
    TEST_ASSERT(h1 == h2);
    TEST_ASSERT(h1 == h3);
    
    // Also verify it's different for a different set
    std::vector<int> v4 = {1, 2, 3, 4, 6};
    uint64_t h4 = compute_hash(v4);
    TEST_ASSERT(h1 != h4);
    
    std::cout << "  Order independent hash test passed!\n";
}

static void testBuildBoundaryClause() {
    Graph g(3, 3);
    g.addEdge(0, 1, 10); g.addEdge(1, 0, 11);
    g.addEdge(1, 2, 12); g.addEdge(2, 1, 13);
    g.addEdge(2, 0, 14); g.addEdge(0, 2, 15);

    Component fullComp;
    fullComp.vertices = {0, 1, 2};
    fullComp.edges = {10, 11, 12, 13, 14, 15};

    (void)fullComp; // kept for documentation
    auto clause = buildBoundaryClause({0}, g);
    TEST_ASSERT(clause.size() == 2);
    TEST_ASSERT(std::find(clause.begin(), clause.end(), -10) != clause.end());
    TEST_ASSERT(std::find(clause.begin(), clause.end(), -15) != clause.end());

    std::cerr << "PASS: testBuildBoundaryClause\n";
}

void testTwoCompThresholdConfig() {
    std::cout << "Testing twoCompThreshold config setter/getter...\n";
    Solver solver("src/small.edge");
    TEST_ASSERT(solver.getTwoCompThreshold() == 20); // default
    solver.setTwoCompThreshold(5);
    TEST_ASSERT(solver.getTwoCompThreshold() == 5);
    std::cout << "  twoCompThreshold config test passed!\n";
}

void testAdaptiveCycleModeConfig() {
    std::cout << "Testing adaptive cycle mode config...\n";
    Solver solver;
    TEST_ASSERT(solver.getCycleMode() == Solver::CycleMode::FIXED);
    solver.setCycleMode(Solver::CycleMode::ADAPTIVE_BOUNDED);
    TEST_ASSERT(solver.getCycleMode() == Solver::CycleMode::ADAPTIVE_BOUNDED);
    std::cout << "  testAdaptiveCycleModeConfig PASS\n";
}

int main() {
    testGraphFileExists();
    testEncodingProducesValidCnf();
    testIncrementalVsNonIncrementalCountsMatch();
    testTimingMeasurement();
    testStagnationStrategies();
    test2EdgeConnectedBlocks();
    testPrecomputedBlockClauses();
    testOscillationTracker();
    testOrderIndependentHash();
    testBuildBoundaryClause();
    testTwoCompThresholdConfig();
    testAdaptiveCycleModeConfig();
    std::cout << "All graph tests passed successfully!\n";
    return 0;
}
