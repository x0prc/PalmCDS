use palmcds::{BuildError, Graph, GraphBuilder, NodeId, Reorder};

#[test]
fn from_edges_should_store_nodes_and_edges() {
    let graph = Graph::from_edges(
        vec!["a", "b", "c"],
        [
            (NodeId::new(0), NodeId::new(1), 10),
            (NodeId::new(0), NodeId::new(2), 20),
            (NodeId::new(2), NodeId::new(0), 30),
        ],
    )
    .unwrap();

    assert_eq!(graph.node_count(), 3);
    assert_eq!(graph.edge_count(), 3);
    assert_eq!(graph.node_data(NodeId::new(1)), Some(&"b"));
}

#[test]
fn edges_from_should_return_contiguous_outgoing_edges() {
    let graph = Graph::from_edges(
        vec![(), (), ()],
        [
            (NodeId::new(2), NodeId::new(0), "c-a"),
            (NodeId::new(0), NodeId::new(1), "a-b"),
            (NodeId::new(0), NodeId::new(2), "a-c"),
        ],
    )
    .unwrap();

    let edges: Vec<_> = graph
        .edges_from(NodeId::new(0))
        .unwrap()
        .map(|edge| (edge.target(), *edge.data()))
        .collect();

    assert_eq!(edges, [(NodeId::new(1), "a-b"), (NodeId::new(2), "a-c")]);
}

#[test]
fn empty_outgoing_range_should_be_supported() {
    let graph = Graph::<_, ()>::from_edges(vec!["a", "b"], []).unwrap();

    assert_eq!(graph.out_degree(NodeId::new(1)), Some(0));
    assert_eq!(graph.edges_from(NodeId::new(1)).unwrap().len(), 0);
}

#[test]
fn from_edges_should_reject_invalid_sources() {
    let err = Graph::from_edges(vec![()], [(NodeId::new(1), NodeId::new(0), ())]).unwrap_err();

    assert_eq!(
        err,
        BuildError::InvalidNodeId {
            id: NodeId::new(1),
            node_count: 1,
        }
    );
}

#[test]
fn from_edges_should_reject_invalid_targets() {
    let err = Graph::from_edges(vec![()], [(NodeId::new(0), NodeId::new(1), ())]).unwrap_err();

    assert_eq!(
        err,
        BuildError::InvalidNodeId {
            id: NodeId::new(1),
            node_count: 1,
        }
    );
}

#[test]
fn accessors_should_return_none_for_invalid_node_ids() {
    let graph = Graph::<_, ()>::from_edges(vec!["a"], []).unwrap();

    assert_eq!(graph.node_data(NodeId::new(2)), None);
    assert!(graph.edges_from(NodeId::new(2)).is_none());
    assert!(graph.neighbors(NodeId::new(2)).is_none());
    assert_eq!(graph.out_degree(NodeId::new(2)), None);
}

#[test]
fn storage_bytes_should_report_allocated_csr_arenas() {
    let graph = Graph::from_edges(
        vec![1_u32, 2, 3],
        [
            (NodeId::new(0), NodeId::new(1), 10_u64),
            (NodeId::new(0), NodeId::new(2), 20),
        ],
    )
    .unwrap();

    assert_eq!(
        graph.node_storage_bytes(),
        3 * Graph::<u32, u64>::node_entry_size()
    );
    assert_eq!(
        graph.edge_storage_bytes(),
        2 * (Graph::<u32, u64>::edge_target_size() + Graph::<u32, u64>::edge_payload_size())
    );
    assert_eq!(
        graph.total_storage_bytes(),
        graph.node_storage_bytes() + graph.edge_storage_bytes()
    );
}

#[test]
fn storage_bytes_should_support_empty_graphs() {
    let graph = Graph::<(), ()>::from_edges(Vec::new(), []).unwrap();

    assert_eq!(graph.node_storage_bytes(), 0);
    assert_eq!(graph.edge_storage_bytes(), 0);
    assert_eq!(graph.total_storage_bytes(), 0);
}

#[test]
fn neighbors_should_return_outgoing_targets_only() {
    let graph = Graph::from_edges(
        vec![(), (), ()],
        [
            (NodeId::new(2), NodeId::new(0), "c-a"),
            (NodeId::new(0), NodeId::new(1), "a-b"),
            (NodeId::new(0), NodeId::new(2), "a-c"),
        ],
    )
    .unwrap();

    let neighbors: Vec<_> = graph.neighbors(NodeId::new(0)).unwrap().collect();

    assert_eq!(neighbors, [NodeId::new(1), NodeId::new(2)]);
}

