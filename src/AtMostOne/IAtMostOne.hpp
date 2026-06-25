#ifndef IATMOSTONE_HPP
#define IATMOSTONE_HPP

#include <vector>

class IAtMostOne {
public:
    virtual ~IAtMostOne() = default;
    
    // maxVar is passed by reference so the constraint encoder can allocate new variables if needed
    // The implementation should print the CNF clauses to standard output (or handle them appropriately)
    virtual void encode(std::vector<int>& array, int size, int& maxVar) = 0;
};

#endif
