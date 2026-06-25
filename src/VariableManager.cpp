#include "VariableManager.hpp"

VariableManager::VariableManager(int32_t firstFree) : nextVar_(firstFree) {}

int32_t VariableManager::newVar() {
    if (!freeVars_.empty()) {
        auto it = freeVars_.begin();
        int32_t v = *it;
        freeVars_.erase(it);
        return v;
    }
    return nextVar_++;
}

void VariableManager::freeVar(int32_t v) {
    if (v < nextVar_) {
        freeVars_.insert(v);
    }
}

void VariableManager::freeVars(int32_t start, int32_t end) {
    for (int32_t v = start; v <= end; ++v) {
        freeVar(v);
    }
}

int32_t VariableManager::getMaxVar() const {
    return nextVar_ - 1;
}

void VariableManager::resetTo(int32_t newFirstFree) {
    nextVar_ = newFirstFree;
    freeVars_.clear();
}
