#ifndef SEC_ENCODER_HPP
#define SEC_ENCODER_HPP

#include <vector>
#include "SubtourDetector.hpp" // For Component struct

class Graph;

class SecEncoder {
public:
    explicit SecEncoder(const Graph& graph);
    
    // Returns SEC clauses for all components (2 clauses per component)
    std::vector<std::vector<int>> encodeSecs(const std::vector<Component>& components);
    
private:
    const Graph& graph_;
    
    // For directed outgoing cut: Σ x_{u,v} ≥ 1 where u∈S, v∉S
    std::vector<int> getOutgoingLiterals(const Component& component);
    // For directed incoming cut: Σ x_{u,v} ≥ 1 where u∉S, v∈S
    std::vector<int> getIncomingLiterals(const Component& component);
};

#endif // SEC_ENCODER_HPP
