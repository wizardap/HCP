#include <iostream>
#include <cstdlib>
#include <cstring>
#include <memory>
#include <fstream>
#include "Solver.hpp"
#include "HcpEncoder.hpp"
#include "HcpDecoder.hpp"
#include "AtMostOne/DefaultAtMostOne.hpp"
#include "AtMostOne/PbLibAtMostOne.hpp"
#include "SymmetryBreaking/DefaultSymmetryBreaker.hpp"
#include "SymmetryBreaking/NoSymmetryBreaker.hpp"
#include "IncrementalSolver.hpp"
#include "SubtourDetector.hpp"
#include "SecEncoder.hpp"
#include "VariableManager.hpp"
#include "TrajectoryLogger.hpp"
#include "GraphPreprocessor.hpp"
#include "ContractedMinCut.hpp"
#include <sstream>
#include <algorithm>
#include <iterator>
#include <climits>
#include <unordered_map>
#include <set>
#include <functional>
#include <cstdint>

// Forward declaration for find2EdgeConnectedBlocks (defined below)
std::vector<std::vector<int>> find2EdgeConnectedBlocks(const Graph& g);

bool Solver::run() {
    Graph g;
    if (!g.loadFromFile(graphFile, true)) {
        std::cerr << "c Error: could not open graph file " << graphFile << "\n";
        return false;
    }

    std::unique_ptr<IAtMostOne> amo;
    if (amoOption == AtMostOneOption::PBLIB) {
        amo.reset(new PbLibAtMostOne());
    } else {
        amo.reset(new DefaultAtMostOne());
    }

    std::unique_ptr<ISymmetryBreaker> sym;
    if (symOption == SymmetryOption::DEFAULT) {
        sym.reset(new DefaultSymmetryBreaker());
    } else if (symOption == SymmetryOption::NONE) {
        sym.reset(new NoSymmetryBreaker());
    }

    int sNode = -1;
    if (startNodeOption == StartNodeOption::MIN_DEGREE) sNode = -1;
    else if (startNodeOption == StartNodeOption::MAX_DEGREE) sNode = -2;
    else if (startNodeOption == StartNodeOption::FIRST_NODE) sNode = -3;
    else if (startNodeOption == StartNodeOption::SPECIFIC_NODE) sNode = specificStartNode;

    HcpEncoder encoder(g, cycle, *amo, *sym, sNode);
    encoder.encode();

    return true;
}

// Canonical fingerprint: sorted vector of sorted vertex vectors
static std::vector<std::vector<int>> computeFingerprint(
    const std::vector<Component>& components
) {
    std::vector<std::vector<int>> vertexSets;
    vertexSets.reserve(components.size());
    for (const auto& comp : components) {
        auto vs = comp.vertices;
        std::sort(vs.begin(), vs.end());
        vertexSets.push_back(std::move(vs));
    }
    std::sort(vertexSets.begin(), vertexSets.end());
    return vertexSets;
}

// True if partition changed between fingerprints
static bool partitionChanged(
    const std::vector<std::vector<int>>& prevFingerprint,
    const std::vector<Component>& components
) {
    auto current = computeFingerprint(components);
    if (current.size() != prevFingerprint.size()) return true;
    for (size_t i = 0; i < current.size(); ++i) {
        if (current[i] != prevFingerprint[i]) return true;
    }
    return false;
}

// Try blocking each component one-at-a-time via assumption-based SAT.
// Returns true if partition was forced to change.
// Returns false if all components failed (no single-component block forced change).
static bool runGreedyBlocking(
    const std::vector<Component>& components,
    IncrementalSolver& isolver,
    const Graph& g,
    std::vector<std::vector<int>>& prevFingerprint,
    std::vector<int>& prevBlockedComponentIds,
    int& usedSkipVars,
    int skipVarStart,
    int maxSkipVars,
    SecEncoder& iterationSecEncoder,
    bool useVertexSep,
    int vtxSepThreshold
) {
    std::vector<int> skipVars;
    skipVars.reserve(components.size());
    for (const auto& comp : components) {
        if (usedSkipVars >= maxSkipVars) {
            std::cerr << "c Error: Skip variables pool exhausted (" << usedSkipVars << "/" << maxSkipVars << ")\n";
            return false;
        }
        int skipVar = skipVarStart + usedSkipVars;
        usedSkipVars++;
        skipVars.push_back(skipVar);
        std::vector<int> clause;
        clause.push_back(-skipVar);
        for (int e : comp.edges) {
            clause.push_back(-e);
        }
        isolver.addClause(clause);
    }

    std::vector<int> compOrder(components.size());
    for (size_t i = 0; i < components.size(); ++i) compOrder[i] = i;
    std::sort(compOrder.begin(), compOrder.end(),
        [&](int a, int b) {
            return components[a].vertices.size() < components[b].vertices.size();
        });

    for (int compIdx : compOrder) {
        isolver.addAssumption(-skipVars[compIdx]);
        for (int j : compOrder) {
            if (j != compIdx) {
                isolver.addAssumption(skipVars[j]);
            }
        }

        auto innerResult = isolver.solve();

        if (innerResult == IncrementalSolver::Result::TIMEOUT) {
            std::cerr << "c TIMEOUT during greedy escalation\n";
            return false;
        }

        if (innerResult == IncrementalSolver::Result::SAT) {
            auto innerModel = isolver.getModel();
            auto innerComps = SubtourDetector::detect(innerModel, g);

            if (partitionChanged(prevFingerprint, innerComps)) {
                std::cerr << "c Stagnation: component " << compIdx
                          << " (size " << components[compIdx].vertices.size()
                          << ") forced partition change\n";

                prevBlockedComponentIds.clear();
                for (size_t i = 0; i < innerComps.size(); ++i) {
                    prevBlockedComponentIds.push_back(static_cast<int>(i));
                }

                iterationSecEncoder.startAuxAt(isolver.getNumVars() + 1);
                auto secClauses = iterationSecEncoder.encodeSecs(innerComps, useVertexSep, vtxSepThreshold, false);
                for (const auto& clause : secClauses) {
                    isolver.addClause(clause);
                }
                std::cerr << "c Iteration: found " << innerComps.size()
                          << " components (greedy), added " << secClauses.size() << " SEC clauses\n";
                return true;
            }
        }
    }

    std::cerr << "c Stagnation: greedy escalation failed, falling back to normal SEC\n";
    return false;
}

