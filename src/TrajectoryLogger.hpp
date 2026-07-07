#ifndef TRAJECTORY_LOGGER_HPP
#define TRAJECTORY_LOGGER_HPP

#include <string>
#include <fstream>
#include <vector>
#include "SubtourDetector.hpp"

class TrajectoryLogger {
public:
    TrajectoryLogger(const std::string& filename);
    ~TrajectoryLogger();

    void logIteration(int iteration, int action, double solveTimeSec, double totalTimeSec,
                      int64_t conflicts, int64_t decisions, int64_t propagations,
                      const std::vector<Component>& components,
                      const std::vector<int>& modelEdgeVars,
                      const std::vector<int>& blockedComponentIds,
                      int stagnationCount = 0,
                      bool escalated = false,
                      const std::string& escalationStrategy = "",
                      const std::string& escalationResult = "");

    void logHamiltonian(const std::vector<int>& cycle);

    bool isOpen() const { return file_.is_open(); }

private:
    std::ofstream file_;

    void writeJsonString(std::ostream& os, const std::string& s);
    void writeJsonIntArray(std::ostream& os, const std::vector<int>& arr);
    void writeComponentArray(std::ostream& os, const std::vector<Component>& components);
};

#endif
