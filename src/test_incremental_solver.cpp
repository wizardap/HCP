#include <iostream>
#include <vector>
#include <sstream>
#include <chrono>
#include <thread>
#include <cstdlib>
#include "IncrementalSolver.hpp"
#include "SubtourDetector.hpp"
#include "SecEncoder.hpp"
#include "VariableManager.hpp"
#include "Graph.hpp"

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
    TEST_ASSERT(clauses.size() == 4); // Outgoing + Incoming for each of the 2 components
    std::cout << "SubtourDetector and SecEncoder passed!\n";
}

int main() {
    testVariableManager();
    testIncrementalSolverBasic();
    testIncrementalSolverTimeout();
    testSubtourDetectorAndSecEncoder();
    std::cout << "All unit tests passed successfully!\n";
    return 0;
}
