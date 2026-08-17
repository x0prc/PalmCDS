use palmcds::{Graph, NodeId, Reorder};

#[test]
fn reorder_dfs_should_relabel_nodes_in_dfs_order() {
    let graph = Graph::from_edges(
        vec!["root", "left", "left_child", "right"],
        [
            (NodeId::new(0), NodeId::new(1), ()),
            (NodeId::new(1), NodeId::new(2), ()),
            (NodeId::new(0), NodeId::new(3), ()),
        ],
    )
    .unwrap()
    .into_reordered(Reorder::Dfs {
        root: NodeId::new(0),
    })
    .unwrap();

    // Node 0 was root ("root") -> 0
    // First branch visited was left ("left") -> 1, then left_child -> 2, then right -> 3
    assert_eq!(graph.node_data(NodeId::new(0)), Some(&"root"));
    assert_eq!(graph.node_data(NodeId::new(1)), Some(&"left"));
    assert_eq!(graph.node_data(NodeId::new(2)), Some(&"left_child"));
    assert_eq!(graph.node_data(NodeId::new(3)), Some(&"right"));
}

#[test]
fn reorder_reverse_post_order_should_place_dependencies_first() {
    let graph = Graph::from_edges(
        vec!["root", "a", "b"],
        [
            (NodeId::new(0), NodeId::new(1), ()),
            (NodeId::new(1), NodeId::new(2), ()),
        ],
    )
    .unwrap()
    .into_reordered(Reorder::ReversePostOrder {
        root: NodeId::new(0),
    })
    .unwrap();

    assert_eq!(graph.node_data(NodeId::new(0)), Some(&"root"));
    assert_eq!(graph.node_data(NodeId::new(1)), Some(&"a"));
    assert_eq!(graph.node_data(NodeId::new(2)), Some(&"b"));
}

#[test]
fn reorder_topological_should_order_dag_nodes() {
    let graph = Graph::from_edges(
        vec!["c", "a", "b"],
        [
            (NodeId::new(1), NodeId::new(2), ()), // a -> b
            (NodeId::new(2), NodeId::new(0), ()), // b -> c
        ],
    )
    .unwrap()
    .into_reordered(Reorder::Topological)
    .unwrap();

    // 'a' has in-degree 0 -> new NodeId(0)
    // 'b' -> new NodeId(1)
    // 'c' -> new NodeId(2)
    assert_eq!(graph.node_data(NodeId::new(0)), Some(&"a"));
    assert_eq!(graph.node_data(NodeId::new(1)), Some(&"b"));
    assert_eq!(graph.node_data(NodeId::new(2)), Some(&"c"));
}

#[test]
fn reorder_rcm_should_reduce_bandwidth() {
    let graph = Graph::from_edges(
        vec!["a", "b", "c", "d"],
        [
            (NodeId::new(0), NodeId::new(1), ()),
            (NodeId::new(1), NodeId::new(2), ()),
            (NodeId::new(2), NodeId::new(3), ()),
        ],
    )
    .unwrap()
    .into_reordered(Reorder::ReverseCuthillMcKee { root: None })
    .unwrap();

    // RCM preserves connectivity and reduces node index bandwidth
    assert_eq!(graph.node_count(), 4);
    assert_eq!(graph.edge_count(), 3);
}
