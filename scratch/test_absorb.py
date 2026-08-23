#!/usr/bin/env python3
import sys

def parse_col(filepath):
    adj = {}
    with open(filepath, 'r') as f:
        for line in f:
            line = line.strip()
            if not line or line.startswith('c'):
                continue
            parts = line.split()
            if parts[0] == 'p':
                n = int(parts[2])
                for i in range(1, n + 1):
                    adj[i] = set()
            elif parts[0] == 'e':
                u, v = int(parts[1]), int(parts[2])
                if u not in adj: adj[u] = set()
                if v not in adj: adj[v] = set()
                adj[u].add(v)
                adj[v].add(u)
    return adj

print("Testing subcycle absorber idea...")
