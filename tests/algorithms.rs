use palmcds::{CycleError, Graph, NodeId};

#[test]
fn topological_sort_should_order_dag_nodes() {
    let graph = Graph::from_edges(
        vec!["a", "b", "c"],
        [
            (NodeId::new(0), NodeId::new(1), ()),
            (NodeId::new(1), NodeId::new(2), ()),
        ],
    )
    .unwrap();

    let order = graph.topological_sort().unwrap();
    assert_eq!(order, vec![NodeId::new(0), NodeId::new(1), NodeId::new(2)]);
    assert!(!graph.has_cycle());
}

#[test]
fn topological_sort_should_fail_on_cycle() {
    let graph = Graph::from_edges(
        vec!["a", "b"],
        [
            (NodeId::new(0), NodeId::new(1), ()),
            (NodeId::new(1), NodeId::new(0), ()),
        ],
    )
    .unwrap();

    assert_eq!(graph.topological_sort(), Err(CycleError));
    assert!(graph.has_cycle());
}

#[test]
fn dijkstra_should_compute_shortest_paths() {
    let graph = Graph::from_edges(
        vec!["a", "b", "c", "d"],
        [
            (NodeId::new(0), NodeId::new(1), 5u64),
            (NodeId::new(0), NodeId::new(2), 2u64),
            (NodeId::new(2), NodeId::new(1), 1u64),
            (NodeId::new(1), NodeId::new(3), 3u64),
        ],
    )
    .unwrap();

    let dists = graph.dijkstra(NodeId::new(0), |&w| w).unwrap();
    assert_eq!(dists[0], Some(0));
    assert_eq!(dists[1], Some(3)); // 0 -> 2 -> 1 is cost 3
    assert_eq!(dists[2], Some(2)); // 0 -> 2 is cost 2
    assert_eq!(dists[3], Some(6)); // 0 -> 2 -> 1 -> 3 is cost 6
}

#[test]
fn scc_should_identify_connected_components() {
    let graph = Graph::from_edges(
        vec!["a", "b", "c"],
        [
            (NodeId::new(0), NodeId::new(1), ()),
            (NodeId::new(1), NodeId::new(0), ()),
        ],
    )
    .unwrap();

    let sccs = graph.strongly_connected_components();
    // 0 and 1 form an SCC, 2 is isolated
    assert_eq!(sccs.len(), 2);
}
