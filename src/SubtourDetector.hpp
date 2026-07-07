#ifndef SUBTOUR_DETECTOR_HPP
#define SUBTOUR_DETECTOR_HPP

#include <vector>
class Graph;

struct Component {
    std::vector<int> vertices;  // 0-indexed vertex IDs
    std::vector<int> edges;     // 1-indexed directed edge variables selected within this component
    
    bool operator<(const Component& other) const {
        return vertices.size() < other.vertices.size();
    }
};

class SubtourDetector {
public:
    static std::vector<Component> detect(
        const std::vector<int>& model,
        const Graph& graph
    );
};

#endif // SUBTOUR_DETECTOR_HPP
