#ifndef DEFAULTATLEASTK_HPP
#define DEFAULTATLEASTK_HPP

#include "IAtLeastK.hpp"

class DefaultAtLeastK : public IAtLeastK {
public:
    std::vector<std::vector<int>> encode(
        const std::vector<int>& literals,
        int k,
        int& nextAuxVar) override
    {
        std::vector<std::vector<int>> clauses;
        int n = (int)literals.size();
        if (n < k || k <= 0) return clauses;

        if (k == 1) {
            clauses.push_back(literals);
            return clauses;
        }

        int auxBase = nextAuxVar;
        nextAuxVar = auxBase + n * k;
        auto s = [&](int i, int j) { return auxBase + i * k + j; };

        for (int j = 0; j < k; j++) {
            if (j == 0) {
                clauses.push_back({-literals[0], s(0,0)});
                clauses.push_back({literals[0], -s(0,0)});
            } else {
                clauses.push_back({-s(0,j)});
            }
        }

        for (int i = 1; i < n; i++) {
            for (int j = 0; j < k; j++) {
                if (j == 0) {
                    clauses.push_back({-s(i-1,0), s(i,0)});
                    clauses.push_back({-literals[i], s(i,0)});
                    clauses.push_back({-s(i,0), s(i-1,0), literals[i]});
                } else {
                    clauses.push_back({-s(i-1,j), s(i,j)});
                    clauses.push_back({-s(i-1,j-1), -literals[i], s(i,j)});
                    clauses.push_back({-s(i,j), s(i-1,j), s(i-1,j-1)});
                    clauses.push_back({-s(i,j), s(i-1,j), literals[i]});
                }
            }
        }

        clauses.push_back({s(n-1,k-1)});
        return clauses;
    }
};

#endif
