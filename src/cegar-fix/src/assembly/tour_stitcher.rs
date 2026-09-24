//! Recursive Tour Assembly
//!
//! Splices subcomponent Hamiltonian paths into skeleton tours across virtual separation pair edges.

/// Splices a subcomponent Hamiltonian path into the skeleton tour where the virtual edge
/// between `port_u` and `port_v` was traversed.
///
/// Handles both forward (u ... v) and reverse (v ... u) traversal orientations in the tour.
pub fn stitch_subpath(
    skeleton_tour: &[i32],
    port_u: i32,
    port_v: i32,
    subpath: &[i32],
) -> Result<Vec<i32>, String> {
    if skeleton_tour.len() < 2 {
        return Err("Skeleton tour must have at least 2 vertices".to_string());
    }
    if subpath.len() < 2 {
        return Err("Subpath must have at least 2 vertices (endpoints)".to_string());
    }
    if port_u == port_v {
        return Err("Port vertices port_u and port_v must be distinct".to_string());
    }

    let sub_start = subpath[0];
    let sub_end = subpath[subpath.len() - 1];
    let endpoints_valid = (sub_start == port_u && sub_end == port_v)
        || (sub_start == port_v && sub_end == port_u);
    if !endpoints_valid {
        return Err(format!(
            "Subpath endpoints ({}, {}) do not match ports ({}, {})",
            sub_start, sub_end, port_u, port_v
        ));
    }

    let n = skeleton_tour.len();
    let mut stitched = Vec::with_capacity(n + subpath.len().saturating_sub(2));
    let mut replaced = false;

    for i in 0..n {
        let u = skeleton_tour[i];
        let v = skeleton_tour[(i + 1) % n];
        stitched.push(u);

        if !replaced && ((u == port_u && v == port_v) || (u == port_v && v == port_u)) {
            // Virtual edge traversed from u to v.
            // Extract intermediate nodes oriented from u to v.
            let intermediates = if sub_start == u {
                // subpath is already oriented u -> ... -> v
                subpath[1..subpath.len() - 1].to_vec()
            } else {
                // subpath is oriented v -> ... -> u; reverse intermediates for u -> v
                subpath[1..subpath.len() - 1].iter().rev().copied().collect()
            };

            stitched.extend(intermediates);
            replaced = true;
        }
    }

    if !replaced {
        return Err(format!(
            "Virtual edge ({}, {}) not found in skeleton tour",
            port_u, port_v
        ));
    }

    Ok(stitched)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stitch_basic() {
        let skeleton = vec![1, 2, 3];
        let subpath = vec![1, 4, 5, 2];
        let stitched = stitch_subpath(&skeleton, 1, 2, &subpath).unwrap();
        assert_eq!(stitched, vec![1, 4, 5, 2, 3]);
    }
}
