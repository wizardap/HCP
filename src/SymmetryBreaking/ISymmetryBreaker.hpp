#ifndef ISYMMETRYBREAKER_HPP
#define ISYMMETRYBREAKER_HPP

#include <vector>
#include "../Graph.hpp"

class ISymmetryBreaker {
public:
    virtual ~ISymmetryBreaker() = default;
    
    // Returns the number of clauses it will generate.
    virtual int getNumClauses(const Graph& graph, int startNode, const std::vector<int>& neighbors) = 0;
    
    // Encodes the symmetry-breaking clauses.
    virtual void encode(const Graph& graph, int startNode, const std::vector<int>& neighbors) = 0;
};

#endif
