#include <iostream>
#include <vector>
#include <sstream>
#include <fstream>
#include <chrono>
#include <thread>
#include <cstdlib>
#include "IncrementalSolver.hpp"
#include "SubtourDetector.hpp"
#include "SecEncoder.hpp"
#include "VariableManager.hpp"
#include "Graph.hpp"
#include "GraphPreprocessor.hpp"
#include "Solver.hpp"
#include "ContractedMinCut.hpp"

#define TEST_ASSERT(cond) \
    do { \
        if (!(cond)) { \
            std::cerr << "Assertion failed: " << #cond << " at " << __FILE__ << ":" << __LINE__ << "\n"; \
            std::abort(); \
        } \
    } while (0)

void testVariableManager() {
    std::cout << "Testing VariableManager...\n";
    VariableManager vm(10);
    TEST_ASSERT(vm.getMaxVar() == 9);
    TEST_ASSERT(vm.newVar() == 10);
    TEST_ASSERT(vm.newVar() == 11);
    vm.freeVar(10);
    TEST_ASSERT(vm.newVar() == 10);
    TEST_ASSERT(vm.newVar() == 12);
    std::cout << "VariableManager passed!\n";
}

void testIncrementalSolverBasic() {
    std::cout << "Testing IncrementalSolver Basic...\n";
    IncrementalSolver solver;
    solver.addClause({1, 2});
    solver.addClause({-1});
    auto res = solver.solve();
    TEST_ASSERT(res == IncrementalSolver::Result::SAT);
    TEST_ASSERT(solver.getModelValue(2) == 1);
    TEST_ASSERT(solver.getModelValue(1) == -1);
    
    std::cout << "Testing printStatistics():\n";
    solver.printStatistics();

    // Verify solve times
    TEST_ASSERT(solver.getFinalSolveTime() >= 0.0);
    TEST_ASSERT(solver.getTotalSolverTime() >= 0.0);
    std::cout << "Solve times verified: final=" << solver.getFinalSolveTime() 
              << "s, total=" << solver.getTotalSolverTime() << "s\n";
    
    solver.addClause({1});
    solver.addClause({-1});
    res = solver.solve();
    TEST_ASSERT(res == IncrementalSolver::Result::UNSAT);
    std::cout << "IncrementalSolver Basic passed!\n";
}

void testIncrementalSolverGetModelOverload() {
    std::cout << "Testing IncrementalSolver getModel overload...\n";
    IncrementalSolver solver;
    solver.addClause({1, 2});
    solver.addClause({-1});
    solver.addClause({3, 4});
    solver.addClause({-3});
    auto res = solver.solve();
    TEST_ASSERT(res == IncrementalSolver::Result::SAT);
    
    // Full model should cover up to max var (which is 4)
    auto fullModel = solver.getModel();
    TEST_ASSERT(fullModel.size() == 5);
    TEST_ASSERT(fullModel[1] == -1);
    TEST_ASSERT(fullModel[2] == 2);
    TEST_ASSERT(fullModel[3] == -3);
    TEST_ASSERT(fullModel[4] == 4);

    // Partial model up to var 2
    auto partialModel2 = solver.getModel(2);
    TEST_ASSERT(partialModel2.size() == 3);
    TEST_ASSERT(partialModel2[1] == -1);
    TEST_ASSERT(partialModel2[2] == 2);

    // Partial model up to var 5 (should clamp to max_var)
    auto partialModel5 = solver.getModel(5);
    TEST_ASSERT(partialModel5.size() == 5);
    TEST_ASSERT(partialModel5[1] == -1);
    TEST_ASSERT(partialModel5[2] == 2);
    TEST_ASSERT(partialModel5[3] == -3);
    TEST_ASSERT(partialModel5[4] == 4);

    std::cout << "IncrementalSolver getModel overload passed!\n";
}

void testIncrementalSolverTimeout() {
    std::cout << "Testing IncrementalSolver Timeout...\n";
    IncrementalSolver solver(1); // 1ms timeout
    
    // Generate Pigeonhole Principle PHP(13, 12): 13 pigeons, 12 holes (guaranteed to take > 1ms)
    constexpr int num_pigeons = 13;
    constexpr int num_holes = 12;
    
    // Each pigeon must be in at least one hole
    for (int i = 1; i <= num_pigeons; ++i) {
        std::vector<int> clause;
        for (int j = 1; j <= num_holes; ++j) {
            clause.push_back((i-1)*num_holes + j);
        }
        solver.addClause(clause);
    }
    
    // No two pigeons in the same hole
    for (int j = 1; j <= num_holes; ++j) {
        for (int i1 = 1; i1 <= num_pigeons; ++i1) {
            for (int i2 = i1 + 1; i2 <= num_pigeons; ++i2) {
                solver.addClause({-((i1-1)*num_holes + j), -((i2-1)*num_holes + j)});
            }
        }
    }
    
    solver.setTimeLimit(1); // 1ms
    auto res = solver.solve();
    TEST_ASSERT(res == IncrementalSolver::Result::TIMEOUT);
    std::cout << "IncrementalSolver Timeout passed!\n";
}

