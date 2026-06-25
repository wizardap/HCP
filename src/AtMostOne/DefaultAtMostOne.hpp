#ifndef DEFAULTATMOSTONE_HPP
#define DEFAULTATMOSTONE_HPP

#include "IAtMostOne.hpp"
#include <iostream>

class DefaultAtMostOne : public IAtMostOne {
public:
    void encode(std::vector<int>& array, int size, int& maxVar) override {
        if (size > 1) {
            std::cout << "-" << array[0] << " -" << array[1] << " 0\n";
        }
        if (size > 2) {
            std::cout << "-" << array[0] << " -" << array[2] << " 0\n";
            std::cout << "-" << array[1] << " -" << array[2] << " 0\n";
        }
        if (size == 4) {
            std::cout << "-" << array[0] << " -" << array[3] << " 0\n";
            std::cout << "-" << array[1] << " -" << array[3] << " 0\n";
            std::cout << "-" << array[2] << " -" << array[3] << " 0\n";
        }
        if (size > 4) {
            std::cout << "-" << array[0] << " " << maxVar << " 0\n";
            std::cout << "-" << array[1] << " " << maxVar << " 0\n";
            std::cout << "-" << array[2] << " " << maxVar << " 0\n";

            for (int i = 3; i < size; i++)
                array[i - 3] = array[i];
            array[size - 3] = maxVar++;
            encode(array, size - 2, maxVar);
        }
    }
};

#endif
