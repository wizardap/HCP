CXX = g++
CXXFLAGS = -O2 -Wall -std=c++17

all: hcp-solver knight-graph grid-graph

hcp-solver: Solver.cpp Solver.hpp HcpEncoder.hpp HcpDecoder.hpp Graph.hpp AtMostOne/IAtMostOne.hpp AtMostOne/DefaultAtMostOne.hpp AtMostOne/PbLibAtMostOne.hpp VariableManager.cpp VariableManager.hpp
	$(CXX) $(CXXFLAGS) -I../refs/cadical/src Solver.cpp VariableManager.cpp -o hcp-solver -L../refs/cadical/build -lcadical -L../refs/pblib/build -lpb

knight-graph: KnightGraphGenerator.cpp
	$(CXX) $(CXXFLAGS) KnightGraphGenerator.cpp -o knight-graph

grid-graph: GridGraphGenerator.cpp
	$(CXX) $(CXXFLAGS) GridGraphGenerator.cpp -o grid-graph

.PHONY: all clean test

clean:
	rm -f hcp-solver knight-graph grid-graph test_graphs *.o

test: test_graphs
	./test_graphs

test_graphs: test_graphs.cpp Graph.hpp HcpEncoder.hpp AtMostOne/IAtMostOne.hpp AtMostOne/DefaultAtMostOne.hpp SymmetryBreaking/ISymmetryBreaker.hpp SymmetryBreaking/DefaultSymmetryBreaker.hpp VariableManager.cpp VariableManager.hpp
	$(CXX) $(CXXFLAGS) -I../refs/cadical/src test_graphs.cpp VariableManager.cpp -o test_graphs -L../refs/cadical/build -lcadical


