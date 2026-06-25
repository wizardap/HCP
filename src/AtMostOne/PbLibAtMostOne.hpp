#ifndef PBLIBATMOSTONE_HPP
#define PBLIBATMOSTONE_HPP

#include "IAtMostOne.hpp"
#include <iostream>
// Uncomment these headers when linking with PbLib
#include "../../refs/pblib/pblib/pb2cnf.h"
#include "../../refs/pblib/pblib/VectorClauseDatabase.h"

class PbLibAtMostOne : public IAtMostOne {
public:
    void encode(std::vector<int>& array, int size, int& maxVar) override {
        // Note: To use PbLib, ensure HCP/refs/pblib is compiled and linked.
        // Below is a typical implementation using PbLib:
        
        PBConfig config = std::make_shared<PBConfigClass>();
        PB2CNF pb2cnf(config);
        std::vector<std::vector<int32_t>> formula;
        
        // Use the actual array values directly (as int32_t)
        pb2cnf.encodeAtMostK(array, 1, formula, maxVar);
        
        // Output the generated CNF clauses
        for(const auto& clause : formula) {
            for(int lit : clause) {
                std::cout << lit << " ";
            }
            std::cout << "0\n";
        }
        
    }
};

#endif
