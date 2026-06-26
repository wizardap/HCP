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
#include "Graph.hpp"
#include "HcpEncoder.hpp"
#include "AtMostOne/DefaultAtMostOne.hpp"
#include "SymmetryBreaking/DefaultSymmetryBreaker.hpp"


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

int main() {
    testGraphFileExists();
    testEncodingProducesValidCnf();
    std::cout << "All graph tests passed successfully!\n";
    return 0;
}
