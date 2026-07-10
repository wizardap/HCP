#ifndef SOLVER_HPP
#define SOLVER_HPP

#include <string>
#include <memory>
#include "Graph.hpp"
#include "AtMostOne/IAtMostOne.hpp"
#include "AtMostOne/DefaultAtMostOne.hpp"

class IncrementalSolver;

class Solver {
public:
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

public:
    Solver(const std::string& gFile) 
        : graphFile(gFile), cycle(2), amoOption(AtMostOneOption::DEFAULT), 
          symOption(SymmetryOption::DEFAULT),
          startNodeOption(StartNodeOption::MIN_DEGREE), specificStartNode(0), 
          satSolverCmd("glucose"), trajectoryFile(""), randomSeed(0),
          stagnationK(3), stagnationStrategy("dfj"), preprocess_(true),
          useVertexSep_(true), vtxSepThreshold_(4), skipVertexDisjoint_(false) {}

    void setCycle(int c) { cycle = c; }
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

    bool run();
    bool runIncremental(int64_t timeLimitMs = 600000);
};

#endif
