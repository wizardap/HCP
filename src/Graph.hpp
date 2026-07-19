#ifndef GRAPH_HPP
#define GRAPH_HPP

#include <vector>
#include <string>
#include <fstream>
#include <iostream>
#include <algorithm>
#include <sstream>

class Graph {
private:
    int nNode;
    int nEdge;
    std::vector<std::vector<std::pair<int, int>>> adj;
    std::vector<int> degree;

public:
    Graph(int nodes = 0, int edges = 0) : nNode(nodes), nEdge(edges) {
        if (nNode > 0) {
            adj.resize(nNode);
            degree.assign(nNode, 0);
        }
    }

    bool loadFromFile(const std::string& filename, bool directedEdgeIndices = false) {
        std::ifstream file(filename);
        if (!file.is_open()) return false;

        std::string p, edge;
        file >> p >> edge >> nNode >> nEdge;
        
        adj.assign(nNode, std::vector<std::pair<int, int>>());
        degree.assign(nNode, 0);

        std::string line;
        int maxIndex = 0;
        while (std::getline(file, line)) {
            if (line.empty()) continue;
            std::istringstream iss(line);
            std::string token;
            iss >> token;
            if (!iss) continue;
            int u, v;
            if (token == "c" || token == "C") {
                continue;
            }
            if (token == "e" || token == "E") {
                if (!(iss >> u >> v)) continue;
            } else {
                std::istringstream(token) >> u;
                if (iss.fail() || !(iss >> v)) continue;
            }
            u--; v--;
            if (directedEdgeIndices) {
                adj[u].push_back({v, ++maxIndex});
                adj[v].push_back({u, ++maxIndex});
            } else {
                adj[u].push_back({v, 1});
                adj[v].push_back({u, 1});
            }
        }

        for (int i = 0; i < nNode; ++i) {
            std::sort(adj[i].begin(), adj[i].end());
            degree[i] = adj[i].size();
        }
        return true;
    }

    int getNodes() const { return nNode; }
    int getEdges() const { return nEdge; }
    int getDegree(int v) const { return degree[v]; }

    int getAdj(int u, int v) const {
        const auto& list = adj[u];
        auto it = std::lower_bound(list.begin(), list.end(), std::make_pair(v, 0));
        if (it != list.end() && it->first == v) return it->second;
        return 0;
    }

    const std::vector<std::pair<int, int>>& getNeighbors(int u) const {
        return adj[u];
    }

    void addEdge(int u, int v, int val = 1) {
        auto& list = adj[u];
        auto it = std::lower_bound(list.begin(), list.end(), std::make_pair(v, 0));
        if (it != list.end() && it->first == v) {
            it->second = val;
        } else {
            list.insert(it, {v, val});
        }
    }

    int getMinDegreeVertex(int& minDegree) const {
        minDegree = nNode;
        int minVertex = 0;
        for (int i = 0; i < nNode; i++) {
            if (degree[i] < minDegree) {
                minDegree = degree[i];
                minVertex = i;
            }
        }
        return minVertex;
    }

    int getMaxDegreeVertex(int& maxDegree) const {
        maxDegree = -1;
        int maxVertex = 0;
        for (int i = 0; i < nNode; i++) {
            if (degree[i] > maxDegree) {
                maxDegree = degree[i];
                maxVertex = i;
            }
        }
        return maxVertex;
    }
};

#endif
