#include "SecEncoder.hpp"
#include "Graph.hpp"

SecEncoder::SecEncoder(const Graph& graph) : graph_(graph) {}

std::vector<std::vector<int>> SecEncoder::encodeSecs(const std::vector<Component>& components) {
    std::vector<std::vector<int>> clauses;
    for (const auto& component : components) {
        std::vector<int> outgoing = getOutgoingLiterals(component);
        if (!outgoing.empty()) {
            clauses.push_back(std::move(outgoing));
        }
        std::vector<int> incoming = getIncomingLiterals(component);
        if (!incoming.empty()) {
            clauses.push_back(std::move(incoming));
        }
    }
    return clauses;
}

std::vector<int> SecEncoder::getOutgoingLiterals(const Component& component) {
    int numNodes = graph_.getNodes();
    std::vector<int> validVertices;
    validVertices.reserve(component.vertices.size());
    
    std::vector<bool> inComponent(numNodes, false);
    int totalDegree = 0;
    for (int u : component.vertices) {
        if (u >= 0 && u < numNodes) {
            inComponent[u] = true;
            totalDegree += graph_.getDegree(u);
            validVertices.push_back(u);
        }
    }
    
    std::vector<int> literals;
    literals.reserve(totalDegree);
    
    for (int u : validVertices) {
        for (int v = 0; v < numNodes; ++v) {
            if (!inComponent[v]) {
                int edgeVar = graph_.getAdj(u, v);
                if (edgeVar > 0) {
                    literals.push_back(edgeVar);
                }
            }
        }
    }
    return literals;
}

std::vector<int> SecEncoder::getIncomingLiterals(const Component& component) {
    int numNodes = graph_.getNodes();
    std::vector<int> validVertices;
    validVertices.reserve(component.vertices.size());
    
    std::vector<bool> inComponent(numNodes, false);
    int totalDegree = 0;
    for (int v : component.vertices) {
        if (v >= 0 && v < numNodes) {
            inComponent[v] = true;
            totalDegree += graph_.getDegree(v);
            validVertices.push_back(v);
        }
    }
    
    std::vector<int> literals;
    literals.reserve(totalDegree);
    
    for (int u = 0; u < numNodes; ++u) {
        if (!inComponent[u]) {
            for (int v : validVertices) {
                int edgeVar = graph_.getAdj(u, v);
                if (edgeVar > 0) {
                    literals.push_back(edgeVar);
                }
            }
        }
    }
    return literals;
}
