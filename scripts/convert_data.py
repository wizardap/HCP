import os
import sys

def convert_hcp(in_path, out_path):
    nNode = None
    edges = []
    in_edge_section = False
    with open(in_path, 'r') as f:
        for line in f:
            line = line.strip()
            if not line:
                continue
            if line.upper().startswith("DIMENSION") or line.upper().startswith("DIMENTION"):
                if ":" in line:
                    nNode = int(line.split(":")[1].strip())
                else:
                    nNode = int(line.split()[1].strip())
            elif line.upper().startswith("EDGE_DATA_SECTION") or line.upper().startswith("EDGE_DATA_SELECTION"):
                in_edge_section = True
            elif in_edge_section:
                if line == "-1" or line.upper().startswith("EOF"):
                    break
                parts = line.split()
                if len(parts) == 2:
                    edges.append((int(parts[0]), int(parts[1])))
    
    if nNode is None:
        raise ValueError(f"Could not parse DIMENSION/DIMENTION from {in_path}")
    
    nEdge = len(edges)
    os.makedirs(os.path.dirname(out_path), exist_ok=True)
    with open(out_path, 'w') as f:
        f.write(f"p edge {nNode} {nEdge}\n")
        for u, v in edges:
            f.write(f"e {u} {v}\n")

def convert_txt(in_path, out_path):
    nNode = None
    nEdge = None
    edges = []
    with open(in_path, 'r') as f:
        for line in f:
            line = line.strip()
            if not line:
                continue
            if line.startswith("p edge"):
                parts = line.split()
                nNode = int(parts[2])
                nEdge = int(parts[3])
            else:
                parts = line.split()
                if len(parts) == 2:
                    edges.append((int(parts[0]), int(parts[1])))
    
    if nNode is None or nEdge is None:
        raise ValueError(f"Could not parse 'p edge' header from {in_path}")
         
    os.makedirs(os.path.dirname(out_path), exist_ok=True)
    with open(out_path, 'w') as f:
        f.write(f"p edge {nNode} {nEdge}\n")
        for u, v in edges:
            f.write(f"e {u} {v}\n")

def main():
    script_dir = os.path.dirname(os.path.abspath(__file__))
    data_dir = os.path.join(script_dir, "../data")
    graphs_dir = os.path.join(script_dir, "../graphs")
    
    for root, dirs, files in os.walk(data_dir):
        for file in files:
            in_path = os.path.join(root, file)
            rel_path = os.path.relpath(in_path, data_dir)
            
            # Form destination filename
            base, ext = os.path.splitext(rel_path)
            out_path = os.path.join(graphs_dir, base + ".edge")
            
            print(f"Converting {rel_path} -> {base}.edge...")
            if file.endswith(".hcp"):
                convert_hcp(in_path, out_path)
            elif file.endswith(".txt"):
                convert_txt(in_path, out_path)
            else:
                print(f"Skipping unknown file format: {file}")

if __name__ == "__main__":
    main()
