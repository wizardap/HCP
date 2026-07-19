#include "SecEncoder.hpp"
#include "AtLeastK/DefaultAtLeastK.hpp"
#include "Graph.hpp"

SecEncoder::SecEncoder(const Graph& graph) : graph_(graph), nextAuxBase_(0) {
    int numNodes = graph_.getNodes();
    inComponent_.resize(numNodes, false);
    isBoundary_.resize(numNodes, false);
    inAdj_.resize(numNodes);
    for (int u = 0; u < numNodes; ++u) {
        for (auto& [v, edgeIdx] : graph_.getNeighbors(u)) {
            if (v >= 0 && v < numNodes) {
                inAdj_[v].push_back({u, edgeIdx});
            }
        }
    }
}

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
    if ((int)inComponent_.size() < numNodes) {
        inComponent_.resize(numNodes, false);
        isBoundary_.resize(numNodes, false);
    }

    if (nextAuxBase_ <= 0) {
        nextAuxBase_ = graph_.getEdges() * 2 + 2;
    }
    int& globalAuxBase = nextAuxBase_;

    for (const auto& component : components) {
        // Set bitmask for S in O(|S|)
        for (int u : component.vertices) {
            if (u >= 0 && u < numNodes) inComponent_[u] = true;
        }

        std::vector<int> outgoing = getOutgoingLiterals(component);
        std::vector<int> incoming = getIncomingLiterals(component);

        if (!useVertexSep) {
            if (!outgoing.empty()) clauses.push_back(std::move(outgoing));
            if (!incoming.empty()) clauses.push_back(std::move(incoming));
        } else {
            // Compute vertex boundary S
            std::vector<int> boundaryVertices;
            for (int u : component.vertices) {
                if (u < 0 || u >= numNodes) continue;
                for (auto& [v, _] : graph_.getNeighbors(u)) {
                    if (v >= 0 && v < numNodes && !inComponent_[v] && !isBoundary_[v]) {
                        isBoundary_[v] = true;
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

                if (!allBoundary.empty()) {
                    DefaultAtLeastK atLeastK;
                    auto kClauses = atLeastK.encode(allBoundary, 2, globalAuxBase);
                    clauses.insert(clauses.end(), kClauses.begin(), kClauses.end());

                    if (sSize == 2 && (int)allBoundary.size() >= 4 && !skipVertexDisjoint && (int)(component.vertices.size() + sSize) < numNodes) {
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
                                if (v >= 0 && v < numNodes && inComponent_[v]) {
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
                }
            } else {
                if (!outgoing.empty()) clauses.push_back(std::move(outgoing));
                if (!incoming.empty()) clauses.push_back(std::move(incoming));
            }

            // Reset isBoundary_
            for (int bv : boundaryVertices) {
                isBoundary_[bv] = false;
            }
        }

        // Small-cycle DFJ clause for |S| <= 3
        if (component.vertices.size() <= 3) {
            std::vector<int> dfjClause;
            for (int u : component.vertices) {
                if (u < 0 || u >= numNodes) continue;
                for (auto& [v, edgeIdx] : graph_.getNeighbors(u)) {
                    if (v >= 0 && v < numNodes && inComponent_[v]) {
                        dfjClause.push_back(-edgeIdx);
                    }
                }
            }
            if (!dfjClause.empty()) {
                clauses.push_back(std::move(dfjClause));
            }
        }

        // Reset inComponent_ in O(|S|)
        for (int u : component.vertices) {
            if (u >= 0 && u < numNodes) inComponent_[u] = false;
        }
    }
    return clauses;
}

std::vector<int> SecEncoder::getOutgoingLiterals(const Component& component) {
    int numNodes = graph_.getNodes();
    std::vector<int> literals;

    for (int u : component.vertices) {
        if (u < 0 || u >= numNodes) continue;
        for (auto& [v, edgeIdx] : graph_.getNeighbors(u)) {
            if (v >= 0 && v < numNodes && !inComponent_[v]) {
                literals.push_back(edgeIdx);
            }
        }
    }
    return literals;
}

std::vector<int> SecEncoder::getIncomingLiterals(const Component& component) {
    int numNodes = graph_.getNodes();
    std::vector<int> literals;

    for (int v : component.vertices) {
        if (v < 0 || v >= numNodes) continue;
        for (auto& [u, edgeIdx] : inAdj_[v]) {
            if (u >= 0 && u < numNodes && !inComponent_[u]) {
                literals.push_back(edgeIdx);
            }
        }
    }
    return literals;
}
