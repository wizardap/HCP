import collections

def test_splice_gadget():
    # An 8-cycle has 8 vertices.
    # Giant cycle has n_g vertices.
    # Suppose u in Giant connects to v in 8-cycle.
    # Can we find a Hamiltonian path in the 8-cycle from v to some w,
    # where w connects to u_next (or some vertex in Giant)?
    print("Testing 8-cycle Hamiltonian path splice...")

test_splice_gadget()
