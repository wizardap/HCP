#ifndef NOSYMMETRYBREAKER_HPP
#define NOSYMMETRYBREAKER_HPP

#include "ISymmetryBreaker.hpp"

class NoSymmetryBreaker : public ISymmetryBreaker {
public:
    int getNumClauses(const Graph& graph, int startNode, const std::vector<int>& neighbors) override {
        return 0;
    }

    void encode(const Graph& graph, int startNode, const std::vector<int>& neighbors) override {
        // Do nothing
    }
};

#endif
