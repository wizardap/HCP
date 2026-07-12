#ifndef IATLEASTK_HPP
#define IATLEASTK_HPP

#include <vector>

class IAtLeastK {
public:
    virtual ~IAtLeastK() = default;
    virtual std::vector<std::vector<int>> encode(
        const std::vector<int>& literals,
        int k,
        int& nextAuxVar
    ) = 0;
};

#endif
