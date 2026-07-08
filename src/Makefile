CXX = g++
CXXFLAGS = -O2 -Wall -std=c++17

all: hcp-solver knight-graph grid-graph

hcp-solver: Solver.cpp Solver.hpp HcpEncoder.hpp HcpDecoder.hpp Graph.hpp AtMostOne/IAtMostOne.hpp AtMostOne/DefaultAtMostOne.hpp AtMostOne/PbLibAtMostOne.hpp IncrementalSolver.cpp IncrementalSolver.hpp SubtourDetector.cpp SubtourDetector.hpp SecEncoder.cpp SecEncoder.hpp VariableManager.cpp VariableManager.hpp TrajectoryLogger.hpp TrajectoryLogger.cpp
	$(CXX) $(CXXFLAGS) -I../refs/cadical/src Solver.cpp IncrementalSolver.cpp SubtourDetector.cpp SecEncoder.cpp VariableManager.cpp TrajectoryLogger.cpp -o hcp-solver -L../refs/cadical/build -lcadical -L../refs/pblib/build -lpb

knight-graph: KnightGraphGenerator.cpp
	$(CXX) $(CXXFLAGS) KnightGraphGenerator.cpp -o knight-graph

grid-graph: GridGraphGenerator.cpp
	$(CXX) $(CXXFLAGS) GridGraphGenerator.cpp -o grid-graph

.PHONY: all clean test

clean:
	rm -f hcp-solver knight-graph grid-graph test_incremental_solver test_graphs *.o

test: test_incremental_solver test_graphs
	./test_incremental_solver
	./test_graphs

test_incremental_solver: test_incremental_solver.cpp IncrementalSolver.cpp IncrementalSolver.hpp SubtourDetector.cpp SubtourDetector.hpp SecEncoder.cpp SecEncoder.hpp VariableManager.cpp VariableManager.hpp Graph.hpp
	$(CXX) $(CXXFLAGS) -I../refs/cadical/src test_incremental_solver.cpp IncrementalSolver.cpp SubtourDetector.cpp SecEncoder.cpp VariableManager.cpp -o test_incremental_solver -L../refs/cadical/build -lcadical -L../refs/pblib/build -lpb

test_graphs: test_graphs.cpp Solver.cpp Solver.hpp IncrementalSolver.cpp IncrementalSolver.hpp SubtourDetector.cpp SubtourDetector.hpp SecEncoder.cpp SecEncoder.hpp VariableManager.cpp VariableManager.hpp TrajectoryLogger.cpp TrajectoryLogger.hpp Graph.hpp HcpEncoder.hpp AtMostOne/IAtMostOne.hpp AtMostOne/DefaultAtMostOne.hpp AtMostOne/PbLibAtMostOne.hpp SymmetryBreaking/ISymmetryBreaker.hpp SymmetryBreaking/DefaultSymmetryBreaker.hpp
	$(CXX) $(CXXFLAGS) -DTESTING -I../refs/cadical/src test_graphs.cpp Solver.cpp IncrementalSolver.cpp SubtourDetector.cpp SecEncoder.cpp VariableManager.cpp TrajectoryLogger.cpp -o test_graphs -L../refs/cadical/build -lcadical -L../refs/pblib/build -lpb