#[test]
fn neighbors_should_support_empty_outgoing_ranges() {
    let graph = Graph::<_, ()>::from_edges(vec!["a", "b"], []).unwrap();
    let neighbors = graph.neighbors(NodeId::new(1)).unwrap();

    assert_eq!(neighbors.len(), 0);
}

#[test]
fn neighbors_should_report_exact_remaining_length() {
    let graph = Graph::from_edges(
        vec![(), (), ()],
        [
            (NodeId::new(0), NodeId::new(1), ()),
            (NodeId::new(0), NodeId::new(2), ()),
        ],
    )
    .unwrap();
    let mut neighbors = graph.neighbors(NodeId::new(0)).unwrap();

    assert_eq!(neighbors.len(), 2);
    assert_eq!(neighbors.next(), Some(NodeId::new(1)));
    assert_eq!(neighbors.len(), 1);
}

#[test]
fn bfs_should_visit_reachable_nodes_by_level() {
    let graph = Graph::from_edges(
        vec![(), (), (), ()],
        [
            (NodeId::new(0), NodeId::new(1), ()),
            (NodeId::new(0), NodeId::new(2), ()),
            (NodeId::new(1), NodeId::new(3), ()),
            (NodeId::new(2), NodeId::new(3), ()),
        ],
    )
    .unwrap();

    let visited: Vec<_> = graph.bfs(NodeId::new(0)).unwrap().collect();

    assert_eq!(
        visited,
        [
            NodeId::new(0),
            NodeId::new(1),
            NodeId::new(2),
            NodeId::new(3)
        ]
    );
}

#[test]
fn dfs_should_visit_reachable_nodes_depth_first() {
    let graph = Graph::from_edges(
        vec![(), (), (), ()],
        [
            (NodeId::new(0), NodeId::new(1), ()),
            (NodeId::new(0), NodeId::new(2), ()),
            (NodeId::new(1), NodeId::new(3), ()),
            (NodeId::new(2), NodeId::new(3), ()),
        ],
    )
    .unwrap();

    let visited: Vec<_> = graph.dfs(NodeId::new(0)).unwrap().collect();

    assert_eq!(
        visited,
        [
            NodeId::new(0),
            NodeId::new(1),
            NodeId::new(3),
            NodeId::new(2)
        ]
    );
}

#[test]
fn traversals_should_not_repeat_nodes_in_cycles() {
    let graph = Graph::from_edges(
        vec![(), (), ()],
        [
            (NodeId::new(0), NodeId::new(1), ()),
            (NodeId::new(1), NodeId::new(2), ()),
            (NodeId::new(2), NodeId::new(0), ()),
            (NodeId::new(2), NodeId::new(2), ()),
        ],
    )
    .unwrap();

    let visited: Vec<_> = graph.bfs(NodeId::new(0)).unwrap().collect();

    assert_eq!(visited, [NodeId::new(0), NodeId::new(1), NodeId::new(2)]);
}

#[test]
fn traversals_should_skip_disconnected_nodes() {
    let graph =
        Graph::<_, ()>::from_edges(vec!["a", "b", "c"], [(NodeId::new(0), NodeId::new(1), ())])
            .unwrap();

    let visited: Vec<_> = graph.bfs(NodeId::new(0)).unwrap().collect();

    assert_eq!(visited, [NodeId::new(0), NodeId::new(1)]);
}

#[test]
fn traversals_should_return_none_for_invalid_start_ids() {
    let graph = Graph::<_, ()>::from_edges(vec!["a"], []).unwrap();

    assert!(graph.bfs(NodeId::new(1)).is_none());
    assert!(graph.dfs(NodeId::new(1)).is_none());
}

