#pragma once
#include <vector>
#include "Graph.hpp"
#include "SubtourDetector.hpp"

struct MinCutResult {
    std::vector<int> sideA_vertices; // original vertices on the source side of the min-cut
    int cutSize = 0;
};

// Given current subtour components and the original graph,
// build the contracted graph and find the minimum s-t cut
// between the smallest component and each neighbor.
// Returns the cut with the smallest cutSize across all neighbor pairs.
// Returns MinCutResult with empty sideA_vertices if no cut < total boundary found.
MinCutResult computeComponentMinCut(
    const std::vector<Component>& components,
    const Graph& graph
);

// Find the minimum edge cut within a single component.
// Used to split a large component when stagnation occurs at moderate count.
// Returns the cut edges (global edge indices) and the vertices on one side.
// Returns empty vectors if no useful cut found.
MinCutResult computeInternalMinCut(
    const Component& component,
    const Graph& graph,
    int maxFlowVertLimit = 500
);
