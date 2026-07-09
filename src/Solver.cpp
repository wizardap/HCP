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
    int maxSkipVars
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

                SecEncoder secEncoder(g);
                auto secClauses = secEncoder.encodeSecs(innerComps);
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

bool Solver::runIncremental(int64_t timeLimitMs) {
    Graph g;
    if (!g.loadFromFile(graphFile, true)) { // Pass true for directed edge indices mapping
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

    VariableManager vm(2 * g.getEdges() + 1);
    IncrementalSolver isolver(timeLimitMs);
    if (randomSeed > 0) {
        std::cerr << "c random seed: " << randomSeed << " (note: CaDiCaL seed not yet forwarded through IncrementalSolver)\n";
    }
    HcpEncoder encoder(g, cycle, *amo, *sym, sNode, vm);
    encoder.encodeBase(isolver);

    // ---- PREPROCESSING: Forced edges from degree-2 vertices and 2-edge-cuts ----
    if (preprocess_) {
        GraphPreprocessor pp(g);

        if (pp.hasBridge()) {
            std::cerr << "c Preprocessing: graph has a bridge — no Hamiltonian Cycle possible\n";
            return false;
        }

        int forcedClauses = 0;

        // Degree-2 vertices: both incident undirected edges must be selected
        for (int u : pp.getDegree2Vertices()) {
            for (auto& [v, _] : g.getNeighbors(u)) {
                int fwd = g.getAdj(u, v);
                int bwd = g.getAdj(v, u);
                if (fwd > 0 && bwd > 0) {
                    isolver.addClause({fwd, bwd}); // one of the two directions must be used
                    forcedClauses++;
                }
            }
        }

        // 2-edge-cuts: both edges must be selected, and directions must be opposite
        for (const auto& ep : pp.getTwoEdgeCuts()) {
            // Force edge 1: one direction must be used
            int fwd1 = g.getAdj(ep.u1, ep.v1);
            int bwd1 = g.getAdj(ep.v1, ep.u1);
            // Force edge 2: one direction must be used
            int fwd2 = g.getAdj(ep.u2, ep.v2);
            int bwd2 = g.getAdj(ep.v2, ep.u2);

            if (fwd1 <= 0 || bwd1 <= 0 || fwd2 <= 0 || bwd2 <= 0) continue;

            isolver.addClause({fwd1, bwd1});   // edge1 must be selected
            isolver.addClause({fwd2, bwd2});   // edge2 must be selected

            // Opposite directions: fwd1 ↔ bwd2, bwd1 ↔ fwd2
            // (fwd1 → bwd2): ¬fwd1 ∨ bwd2
            isolver.addClause({-fwd1, bwd2});
            // (bwd2 → fwd1): ¬bwd2 ∨ fwd1
            isolver.addClause({-bwd2, fwd1});
            // (bwd1 → fwd2): ¬bwd1 ∨ fwd2
            isolver.addClause({-bwd1, fwd2});
            // (fwd2 → bwd1): ¬fwd2 ∨ bwd1
            isolver.addClause({-fwd2, bwd1});

            forcedClauses += 6;
        }

        std::cerr << "c Preprocessing: added " << forcedClauses
                  << " forced clauses ("
                  << pp.getDegree2Vertices().size() << " deg-2 vertices, "
                  << pp.getTwoEdgeCuts().size() << " 2-edge-cuts)\n";
    }
    // ---- END PREPROCESSING ----

    std::cerr << "c total variables: " << isolver.getNumVars() << "\n";
    std::cerr << "c total clauses: " << isolver.getNumClauses() << "\n";

    // Reserve a pool of skip variables to avoid dynamic declaration clashes with CaDiCaL's BVA
    int baseVars = isolver.getNumVars();
    int maxSkipVars = g.getNodes() * 15;
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

    SecEncoder iterationSecEncoder(g);
    iterationSecEncoder.startAuxAt(isolver.getNumVars() + 1);

    while (true) {
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
            return false;
        }
        if (result == IncrementalSolver::Result::TIMEOUT) {
            std::cerr << "c TIMEOUT\n";
            std::cerr << "c incremental actions: " << actions << "\n";
            std::cerr << "c total variables: " << isolver.getNumVars() << "\n";
            std::cerr << "c total clauses: " << isolver.getNumClauses() << "\n";
            std::cerr << "c final solve time: " << isolver.getFinalSolveTime() << "\n";
            std::cerr << "c total solver time: " << isolver.getTotalSolverTime() << "\n";
            isolver.printStatistics();
            return false;
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
                bool isStagnant = !prevEdges.empty() && (jaccardSim >= 0.85);

                if (!isStagnant) {
                    stagnationCount = 0;
                    escalated = false;
                    escalationResult = "";
                } else {
                    stagnationCount++;
                    std::cerr << "c Stagnation count: " << stagnationCount
                              << "/" << stagnationK << " (Jaccard: " << jaccardSim << ")\n";

                    if (stagnationCount >= stagnationK && !escalated) {
                        escalated = true;
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
                            escalated = false;
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

                                    auto unionClauses = iterationSecEncoder.encodeSecs({unionComp}, useVertexSep_, vtxSepThreshold_);
                                    for (const auto& clause : unionClauses) {
                                        isolver.addClause(clause);
                                        addedCount++;
                                    }
                                }
                            }
                            std::cerr << "c Escalation (Union): Added " << addedCount << " union SEC clauses\n";
                            escalationResult = "union_added";
                            stagnationCount = 0;
                            escalated = false;
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

                                    auto unionClauses = iterationSecEncoder.encodeSecs({unionComp}, useVertexSep_, vtxSepThreshold_);
                                    for (const auto& clause : unionClauses) {
                                        isolver.addClause(clause);
                                        addedUnion++;
                                    }
                                }
                            }
                            std::cerr << "c Escalation (Both): Added " << addedDfj << " DFJ and " << addedUnion << " union SEC clauses\n";
                            escalationResult = "both_added";
                            stagnationCount = 0;
                            escalated = false;
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
                                auto secClauses = iterationSecEncoder.encodeSecs({cutComp}, useVertexSep_, vtxSepThreshold_);
                                for (const auto& clause : secClauses) {
                                    isolver.addClause(clause);
                                    addedCount++;
                                }
                                std::cerr << "c Escalation (MinCut): cut size " << mcr.cutSize
                                          << ", added " << addedCount << " SEC clauses for "
                                          << mcr.sideA_vertices.size() << " vertices\n";
                                escalationResult = "mincut_added";
                                stagnationCount = 0;
                                escalated = false;
                            } else {
                                std::cerr << "c Escalation (MinCut): no useful cut found, falling back\n";
                                // Fall through to greedy below
                                if (runGreedyBlocking(components, isolver, g, prevFingerprint,
                                                      prevBlockedComponentIds, usedSkipVars,
                                                      skipVarStart, maxSkipVars)) {
                                    escalationResult = "partition_changed";
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
                                                  usedSkipVars, skipVarStart, maxSkipVars)) {
                                escalationResult = "partition_changed";
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
                    return false;
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
                    return false;
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
                return true;
            } else {
                std::vector<int> currentComponentIds;
                for (size_t i = 0; i < components.size(); ++i) {
                    currentComponentIds.push_back(static_cast<int>(i));
                }
                prevBlockedComponentIds = std::move(currentComponentIds);

                iterationSecEncoder.startAuxAt(isolver.getNumVars() + 1);
                auto secClauses = iterationSecEncoder.encodeSecs(components, useVertexSep_, vtxSepThreshold_);

                // Algorithmic Improvement: Add union SECs for the smallest components in every iteration
                // to force faster component merging and reduce total iterations.
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
                        
                        auto unionClauses = iterationSecEncoder.encodeSecs({unionComp});
                        secClauses.insert(secClauses.end(), unionClauses.begin(), unionClauses.end());
                    }
                }

                for (const auto& clause : secClauses) {
                    isolver.addClause(clause);
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
              << "  -c, --cycle <int>       Cycle multiplier (default: 2)\n"
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
              << "  --preprocess            Enable forced-edge preprocessing (degree-2 and 2-edge-cuts)\n"
              << "  --vertex-sep            Enable vertex-separator SEC (cardinality + vertex-disjoint)\n"
              << "  --vtx-sep-threshold <int>  |S| threshold for cardinality encoding (default: 4)\n"
              << "  -h, --help              Show this help\n";
}

#ifndef TESTING
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
                try {
                    solver.setCycle(std::stoi(cycleStr));
                } catch (const std::exception& e) {
                    std::cerr << "Error: invalid cycle value \"" << cycleStr << "\"\n";
                    return 1;
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
        }
    }

    if (!solFile.empty()) {
        HcpDecoder decoder(graphFile, solFile);
        decoder.decode();
        return 0;
    }

    if (incremental) {
        if (!solver.runIncremental(timeLimitMs)) {
            return 1;
        }
    } else {
        if (!solver.run()) {
            return 1;
        }
    }

    return 0;
}
#endif