std::vector<int> buildBoundaryClause(
    const std::vector<int>& sideA_vertices,
    const Graph& graph)
{
    std::vector<bool> inSideA(graph.getNodes(), false);
    for (int v : sideA_vertices) inSideA[v] = true;

    std::vector<int> clause;
    for (int u : sideA_vertices) {
        for (auto& [v, _] : graph.getNeighbors(u)) {
            if (!inSideA[v]) {
                int lit = graph.getAdj(u, v);
                if (lit > 0) clause.push_back(-lit);
            }
        }
    }
    return clause;
}

Solver::SolveResult Solver::runIncremental(int64_t timeLimitMs) {
    Graph g;
    if (!g.loadFromFile(graphFile, true)) { // Pass true for directed edge indices mapping
        std::cerr << "c Error: could not open graph file " << graphFile << "\n";
        return SolveResult::ERROR;
    }

    std::unique_ptr<IAtMostOne> amo;
    if (amoOption == AtMostOneOption::PBLIB) {
        amo.reset(new PbLibAtMostOne());
    } else {
        amo.reset(new DefaultAtMostOne());
    }

    std::unique_ptr<ISymmetryBreaker> sym;
    if (symOption == SymmetryOption::DEFAULT) {
        sym.reset(new DefaultSymmetryBreaker());
    } else if (symOption == SymmetryOption::NONE) {
        sym.reset(new NoSymmetryBreaker());
    }

    int sNode = -1;
    if (startNodeOption == StartNodeOption::MIN_DEGREE) sNode = -1;
    else if (startNodeOption == StartNodeOption::MAX_DEGREE) sNode = -2;
    else if (startNodeOption == StartNodeOption::FIRST_NODE) sNode = -3;
    else if (startNodeOption == StartNodeOption::SPECIFIC_NODE) sNode = specificStartNode;

    VariableManager vm(2 * g.getEdges() + 1);
    IncrementalSolver isolver(timeLimitMs);
    if (randomSeed > 0) {
        std::cerr << "c random seed: " << randomSeed << " (note: CaDiCaL seed not yet forwarded through IncrementalSolver)\n";
    }
    HcpEncoder encoder(g, cycle, *amo, *sym, sNode, vm);
    encoder.encodeBase(isolver);

    // ---- PREPROCESSING: Forced edges from degree-2 vertices and 2-edge-cuts ----
    std::vector<std::vector<int>> forcedClausesVec;
    int forcedClausesCount = 0;
    if (preprocess_) {
        GraphPreprocessor pp(g);

        if (pp.hasBridge()) {
            std::cerr << "c Preprocessing: graph has a bridge — no Hamiltonian Cycle possible\n";
            return SolveResult::UNSAT;
        }

        // Degree-2 vertices: both incident undirected edges must be selected
        for (int u : pp.getDegree2Vertices()) {
            for (auto& [v, _] : g.getNeighbors(u)) {
                int fwd = g.getAdj(u, v);
                int bwd = g.getAdj(v, u);
                if (fwd > 0 && bwd > 0) {
                    std::vector<int> clause = {fwd, bwd};
                    isolver.addClause(clause);
                    forcedClausesVec.push_back(std::move(clause));
                    forcedClausesCount++;
                }
            }
        }

        // 2-edge-cuts: both edges forced, opposite directions
        for (const auto& ep : pp.getTwoEdgeCuts()) {
            int f1 = g.getAdj(ep.u1, ep.v1);
            int b1 = g.getAdj(ep.v1, ep.u1);
            int f2 = g.getAdj(ep.u2, ep.v2);
            int b2 = g.getAdj(ep.v2, ep.u2);
            if (f1 <= 0 || b1 <= 0 || f2 <= 0 || b2 <= 0) continue;

            {
                std::vector<int> clause = {f1, b1};
                isolver.addClause(clause);
                forcedClausesVec.push_back(std::move(clause));
                forcedClausesCount++;
            }
            {
                std::vector<int> clause = {f2, b2};
                isolver.addClause(clause);
                forcedClausesVec.push_back(std::move(clause));
                forcedClausesCount++;
            }
            {
                std::vector<int> clause = {-f1, b2};
                isolver.addClause(clause);
                forcedClausesVec.push_back(std::move(clause));
                forcedClausesCount++;
            }
            {
                std::vector<int> clause = {-b2, f1};
                isolver.addClause(clause);
                forcedClausesVec.push_back(std::move(clause));
                forcedClausesCount++;
            }
            {
                std::vector<int> clause = {-b1, f2};
                isolver.addClause(clause);
                forcedClausesVec.push_back(std::move(clause));
                forcedClausesCount++;
            }
            {
                std::vector<int> clause = {-f2, b1};
                isolver.addClause(clause);
                forcedClausesVec.push_back(std::move(clause));
                forcedClausesCount++;
            }
        }

        std::cerr << "c Preprocessing: graph has " << pp.getDegree2Vertices().size()
                  << " deg-2 vertices, " << pp.getTwoEdgeCuts().size()
                   << " 2-edge-cuts, added " << forcedClausesCount << " forced clauses\n";
    }
    // ---- END PREPROCESSING ----

    std::cerr << "c total variables: " << isolver.getNumVars() << "\n";
    std::cerr << "c total clauses: " << isolver.getNumClauses() << "\n";

    // ---- PHASE 0: Precomputed 2-EC block DFJ clauses ----
    int blockClauseCount = 0;
    if (precomputeBlocks_) {
        auto blocks = find2EdgeConnectedBlocks(g);
        for (const auto& block : blocks) {
            if ((int)block.size() >= g.getNodes()) continue; // not proper subset
            std::vector<bool> inBlock(g.getNodes(), false);
            for (int v : block) inBlock[v] = true;
            std::vector<int> clause;
            for (int u : block) {
                for (auto& [v, _] : g.getNeighbors(u)) {
                    if (!inBlock[v]) {
                        int lit = g.getAdj(u, v);
                        if (lit > 0) clause.push_back(-lit);
                    }
                }
            }
            if (clause.size() >= 2) {
                isolver.addClause(clause);
                blockClauseCount++;
            }
        }
        std::cerr << "c Phase 0: added " << blockClauseCount
                  << " DFJ clauses for " << blocks.size() << " 2-EC blocks\n";
    }

    // Reserve a pool of skip variables to avoid dynamic declaration clashes with CaDiCaL's BVA
    int baseVars = isolver.getNumVars();
    int maxSkipVars = g.getNodes();
    isolver.declareVariables(maxSkipVars);
    int skipVarStart = baseVars + 1;
    int usedSkipVars = 0;

    std::unique_ptr<TrajectoryLogger> tracer;
    if (!trajectoryFile.empty()) {
        tracer.reset(new TrajectoryLogger(trajectoryFile));
    }

    int actions = 0;
    std::vector<int> prevBlockedComponentIds;
    std::vector<int> prevEdges;
    auto startTime = std::chrono::steady_clock::now();

    std::vector<std::vector<int>> prevFingerprint;
    int stagnationCount = 0;
    bool escalated = false;
    std::string escalationResult = "";
    std::string stagnationStrategy = this->stagnationStrategy;
    int lowCompCount = 0;
    int prevComps = 0;

    SecEncoder iterationSecEncoder(g);
    iterationSecEncoder.startAuxAt(isolver.getNumVars() + 1);

    OscillationTracker oscillationTracker_(oscillationWindow_, cutThreshold_, 10);

    while (true) {
        {
            auto now = std::chrono::steady_clock::now();
            double totalTime = std::chrono::duration<double>(now - startTime).count();
            if (totalTime * 1000 >= timeLimitMs) {
                std::cerr << "c TIMEOUT (total)\n";
                std::cerr << "c incremental actions: " << actions << "\n";
                std::cerr << "c total variables: " << isolver.getNumVars() << "\n";
                std::cerr << "c total clauses: " << isolver.getNumClauses() << "\n";
                std::cerr << "c final solve time: " << isolver.getFinalSolveTime() << "\n";
                std::cerr << "c total solver time: " << isolver.getTotalSolverTime() << "\n";
                isolver.printStatistics();
                return SolveResult::TIMEOUT;
            }
        }
        actions++;
        auto result = isolver.solve();
        auto now = std::chrono::steady_clock::now();
        double totalTime = std::chrono::duration<double>(now - startTime).count();

        if (result == IncrementalSolver::Result::UNSAT) {
            std::cerr << "c UNSAT\n";
            std::cerr << "c incremental actions: " << actions << "\n";
            std::cerr << "c total variables: " << isolver.getNumVars() << "\n";
            std::cerr << "c total clauses: " << isolver.getNumClauses() << "\n";
            std::cerr << "c final solve time: " << isolver.getFinalSolveTime() << "\n";
            std::cerr << "c total solver time: " << isolver.getTotalSolverTime() << "\n";
            isolver.printStatistics();
            return SolveResult::UNSAT;
        }
        if (result == IncrementalSolver::Result::TIMEOUT) {
            std::cerr << "c TIMEOUT\n";
            std::cerr << "c incremental actions: " << actions << "\n";
            std::cerr << "c total variables: " << isolver.getNumVars() << "\n";
            std::cerr << "c total clauses: " << isolver.getNumClauses() << "\n";
            std::cerr << "c final solve time: " << isolver.getFinalSolveTime() << "\n";
            std::cerr << "c total solver time: " << isolver.getTotalSolverTime() << "\n";
            isolver.printStatistics();
            return SolveResult::TIMEOUT;
        }
        if (result == IncrementalSolver::Result::SAT) {
            auto model = isolver.getModel();
            auto components = SubtourDetector::detect(model, g);

            // Compute edge Jaccard similarity
            std::vector<int> currEdges;
            for (const auto& comp : components) {
                currEdges.insert(currEdges.end(), comp.edges.begin(), comp.edges.end());
            }
            std::sort(currEdges.begin(), currEdges.end());

            double jaccardSim = 0.0;
            if (!prevEdges.empty() && !currEdges.empty()) {
                std::vector<int> intersectionEdges;
                std::set_intersection(prevEdges.begin(), prevEdges.end(),
                                      currEdges.begin(), currEdges.end(),
                                      std::back_inserter(intersectionEdges));

                size_t unionSize = prevEdges.size() + currEdges.size() - intersectionEdges.size();
                if (unionSize > 0) {
                    jaccardSim = static_cast<double>(intersectionEdges.size()) / unionSize;
                }
            }

            // ----- STAGNATION DETECTION -----
            if (stagnationK > 0 && !components.empty()) {
                // Near convergence (<=2 comps): high Jaccard means steady progress,
                // not stagnation. Greedy escalation breaks the 2-comp pattern and
                // triggers 4↔2 oscillation on 3-regular graphs. Let SECs converge.
                bool isStagnant = !prevEdges.empty() && (components.size() > 4) && (jaccardSim >= 0.85);

                if (!isStagnant) {
                    stagnationCount = 0;
                    escalated = false;
                    escalationResult = "";
                } else {
                    stagnationCount++;
                    std::cerr << "c Stagnation count: " << stagnationCount
                              << "/" << stagnationK << " (Jaccard: " << jaccardSim << ")\n";

                    if (stagnationCount >= stagnationK) {
                        std::cerr << "c Stagnation detected! Escalating with strategy: "
                                  << stagnationStrategy << "\n";

                        if (stagnationStrategy == "dfj") {
                            int addedCount = 0;
                            for (const auto& comp : components) {
                                if (comp.edges.empty()) continue;
                                std::vector<int> clause;
                                clause.reserve(comp.edges.size());
                                for (int e : comp.edges) {
                                    clause.push_back(-e);
                                }
                                isolver.addClause(clause);
                                addedCount++;
                            }
                            std::cerr << "c Escalation (DFJ): Added " << addedCount << " cycle-blocking clauses\n";
                            escalationResult = "dfj_added";
                            stagnationCount = 0;
                        } 
                        else if (stagnationStrategy == "union") {
                            int addedCount = 0;
                            iterationSecEncoder.startAuxAt(isolver.getNumVars() + 1);
                            
                            int P = std::min(3, static_cast<int>(components.size()));
                            for (int a = 0; a < P; ++a) {
                                for (int b = a + 1; b < P; ++b) {
                                    Component unionComp;
                                    unionComp.vertices = components[a].vertices;
                                    unionComp.vertices.insert(unionComp.vertices.end(), 
                                                              components[b].vertices.begin(), 
                                                              components[b].vertices.end());
                                    
                                    if (unionComp.vertices.size() >= static_cast<size_t>(g.getNodes())) continue;

                                    auto unionClauses = iterationSecEncoder.encodeSecs({unionComp}, useVertexSep_, vtxSepThreshold_, skipVertexDisjoint_);
                                    for (const auto& clause : unionClauses) {
                                        isolver.addClause(clause);
                                        addedCount++;
                                    }
                                }
                            }
                            std::cerr << "c Escalation (Union): Added " << addedCount << " union SEC clauses\n";
                            escalationResult = "union_added";
                            stagnationCount = 0;
                        }
                        else if (stagnationStrategy == "both") {
                            int addedDfj = 0;
                            for (const auto& comp : components) {
                                if (comp.edges.empty()) continue;
                                std::vector<int> clause;
                                clause.reserve(comp.edges.size());
                                for (int e : comp.edges) {
                                    clause.push_back(-e);
                                }
                                isolver.addClause(clause);
                                addedDfj++;
                            }
                            
                            int addedUnion = 0;
                            iterationSecEncoder.startAuxAt(isolver.getNumVars() + 1);
                            
                            int P = std::min(3, static_cast<int>(components.size()));
                            for (int a = 0; a < P; ++a) {
                                for (int b = a + 1; b < P; ++b) {
                                    Component unionComp;
                                    unionComp.vertices = components[a].vertices;
                                    unionComp.vertices.insert(unionComp.vertices.end(), 
                                                              components[b].vertices.begin(), 
                                                              components[b].vertices.end());
                                    
                                    if (unionComp.vertices.size() >= static_cast<size_t>(g.getNodes())) continue;

                                    auto unionClauses = iterationSecEncoder.encodeSecs({unionComp}, useVertexSep_, vtxSepThreshold_, skipVertexDisjoint_);
                                    for (const auto& clause : unionClauses) {
                                        isolver.addClause(clause);
                                        addedUnion++;
                                    }
                                }
                            }
                            std::cerr << "c Escalation (Both): Added " << addedDfj << " DFJ and " << addedUnion << " union SEC clauses\n";
                            escalationResult = "both_added";
                            stagnationCount = 0;
                        }
                        else if (stagnationStrategy == "mincut") {
                            MinCutResult mcr = computeComponentMinCut(components, g);
                            int addedCount = 0;

                            if (!mcr.sideA_vertices.empty()) {
                                // Build a synthetic Component for SecEncoder
                                Component cutComp;
                                cutComp.vertices = mcr.sideA_vertices;
                                // Edges field not needed for encodeSecs (only vertices used for cut-set)

                                iterationSecEncoder.startAuxAt(isolver.getNumVars() + 1);
                                auto secClauses = iterationSecEncoder.encodeSecs({cutComp}, useVertexSep_, vtxSepThreshold_, skipVertexDisjoint_);
                                for (const auto& clause : secClauses) {
                                    isolver.addClause(clause);
                                    addedCount++;
                                }
                                std::cerr << "c Escalation (MinCut): cut size " << mcr.cutSize
                                          << ", added " << addedCount << " SEC clauses for "
                                          << mcr.sideA_vertices.size() << " vertices\n";
                                escalationResult = "mincut_added";
                                stagnationCount = 0;
                            } else {
                                std::cerr << "c Escalation (MinCut): no useful cut found, falling back\n";
                                // Fall through to greedy below
                                if (runGreedyBlocking(components, isolver, g, prevFingerprint,
                                                      prevBlockedComponentIds, usedSkipVars,
                                                      skipVarStart, maxSkipVars,
                                                       iterationSecEncoder, useVertexSep_, vtxSepThreshold_)) {
                                    escalationResult = "partition_changed";
                                    if (tracer) {
                                        tracer->logIteration(actions, actions, isolver.getFinalSolveTime(),
                                                             totalTime, 0, 0, 0,
                                                             components, std::vector<int>(), prevBlockedComponentIds,
                                                             stagnationCount, escalated,
                                                             stagnationStrategy, escalationResult);
                                    }
                                    prevEdges = std::move(currEdges);
                                    continue;
                                } else {
                                    escalationResult = "failed";
                                }
                            }
                        }
                        else {
                            // Fallback to greedy blocking
                            if (runGreedyBlocking(components, isolver, g, prevFingerprint, prevBlockedComponentIds,
                                                  usedSkipVars, skipVarStart, maxSkipVars,
                                                  iterationSecEncoder, useVertexSep_, vtxSepThreshold_)) {
                                escalationResult = "partition_changed";
                                if (tracer) {
                                    tracer->logIteration(actions, actions, isolver.getFinalSolveTime(),
                                                         totalTime, 0, 0, 0,
                                                         components, std::vector<int>(), prevBlockedComponentIds,
                                                         stagnationCount, escalated,
                                                         stagnationStrategy, escalationResult);
                                }
                                prevEdges = std::move(currEdges);
                                continue; 
                            } else {
                                escalationResult = "failed";
                            }
                        }
                    }
                }
            }

            // ----- TRACER LOG -----
            if (tracer) {
                std::vector<int> modelEdgeVars;
                int numVars = isolver.getNumVars();
                for (int v = 1; v <= numVars; ++v) {
                    if (isolver.getModelValue(v) > 0) {
                        modelEdgeVars.push_back(v);
                    }
                }
                tracer->logIteration(actions, actions, isolver.getFinalSolveTime(),
                                     totalTime, 0, 0, 0,
                                     components, modelEdgeVars, prevBlockedComponentIds,
                                     stagnationCount, escalated,
                                     stagnationStrategy, escalationResult);
            }

            // ----- HAMILTONIAN CHECK / SEC ADDITION -----
            if (components.empty()) {
                // HAMILTONIAN found — unchanged from original
                std::cerr << "c HAMILTONIAN found\n";
                std::string solFile = "solution.sat";
                std::ofstream solOut(solFile);
                if (!solOut.is_open() || solOut.fail()) {
                    std::cerr << "c Error: Could not write solution to " << solFile << "\n";
                    return SolveResult::ERROR;
                }
                solOut << "s SATISFIABLE\nv ";
                for (int var = 1; var <= isolver.getNumVars(); ++var) {
                    int val = isolver.getModelValue(var);
                    if (val > 0) {
                        solOut << var << " ";
                    } else if (val < 0) {
                        solOut << -var << " ";
                    }
                }
                solOut << "0\n";
                if (solOut.fail()) {
                    std::cerr << "c Error: Failed while writing solution to " << solFile << "\n";
                    solOut.close();
                    return SolveResult::ERROR;
                }
                solOut.close();

                if (tracer) {
                    std::vector<int> cycle;
                    Graph& rg = g;
                    int n = rg.getNodes();
                    int first = 0;
                    for (int i = 0; i < n; ++i) {
                        cycle.push_back(first);
                        for (auto& [next, edgeIdx] : rg.getNeighbors(first)) {
                            if (edgeIdx < static_cast<int>(model.size()) && model[edgeIdx] > 0) {
                                first = next;
                                break;
                            }
                        }
                    }
                    tracer->logHamiltonian(cycle);
                }

                std::cerr << "c incremental actions: " << actions << "\n";
                std::cerr << "c total variables: " << isolver.getNumVars() << "\n";
                std::cerr << "c total clauses: " << isolver.getNumClauses() << "\n";
                std::cerr << "c final solve time: " << isolver.getFinalSolveTime() << "\n";
                std::cerr << "c total solver time: " << isolver.getTotalSolverTime() << "\n";
                isolver.printStatistics();
                return SolveResult::HAMILTONIAN;
            } else {
                std::vector<int> currentComponentIds;
                for (size_t i = 0; i < components.size(); ++i) {
                    currentComponentIds.push_back(static_cast<int>(i));
                }
                prevBlockedComponentIds = std::move(currentComponentIds);

                // ---- Phase 1: oscillation-guided cut escalation ----
                {
                    int oscClausesAdded = 0;
                    for (const auto& comp : components) {
                        if ((int)comp.vertices.size() < oscillationTracker_.minCutThreshold)
                            continue;

                        uint64_t hash = 0;
                        for (int v : comp.vertices) {
                            hash ^= std::hash<int>{}(v) + 0x9e3779b9 + (hash << 6) + (hash >> 2);
                        }

                        if (oscillationTracker_.isOscillating(hash, actions)) {
                            const int maxFlowVertLimit = 2000;
                            auto mcr = computeInternalMinCut(comp, g, maxFlowVertLimit);
                            if (mcr.cutSize >= 2 && mcr.cutSize <= oscillationTracker_.maxCutSize
                                && !mcr.sideA_vertices.empty()
                                && (int)mcr.sideA_vertices.size() < (int)comp.vertices.size())
                            {
                                auto clause = buildBoundaryClause(mcr.sideA_vertices, g);
                                if ((int)clause.size() >= 2) {
                                    isolver.addClause(clause);
                                    oscClausesAdded++;
                                }
                            }
                        }

                        oscillationTracker_.record(hash, actions);
                    }
                    if (oscClausesAdded > 0) {
                        std::cerr << "c Iteration: oscillation cut added for "
                                  << oscClausesAdded << " components\n";
                    }
                }

                iterationSecEncoder.startAuxAt(isolver.getNumVars() + 1);
                auto secClauses = iterationSecEncoder.encodeSecs(components, useVertexSep_, vtxSepThreshold_, skipVertexDisjoint_);

                // Algorithmic Improvement: Add union SECs for the smallest components to
                // force faster component merging. Only applies when many components remain;
                // near convergence (<=10 comps) union SECs cause oscillation between
                // component counts (e.g. 4↔2 cycle on 3-regular graphs).
                if (components.size() > 10) {
                    std::vector<Component> sortedComps = components;
                    std::sort(sortedComps.begin(), sortedComps.end(), [](const Component& a, const Component& b) {
                        return a.vertices.size() < b.vertices.size();
                    });
                    int P = std::min(4, static_cast<int>(sortedComps.size()));
                    for (int a = 0; a < P; ++a) {
                        for (int b = a + 1; b < P; ++b) {
                            Component unionComp;
                            unionComp.vertices = sortedComps[a].vertices;
                            unionComp.vertices.insert(unionComp.vertices.end(), 
                                                      sortedComps[b].vertices.begin(), 
                                                      sortedComps[b].vertices.end());
                            if (unionComp.vertices.size() >= static_cast<size_t>(g.getNodes())) continue;
                            auto unionClauses = iterationSecEncoder.encodeSecs({unionComp}, useVertexSep_, vtxSepThreshold_, skipVertexDisjoint_);
                            secClauses.insert(secClauses.end(), unionClauses.begin(), unionClauses.end());
                        }
                    }
                }

                for (const auto& clause : secClauses) {
                    isolver.addClause(clause);
                }

                // ----- LOW-COMPONENT DFJ PUSH (Phase B) -----
                // Only for small components (≤3 vertices): their DFJ clauses
                // (negating all 4-6 directed edges) are strong constraints that
                // force merging. For larger components, full DFJ clauses are
                // weak (thousands of literals) and partitioned DFJ is UNSOUND
                // — random groups of 6 edges may all be unchanged in a valid
                // Hamiltonian path through the component.
                {
                    int curComps = static_cast<int>(components.size());
                    if (curComps == prevComps) {
                        lowCompCount++;
                    } else {
                        lowCompCount = 0;
                    }
                    if (lowCompCount > 0 && lowCompCount % 10 == 0) {
                        if (curComps <= 4) {
                            std::cerr << "c Low-comp DFJ push (≤4 comps) at count=" << lowCompCount
                                      << ", comps=" << curComps << " — SKIPPED\n";
                        } else {
                            int addedCount = 0;
                            for (const auto& comp : components) {
                                if (comp.edges.empty()) continue;
                                if (comp.vertices.size() <= 3) {
                                    std::vector<int> dfjClause;
                                    dfjClause.reserve(comp.edges.size());
                                    for (int e : comp.edges) {
                                        dfjClause.push_back(-e);
                                    }
                                    isolver.addClause(dfjClause);
                                    addedCount++;
                                }
                            }
                            if (addedCount > 0) {
                                std::cerr << "c Low-comp DFJ push (>4 comps) at count=" << lowCompCount
                                          << ", comps=" << curComps << " — added " << addedCount << " small-component DFJ\n";
                            }
                        }
                    }
                    prevComps = curComps;
                }

                std::cerr << "c Iteration: found " << components.size()
                          << " components, added " << secClauses.size() << " SEC clauses\n";
                prevEdges = std::move(currEdges);
                prevFingerprint = computeFingerprint(components);
            }
        }
    }
}

