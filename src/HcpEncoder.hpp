#ifndef HCPENCODER_HPP
#define HCPENCODER_HPP

#include <iostream>
#include <vector>
#include <fstream>
#include <sstream>
#include <cstdio>
#include <cmath>
#include <cstdlib>
#include <memory>
#include "Graph.hpp"
#include "AtMostOne/IAtMostOne.hpp"
#include "AtMostOne/DefaultAtMostOne.hpp"
#include "SymmetryBreaking/ISymmetryBreaker.hpp"
#include "VariableManager.hpp"
#include "IncrementalSolver.hpp"

class HcpEncoder {
private:
    struct CoutRedirectGuard {
        std::streambuf* oldCoutBuf;
        CoutRedirectGuard(std::streambuf* newBuf) {
            oldCoutBuf = std::cout.rdbuf(newBuf);
        }
        ~CoutRedirectGuard() {
            std::cout.rdbuf(oldCoutBuf);
        }
    };

    Graph& graph;
    IAtMostOne& atMostOneEncoder;
    ISymmetryBreaker& symBreaker;
    std::vector<std::vector<int>> bitVars;
    int maxVar;
    int cycle;
    int startNode;
    std::unique_ptr<VariableManager> localVarManager_;
    VariableManager& varManager;

    int lfsr(int n, int size, int xor_val) {
        int m = n << 1;
        int x = m & (1 << size);
        if (x) m = m - x + 1;
        m ^= x >> xor_val;
        return m;
    }

    int bit(int n, int pos) {
        if (n >= (int)bitVars.size()) bitVars.resize(n + 1);
        if (pos >= (int)bitVars[n].size()) bitVars[n].resize(pos + 1, 0);
        if (bitVars[n][pos] == 0) {
            bitVars[n][pos] = varManager.newVar();
        }
        return bitVars[n][pos];
    }

public:
    // Backward compatible constructor (for one-shot mode)
    HcpEncoder(Graph& g, int c, IAtMostOne& amo, ISymmetryBreaker& sym, int sNode = -1) 
        : graph(g), atMostOneEncoder(amo), symBreaker(sym), maxVar(0), cycle(c), startNode(sNode),
          localVarManager_(new VariableManager(2 * g.getEdges() + 1)),
          varManager(*localVarManager_) {
    }
    
    // New constructor for incremental mode (uses external VariableManager)
    HcpEncoder(Graph& g, int c, IAtMostOne& amo, ISymmetryBreaker& sym, int sNode, VariableManager& vm) 
        : graph(g), atMostOneEncoder(amo), symBreaker(sym), maxVar(0), cycle(c), startNode(sNode),
          localVarManager_(nullptr),
          varManager(vm) {
    }
    
    ~HcpEncoder() = default;

    // Backward compatible method - outputs CNF to stdout (uses stringstream and CoutRedirectGuard, no disk write)
    void encode() {
        std::stringstream ss;
        {
            CoutRedirectGuard guard(ss.rdbuf());
            encodeBaseOutput(std::cout);
            std::cout.flush();
        }

        // Read the stringstream to count clauses (newlines)
        std::string line;
        int actual_nCls = 0;
        while (std::getline(ss, line)) {
            if (!line.empty() && line[0] != 'c') { // Count non-comment lines
                actual_nCls++;
            }
        }
        
        // Print the correct DIMACS header (need to get actual max var from VariableManager)
        std::cout << "p cnf " << varManager.getMaxVar() << " " << actual_nCls << "\n";

        // Dump the stringstream content to standard output
        ss.clear();
        ss.seekg(0, std::ios::beg);
        std::cout << ss.rdbuf();
    }

    // New method for incremental encoding - adds clauses to solver
    void encodeBase(IncrementalSolver& solver) {
        std::stringstream ss;
        {
            CoutRedirectGuard guard(ss.rdbuf());
            encodeBaseOutput(std::cout);
            std::cout.flush();
        }

        solver.addClausesFromStream(ss);
    }

