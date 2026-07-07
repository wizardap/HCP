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
#include <sstream>

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

    std::cerr << "c total variables: " << isolver.getNumVars() << "\n";
    std::cerr << "c total clauses: " << isolver.getNumClauses() << "\n";

    std::unique_ptr<TrajectoryLogger> tracer;
    if (!trajectoryFile.empty()) {
        tracer.reset(new TrajectoryLogger(trajectoryFile));
    }

    int actions = 0;
    std::vector<int> prevBlockedComponentIds;
    auto startTime = std::chrono::steady_clock::now();

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
                                     components, modelEdgeVars, prevBlockedComponentIds);
            }

            if (components.empty()) {
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
                    // Reconstruct cycle from model for the trace
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

                SecEncoder secEncoder(g);
                auto secClauses = secEncoder.encodeSecs(components);
                for (const auto& clause : secClauses) {
                    isolver.addClause(clause);
                }
                std::cerr << "c Iteration: found " << components.size()
                          << " components, added " << secClauses.size() << " SEC clauses\n";
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
              << "  -h, --help              Show this help\n";
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
