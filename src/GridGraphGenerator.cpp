#include <iostream>
#include <cstdlib>

class GridGraphGenerator {
private:
    int cols;
    int rows;

public:
    GridGraphGenerator(int c, int r) : cols(c), rows(r) {}

    void generate() {
        int nVtx  = rows * cols;
        int nEdge = 0;
        for (int i = 1; i <= cols; i++) {
            for (int j = 1; j <= rows; j++) {
                if (j < rows) nEdge++;
                if (i < cols) nEdge++;
            }
        }

        std::cout << "p edge " << nVtx << " " << nEdge << "\n";

        for (int i = 1; i <= cols; i++) {
            for (int j = 1; j <= rows; j++) {
                if (j < rows) std::cout << "e " << ((i - 1) * rows + j) << " " << ((i - 1) * rows + j + 1) << "\n";
                if (i < cols) std::cout << "e " << ((i - 1) * rows + j) << " " << (i * rows + j) << "\n";
            }
        }
    }
};

int main(int argc, char** argv) {
    if (argc < 3) {
        std::cerr << "Usage: " << argv[0] << " <cols> <rows>\n";
        return 1;
    }
    int cols = std::atoi(argv[1]);
    int rows = std::atoi(argv[2]);

    GridGraphGenerator generator(cols, rows);
    generator.generate();
    return 0;
}