#[test]
fn into_reordered_should_relabel_nodes_in_bfs_order() {
    let graph = Graph::from_edges(
        vec!["a", "b", "c", "d"],
        [
            (NodeId::new(2), NodeId::new(0), ()),
            (NodeId::new(0), NodeId::new(1), ()),
        ],
    )
    .unwrap()
    .into_reordered(Reorder::Bfs {
        root: NodeId::new(2),
    })
    .unwrap();

    assert_eq!(graph.node_data(NodeId::new(0)), Some(&"c"));
    assert_eq!(graph.node_data(NodeId::new(1)), Some(&"a"));
    assert_eq!(graph.node_data(NodeId::new(2)), Some(&"b"));
    assert_eq!(graph.node_data(NodeId::new(3)), Some(&"d"));
    assert_eq!(
        graph.neighbors(NodeId::new(0)).unwrap().collect::<Vec<_>>(),
        [NodeId::new(1)]
    );
}

#[test]
fn reordered_should_clone_without_changing_original_graph() {
    let graph =
        Graph::from_edges(vec!["a", "b", "c"], [(NodeId::new(2), NodeId::new(0), ())]).unwrap();

    let reordered = graph
        .reordered(Reorder::Bfs {
            root: NodeId::new(2),
        })
        .unwrap();

    assert_eq!(graph.node_data(NodeId::new(0)), Some(&"a"));
    assert_eq!(reordered.node_data(NodeId::new(0)), Some(&"c"));
}

#[test]
fn builder_should_apply_configured_reorder_during_build() {
    let mut builder = GraphBuilder::<_, ()>::new();
    let a = builder.add_node("a").unwrap();
    let b = builder.add_node("b").unwrap();
    let c = builder.add_node("c").unwrap();

    builder.add_edge(c, a, ()).unwrap();
    builder.add_edge(a, b, ()).unwrap();
    builder.reorder(Reorder::Bfs { root: c });

    let graph = builder.build().unwrap();

    assert_eq!(graph.node_data(NodeId::new(0)), Some(&"c"));
    assert_eq!(graph.node_data(NodeId::new(1)), Some(&"a"));
    assert_eq!(graph.node_data(NodeId::new(2)), Some(&"b"));
}

#[test]
fn reorder_none_should_keep_node_order() {
    let graph = Graph::<_, ()>::from_edges(vec!["a", "b"], [])
        .unwrap()
        .into_reordered(Reorder::None)
        .unwrap();

    assert_eq!(graph.node_data(NodeId::new(0)), Some(&"a"));
    assert_eq!(graph.node_data(NodeId::new(1)), Some(&"b"));
}

#[test]
fn reordering_should_reject_invalid_roots() {
    let err = Graph::<_, ()>::from_edges(vec!["a"], [])
        .unwrap()
        .into_reordered(Reorder::Bfs {
            root: NodeId::new(1),
        })
        .unwrap_err();

    assert_eq!(
        err,
        BuildError::InvalidNodeId {
            id: NodeId::new(1),
            node_count: 1,
        }
    );
}

#[test]
fn builder_should_build_compact_graph() {
    let mut builder = GraphBuilder::new();
    let a = builder.add_node("a").unwrap();
    let b = builder.add_node("b").unwrap();
    let c = builder.add_node("c").unwrap();

    builder.add_edge(a, b, 10).unwrap();
    builder.add_edge(a, c, 20).unwrap();

    let graph = builder.build().unwrap();
    let edges: Vec<_> = graph
        .edges_from(a)
        .unwrap()
        .map(|edge| (edge.target(), *edge.data()))
        .collect();

    assert_eq!(edges, [(b, 10), (c, 20)]);
}

#[test]
fn builder_should_track_pending_counts() {
    let mut builder = GraphBuilder::<_, ()>::with_capacity(2, 1);
    let a = builder.add_node("a").unwrap();
    let b = builder.add_node("b").unwrap();

    builder.add_edge(a, b, ()).unwrap();

    assert_eq!(builder.node_count(), 2);
    assert_eq!(builder.edge_count(), 1);
}

#[test]
fn builder_should_reject_invalid_edges_before_build() {
    let mut builder = GraphBuilder::new();
    let a = builder.add_node("a").unwrap();

    let err = builder.add_edge(a, NodeId::new(1), ()).unwrap_err();

    assert_eq!(
        err,
        BuildError::InvalidNodeId {
            id: NodeId::new(1),
            node_count: 1,
        }
    );
}

#[test]
fn graph_should_create_builder_for_matching_payload_types() {
    let mut builder = Graph::<&str, i32>::builder();
    let a = builder.add_node("a").unwrap();
    let b = builder.add_node("b").unwrap();

    builder.add_edge(a, b, 5).unwrap();

    assert_eq!(builder.build().unwrap().edge_count(), 1);
}
