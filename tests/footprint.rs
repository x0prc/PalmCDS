use palmcds::{Graph, GraphBuilder, NodeId};
use std::mem::size_of;

const NODE_COUNT: usize = 128;
const FANOUT: usize = 4;

fn adjacency_storage_bytes(adjacency: &Vec<Vec<NodeId>>) -> usize {
    let outer_storage = adjacency.capacity() * size_of::<Vec<NodeId>>();
    let inner_storage = adjacency
        .iter()
        .map(|targets| targets.capacity() * size_of::<NodeId>())
        .sum::<usize>();

    outer_storage + inner_storage
}

fn build_graph() -> Graph<(), ()> {
    let mut builder = GraphBuilder::with_capacity(NODE_COUNT, NODE_COUNT * FANOUT);
    let nodes: Vec<_> = (0..NODE_COUNT)
        .map(|_| builder.add_node(()).unwrap())
        .collect();

    for source in 0..NODE_COUNT {
        for offset in 1..=FANOUT {
            let target = (source + offset) % NODE_COUNT;
            builder.add_edge(nodes[source], nodes[target], ()).unwrap();
        }
    }

    builder.build().unwrap()
}

fn build_adjacency_list() -> Vec<Vec<NodeId>> {
    let mut adjacency = (0..NODE_COUNT)
        .map(|_| Vec::with_capacity(FANOUT))
        .collect::<Vec<_>>();

    for (source, targets) in adjacency.iter_mut().enumerate() {
        for offset in 1..=FANOUT {
            let target = (source + offset) % NODE_COUNT;
            targets.push(NodeId::new(target as u32));
        }
    }

    adjacency
}

#[test]
fn adjacency_storage_bytes_should_include_outer_and_inner_capacity() {
    let mut adjacency = Vec::with_capacity(2);
    let mut first = Vec::with_capacity(3);
    first.push(NodeId::new(1));
    let second = Vec::with_capacity(5);

    adjacency.push(first);
    adjacency.push(second);

    assert_eq!(
        adjacency_storage_bytes(&adjacency),
        2 * size_of::<Vec<NodeId>>() + 8 * size_of::<NodeId>()
    );
}

#[test]
fn palmcds_should_use_less_storage_than_vec_adjacency_for_fixed_fanout_topology() {
    let graph = build_graph();
    let adjacency = build_adjacency_list();

    assert!(graph.total_storage_bytes() < adjacency_storage_bytes(&adjacency));
}
