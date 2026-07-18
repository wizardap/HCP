#ifndef INCREMENTAL_SOLVER_HPP
#define INCREMENTAL_SOLVER_HPP

#include <vector>
#include <istream>
#include <chrono>
#include <cstdint>
#include <climits>
#include <stdexcept>

// Forward declaration of the CaDiCaL wrapper struct
struct CCaDiCaL;

class IncrementalSolver {
public:
    enum class Result {
        SAT,
        UNSAT,
        TIMEOUT
    };

    enum class SolverState {
        UNSOLVED,
        SAT,
        UNSAT,
        TIMEOUT
    };

    explicit IncrementalSolver(int64_t timeLimitMs = 0);
    ~IncrementalSolver();

    // Disable copy and move operations to manage CaDiCaL resource safely
    IncrementalSolver(const IncrementalSolver&) = delete;
    IncrementalSolver& operator=(const IncrementalSolver&) = delete;
    IncrementalSolver(IncrementalSolver&&) = delete;
    IncrementalSolver& operator=(IncrementalSolver&&) = delete;

    // Adds a clause to the solver.
    void addClause(std::vector<int> const& clause);

    // Parses DIMACS CNF clauses from a std::istream and adds them directly,
    // ignoring comments (starting with 'c') and the 'p cnf' header.
    void addClausesFromStream(std::istream& in);
    
    // Solves the formula. Returns SAT, UNSAT, or TIMEOUT.
    Result solve();
    
    // Returns the value of a literal in the model (1 for true, -1 for false, 0 for unassigned).
    int getModelValue(int lit) const;

    // Returns a vector of literals representing the full model (1-indexed, size: maxVar + 1).
    std::vector<int> getModel() const;
    
    // Returns a partial model covering only variables 1..maxEdgeVar.
    // Useful when only edge variables are needed (avoids querying auxiliary variables).
    std::vector<int> getModel(int maxEdgeVar) const;
    
    // Returns the maximum variable index added or solved.
    int getNumVars() const;

    // Returns the total number of clauses added.
    int64_t getNumClauses() const;

    // NEW: Get timing statistics in seconds
    double getFinalSolveTime() const;
    double getTotalSolverTime() const;

    // NEW: Print CaDiCaL statistics
    void printStatistics() const;

    // Add an assumption literal for the next solve call.
    void addAssumption(int lit);

    // Set the preferred phase of a literal.
    void phase(int lit);

    // Clear the preferred phase of a literal.
    void unphase(int lit);

    // After solve returned UNSAT, check if a specific assumption caused it.
    // Returns true if the assumption literal was the reason for UNSAT.
    bool didAssumptionFail(int lit) const;

    // Declare one fresh variable, returns its index. Updates max_var.
    int declareVariable();

    // Pre-declare multiple fresh variables. Updates max_var.
    void declareVariables(int count);

    // Sets the execution time limit in milliseconds.
    void setTimeLimit(int64_t ms);

    // Reset: release the old solver and create a fresh one (all clauses lost).
    void reset(int64_t timeLimitMs = 0);

    // Callback helper for timeout (exposed for C-linkage callback)
    static int terminateCallback(void* state);

private:
    CCaDiCaL* solver = nullptr;
    int max_var = 0;
    int64_t numClauses = 0;
    int64_t timeLimitMs = 0;
    std::chrono::steady_clock::time_point startTime;
    SolverState state = SolverState::UNSOLVED;

    // NEW: Timings tracking in seconds
    double finalSolveTime = 0.0;
    double totalSolverTime = 0.0;

    bool checkTimeout() const;
};

#endif // INCREMENTAL_SOLVER_HPP
