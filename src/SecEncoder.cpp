#include "SecEncoder.hpp"
#include "AtLeastK/DefaultAtLeastK.hpp"
#include "Graph.hpp"

SecEncoder::SecEncoder(const Graph& graph) : graph_(graph), nextAuxBase_(0) {}

void SecEncoder::startAuxAt(int base) {
    nextAuxBase_ = base;
}

std::vector<std::vector<int>> SecEncoder::encodeSecs(
    const std::vector<Component>& components,
    bool useVertexSep,
    int vtxSepThreshold,
    bool skipVertexDisjoint)
{
    std::vector<std::vector<int>> clauses;
    int numNodes = graph_.getNodes();

    if (nextAuxBase_ <= 0) {
        nextAuxBase_ = graph_.getEdges() * 2 + 2;
    }
    int& globalAuxBase = nextAuxBase_;
    for (const auto& component : components) {
        std::vector<int> outgoing = getOutgoingLiterals(component);
        std::vector<int> incoming = getIncomingLiterals(component);

        if (!useVertexSep) {
            if (!outgoing.empty()) clauses.push_back(std::move(outgoing));
            if (!incoming.empty()) clauses.push_back(std::move(incoming));
            continue;
        }

        // Build inComponent bitmask
        std::vector<bool> inComponent(numNodes, false);
        for (int u : component.vertices) {
            if (u >= 0 && u < numNodes) inComponent[u] = true;
        }

        // Compute vertex boundary S
        std::vector<bool> isBoundary(numNodes, false);
        std::vector<int> boundaryVertices;
        for (int u : component.vertices) {
            if (u < 0 || u >= numNodes) continue;
            for (auto& [v, _] : graph_.getNeighbors(u)) {
                if (v >= 0 && v < numNodes && !inComponent[v] && !isBoundary[v]) {
                    isBoundary[v] = true;
                    boundaryVertices.push_back(v);
                }
            }
        }

        int sSize = (int)boundaryVertices.size();

        if (sSize <= vtxSepThreshold) {
            // Merge and dedup all boundary edges
            std::vector<int> allBoundary = outgoing;
            allBoundary.insert(allBoundary.end(), incoming.begin(), incoming.end());
            std::sort(allBoundary.begin(), allBoundary.end());
            allBoundary.erase(std::unique(allBoundary.begin(), allBoundary.end()), allBoundary.end());

            if (allBoundary.empty()) continue;
            int n = (int)allBoundary.size();

            DefaultAtLeastK atLeastK;
            auto kClauses = atLeastK.encode(allBoundary, 2, globalAuxBase);
            clauses.insert(clauses.end(), kClauses.begin(), kClauses.end());

            // Cross-direction vertex-disjoint for |S| = 2
            if (sSize == 2 && n >= 4 && !skipVertexDisjoint) {
                for (int bv : boundaryVertices) {
                    std::vector<int> edgesOut;
                    for (int u : component.vertices) {
                        if (u < 0 || u >= numNodes) continue;
                        for (auto& [v, edgeIdx] : graph_.getNeighbors(u)) {
                            if (v == bv) edgesOut.push_back(edgeIdx);
                        }
                    }
                    std::vector<int> edgesIn;
                    for (auto& [v, edgeIdx] : graph_.getNeighbors(bv)) {
                        if (v >= 0 && v < numNodes && inComponent[v]) {
                            edgesIn.push_back(edgeIdx);
                        }
                    }
                    for (int eOut : edgesOut) {
                        for (int eIn : edgesIn) {
                            clauses.push_back({-eOut, -eIn});
                        }
                    }
                }
            }
        } else {
            if (!outgoing.empty()) clauses.push_back(std::move(outgoing));
            if (!incoming.empty()) clauses.push_back(std::move(incoming));
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
        for (auto& [v, edgeIdx] : graph_.getNeighbors(u)) {
            if (!inComponent[v]) {
                literals.push_back(edgeIdx);
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
            for (auto& [v, edgeIdx] : graph_.getNeighbors(u)) {
                if (inComponent[v]) {
                    literals.push_back(edgeIdx);
                }
            }
        }
    }
    return literals;
}