void testSubtourDetectorAndSecEncoder() {
    std::cout << "Testing SubtourDetector and SecEncoder...\n";
    Graph g(4, 6);
    // Disjoint cycles
    g.addEdge(0, 1, 1);
    g.addEdge(1, 0, 2);
    g.addEdge(2, 3, 3);
    g.addEdge(3, 2, 4);
    
    // Add crossing boundary edges (not selected in the model)
    g.addEdge(0, 2, 5);
    g.addEdge(3, 1, 6);
    
    const std::vector<int> model = {0, 1, 1, 1, 1, 0, 0};
    
    auto components = SubtourDetector::detect(model, g);
    TEST_ASSERT(components.size() == 2);
    TEST_ASSERT(components[0].vertices.size() == 2);
    TEST_ASSERT(components[1].vertices.size() == 2);
    
    SecEncoder secEncoder(g);
    auto clauses = secEncoder.encodeSecs(components);
    TEST_ASSERT(clauses.size() == 6); // Outgoing + Incoming + Small-cycle DFJ for each of the 2 components
    std::cout << "SubtourDetector and SecEncoder passed!\n";
}

void testGraphPreprocessor() {
    std::cout << "Testing GraphPreprocessor...\n";

    // Test 1: DetectsBridgeOnPathGraph
    {
        // Path: 0-1-2  (bridge at every edge)
        Graph g(3, 2);
        g.addEdge(0, 1); g.addEdge(1, 0);
        g.addEdge(1, 2); g.addEdge(2, 1);
        GraphPreprocessor pp(g);
        TEST_ASSERT(pp.hasBridge());
        TEST_ASSERT(pp.getTwoEdgeCuts().empty());
        TEST_ASSERT(pp.getDegree2Vertices().size() == 1u);
        TEST_ASSERT(pp.getDegree2Vertices()[0] == 1);
    }

    // Test 2: DetectsDegree2OnCycle
    {
        // 4-cycle: 0-1-2-3-0, every vertex has degree 2
        Graph g(4, 4);
        g.addEdge(0, 1); g.addEdge(1, 0);
        g.addEdge(1, 2); g.addEdge(2, 1);
        g.addEdge(2, 3); g.addEdge(3, 2);
        g.addEdge(3, 0); g.addEdge(0, 3);
        GraphPreprocessor pp(g);
        TEST_ASSERT(!pp.hasBridge());
        TEST_ASSERT(pp.getDegree2Vertices().size() == 4u);
    }

    // Test 3: Detects2EdgeCutOnDumbbellGraph
    {
        // theta graph: vertices 0,1,2,3: paths 0-1-2, 0-3-2, and direct edge 0-2
        // Edges: {0,1},{1,2},{0,3},{3,2},{0,2}
        // Removing {0,1} and {0,3} disconnects vertex 0 from rest -- that's a 2-edge-cut
        Graph g(4, 5);
        g.addEdge(0, 1); g.addEdge(1, 0);
        g.addEdge(1, 2); g.addEdge(2, 1);
        g.addEdge(0, 3); g.addEdge(3, 0);
        g.addEdge(3, 2); g.addEdge(2, 3);
        g.addEdge(1, 3); g.addEdge(3, 1);
        GraphPreprocessor pp(g);
        TEST_ASSERT(!pp.hasBridge());
        bool found = false;
        for (const auto& ep : pp.getTwoEdgeCuts()) {
            bool e1 = (ep.u1==0&&ep.v1==1)||(ep.u1==1&&ep.v1==0)||
                      (ep.u1==0&&ep.v1==3)||(ep.u1==3&&ep.v1==0);
            bool e2 = (ep.u2==0&&ep.v2==1)||(ep.u2==1&&ep.v2==0)||
                      (ep.u2==0&&ep.v2==3)||(ep.u2==3&&ep.v2==0);
            if (e1 && e2) found = true;
        }
        TEST_ASSERT(found);
    }
    std::cout << "GraphPreprocessor passed!\n";
}

void testSolverPreprocessing() {
    std::cout << "Testing Solver Preprocessing...\n";
    // 4-cycle is Hamiltonian; every vertex has degree 2
    // After preprocessing, all 8 directed edge vars are constrained
    // We just verify it still finds HC and doesn't crash
    std::ofstream f("/tmp/test_cycle4.edge");
    f << "p edge 4 4\ne 1 2\ne 2 3\ne 3 4\ne 4 1\n";
    f.close();

    Solver s2("/tmp/test_cycle4.edge");
    s2.setPreprocess(true);
    TEST_ASSERT(s2.runIncremental(5000) == Solver::SolveResult::HAMILTONIAN); // 5s time limit, must find HC
    std::cout << "Solver Preprocessing passed!\n";
}