    // Accessor for variable manager (needed by SecEncoder)
    VariableManager& getVariableManager() {
        return varManager;
    }

private:

    void encodeBaseOutput(std::ostream& out) {
        // This contains the original encode() logic but outputs to the given stream
        int nNode = graph.getNodes();

        int first;
        int firstDegree;
        if (startNode == -1) {
            first = graph.getMinDegreeVertex(firstDegree);
        } else if (startNode == -2) {
            first = graph.getMaxDegreeVertex(firstDegree);
        } else if (startNode == -3) {
            first = 0;
            firstDegree = graph.getDegree(first);
        } else {
            first = startNode;
            firstDegree = graph.getDegree(first);
        }

        if (first == -1) {
            first = 0;
            firstDegree = graph.getDegree(first);
        }

        std::vector<int> firstNeighbors;
        for (auto& [v, _] : graph.getNeighbors(first)) {
            firstNeighbors.push_back(v);
        }

        // Output preamble will be handled by caller in incremental mode
        // For backward compatibility in encode(), the caller handles the preamble

        // ENCODE CONSTRAINTS - same as original encode() but output to 'out' stream

        for (int i = 0; i < nNode; i++) {
            int b = 0;
            int k = 1;
            while (true) {
                if ((cycle % (1 << k)) == 0) b++;
                else break;
                k++;
            }
            if ((cycle % 3) == 0) { out << bit(i, b) << " " << bit(i, b+1) << " 0\n"; b+=2; }
            if ((cycle % 5) == 0) {
                out << "-" << bit(i, b) << " -" << bit(i, b+2) << " 0\n";
                out << "-" << bit(i, b+1) << " -" << bit(i, b+2) << " 0\n"; b+=3;
            }
            if ((cycle % 7) == 0) { out << bit(i, b) << " " << bit(i, b+1) << " " << bit(i, b+2) << " 0\n"; b+=3; }
            if ((cycle % 511) == 0) { for (int j = 0; j < 9; j++) out << bit(i, b+j) << " "; out << "0\n"; b+=9; }
            if ((cycle % 1023) == 0) { for (int j = 0; j < 10; j++) out << bit(i, b+j) << " "; out << "0\n"; b+=10; }
            if ((cycle % 2047) == 0) { for (int j = 0; j < 11; j++) out << bit(i, b+j) << " "; out << "0\n"; b+=11; }
        }

        std::vector<int> neighbors;

        // Exactly one successor
        for (int i = 0; i < nNode; i++) {
            neighbors.clear();
            for (auto& [j, edgeIdx] : graph.getNeighbors(i)) {
                neighbors.push_back(edgeIdx);
            }
            for (size_t j = 0; j < neighbors.size(); j++) out << neighbors[j] << " ";
            out << "0\n";
            int maxVarLegacy = varManager.getMaxVar() + 1;
            atMostOneEncoder.encode(neighbors, neighbors.size(), maxVarLegacy);
            varManager.resetTo(maxVarLegacy);
        }

        // Exactly one predecessor
        for (int i = 0; i < nNode; i++) {
            neighbors.clear();
            for (auto& [j, _] : graph.getNeighbors(i)) {
                int edgeIdx = graph.getAdj(j, i);
                if (edgeIdx > 0) neighbors.push_back(edgeIdx);
            }
            for (size_t j = 0; j < neighbors.size(); j++) out << neighbors[j] << " ";
            out << "0\n";
            int maxVarLegacy = varManager.getMaxVar() + 1;
            atMostOneEncoder.encode(neighbors, neighbors.size(), maxVarLegacy);
            varManager.resetTo(maxVarLegacy);
        }

        // one of first neighbors must be the final connection
        neighbors.clear();
        for (auto& [v, _] : graph.getNeighbors(first)) {
            neighbors.push_back(v);
        }
        for (size_t j = 0; j < neighbors.size(); j++) out << graph.getAdj(neighbors[j], first) << " ";
        out << "0\n";

        // symmetry breaking
        symBreaker.encode(graph, first, neighbors);

        // initialize the starting position
        int b = 0;
        int k = 1;
        while (true) {
            if ((cycle % (1 << k)) == 0) {
                out << "-" << bit(first, b) << " 0\n"; b++;
            } else break;
            k++;
        }

        if ((cycle % 3) == 0) {
            out << bit(first, b) << " 0\n"; b++;
            out << "-" << bit(first, b) << " 0\n"; b++;
        }
        if ((cycle % 5) == 0) {
            out << "-" << bit(first, b) << " 0\n"; b++;
            out << "-" << bit(first, b) << " 0\n"; b++;
            out << "-" << bit(first, b) << " 0\n"; b++;
        }
        if ((cycle % 7) == 0) {
            out << bit(first, b) << " 0\n"; b++;
            out << "-" << bit(first, b) << " 0\n"; b++;
            out << "-" << bit(first, b) << " 0\n"; b++;
        }
        if ((cycle % 511) == 0) {
            out << bit(first, b) << " 0\n"; b++;
            for (int k = 2; k <= 9; k++) { out << "-" << bit(first, b) << " 0\n"; b++; }
        }
        if ((cycle % 1023) == 0) {
            out << bit(first, b) << " 0\n"; b++;
            for (int k = 2; k <= 10; k++) { out << "-" << bit(first, b) << " 0\n"; b++; }
        }
        if ((cycle % 2047) == 0) {
            out << bit(first, b) << " 0\n"; b++;
            for (int k = 2; k <= 11; k++) { out << "-" << bit(first, b) << " 0\n"; b++; }
        }

        // initialize the termination position (one of the neighbors of first)
        for (int j = 0; j < firstDegree; j++) {
            b = 0;
            int k = 1;
            while (true) {
                if ((cycle % (1 << k)) == 0) {
                    out << "-" << graph.getAdj(neighbors[j], first) << " ";
                    if (((nNode - 1) & (1 << k) / 2) == 0) out << "-";
                    out << bit(neighbors[j], b) << " 0\n";
                    b++;
                } else break;
                k++;
            }

            if ((cycle % 3) == 0) {
                int mask = 1;
                for (int i = 0; i < (nNode - 1) % 3; i++) mask = lfsr(mask, 2, 1);
                for (int i = 0; i < 2; i++) {
                    out << "-" << graph.getAdj(neighbors[j], first) << " ";
                    if ((mask & 1) == 0) out << "-";
                    mask = mask >> 1;
                    out << bit(neighbors[j], b + i) << " 0\n";
                }
                b += 2;
            }

            if ((cycle % 5) == 0) {
                int mask = (nNode + 4) % 5;
                for (int i = 0; i < 3; i++) {
                    out << "-" << graph.getAdj(neighbors[j], first) << " ";
                    if ((mask & 1) == 0) out << "-";
                    mask = mask >> 1;
                    out << bit(neighbors[j], b + i) << " 0\n";
                }
                b += 3;
            }

            if ((cycle % 7) == 0) {
                int mask = 1;
                for (int i = 0; i < (nNode - 1) % 7; i++) mask = lfsr(mask, 3, 1);
                for (int i = 0; i < 3; i++) {
                    out << "-" << graph.getAdj(neighbors[j], first) << " ";
                    if ((mask & 1) == 0) out << "-";
                    mask = mask >> 1;
                    out << bit(neighbors[j], b + i) << " 0\n";
                }
                b += 3;
            }

            if ((cycle % 511) == 0) {
                int mask = 1;
                for (int i = 0; i < (nNode - 1) % 511; i++) mask = lfsr(mask, 9, 4);
                for (int i = 0; i < 9; i++) {
                    out << "-" << graph.getAdj(neighbors[j], first) << " ";
                    if ((mask & 1) == 0) out << "-";
                    mask = mask >> 1;
                    out << bit(neighbors[j], b + i) << " 0\n";
                }
                b += 9;
            }

            if ((cycle % 1023) == 0) {
                int mask = 1;
                for (int i = 0; i < (nNode - 1) % 1023; i++) mask = lfsr(mask, 10, 3);
                for (int i = 0; i < 10; i++) {
                    out << "-" << graph.getAdj(neighbors[j], first) << " ";
                    if ((mask & 1) == 0) out << "-";
                    mask = mask >> 1;
                    out << bit(neighbors[j], b + i) << " 0\n";
                }
                b += 10;
            }

            if ((cycle % 2047) == 0) {
                int mask = 1;
                for (int i = 0; i < (nNode - 1) % 2047; i++) mask = lfsr(mask, 11, 2);
                for (int i = 0; i < 11; i++) {
                    out << "-" << graph.getAdj(neighbors[j], first) << " ";
                    if ((mask & 1) == 0) out << "-";
                    mask = mask >> 1;
                    out << bit(neighbors[j], b + i) << " 0\n";
                }
                b += 11;
            }
        }

        // enforce the next relationship
        for (int i = 0; i < nNode; i++) {
            for (auto& [j, edgeIdx] : graph.getNeighbors(i)) {
                if (j != first) {
                    int b = 0;
                    if ((cycle % 2) == 0) {
                        out << "-" << edgeIdx << " " << bit(j, b) << " " << bit(i, b) << " 0\n";
                        out << "-" << edgeIdx << " -" << bit(j, b) << " -" << bit(i, b) << " 0\n";
                        b++;
                    }

                    k = 2;
                    while (true) {
                        if ((cycle % (1 << k)) == 0) {
                            for (int l = 1; l < k; l++) {
                                out << "-" << edgeIdx << " " << bit(i, b - l) << " " << bit(j, b) << " -" << bit(i, b) << " 0\n";
                                out << "-" << edgeIdx << " " << bit(i, b - l) << " -" << bit(j, b) << " " << bit(i, b) << " 0\n";
                            }
                            for (int l = 1; l < k; l++) out << "-" << bit(i, b - l) << " ";
                            out << "-" << edgeIdx << " " << bit(j, b) << " " << bit(i, b) << " 0\n";
                            for (int l = 1; l < k; l++) out << "-" << bit(i, b - l) << " ";
                            out << "-" << edgeIdx << " -" << bit(j, b) << " -" << bit(i, b) << " 0\n";
                            b++;
                        } else break;
                        k++;
                    }

                    if ((cycle % 3) == 0) {
                        out << "-" << edgeIdx << " " << bit(j, b) << " -" << bit(i, b + 1) << " 0\n";
                        out << "-" << edgeIdx << " -" << bit(j, b) << " " << bit(i, b + 1) << " 0\n";
                        out << "-" << edgeIdx << " " << bit(j, b + 1) << " " << bit(i, b) << " -" << bit(i, b + 1) << " 0\n";
                        out << "-" << edgeIdx << " " << bit(j, b + 1) << " -" << bit(i, b) << " " << bit(i, b + 1) << " 0\n";
                        out << "-" << edgeIdx << " -" << bit(j, b + 1) << " " << bit(i, b) << " " << bit(i, b + 1) << " 0\n";
                        out << "-" << edgeIdx << " -" << bit(j, b + 1) << " -" << bit(i, b) << " -" << bit(i, b + 1) << " 0\n";
                        b += 2;
                    }

                    if ((cycle % 5) == 0) {
                        out << "-" << edgeIdx << " -" << bit(j, b) << " -" << bit(i, b) << " 0\n";
                        out << "-" << edgeIdx << " -" << bit(j, b) << " -" << bit(i, b + 2) << " 0\n";
                        out << "-" << edgeIdx << " " << bit(j, b) << " " << bit(i, b) << " " << bit(i, b + 2) << " 0\n";
                        out << "-" << edgeIdx << " " << bit(j, b + 1) << " " << bit(i, b) << " -" << bit(i, b + 1) << " 0\n";
                        out << "-" << edgeIdx << " " << bit(j, b + 1) << " -" << bit(i, b) << " " << bit(i, b + 1) << " 0\n";
                        out << "-" << edgeIdx << " -" << bit(j, b + 1) << " " << bit(i, b) << " " << bit(i, b + 1) << " 0\n";
                        out << "-" << edgeIdx << " -" << bit(j, b + 1) << " -" << bit(i, b) << " -" << bit(i, b + 1) << " 0\n";
                        out << "-" << edgeIdx << " -" << bit(j, b + 2) << " " << bit(i, b) << " 0\n";
                        out << "-" << edgeIdx << " -" << bit(j, b + 2) << " " << bit(i, b + 1) << " 0\n";
                        out << "-" << edgeIdx << " " << bit(j, b + 2) << " -" << bit(i, b) << " -" << bit(i, b + 1) << " 0\n";
                        b += 3;
                    }

                    if ((cycle % 7) == 0) {
                        out << "-" << edgeIdx << " " << bit(j, b) << " -" << bit(i, b + 2) << " 0\n";
                        out << "-" << edgeIdx << " -" << bit(j, b) << " " << bit(i, b + 2) << " 0\n";
                        out << "-" << edgeIdx << " " << bit(j, b + 1) << " -" << bit(i, b) << " 0\n";
                        out << "-" << edgeIdx << " -" << bit(j, b + 1) << " " << bit(i, b) << " 0\n";
                        out << "-" << edgeIdx << " " << bit(j, b + 2) << " " << bit(i, b + 1) << " -" << bit(i, b + 2) << " 0\n";
                        out << "-" << edgeIdx << " " << bit(j, b + 2) << " -" << bit(i, b + 1) << " " << bit(i, b + 2) << " 0\n";
                        out << "-" << edgeIdx << " -" << bit(j, b + 2) << " " << bit(i, b + 1) << " " << bit(i, b + 2) << " 0\n";
                        out << "-" << edgeIdx << " -" << bit(j, b + 2) << " -" << bit(i, b + 1) << " -" << bit(i, b + 2) << " 0\n";
                        b += 3;
                    }

                    if ((cycle % 511) == 0) {
                        out << "-" << edgeIdx << " " << bit(j, b) << " -" << bit(i, b + 8) << " 0\n";
                        out << "-" << edgeIdx << " -" << bit(j, b) << " " << bit(i, b + 8) << " 0\n";
                        out << "-" << edgeIdx << " " << bit(j, b + 1) << " -" << bit(i, b) << " 0\n";
                        out << "-" << edgeIdx << " -" << bit(j, b + 1) << " " << bit(i, b) << " 0\n";
                        out << "-" << edgeIdx << " " << bit(j, b + 2) << " -" << bit(i, b + 1) << " 0\n";
                        out << "-" << edgeIdx << " -" << bit(j, b + 2) << " " << bit(i, b + 1) << " 0\n";
                        out << "-" << edgeIdx << " " << bit(j, b + 3) << " -" << bit(i, b + 2) << " 0\n";
                        out << "-" << edgeIdx << " -" << bit(j, b + 3) << " " << bit(i, b + 2) << " 0\n";
                        out << "-" << edgeIdx << " " << bit(j, b + 4) << " -" << bit(i, b + 3) << " 0\n";
                        out << "-" << edgeIdx << " -" << bit(j, b + 4) << " " << bit(i, b + 3) << " 0\n";
                        out << "-" << edgeIdx << " " << bit(j, b + 5) << " " << bit(i, b + 4) << " -" << bit(i, b + 8) << " 0\n";
                        out << "-" << edgeIdx << " " << bit(j, b + 5) << " -" << bit(i, b + 4) << " " << bit(i, b + 8) << " 0\n";
                        out << "-" << edgeIdx << " -" << bit(j, b + 5) << " " << bit(i, b + 4) << " " << bit(i, b + 8) << " 0\n";
                        out << "-" << edgeIdx << " -" << bit(j, b + 5) << " -" << bit(i, b + 4) << " -" << bit(i, b + 8) << " 0\n";
                        out << "-" << edgeIdx << " " << bit(j, b + 6) << " -" << bit(i, b + 5) << " 0\n";
                        out << "-" << edgeIdx << " -" << bit(j, b + 6) << " " << bit(i, b + 5) << " 0\n";
                        out << "-" << edgeIdx << " " << bit(j, b + 7) << " -" << bit(i, b + 6) << " 0\n";
                        out << "-" << edgeIdx << " -" << bit(j, b + 7) << " " << bit(i, b + 6) << " 0\n";
                        out << "-" << edgeIdx << " " << bit(j, b + 8) << " -" << bit(i, b + 7) << " 0\n";
                        out << "-" << edgeIdx << " -" << bit(j, b + 8) << " " << bit(i, b + 7) << " 0\n";
                        b += 9;
                    }

                    if ((cycle % 1023) == 0) {
                        out << "-" << edgeIdx << " " << bit(j, b) << " -" << bit(i, b + 9) << " 0\n";
                        out << "-" << edgeIdx << " -" << bit(j, b) << " " << bit(i, b + 9) << " 0\n";
                        out << "-" << edgeIdx << " " << bit(j, b + 1) << " -" << bit(i, b) << " 0\n";
                        out << "-" << edgeIdx << " -" << bit(j, b + 1) << " " << bit(i, b) << " 0\n";
                        out << "-" << edgeIdx << " " << bit(j, b + 2) << " -" << bit(i, b + 1) << " 0\n";
                        out << "-" << edgeIdx << " -" << bit(j, b + 2) << " " << bit(i, b + 1) << " 0\n";
                        out << "-" << edgeIdx << " " << bit(j, b + 3) << " -" << bit(i, b + 2) << " 0\n";
                        out << "-" << edgeIdx << " -" << bit(j, b + 3) << " " << bit(i, b + 2) << " 0\n";
                        out << "-" << edgeIdx << " " << bit(j, b + 4) << " -" << bit(i, b + 3) << " 0\n";
                        out << "-" << edgeIdx << " -" << bit(j, b + 4) << " " << bit(i, b + 3) << " 0\n";
                        out << "-" << edgeIdx << " " << bit(j, b + 5) << " " << bit(j, b + 4) << " -" << bit(i, b + 5) << " 0\n";
                        out << "-" << edgeIdx << " -" << bit(j, b + 5) << " " << bit(i, b + 4) << " " << bit(i, b + 5) << " 0\n";
                        out << "-" << edgeIdx << " " << bit(j, b + 6) << " -" << bit(i, b + 5) << " 0\n";
                        out << "-" << edgeIdx << " -" << bit(j, b + 6) << " " << bit(i, b + 5) << " 0\n";
                        out << "-" << edgeIdx << " " << bit(j, b + 7) << " -" << bit(i, b + 6) << " 0\n";
                        out << "-" << edgeIdx << " -" << bit(j, b + 7) << " " << bit(i, b + 6) << " 0\n";
                        out << "-" << edgeIdx << " " << bit(j, b + 8) << " -" << bit(i, b + 7) << " 0\n";
                        out << "-" << edgeIdx << " -" << bit(j, b + 8) << " " << bit(i, b + 7) << " 0\n";
                        out << "-" << edgeIdx << " " << bit(j, b + 9) << " " << bit(i, b + 8) << " -" << bit(i, b + 10) << " 0\n";
                        out << "-" << edgeIdx << " " << bit(j, b + 9) << " -" << bit(i, b + 8) << " " << bit(i, b + 10) << " 0\n";
                        out << "-" << edgeIdx << " -" << bit(j, b + 9) << " " << bit(i, b + 8) << " " << bit(i, b + 10) << " 0\n";
                        out << "-" << edgeIdx << " -" << bit(j, b + 9) << " " << bit(i, b + 8) << " -" << bit(i, b + 10) << " 0\n";
                        out << "-" << edgeIdx << " " << bit(j, b + 10) << " -" << bit(i, b + 9) << " 0\n";
                        out << "-" << edgeIdx << " -" << bit(j, b + 10) << " " << bit(i, b + 9) << " 0\n";
                        b += 11;
                    }
                }
            }
        }
    }

};

#endif