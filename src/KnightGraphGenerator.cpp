#include <iostream>
#include <cstdlib>

class KnightGraphGenerator {
private:
    int size;

    bool hasEdge(int a, int b) {
        int r = std::abs((a % size) - (b % size));
        int c = std::abs((a / size) - (b / size));

        if ((r == 1) && (c == 2)) return true;
        if ((r == 2) && (c == 1)) return true;
        return false;
    }

public:
    KnightGraphGenerator(int s) : size(s) {}

    void generate() {
        int sqsize = size * size;
        int nEdges = 0;

        for (int i = 0; i < sqsize; i++) {
            for (int j = i + 1; j < sqsize; j++) {
                if (hasEdge(i, j)) nEdges++;
            }
        }

        std::cout << "p edge " << sqsize << " " << nEdges << "\n";

        for (int i = 0; i < sqsize; i++) {
            for (int j = i + 1; j < sqsize; j++) {
                if (hasEdge(i, j)) {
                    std::cout << "e " << (i + 1) << " " << (j + 1) << "\n";
                }
            }
        }
    }
};

int main(int argc, char** argv) {
    int size = 8;
    if (argc > 1) size = std::atoi(argv[1]);

    KnightGraphGenerator generator(size);
    generator.generate();
    return 0;
}
