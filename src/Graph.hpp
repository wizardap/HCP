#ifndef GRAPH_HPP
#define GRAPH_HPP

#include <vector>
#include <string>
#include <fstream>
#include <iostream>

class Graph {
private:
    int nNode;
    int nEdge;
    std::vector<std::vector<int>> adj;
    std::vector<int> degree;

public:
    Graph(int nodes = 0, int edges = 0) : nNode(nodes), nEdge(edges) {
        if (nNode > 0) {
            adj.assign(nNode, std::vector<int>(nNode, 0));
            degree.assign(nNode, 0);
        }
    }

    bool loadFromFile(const std::string& filename, bool directedEdgeIndices = false) {
        std::ifstream file(filename);
        if (!file.is_open()) return false;

        std::string p, edge;
        file >> p >> edge >> nNode >> nEdge;
        
        adj.assign(nNode, std::vector<int>(nNode, 0));
        degree.assign(nNode, 0);

        std::string e;
        int u, v;
        int maxIndex = 0;
        while (file >> e >> u >> v) {
            if (e == "e" || e == "E") {
                if (directedEdgeIndices) {
                    adj[u-1][v-1] = ++maxIndex;
                    adj[v-1][u-1] = ++maxIndex;
                } else {
                    adj[u-1][v-1] = 1;
                    adj[v-1][u-1] = 1;
                }
            }
        }
        
        for (int i = 0; i < nNode; ++i) {
            for (int j = i + 1; j < nNode; ++j) {
                if (adj[i][j]) {
                    degree[i]++;
                    degree[j]++;
                }
            }
        }
        return true;
    }

    int getNodes() const { return nNode; }
    int getEdges() const { return nEdge; }
    int getDegree(int v) const { return degree[v]; }
    int getAdj(int u, int v) const { return adj[u][v]; }
    void addEdge(int u, int v, int val = 1) {
        adj[u][v] = val;
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
