#ifndef VARIABLE_MANAGER_HPP
#define VARIABLE_MANAGER_HPP

#include <cstdint>
#include <unordered_set>

class VariableManager {
public:
    explicit VariableManager(int32_t firstFree = 1);
    int32_t newVar();                    // Allocate fresh variable
    void freeVar(int32_t v);             // Recycle variable
    void freeVars(int32_t start, int32_t end);
    int32_t getMaxVar() const;
    void resetTo(int32_t newFirstFree);  // For testing
private:
    int32_t nextVar_;
    std::unordered_set<int32_t> freeVars_;
};

#endif // VARIABLE_MANAGER_HPP
