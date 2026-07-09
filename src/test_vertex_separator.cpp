#include <iostream>
#include <vector>
#include <algorithm>
#include "SecEncoder.hpp"
#include "Graph.hpp"
#include "SubtourDetector.hpp"

#define TEST_ASSERT(cond) \
    do { \
        if (!(cond)) { \
            std::cerr << "FAIL at " << __FILE__ << ":" << __LINE__ \
                      << ": " << #cond << "\n"; \
            std::abort(); \
        } \
    } while (0)

static bool hasClauseContaining(const std::vector<std::vector<int>>& clauses,
                                 int lit) {
    for (const auto& c : clauses)
        for (int l : c)
            if (l == lit) return true;
    return false;
}

static int countUnitClauses(const std::vector<std::vector<int>>& clauses) {
    int cnt = 0;
    for (const auto& c : clauses)
        if (c.size() == 1) cnt++;
    return cnt;
}

void testDefaultBehavior() {
    Graph g(4, 4);
    g.addEdge(0, 1, 1); g.addEdge(1, 0, 2);
    g.addEdge(1, 2, 3); g.addEdge(2, 1, 4);
    g.addEdge(2, 3, 5); g.addEdge(3, 2, 6);

    Component comp;
    comp.vertices = {0, 1, 2};
    std::vector<Component> components = {comp};

    SecEncoder enc(g);
    auto cDefault = enc.encodeSecs(components);
    auto cWithSep = enc.encodeSecs(components, false, 4);

    TEST_ASSERT(cDefault.size() == cWithSep.size());
    for (size_t i = 0; i < cDefault.size(); i++)
        TEST_ASSERT(cDefault[i] == cWithSep[i]);

    std::cout << "testDefaultBehavior PASS\n";
}

void testVertexBoundaryCardinality() {
    Graph g(4, 4);
    g.addEdge(0, 1, 1); g.addEdge(1, 0, 2);
    g.addEdge(1, 2, 3); g.addEdge(2, 1, 4);
    g.addEdge(2, 3, 5); g.addEdge(3, 2, 6);

    Component comp;
    comp.vertices = {0, 1};
    std::vector<Component> components = {comp};

    SecEncoder enc(g);
    auto clauses = enc.encodeSecs(components, true, 4);

    TEST_ASSERT(clauses.size() >= 3);
    int unitCount = countUnitClauses(clauses);
    TEST_ASSERT(unitCount >= 1);

    std::cout << "testVertexBoundaryCardinality PASS (clauses="
              << clauses.size() << ", unit=" << unitCount << ")\n";
}

void testVertexDisjoint() {
    Graph g2(5, 5);
    g2.addEdge(0, 1, 1); g2.addEdge(1, 0, 2);
    g2.addEdge(0, 2, 3); g2.addEdge(2, 0, 4);
    g2.addEdge(1, 2, 5); g2.addEdge(2, 1, 6);

    Component comp;
    comp.vertices = {0};
    std::vector<Component> components = {comp};

    SecEncoder enc(g2);
    auto clauses = enc.encodeSecs(components, true, 4);

    TEST_ASSERT(clauses.size() > 2);

    std::cout << "testVertexDisjoint PASS (clauses=" << clauses.size() << ")\n";
}

void testThresholdBoundary() {
    Graph g(6, 6);
    g.addEdge(0, 1, 1); g.addEdge(1, 0, 2);
    g.addEdge(0, 2, 3); g.addEdge(2, 0, 4);
    g.addEdge(0, 3, 5); g.addEdge(3, 0, 6);
    g.addEdge(0, 4, 7); g.addEdge(4, 0, 8);
    g.addEdge(0, 5, 9); g.addEdge(5, 0, 10);

    Component comp;
    comp.vertices = {0};
    std::vector<Component> components = {comp};

    SecEncoder enc(g);
    auto cDefault = enc.encodeSecs(components, false, 2);
    auto cSep = enc.encodeSecs(components, true, 2);

    TEST_ASSERT(cDefault.size() == cSep.size());
    std::cout << "testThresholdBoundary PASS\n";
}

int main() {
    testDefaultBehavior();
    testVertexBoundaryCardinality();
    testVertexDisjoint();
    testThresholdBoundary();
    std::cout << "All vertex-separator tests PASS\n";
    return 0;
}