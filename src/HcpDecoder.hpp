#ifndef HCPDECODER_HPP
#define HCPDECODER_HPP

#include <iostream>
#include <vector>
#include <string>
#include <fstream>
#include <map>

class HcpDecoder {
private:
    std::string graphFile;
    std::string solFile;
    int nNode;
    int nEdge;
    std::vector<int> lookup;
    std::vector<int> nextNode;
    std::vector<int> visited;

    bool parseGraph() {
        std::ifstream file(graphFile);
        if (!file.is_open()) return false;

        std::string p, edge_str;
        file >> p >> edge_str >> nNode >> nEdge;
        
        lookup.resize(4 * nEdge + 10, 0);

        std::string e;
        int i, j;
        int max = 0;
        while (file >> e >> i >> j) {
            if (e == "e" || e == "E") {
                max++;
                lookup[2 * max] = i - 1;
                lookup[2 * max + 1] = j - 1;
                
                max++;
                lookup[2 * max] = j - 1;
                lookup[2 * max + 1] = i - 1;
            }
        }
        return true;
    }

    bool parseSolution() {
        std::ifstream sol(solFile);
        if (!sol.is_open()) return false;

        std::string line;
        bool isSat = false;
        while (std::getline(sol, line)) {
            if (line.empty()) continue;
            if (line[0] == 'c') continue; // Skip comment lines completely
            
            if (line.find("SATISFIABLE") != std::string::npos) {
                isSat = true;
            }
            
            std::stringstream ss(line);
            std::string prefix;
            ss >> prefix;
            
            if (prefix == "v") {
                std::string token;
                while (ss >> token) {
                    try {
                        int edge = std::stoi(token);
                        if (edge > 0 && edge <= 2 * nEdge) {
                            int a = lookup[2 * edge] + 1;
                            int b = lookup[2 * edge + 1] + 1;
                            nextNode[a] = b;
                        }
                    } catch (...) {
                        // Ignore non-integer tokens
                    }
                }
            }
        }
        return true;
    }

public:
    HcpDecoder(const std::string& gFile, const std::string& sFile) 
        : graphFile(gFile), solFile(sFile), nNode(0), nEdge(0) {}

    void decode() {
        if (!parseGraph()) {
            std::cerr << "Failed to parse graph file\n";
            return;
        }

        nextNode.assign(nNode + 1, 0);
        
        if (!parseSolution()) {
            std::cerr << "Failed to parse solution file\n";
            return;
        }

        visited.assign(nNode + 1, 0);

        std::vector<int> path;
        int a = 1;
        for (int i = 1; i <= nNode + 1; i++) {
            path.push_back(a);
            if (visited[a]) {
                if ((i - visited[a]) == nNode) {
                    std::cout << "c VERIFIED HCP of size " << nNode << "\n";
                    
                    std::ofstream pathOut("solution.path");
                    if (pathOut.is_open()) {
                        for (size_t k = 0; k < path.size(); ++k) {
                            pathOut << path[k] << (k == path.size() - 1 ? "" : " ");
                        }
                        pathOut << "\n";
                        pathOut.close();
                    } else {
                        std::cerr << "c Error: Could not write cycle path to solution.path\n";
                    }
                } else {
                    std::cout << "c ERROR: cycle of size " << (i - visited[a]) << " out of " << nNode << "\n";
                }
                break;
            }
            visited[a] = i;
            a = nextNode[a];
        }
    }
};

#endif
