#include "SecEncoder.hpp"
#include "Graph.hpp"

SecEncoder::SecEncoder(const Graph& graph) : graph_(graph) {}

std::vector<std::vector<int>> SecEncoder::encodeSecs(
    const std::vector<Component>& components,
    bool useVertexSep,
    int vtxSepThreshold)
{
    std::vector<std::vector<int>> clauses;
    int numNodes = graph_.getNodes();

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

            // Sequential counter: sum(allBoundary) >= 2
            int maxLit = 0;
            for (int l : allBoundary) if (l > maxLit) maxLit = l;
            int auxBase = maxLit + 1;
            auto s = [&](int i, int j) { return auxBase + i * 2 + j; };

            // Base: i=0
            clauses.push_back({-allBoundary[0], s(0,0)});
            clauses.push_back({allBoundary[0], -s(0,0)});
            clauses.push_back({-s(0,1)});

            // Inductive: i=1..n-1
            for (int i = 1; i < n; i++) {
                // s[i][0] <-> (s[i-1][0] OR allBoundary[i])
                clauses.push_back({-s(i-1,0), s(i,0)});
                clauses.push_back({-allBoundary[i], s(i,0)});
                clauses.push_back({s(i,0), -allBoundary[i], -s(i-1,0)});

                // s[i][1] <-> (s[i-1][1] OR (s[i-1][0] AND allBoundary[i]))
                clauses.push_back({-s(i-1,1), s(i,1)});
                clauses.push_back({-s(i-1,0), -allBoundary[i], s(i,1)});
                clauses.push_back({s(i,1), -s(i-1,1), -s(i-1,0)});
                clauses.push_back({-s(i,1), s(i-1,0), allBoundary[i]});
            }

            // Enforce sum >= 2
            clauses.push_back({s(n-1,1)});

            // Cross-direction vertex-disjoint for |S| = 2
            if (sSize == 2 && n >= 4) {
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
