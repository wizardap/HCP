CXX = g++
CXXFLAGS = -O2 -Wall -std=c++17

all: hcp-solver knight-graph grid-graph test_sec_encoder

hcp-solver: Solver.cpp Solver.hpp HcpEncoder.hpp HcpDecoder.hpp Graph.hpp AtMostOne/IAtMostOne.hpp AtMostOne/DefaultAtMostOne.hpp AtMostOne/PbLibAtMostOne.hpp AtLeastK/IAtLeastK.hpp AtLeastK/DefaultAtLeastK.hpp IncrementalSolver.cpp IncrementalSolver.hpp SubtourDetector.cpp SubtourDetector.hpp SecEncoder.cpp SecEncoder.hpp VariableManager.cpp VariableManager.hpp TrajectoryLogger.hpp TrajectoryLogger.cpp GraphPreprocessor.cpp GraphPreprocessor.hpp ContractedMinCut.cpp ContractedMinCut.hpp GomoryHuTree.cpp GomoryHuTree.hpp
	$(CXX) $(CXXFLAGS) -I../refs/cadical/src Solver.cpp IncrementalSolver.cpp SubtourDetector.cpp SecEncoder.cpp VariableManager.cpp TrajectoryLogger.cpp GraphPreprocessor.cpp ContractedMinCut.cpp GomoryHuTree.cpp -o hcp-solver -L../refs/cadical/build -lcadical -L../refs/pblib/build -lpb

knight-graph: KnightGraphGenerator.cpp
	$(CXX) $(CXXFLAGS) KnightGraphGenerator.cpp -o knight-graph

grid-graph: GridGraphGenerator.cpp
	$(CXX) $(CXXFLAGS) GridGraphGenerator.cpp -o grid-graph

.PHONY: all clean test

clean:
	rm -f hcp-solver knight-graph grid-graph test_incremental_solver test_graphs test_vertex_separator test_gomory_hu test_sec_encoder *.o

test: test_incremental_solver test_graphs test_vertex_separator test_gomory_hu test_sec_encoder
	./test_incremental_solver
	./test_graphs
	./test_vertex_separator
	./test_gomory_hu
	./test_sec_encoder

test_sec_encoder: test_sec_encoder.cpp SecEncoder.cpp SecEncoder.hpp SubtourDetector.cpp SubtourDetector.hpp Graph.hpp
	$(CXX) $(CXXFLAGS) test_sec_encoder.cpp SecEncoder.cpp SubtourDetector.cpp -o test_sec_encoder

test_vertex_separator: test_vertex_separator.cpp SecEncoder.cpp SecEncoder.hpp SubtourDetector.cpp SubtourDetector.hpp Graph.hpp
	$(CXX) $(CXXFLAGS) test_vertex_separator.cpp SecEncoder.cpp SubtourDetector.cpp -o test_vertex_separator

test_gomory_hu: test_gomory_hu.cpp GomoryHuTree.cpp GomoryHuTree.hpp ContractedMinCut.cpp ContractedMinCut.hpp SubtourDetector.cpp SubtourDetector.hpp Graph.hpp SecEncoder.cpp SecEncoder.hpp
	$(CXX) $(CXXFLAGS) test_gomory_hu.cpp GomoryHuTree.cpp ContractedMinCut.cpp SubtourDetector.cpp SecEncoder.cpp -o test_gomory_hu

test_incremental_solver: test_incremental_solver.cpp Solver.cpp Solver.hpp IncrementalSolver.cpp IncrementalSolver.hpp SubtourDetector.cpp SubtourDetector.hpp SecEncoder.cpp SecEncoder.hpp VariableManager.cpp VariableManager.hpp Graph.hpp GraphPreprocessor.cpp GraphPreprocessor.hpp TrajectoryLogger.cpp TrajectoryLogger.hpp ContractedMinCut.cpp ContractedMinCut.hpp GomoryHuTree.cpp GomoryHuTree.hpp
	$(CXX) $(CXXFLAGS) -DTESTING -I../refs/cadical/src test_incremental_solver.cpp Solver.cpp IncrementalSolver.cpp SubtourDetector.cpp SecEncoder.cpp VariableManager.cpp TrajectoryLogger.cpp GraphPreprocessor.cpp ContractedMinCut.cpp GomoryHuTree.cpp -o test_incremental_solver -L../refs/cadical/build -lcadical -L../refs/pblib/build -lpb

test_graphs: test_graphs.cpp Solver.cpp Solver.hpp IncrementalSolver.cpp IncrementalSolver.hpp SubtourDetector.cpp SubtourDetector.hpp SecEncoder.cpp SecEncoder.hpp VariableManager.cpp VariableManager.hpp TrajectoryLogger.cpp TrajectoryLogger.hpp Graph.hpp HcpEncoder.hpp AtMostOne/IAtMostOne.hpp AtMostOne/DefaultAtMostOne.hpp AtMostOne/PbLibAtMostOne.hpp SymmetryBreaking/ISymmetryBreaker.hpp SymmetryBreaking/DefaultSymmetryBreaker.hpp GraphPreprocessor.cpp GraphPreprocessor.hpp ContractedMinCut.cpp ContractedMinCut.hpp GomoryHuTree.cpp GomoryHuTree.hpp
	$(CXX) $(CXXFLAGS) -DTESTING -I../refs/cadical/src test_graphs.cpp Solver.cpp IncrementalSolver.cpp SubtourDetector.cpp SecEncoder.cpp VariableManager.cpp TrajectoryLogger.cpp GraphPreprocessor.cpp ContractedMinCut.cpp GomoryHuTree.cpp -o test_graphs -L../refs/cadical/build -lcadical -L../refs/pblib/build -lpb