void printHelp(const char* progName) {
    std::cout << "Usage: " << progName << " <graph.dimacs> [options]\n"
              << "Options:\n"
              << "  -c, --cycle <int|auto>  Cycle multiplier (default: 2, auto: 3*5*7*2^k > nNode, fallback to 2)\n"
              << "  -a, --amo <opt>         AtMostOne module: default, pblib\n"
              << "  -s, --start <opt>       Start node: min (min degree), max (max degree), first (node 0), or node index\n"
              << "  -b, --sym-break <opt>   Symmetry breaking module: default, none\n"
              << "  --no-symmetry           (Deprecated) Equivalent to -b none\n"
              << "  --incremental           Use incremental SAT solving with subtour detection\n"
              << "  --time-limit <sec>      Set solver time limit in seconds (default: 600)\n"
              << "  --trajectory <file>     Write per-iteration NDJSON trajectory trace\n"
               << "  --random <int>          Set random seed for SAT solver\n"
              << "  --stagnation-k <int>    Stagnation threshold (default: 3, 0=disable)\n"
              << "  --stagnation-strategy <opt>  Escalation: greedy (default), dfj, union, both, mincut\n"
              << "  --preprocess            Enable preprocessing (deg-2, 2-edge-cut forcing)\n"
              << "  --no-preprocess         Disable preprocessing\n"
              << "  --vertex-sep            Enable vertex-separator SEC (cardinality + vertex-disjoint)\n"
              << "  --vtx-sep-threshold <int>  |S| threshold for cardinality encoding (default: 4)\n"
              << "  --vtx-sep-card-only     Like --vertex-sep but skip vertex-disjoint clauses\n"
              << "  -h, --help              Show this help\n";
}

