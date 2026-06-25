#ifndef DEFAULTSYMMETRYBREAKER_HPP
#define DEFAULTSYMMETRYBREAKER_HPP

#include "ISymmetryBreaker.hpp"
#include <iostream>

class DefaultSymmetryBreaker : public ISymmetryBreaker {
public:
    int getNumClauses(const Graph& graph, int startNode, const std::vector<int>& neighbors) override {
        return neighbors.size();
    }

    void encode(const Graph& graph, int startNode, const std::vector<int>& neighbors) override {
        for (size_t i = 0; i < neighbors.size(); i++) {
            for (size_t j = 0; j < i; j++) {
                std::cout << graph.getAdj(startNode, neighbors[j]) << " ";
            }
            std::cout << "-" << graph.getAdj(neighbors[i], startNode) << " 0\n";
        }
    }
};

#endif
