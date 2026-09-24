use cegar_fix::assembly::tour_stitcher::stitch_subpath;

#[test]
fn test_stitch_forward_subpath() {
    // Skeleton: 1 -> 2 -> 3 -> 1, with virtual edge (1, 2)
    let skeleton = vec![1, 2, 3];
    // Subpath from 1 to 2 visiting {4, 5}: 1 -> 4 -> 5 -> 2
    let subpath = vec![1, 4, 5, 2];

    let stitched = stitch_subpath(&skeleton, 1, 2, &subpath).unwrap();
    // Expected stitched tour: 1, 4, 5, 2, 3
    assert_eq!(stitched, vec![1, 4, 5, 2, 3]);
}

#[test]
fn test_stitch_reversed_subpath() {
    // Skeleton: 2 -> 1 -> 3 -> 2 (edge traversed as 2 -> 1)
    let skeleton = vec![2, 1, 3];
    let subpath = vec![1, 4, 5, 2];

    let stitched = stitch_subpath(&skeleton, 1, 2, &subpath).unwrap();
    // Traversed 2 -> 1, so subpath reversed: 2, 5, 4, 1, 3
    assert_eq!(stitched, vec![2, 5, 4, 1, 3]);
}

#[test]
fn test_stitch_wraparound_forward() {
    // Skeleton: 3 -> 1 with virtual edge (1, 2) traversed as 1 -> 2 across wrap-around
    let skeleton = vec![2, 3, 1];
    let subpath = vec![1, 4, 5, 2];

    let stitched = stitch_subpath(&skeleton, 1, 2, &subpath).unwrap();
    // 2 -> 3 -> 1 -> 4 -> 5 -> (wrap to 2)
    assert_eq!(stitched, vec![2, 3, 1, 4, 5]);
}

#[test]
fn test_stitch_wraparound_reversed() {
    // Skeleton: 1 -> 3 -> 2 with virtual edge (1, 2) traversed as 2 -> 1 across wrap-around
    let skeleton = vec![1, 3, 2];
    let subpath = vec![1, 4, 5, 2];

    let stitched = stitch_subpath(&skeleton, 1, 2, &subpath).unwrap();
    // 1 -> 3 -> 2 -> 5 -> 4 -> (wrap to 1)
    assert_eq!(stitched, vec![1, 3, 2, 5, 4]);
}

#[test]
fn test_stitch_subpath_already_reversed_in_input() {
    // Port u=1, v=2, but subpath given as [2, 5, 4, 1]
    let skeleton = vec![1, 2, 3];
    let subpath = vec![2, 5, 4, 1];

    let stitched = stitch_subpath(&skeleton, 1, 2, &subpath).unwrap();
    assert_eq!(stitched, vec![1, 4, 5, 2, 3]);
}

#[test]
fn test_stitch_direct_edge_no_internal_nodes() {
    let skeleton = vec![1, 2, 3];
    let subpath = vec![1, 2];

    let stitched = stitch_subpath(&skeleton, 1, 2, &subpath).unwrap();
    assert_eq!(stitched, vec![1, 2, 3]);
}

#[test]
fn test_stitch_edge_not_found() {
    let skeleton = vec![1, 2, 3];
    let subpath = vec![1, 4, 3];

    // (1, 4) is not in skeleton
    assert!(stitch_subpath(&skeleton, 1, 4, &subpath).is_err());
}

#[test]
fn test_stitch_invalid_endpoints() {
    let skeleton = vec![1, 2, 3];
    let subpath = vec![1, 4, 5]; // endpoints 1, 5 != {1, 2}

    assert!(stitch_subpath(&skeleton, 1, 2, &subpath).is_err());
}
