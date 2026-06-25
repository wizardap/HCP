CXX = g++
CXXFLAGS = -O2 -Wall -std=c++17

all: hcp-solver knight-graph grid-graph

hcp-solver: Solver.cpp Solver.hpp HcpEncoder.hpp HcpDecoder.hpp Graph.hpp AtMostOne/IAtMostOne.hpp AtMostOne/DefaultAtMostOne.hpp AtMostOne/PbLibAtMostOne.hpp IncrementalSolver.cpp IncrementalSolver.hpp SubtourDetector.cpp SubtourDetector.hpp SecEncoder.cpp SecEncoder.hpp VariableManager.cpp VariableManager.hpp
	$(CXX) $(CXXFLAGS) -I../refs/cadical/src Solver.cpp IncrementalSolver.cpp SubtourDetector.cpp SecEncoder.cpp VariableManager.cpp -o hcp-solver -L../refs/cadical/build -lcadical -L../refs/pblib/build -lpb

knight-graph: KnightGraphGenerator.cpp
	$(CXX) $(CXXFLAGS) KnightGraphGenerator.cpp -o knight-graph

grid-graph: GridGraphGenerator.cpp
	$(CXX) $(CXXFLAGS) GridGraphGenerator.cpp -o grid-graph

clean:
	rm -f hcp-solver knight-graph grid-graph
