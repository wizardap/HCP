#include "IncrementalSolver.hpp"
#include "ccadical.h"
#include <cmath>
#include <algorithm>
#include <string>
#include <sstream>

// Forward declaration of C-linkage callback
extern "C" int ccadical_terminate_callback(void* state);

IncrementalSolver::IncrementalSolver(int64_t timeLimitMs) : timeLimitMs(timeLimitMs) {
    solver = ccadical_init();
    if (!solver) {
        throw std::runtime_error("Failed to initialize CaDiCaL solver: ccadical_init returned nullptr");
    }
    max_var = 0;
    state = SolverState::UNSOLVED;
}

IncrementalSolver::~IncrementalSolver() {
    if (solver) {
        ccadical_release(solver);
        solver = nullptr;
    }
}

void IncrementalSolver::addClause(std::vector<int> const& clause) {
    state = SolverState::UNSOLVED;
    numClauses++;
    for (int lit : clause) {
        if (lit == INT_MIN) {
            throw std::out_of_range("Literal index cannot be INT_MIN");
        }
        int abs_lit = std::abs(lit);
        if (abs_lit > max_var) {
            int diff = abs_lit - max_var;
            ccadical_declare_more_variables(solver, diff);
            max_var = abs_lit;
        }
        ccadical_add(solver, lit);
    }
    ccadical_add(solver, 0);
}

void IncrementalSolver::addClausesFromStream(std::istream& in) {
    state = SolverState::UNSOLVED;
    std::string line;
    while (std::getline(in, line)) {
        size_t first_non_ws = line.find_first_not_of(" \t\r\n");
        if (first_non_ws == std::string::npos) continue;
        char first_char = line[first_non_ws];
        if (first_char == 'c' || first_char == 'p') {
            continue;
        }
        std::stringstream ss(line);
        int lit;
        while (true) {
            ss >> lit;
            if (ss.fail()) {
                if (ss.eof()) {
                    break;
                }
                throw std::runtime_error("Malformed DIMACS input: failed to parse integer in line: \"" + line + "\"");
            }
            if (lit == INT_MIN) {
                throw std::out_of_range("Literal index cannot be INT_MIN");
            }
            int abs_lit = std::abs(lit);
            if (abs_lit > max_var) {
                int diff = abs_lit - max_var;
                ccadical_declare_more_variables(solver, diff);
                max_var = abs_lit;
            }
            ccadical_add(solver, lit);
            if (lit == 0) {
                numClauses++;
            }
        }
    }
}

IncrementalSolver::Result IncrementalSolver::solve() {
    startTime = std::chrono::steady_clock::now();
    if (timeLimitMs > 0) {
        ccadical_set_terminate(solver, this, ccadical_terminate_callback);
    } else {
        ccadical_set_terminate(solver, nullptr, nullptr);
    }

    auto solveStart = std::chrono::steady_clock::now();
    int res = ccadical_solve(solver);
    auto solveEnd = std::chrono::steady_clock::now();

    // Clean up callback after solving
    ccadical_set_terminate(solver, nullptr, nullptr);

    double duration = std::chrono::duration<double>(solveEnd - solveStart).count();
    finalSolveTime = duration;
    totalSolverTime += duration;

    if (res == 10) {
        state = SolverState::SAT;
        return Result::SAT;
    } else if (res == 20) {
        state = SolverState::UNSAT;
        return Result::UNSAT;
    } else {
        state = SolverState::TIMEOUT;
        return Result::TIMEOUT;
    }
}

int IncrementalSolver::getModelValue(int lit) const {
    if (state != SolverState::SAT) {
        throw std::logic_error("Cannot query model value: solver is not in SAT state");
    }
    if (lit == 0 || lit == INT_MIN) return 0;
    int abs_lit = std::abs(lit);
    if (abs_lit > getNumVars()) return 0;

    int v = ccadical_val(solver, lit);
    if (v == 0) return 0;
    return ((v > 0) == (lit > 0)) ? 1 : -1;
}

std::vector<int> IncrementalSolver::getModel() const {
    if (state != SolverState::SAT) {
        throw std::logic_error("Cannot get model: solver is not in SAT state");
    }
    int maxVar = getNumVars();
    std::vector<int> model(maxVar + 1, 0);
    for (int i = 1; i <= maxVar; ++i) {
        int val = getModelValue(i);
        if (val == 1) {
            model[i] = i;
        } else if (val == -1) {
            model[i] = -i;
        } else {
            model[i] = 0;
        }
    }
    return model;
}

int IncrementalSolver::getNumVars() const {
    if (solver) {
        return ccadical_vars(solver);
    }
    return max_var;
}

int64_t IncrementalSolver::getNumClauses() const {
    return numClauses;
}

double IncrementalSolver::getFinalSolveTime() const {
    return finalSolveTime;
}

double IncrementalSolver::getTotalSolverTime() const {
    return totalSolverTime;
}

void IncrementalSolver::printStatistics() const {
    ccadical_print_statistics(solver);
}

void IncrementalSolver::setTimeLimit(int64_t ms) {
    timeLimitMs = ms;
}

bool IncrementalSolver::checkTimeout() const {
    if (timeLimitMs <= 0) return false;
    auto now = std::chrono::steady_clock::now();
    auto elapsed = std::chrono::duration_cast<std::chrono::milliseconds>(now - startTime).count();
    return elapsed >= timeLimitMs;
}

int IncrementalSolver::terminateCallback(void* state) {
    if (!state) return 0;
    auto* self = static_cast<IncrementalSolver*>(state);
    return self->checkTimeout() ? 1 : 0;
}

// C-linkage callback definition
extern "C" int ccadical_terminate_callback(void* state) {
    return IncrementalSolver::terminateCallback(state);
}
