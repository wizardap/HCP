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
    
    // Returns the maximum variable index added or solved.
    int getNumVars() const;

    // Returns the total number of clauses added.
    int64_t getNumClauses() const;

    // Sets the execution time limit in milliseconds.
    void setTimeLimit(int64_t ms);

    // Callback helper for timeout (exposed for C-linkage callback)
    static int terminateCallback(void* state);

private:
    CCaDiCaL* solver = nullptr;
    int max_var = 0;
    int64_t numClauses = 0;
    int64_t timeLimitMs = 0;
    std::chrono::steady_clock::time_point startTime;
    SolverState state = SolverState::UNSOLVED;

    bool checkTimeout() const;
};

#endif // INCREMENTAL_SOLVER_HPP
