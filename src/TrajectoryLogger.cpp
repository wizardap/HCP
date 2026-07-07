#include "TrajectoryLogger.hpp"
#include <iostream>
#include <sstream>

TrajectoryLogger::TrajectoryLogger(const std::string& filename) {
    file_.open(filename);
    if (!file_.is_open()) {
        std::cerr << "c Error: could not open trajectory file " << filename << "\n";
    }
}

TrajectoryLogger::~TrajectoryLogger() {
    if (file_.is_open()) {
        file_.close();
    }
}

void TrajectoryLogger::writeJsonString(std::ostream& os, const std::string& s) {
    os << '"';
    for (char c : s) {
        if (c == '"' || c == '\\') os << '\\';
        os << c;
    }
    os << '"';
}

void TrajectoryLogger::writeJsonIntArray(std::ostream& os, const std::vector<int>& arr) {
    os << '[';
    for (size_t i = 0; i < arr.size(); ++i) {
        if (i > 0) os << ',';
        os << arr[i];
    }
    os << ']';
}

void TrajectoryLogger::writeComponentArray(std::ostream& os, const std::vector<Component>& components) {
    os << '[';
    for (size_t i = 0; i < components.size(); ++i) {
        if (i > 0) os << ',';
        os << "{\"id\":" << i
           << ",\"size\":" << components[i].vertices.size()
           << ",\"vertices\":";
        writeJsonIntArray(os, components[i].vertices);
        os << ",\"edges\":";
        writeJsonIntArray(os, components[i].edges);
        os << '}';
    }
    os << ']';
}

void TrajectoryLogger::logIteration(int iteration, int action, double solveTimeSec, double totalTimeSec,
                                     int64_t conflicts, int64_t decisions, int64_t propagations,
                                     const std::vector<Component>& components,
                                     const std::vector<int>& modelEdgeVars,
                                     const std::vector<int>& blockedComponentIds) {
    if (!file_.is_open()) return;

    std::ostringstream row;
    row << "{\"iteration\":" << iteration
        << ",\"action\":" << action
        << ",\"solve_time_s\":" << solveTimeSec
        << ",\"total_time_s\":" << totalTimeSec
        << ",\"solver_conflicts\":" << conflicts
        << ",\"solver_decisions\":" << decisions
        << ",\"solver_propagations\":" << propagations
        << ",\"components\":";
    writeComponentArray(row, components);
    row << ",\"model_edge_vars\":";
    writeJsonIntArray(row, modelEdgeVars);
    row << ",\"blocked_component_ids\":";
    writeJsonIntArray(row, blockedComponentIds);
    row << "}\n";

    file_ << row.str();
}

void TrajectoryLogger::logHamiltonian(const std::vector<int>& cycle) {
    if (!file_.is_open()) return;

    std::ostringstream row;
    row << "{\"iteration\":-1,\"action\":-1,\"hamiltonian\":true,\"cycle\":";
    writeJsonIntArray(row, cycle);
    row << "}\n";

    file_ << row.str();
}