// Returns the 2-edge-connected components (blocks) of graph g.
// A block is a maximal subgraph without bridges.
std::vector<std::vector<int>> find2EdgeConnectedBlocks(const Graph& g) {
    int n = g.getNodes();
    // --- First pass: find all bridges ---
    std::vector<int> disc(n, -1), low(n, -1), parent(n, -1);
    std::vector<std::pair<int,int>> bridges;
    int timer = 0;
    std::function<void(int)> dfs = [&](int u) {
        disc[u] = low[u] = timer++;
        for (auto& [v, _] : g.getNeighbors(u)) {
            if (disc[v] == -1) {
                parent[v] = u;
                dfs(v);
                low[u] = std::min(low[u], low[v]);
                if (low[v] > disc[u]) {
                    int bu = std::min(u, v), bv = std::max(u, v);
                    bridges.push_back({bu, bv});
                }
            } else if (v != parent[u]) {
                low[u] = std::min(low[u], disc[v]);
            }
        }
    };
    for (int i = 0; i < n; ++i)
        if (disc[i] == -1) dfs(i);

    // --- Second pass: assign block IDs via DFS skipping bridges ---
    std::set<std::pair<int,int>> bridgeSet(bridges.begin(), bridges.end());
    std::vector<int> blockId(n, -1);
    int blockCount = 0;
    for (int i = 0; i < n; ++i) {
        if (blockId[i] >= 0) continue;
        std::vector<int> stack = {i};
        blockId[i] = blockCount;
        std::vector<int> vertices;
        while (!stack.empty()) {
            int u = stack.back(); stack.pop_back();
            vertices.push_back(u);
            for (auto& [v, _] : g.getNeighbors(u)) {
                if (blockId[v] >= 0) continue;
                int a = std::min(u, v), b = std::max(u, v);
                if (bridgeSet.count({a, b})) continue;
                blockId[v] = blockCount;
                stack.push_back(v);
            }
        }
        blockCount++;
    }

    std::vector<std::vector<int>> blocks(blockCount);
    for (int v = 0; v < n; ++v)
        blocks[blockId[v]].push_back(v);
    return blocks;
}

