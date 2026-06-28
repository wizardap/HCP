import os
import sys
import argparse
import re
import matplotlib
matplotlib.use('Agg') # Headless backend
import matplotlib.pyplot as plt
import networkx as nx

def parse_args():
    parser = argparse.ArgumentParser(description="Visualize Hamiltonian Cycle solution paths")
    parser.add_argument("--graph", type=str, default=None, help="Path to DIMACS .edge file")
    parser.add_argument("--path", type=str, default=None, help="Path to space-separated solution .path file")
    parser.add_argument("--output", type=str, default=None, help="Output image file path (.png)")
    return parser.parse_args()

def read_graph(graph_path):
    G = nx.Graph()
    num_nodes = 0
    with open(graph_path, "r") as f:
        for line in f:
            line = line.strip()
            if not line:
                continue
            if line.startswith("p edge"):
                parts = line.split()
                num_nodes = int(parts[2])
                G.add_nodes_from(range(1, num_nodes + 1))
            elif line.startswith("e"):
                parts = line.split()
                u, v = int(parts[1]), int(parts[2])
                G.add_edge(u, v)
    return G, num_nodes

def read_path(path_path):
    with open(path_path, "r") as f:
        content = f.read().strip()
    if not content:
        return []
    return [int(x) for x in content.split()]

def detect_layout(G, num_nodes, path_nodes=None, cycle_edges=None, optimize_spring=False):
    # 1. Grid Check
    # Try to factor num_nodes = C * R
    factors = []
    for r in range(1, int(num_nodes**0.5) + 1):
        if num_nodes % r == 0:
            factors.append((r, num_nodes // r))
            factors.append((num_nodes // r, r))
            
    for rows, cols in factors:
        # Check if edges match grid structure (u is connected to u+1 and u+rows)
        matching_edges = 0
        for u, v in G.edges():
            diff = abs(u - v)
            if diff == 1 or diff == rows:
                matching_edges += 1
        # If a significant majority of edges match a grid, assume it's a grid
        if len(G.edges()) > 0 and matching_edges > 0.8 * len(G.edges()):
            pos = {}
            for u in range(1, num_nodes + 1):
                col = (u - 1) // rows
                row = (u - 1) % rows
                pos[u] = (col, rows - 1 - row)
            return pos, "Grid"
            
    # 2. Knight's Chessboard Check (perfect square layout)
    s_float = num_nodes**0.5
    s = int(round(s_float))
    if s * s == num_nodes:
        matching_moves = 0
        for u, v in G.edges():
            i_u = u - 1
            i_v = v - 1
            r = abs((i_u % s) - (i_v % s))
            c = abs((i_u // s) - (i_v // s))
            if (r == 1 and c == 2) or (r == 2 and c == 1):
                matching_moves += 1
        # If majority of edges match knight moves, classify as Knight Chessboard
        if len(G.edges()) > 0 and matching_moves > 0.8 * len(G.edges()):
            pos = {}
            for u in range(1, num_nodes + 1):
                i = u - 1
                col = i % s
                row = i // s
                pos[u] = (col, s - 1 - row)
            return pos, "Knight Chessboard"

    # 3. Fallback: Spring Layout
    if optimize_spring and path_nodes and cycle_edges:
        CycleG = nx.Graph()
        CycleG.add_nodes_from(path_nodes)
        CycleG.add_edges_from(cycle_edges)
        return nx.spring_layout(CycleG), "Spring (Cycle Subgraph)"
    return nx.spring_layout(G), "Spring"

def visualize(graph_path, path_path, output_path):
    print(f"c Loading graph {graph_path}...")
    G, num_nodes = read_graph(graph_path)
    
    print(f"c Loading solution path {path_path}...")
    path_nodes = read_path(path_path)
    
    # Build directed cycle edges
    cycle_edges = []
    if len(path_nodes) > 1:
        for i in range(len(path_nodes) - 1):
            cycle_edges.append((path_nodes[i], path_nodes[i+1]))
        # Close cycle
        cycle_edges.append((path_nodes[-1], path_nodes[0]))
        
    is_large_or_dense = (num_nodes > 500 or len(G.edges()) > 5000)
    pos, layout_type = detect_layout(G, num_nodes, path_nodes=path_nodes, cycle_edges=cycle_edges, optimize_spring=is_large_or_dense)
    print(f"c Detected layout style: {layout_type}")
    
    plt.figure(figsize=(10, 10))
    plt.title(f"Hamiltonian Cycle on {os.path.basename(graph_path)}\nNodes: {num_nodes}, Layout: {layout_type}")
    
    # Rendering threshold for large/dense graphs to prevent CPU/memory exhaust
    if is_large_or_dense:
        print("c Large/dense graph detected. Rendering ONLY the cycle path to prevent memory exhaustion.")
        # Draw only cycle nodes and cycle edges
        nx.draw_networkx_nodes(G, pos, nodelist=path_nodes, node_size=10, node_color='blue')
        nx.draw_networkx_edges(G, pos, edgelist=cycle_edges, edge_color='red', width=2.0, arrows=True, arrowsize=8)
    else:
        # Draw full graph in background (nodes + edges)
        nx.draw_networkx_nodes(G, pos, node_size=15, node_color='lightblue')
        nx.draw_networkx_edges(G, pos, edge_color='lightgray', width=0.5)
        
        # Draw cycle overlay
        nx.draw_networkx_edges(G, pos, edgelist=cycle_edges, edge_color='red', width=2.5, arrows=True, arrowsize=10)
        
        # Highlight start node in green
        if path_nodes:
            nx.draw_networkx_nodes(G, pos, nodelist=[path_nodes[0]], node_size=40, node_color='green')

    plt.axis('off')
    plt.tight_layout()
    plt.savefig(output_path, dpi=150)
    plt.close()
    print(f"c Visualization saved successfully to {output_path}")

def main():
    script_dir = os.path.dirname(os.path.abspath(__file__))
    args = parse_args()
    
    if args.graph and args.path and args.output:
        # Single run
        visualize(args.graph, args.path, args.output)
    else:
        # Batch run
        solution_paths_dir = os.path.join(script_dir, "../solution_paths")
        graphs_dir = os.path.join(script_dir, "../graphs")
        visualizations_dir = os.path.join(script_dir, "../visualizations")
        
        if not os.path.exists(visualizations_dir):
            os.makedirs(visualizations_dir)
            
        if not os.path.exists(solution_paths_dir):
            print(f"Error: Solution paths directory {solution_paths_dir} does not exist.")
            sys.exit(1)
            
        path_files = [f for f in os.listdir(solution_paths_dir) if f.endswith(".path")]
        if not path_files:
            print("No solution path files found.")
            sys.exit(0)
            
        print(f"c Found {len(path_files)} solution paths. Generating visualizations...")
        for path_file in path_files:
            graph_name = os.path.splitext(path_file)[0]
            graph_path = os.path.join(graphs_dir, f"{graph_name}.edge")
            path_path = os.path.join(solution_paths_dir, path_file)
            output_path = os.path.join(visualizations_dir, f"{graph_name}.png")
            
            if os.path.exists(graph_path):
                try:
                    visualize(graph_path, path_path, output_path)
                except Exception as e:
                    print(f"Error visualizing {graph_name}: {e}")
            else:
                print(f"Warning: Graph file {graph_path} not found for path {path_file}")

if __name__ == "__main__":
    main()
