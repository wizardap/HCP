#ifndef SEC_ENCODER_HPP
#define SEC_ENCODER_HPP

#include <vector>
#include "SubtourDetector.hpp" // For Component struct

class Graph;

class SecEncoder {
public:
    explicit SecEncoder(const Graph& graph);
    
    // Returns SEC clauses for all components.
    // If useVertexSep, applies cardinality encoding for components
    // with small vertex boundary (|S| <= vtxSepThreshold).
    // Uses nextAuxBase_ internally and advances it for each call.
    std::vector<std::vector<int>> encodeSecs(
        const std::vector<Component>& components,
        bool useVertexSep = false,
        int vtxSepThreshold = 4,
        bool skipVertexDisjoint = false
    );
    
    // Reset internal aux variable counter (call when solver's max_var has been updated)
    void startAuxAt(int base);
    int getNextAuxBase() const { return nextAuxBase_; }
    
private:
    const Graph& graph_;
    int nextAuxBase_;
    
    // For directed outgoing cut: Σ x_{u,v} ≥ 1 where u∈S, v∉S
    std::vector<int> getOutgoingLiterals(const Component& component);
    // For directed incoming cut: Σ x_{u,v} ≥ 1 where u∉S, v∈S
    std::vector<int> getIncomingLiterals(const Component& component);
};

#endif // SEC_ENCODER_HPP
