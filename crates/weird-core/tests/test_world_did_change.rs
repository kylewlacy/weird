use std::collections::HashMap;

use weird_core::world::{ConnectionId, FlatNode, Node, NodeId, World, WorldDidChangeResponse};

macro_rules! assert_inserted_eq {
    ($inserted:expr, $parent_id:expr, $node:expr) => {{
        let inserted = &$inserted;
        let parent_id = &$parent_id;
        let node = &$node;
        let parent_id_matches = inserted.parent == *parent_id;
        let node_matches = *inserted.node == *node;
        match (parent_id_matches, node_matches) {
            (true, true) => {},
            (false, true) => {
                panic!("assertion failed, parent ID does not match:\n  expected parent ID: {parent_id:?}\n  actual parent ID: {:?}", inserted.parent);
            }
            (true, false) => {
                panic!("assertion failed, node does not match:\n  expected node: {node:?}\n  actual node: {:?}", inserted.node);
            }
            (false, false) => {
                panic!("assertion failed, node and parent ID don't match:\n  expected parent ID {parent_id:?}, got {:?}\n  expected node: {node:?}\n  actual node: {:?}", inserted.parent, inserted.node);
            }
        }
    }};
    ($inserted_array:expr, [$(($parent_id:expr, $node:expr)),+]) => {{
        let actual_array = &$inserted_array;
        let expected_array = [$(($parent_id, $node)),*];
        let actual_array_len = ::std::iter::IntoIterator::into_iter(actual_array).count();
        let expected_array_len = expected_array.len();
        for (inserted, (parent_id, node)) in std::iter::zip(actual_array, &expected_array) {
            assert_inserted_eq!(inserted, *parent_id, *node);
        }
        assert_eq!(
            actual_array_len,
            expected_array_len,
            "expected inserted to contain {expected_array_len} item{}, contained {actual_array_len}",
            if expected_array_len == 1 { "" } else { "s" }
        );
    }};
    ($inserted_array:expr, [$(($parent_id:expr, $node:expr)),+,]) => {{
        assert_inserted_eq!($inserted_array, [$(($parent_id, $node)),*]);
    }};
}

pub async fn render(
    world: &mut World,
    conn_id: ConnectionId,
    node_id: NodeId,
    children: Vec<Node>,
) -> WorldDidChangeResponse {
    let mut rx = world.subscribe_to_world_did_change_events();
    world.set_node_children(node_id, children, conn_id).unwrap();
    rx.recv().await.unwrap()
}

pub fn node_id(world: &mut World, id: &str) -> NodeId {
    world
        .nodes()
        .find_map(|(node_id, node)| {
            if node.get_id() == Some(id) {
                Some(node_id)
            } else {
                None
            }
        })
        .unwrap()
}

#[tokio::test]
async fn test_world_did_change_nothing() {
    let mut world = World::default();
    let conn = world.create_connection();

    let window = world.create_node(Node::element("Window"), conn.id);
    let diff = render(&mut world, conn.id, window, vec![]).await;

    assert_eq!(diff, WorldDidChangeResponse::default());
}

#[tokio::test]
async fn test_world_did_change_add_to_empty() {
    let mut world = World::default();
    let conn = world.create_connection();

    let window = world.create_node(Node::element("Window"), conn.id);
    let diff = render(
        &mut world,
        conn.id,
        window,
        vec![
            Node::element("Box")
                .id("node1")
                .child(
                    Node::element("Box")
                        .attr("id", "node2")
                        .child(Node::text("Foo")),
                )
                .child(Node::text("Bar")),
            Node::element("Button").child(Node::text("Click me")),
            Node::text("hello world"),
        ],
    )
    .await;

    let WorldDidChangeResponse { inserted, removed } = diff;
    assert_eq!(removed, vec![]);
    assert_inserted_eq!(
        inserted,
        [
            (window, FlatNode::element("Box").id("node1")),
            (inserted[0].id, FlatNode::element("Box").id("node2")),
            (inserted[0].id, FlatNode::text("Bar")),
            (inserted[1].id, FlatNode::text("Foo")),
            (window, FlatNode::element("Button")),
            (inserted[4].id, FlatNode::text("Click me")),
            (window, FlatNode::text("hello world")),
        ]
    );
}

#[tokio::test]
async fn test_world_did_change_remove_all() {
    let mut world = World::default();
    let conn = world.create_connection();

    let window = world.create_node(
        Node::element("Window")
            .child(
                Node::element("Box")
                    .id("node1")
                    .child(
                        Node::element("Box")
                            .attr("id", "node2")
                            .child(Node::text("Foo")),
                    )
                    .child(Node::text("Bar")),
            )
            .child(Node::element("Button").child(Node::text("Click me")))
            .child(Node::text("hello world")),
        conn.id,
    );
    let nodes = world
        .nodes()
        .map(|(node_id, node)| (node_id, node.clone()))
        .collect::<HashMap<_, _>>();
    let diff = render(&mut world, conn.id, window, vec![]).await;

    let WorldDidChangeResponse { inserted, removed } = diff;
    assert_eq!(inserted, vec![]);

    let removed = removed
        .iter()
        .map(|node_id| nodes.get(node_id).unwrap().clone())
        .collect::<Vec<_>>();
    assert_eq!(
        removed,
        vec![
            FlatNode::element("Box").id("node1"),
            FlatNode::element("Button"),
            FlatNode::text("hello world"),
        ]
    );
}

#[tokio::test]
async fn test_world_did_change_replace_all() {
    let mut world = World::default();
    let conn = world.create_connection();

    let window = world.create_node(
        Node::element("Window")
            .child(Node::element("Fizz"))
            .child(Node::element("Buzz"))
            .child(Node::text("foobar")),
        conn.id,
    );
    let nodes = world
        .nodes()
        .map(|(node_id, node)| (node_id, node.clone()))
        .collect::<HashMap<_, _>>();
    let diff = render(
        &mut world,
        conn.id,
        window,
        vec![
            Node::element("Foo"),
            Node::element("Bar"),
            Node::element("Other"),
        ],
    )
    .await;

    let WorldDidChangeResponse { inserted, removed } = diff;

    let removed = removed
        .iter()
        .map(|node_id| nodes.get(node_id).unwrap().clone())
        .collect::<Vec<_>>();
    assert_eq!(
        removed,
        vec![
            FlatNode::element("Fizz"),
            FlatNode::element("Buzz"),
            FlatNode::text("foobar"),
        ]
    );

    assert_inserted_eq!(
        inserted,
        [
            (window, FlatNode::element("Foo")),
            (window, FlatNode::element("Bar")),
            (window, FlatNode::element("Other")),
        ]
    );
}