void testContractedMinCut() {
    std::cout << "Testing ContractedMinCut...\n";
    // Graph: two triangles (0-1-2) and (3-4-5) connected by two edges: 2-3, 0-5.
    // Vertices: 0,1,2 in triangle A; 3,4,5 in triangle B
    // Edges: {0,1},{1,2},{2,0},{3,4},{4,5},{5,3},{2,3},{0,5}
    Graph g(6, 8);
    g.addEdge(0,1); g.addEdge(1,2); g.addEdge(2,0);
    g.addEdge(3,4); g.addEdge(4,5); g.addEdge(5,3);
    g.addEdge(2,3); g.addEdge(0,5);

    // Simulate two components: C0={0,1,2}, C1={3,4,5}
    Component c0; c0.vertices = {0,1,2};
    Component c1; c1.vertices = {3,4,5};

    MinCutResult res = computeComponentMinCut({c0, c1}, g);

    // The min cut between C0 and C1 is 2 (edges 2-3 and 0-5)
    TEST_ASSERT(res.cutSize == 2);
    // sideA should contain all of {0,1,2} or all of {3,4,5}
    TEST_ASSERT(res.sideA_vertices.size() == 3u);
    std::cout << "ContractedMinCut passed!\n";
}

void testDinicMaxFlow() {
    std::cout << "Testing Dinic max-flow...\n";
    int n = 4;
    std::vector<std::vector<int>> cap(n, std::vector<int>(n, 0));
    cap[0][1] = cap[1][0] = 1;
    cap[1][2] = cap[2][1] = 1;
    cap[2][3] = cap[3][2] = 1;
    std::vector<bool> sideA;
    int flow = maxFlowDinic(n, cap, 0, 3, sideA);
    TEST_ASSERT(flow == 1);
    TEST_ASSERT(sideA.size() == 4);
    TEST_ASSERT(sideA[0] == true);
    TEST_ASSERT(sideA[3] == false);
    std::cout << "Dinic max-flow passed!\n";
}

void testGetIncomingLiterals() {
    std::cout << "Testing getIncomingLiterals correctness...\n";
    Graph g(3, 3);
    g.addEdge(0, 1, 1); g.addEdge(1, 0, 2);
    g.addEdge(1, 2, 3); g.addEdge(2, 1, 4);
    g.addEdge(2, 0, 5); g.addEdge(0, 2, 6);

    Component c;
    c.vertices = {1, 2};
    
    SecEncoder secEncoder(g);
    auto clauses = secEncoder.encodeSecs({c}, false);
    TEST_ASSERT(clauses.size() == 3);
    TEST_ASSERT(clauses[0] == std::vector<int>({2, 5}));
    TEST_ASSERT(clauses[1] == std::vector<int>({1, 6}));
    TEST_ASSERT(clauses[2] == std::vector<int>({-3, -4}));
    std::cout << "testGetIncomingLiterals passed!\n";
}

void testInternalMinCut() {
    std::cout << "Testing computeInternalMinCut...\n";
    Graph g2(6, 6);
    g2.addEdge(0, 1); g2.addEdge(1, 0);
    g2.addEdge(1, 2); g2.addEdge(2, 1);
    g2.addEdge(2, 3); g2.addEdge(3, 2);
    g2.addEdge(3, 0); g2.addEdge(0, 3);
    g2.addEdge(0, 4); g2.addEdge(4, 0);
    g2.addEdge(3, 5); g2.addEdge(5, 3);

    Component comp2;
    comp2.vertices = {0, 1, 2, 3};
    
    auto mcr = computeInternalMinCut(comp2, g2, 100);
    TEST_ASSERT(mcr.cutSize == 2);
    TEST_ASSERT(mcr.sideA_vertices.size() >= 1);
    TEST_ASSERT(mcr.sideA_vertices.size() <= 3);
    std::cout << "testInternalMinCut passed!\n";
}

void testAdaptiveBoundedEscalation() {
    std::cout << "Testing Adaptive Bounded Escalation...\n";
    Solver solver;
    std::string graphPath = "graphs/small.edge";
    {
        std::ifstream f(graphPath);
        if (!f.good()) {
            graphPath = "../graphs/small.edge";
        }
    }
    solver.setGraphFile(graphPath);
    solver.setCycleMode(Solver::CycleMode::ADAPTIVE_BOUNDED);
    auto res = solver.runIncremental(10000);
    TEST_ASSERT(res == Solver::SolveResult::HAMILTONIAN);
    std::cout << "testAdaptiveBoundedEscalation PASS\n";
}

int main() {
    testVariableManager();
    testIncrementalSolverBasic();
    testIncrementalSolverGetModelOverload();
    testIncrementalSolverTimeout();
    testSubtourDetectorAndSecEncoder();
    testGraphPreprocessor();
    testSolverPreprocessing();
    testContractedMinCut();
    testDinicMaxFlow();
    testGetIncomingLiterals();
    testInternalMinCut();
    testAdaptiveBoundedEscalation();
    std::cout << "All unit tests passed successfully!\n";
    return 0;
}