#ifndef TESTING

static int computeAutoScaleCycle(int nNode) {
    long long cycle = 2;
    if (cycle <= nNode) cycle *= 3;
    if (cycle <= nNode) cycle *= 5;
    if (cycle <= nNode) cycle *= 7;
    while (cycle <= nNode) cycle *= 2;
    if (cycle > static_cast<long long>(INT_MAX)) cycle = INT_MAX;
    return static_cast<int>(cycle);
}

int main(int argc, char** argv) {
    if (argc < 2) {
        printHelp(argv[0]);
        return 1;
    }

    // Check for -h/--help as the very first arg (before graph file)
    if (std::string(argv[1]) == "-h" || std::string(argv[1]) == "--help") {
        printHelp(argv[0]);
        return 0;
    }

    std::string graphFile = argv[1];
    Solver solver(graphFile);
    std::string solFile = "";
    bool incremental = false;
    int64_t timeLimitMs = 600000;

    for (int i = 2; i < argc; i++) {
        std::string arg = argv[i];
        if (arg == "-h" || arg == "--help") {
            printHelp(argv[0]);
            return 0;
        } else if (arg == "--incremental") {
            incremental = true;
        } else if (arg == "--time-limit") {
            if (i + 1 < argc) {
                std::string limitStr = argv[++i];
                try {
                    timeLimitMs = std::stoll(limitStr) * 1000;
                } catch (const std::exception& e) {
                    std::cerr << "Error: invalid time limit value \"" << limitStr << "\"\n";
                    return 1;
                }
            } else {
                std::cerr << "Error: --time-limit requires a value\n";
                return 1;
            }
        } else if (arg == "-c" || arg == "--cycle") {
            if (i + 1 < argc) {
                std::string cycleStr = argv[++i];
                if (cycleStr == "auto") {
                    solver.setCycle(0);
                } else {
                    try {
                        solver.setCycle(std::stoi(cycleStr));
                    } catch (const std::exception& e) {
                        std::cerr << "Error: invalid cycle value \"" << cycleStr << "\"\n";
                        return 1;
                    }
                }
            } else {
                std::cerr << "Error: -c/--cycle requires a value\n";
                return 1;
            }
        } else if (arg == "-a" || arg == "--amo") {
            if (i + 1 < argc) {
                std::string amoStr = argv[++i];
                if (amoStr == "pblib") {
                    solver.setAtMostOneOption(Solver::AtMostOneOption::PBLIB);
                } else if (amoStr == "default") {
                    solver.setAtMostOneOption(Solver::AtMostOneOption::DEFAULT);
                } else {
                    std::cerr << "Unknown AtMostOne option: " << amoStr << "\n";
                    return 1;
                }
            }
        } else if (arg == "-s" || arg == "--start") {
            if (i + 1 < argc) {
                std::string startStr = argv[++i];
                if (startStr == "min") {
                    solver.setStartNodeOption(Solver::StartNodeOption::MIN_DEGREE);
                } else if (startStr == "max") {
                    solver.setStartNodeOption(Solver::StartNodeOption::MAX_DEGREE);
                } else if (startStr == "first") {
                    solver.setStartNodeOption(Solver::StartNodeOption::FIRST_NODE);
                } else {
                    try {
                        solver.setStartNodeOption(Solver::StartNodeOption::SPECIFIC_NODE, std::stoi(startStr));
                    } catch (const std::exception& e) {
                        std::cerr << "Error: invalid start node value \"" << startStr << "\"\n";
                        return 1;
                    }
                }
            } else {
                std::cerr << "Error: -s/--start requires a value\n";
                return 1;
            }
        } else if (arg == "-b" || arg == "--sym-break") {
            if (i + 1 < argc) {
                std::string symStr = argv[++i];
                if (symStr == "default") {
                    solver.setSymmetryOption(Solver::SymmetryOption::DEFAULT);
                } else if (symStr == "none") {
                    solver.setSymmetryOption(Solver::SymmetryOption::NONE);
                } else {
                    std::cerr << "Unknown symmetry option: " << symStr << "\n";
                    return 1;
                }
            }
        } else if (arg == "--no-symmetry") {
            solver.setSymmetryOption(Solver::SymmetryOption::NONE);
        } else if (arg == "-d" || arg == "--decode") {
            if (i + 1 < argc) {
                solFile = argv[++i];
            }
        } else if (arg == "--trajectory") {
            if (i + 1 < argc) {
                solver.setTrajectoryFile(argv[++i]);
            } else {
                std::cerr << "Error: --trajectory requires a filename\n";
                return 1;
            }
        } else if (arg == "--random") {
            if (i + 1 < argc) {
                try {
                    solver.setRandomSeed(std::stoi(argv[++i]));
                } catch (const std::exception& e) {
                    std::cerr << "Error: invalid random seed value\n";
                    return 1;
                }
            } else {
                std::cerr << "Error: --random requires a value\n";
                return 1;
            }
        } else if (arg == "--stagnation-k") {
            if (i + 1 < argc) {
                try {
                    int k = std::stoi(argv[++i]);
                    solver.setStagnationK(k);
                } catch (const std::exception& e) {
                    std::cerr << "Error: invalid stagnation-k value \"" << argv[i] << "\"\n";
                    return 1;
                }
            } else {
                std::cerr << "Error: --stagnation-k requires an integer\n";
                return 1;
            }
        } else if (arg == "--stagnation-strategy") {
            if (i + 1 < argc) {
                solver.setStagnationStrategy(argv[++i]);
            } else {
                std::cerr << "Error: --stagnation-strategy requires a value\n";
                return 1;
            }
        } else if (arg == "--preprocess") {
            solver.setPreprocess(true);
        } else if (arg == "--no-preprocess") {
            solver.setPreprocess(false);
        } else if (arg == "--vertex-sep") {
            solver.setVertexSep(true);
        } else if (arg == "--vtx-sep-threshold") {
            if (i + 1 < argc) {
                solver.setVtxSepThreshold(std::stoi(argv[++i]));
            } else {
                std::cerr << "Error: --vtx-sep-threshold requires a value\n";
                return 1;
            }
        } else if (arg == "--vtx-sep-card-only") {
            solver.setVertexSep(true);
            solver.setSkipVertexDisjoint(true);
        }
    }

    if (!solFile.empty()) {
        HcpDecoder decoder(graphFile, solFile);
        decoder.decode();
        return 0;
    }

    if (incremental) {
        if (solver.getCycle() == 0) {
            // Load graph to compute auto-scaled cycle m = 3*5*7*2^k > nNode
            Graph g;
            int autoCycle = 2;
            if (g.loadFromFile(graphFile, true)) {
                int nNode = g.getNodes();
                autoCycle = computeAutoScaleCycle(nNode);
                std::cerr << "c Auto cycle: n=" << nNode << " m=" << autoCycle << "\n";
            } else {
                std::cerr << "c Auto cycle: could not load graph, using default\n";
            }

            // Phase 1: try auto-scaled cycle m > n (one-shot, no SEC loop).
            // Skip when m >= 3360 — formula too large (48K+ vars, 529K+ clauses
            // for graph470) for the 30s budget; goes straight to cycle=2 SEC loop.
            bool skipPhase1 = (autoCycle >= 3360);
            Solver::SolveResult result;
            if (!skipPhase1) {
                std::cerr << "c Auto cycle: trying m=" << autoCycle << " (30s budget, one-shot when m > n)\n";
                solver.setCycle(autoCycle);
                int64_t phase1Ms = std::min<int64_t>(30000, timeLimitMs * 30 / 100);
                result = solver.runIncremental(phase1Ms);
                if (result == Solver::SolveResult::HAMILTONIAN) {
                    std::cerr << "c Auto mode: solved with cycle=" << autoCycle << "\n";
                    return 0;
                }
                if (result == Solver::SolveResult::UNSAT) {
                    std::cerr << "c Auto mode: UNSAT with cycle=" << autoCycle << "\n";
                    return 1;
                }
                std::cerr << "c Auto cycle: m=" << autoCycle
                          << (result == Solver::SolveResult::TIMEOUT ? " TIMEOUT" : " ERROR")
                          << ", retrying with cycle=2 SEC loop\n";
            } else {
                std::cerr << "c Auto cycle: m=" << autoCycle << " >= 3360, skipping Phase 1 (would timeout)\n";
            }
            solver.setCycle(2);
            int64_t phase2Ms = skipPhase1 ? timeLimitMs : timeLimitMs - std::min<int64_t>(30000, timeLimitMs * 30 / 100);
            result = solver.runIncremental(phase2Ms);
            if (result == Solver::SolveResult::HAMILTONIAN) {
                std::cerr << "c Auto mode: solved with cycle=2\n";
                return 0;
            }
            if (result == Solver::SolveResult::UNSAT) {
                std::cerr << "c Auto mode: UNSAT with cycle=2\n";
                return 1;
            }
            std::cerr << "c Auto mode: cycle=2 " << (result == Solver::SolveResult::TIMEOUT ? "TIMEOUT" : "ERROR") << "\n";
            return 1;
        }
        auto result = solver.runIncremental(timeLimitMs);
        return (result == Solver::SolveResult::HAMILTONIAN) ? 0 : 1;
    } else {
        if (!solver.run()) {
            return 1;
        }
    }

    return 0;
}
#endif
