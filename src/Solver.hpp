#ifndef SOLVER_HPP
#define SOLVER_HPP

#include <string>
#include <memory>
#include <cstdint>
#include <unordered_map>
#include "Graph.hpp"
#include "AtMostOne/IAtMostOne.hpp"
#include "AtMostOne/DefaultAtMostOne.hpp"

class IncrementalSolver;

struct OscillationTracker {
    int window;
    int minCutThreshold;
    int maxCutSize;
    std::unordered_map<uint64_t, int> history;

    OscillationTracker(int win, int minC, int maxC)
        : window(win), minCutThreshold(minC), maxCutSize(maxC) {}

    bool isOscillating(uint64_t hash, int currentIter) const {
        auto it = history.find(hash);
        if (it == history.end()) return false;
        return (currentIter - it->second) < window;
    }

    void record(uint64_t hash, int currentIter) {
        history[hash] = currentIter;
    }
};

std::vector<int> buildBoundaryClause(
    const std::vector<int>& sideA_vertices,
    const Graph& graph);

class Solver {
public:
    enum class SolveResult {
        HAMILTONIAN,
        UNSAT,
        TIMEOUT,
        ERROR
    };

    enum class AtMostOneOption {
        DEFAULT,
        PBLIB
    };

    enum class SymmetryOption {
        DEFAULT,
        NONE
    };

    enum class StartNodeOption {
        MIN_DEGREE,
        MAX_DEGREE,
        FIRST_NODE,
        SPECIFIC_NODE
    };

private:
    std::string graphFile;
    int cycle;
    AtMostOneOption amoOption;
    SymmetryOption symOption;
    StartNodeOption startNodeOption;
    int specificStartNode;
    std::string satSolverCmd;
    std::string trajectoryFile;
    int randomSeed;
    int stagnationK;
    std::string stagnationStrategy;
    bool preprocess_;
    bool useVertexSep_;
    int vtxSepThreshold_;
    bool skipVertexDisjoint_;
    bool precomputeBlocks_;
    int oscillationWindow_ = 10;
    int cutThreshold_ = 100;
    int ghAtLeast2Threshold_ = 4;  // Gomory-Hu: use at-least-2 for cuts with weight <= this
    int twoCompThreshold_ = 20;    // trigger 2-comp strategy after this many consecutive 2-comp iterations

public:
    Solver(const std::string& gFile) 
        : graphFile(gFile), cycle(2), amoOption(AtMostOneOption::DEFAULT), 
          symOption(SymmetryOption::DEFAULT),
          startNodeOption(StartNodeOption::MIN_DEGREE), specificStartNode(0), 
          satSolverCmd("glucose"), trajectoryFile(""), randomSeed(0),
          stagnationK(3), stagnationStrategy("dfj"), preprocess_(true),
          useVertexSep_(true), vtxSepThreshold_(4), skipVertexDisjoint_(false),
          precomputeBlocks_(true) {}

    void setCycle(int c) { cycle = c; }
    int getCycle() const { return cycle; }
    void setAtMostOneOption(AtMostOneOption opt) { amoOption = opt; }
    void setStartNodeOption(StartNodeOption opt, int node = 0) { 
        startNodeOption = opt; 
            specificStartNode = node; 
    }
    void setSymmetryOption(SymmetryOption opt) { symOption = opt; }
    void setSatSolverCmd(const std::string& cmd) { satSolverCmd = cmd; }
    void setTrajectoryFile(const std::string& f) { trajectoryFile = f; }
    void setRandomSeed(int seed) { randomSeed = seed; }
    void setStagnationK(int k) { stagnationK = k; }
    void setStagnationStrategy(const std::string& s) { stagnationStrategy = s; }
    void setPreprocess(bool v) { preprocess_ = v; }
    void setVertexSep(bool v) { useVertexSep_ = v; }
    void setVtxSepThreshold(int t) { vtxSepThreshold_ = t; }
    void setSkipVertexDisjoint(bool v) { skipVertexDisjoint_ = v; }
    void setPrecomputeBlocks(bool b) { precomputeBlocks_ = b; }
    void setOscillationWindow(int w) { oscillationWindow_ = w; }
    void setCutThreshold(int t) { cutThreshold_ = t; }
    void setTwoCompThreshold(int t) { twoCompThreshold_ = t; }
    int getTwoCompThreshold() const { return twoCompThreshold_; }

    bool run();
    SolveResult runIncremental(int64_t timeLimitMs = 600000);
};

#endif
