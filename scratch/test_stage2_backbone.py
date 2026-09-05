import pickle, os, sys
sys.path.insert(0, os.path.abspath(os.path.join(os.path.dirname(__file__), '..')))

from scratch.solve_graph788_dp_bitmask import load_graph, setup_stage1, acquire_giant_backbone

def test_cached_it15():
    assert os.path.exists('scratch/graph788_model_it15.pkl')
    with open('scratch/graph788_model_it15.pkl', 'rb') as f:
        data = pickle.load(f)
    assert len(data['active_edges']) == 1540
    print("Stage 2 cache test PASSED: 1540 active edges ready for 4-opt flip.")

def test_acquire_giant_backbone():
    G, d = load_graph('FHCPCS-col/graph788.col')
    b, n2b, _, _, _, _, _, _, _ = setup_stage1(G, d)
    e, c, pn, gi = acquire_giant_backbone(b, n2b)
    giant_len = len(c[gi]) // 2
    assert giant_len == 1432, f"Expected Giant to have 1432 blocks, got {giant_len}"
    assert len(c) == 19, f"Expected 19 cycles, got {len(c)}"
    print(f"Stage 2 Verified: Giant = {giant_len} blocks (93.0%), total cycles = {len(c)}.")

if __name__ == '__main__':
    test_cached_it15()
    test_acquire_giant_backbone()
