use std::collections::{HashMap, HashSet};

use weird_core::world::{
    Connection, ConnectionId, FlatNode, InsertNode, InsertedNode, Node, NodeId, NodeUpdate,
    ROOT_NODE_ID, UpdatedNode, World, WorldDidChangeResponse,
};

macro_rules! assert_inserted_eq {
    ($inserted:expr, $parent_id:expr, $node:expr) => {{
        let inserted = &$inserted;
        let parent_id = &$parent_id;
        let node = &$node;
        let parent_id_matches = inserted.parent_id == *parent_id;
        let node_matches = inserted.node.as_deref() == Some(node);
        match (parent_id_matches, node_matches) {
            (true, true) => {},
            (false, true) => {
                panic!("assertion failed, parent ID does not match:\n  expected parent ID: {parent_id:?}\n  actual parent ID: {:?}", inserted.parent_id);
            }
            (true, false) => {
                panic!("assertion failed, node does not match:\n  expected node: {node:?}\n  actual node: {:?}", inserted.node);
            }
            (false, false) => {
                panic!("assertion failed, node and parent ID don't match:\n  expected parent ID {parent_id:?}, got {:?}\n  expected node: {node:?}\n  actual node: {:?}", inserted.parent_id, inserted.node);
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
    world.assert_internally_consistent();

    let mut rx = world.subscribe_to_world_did_change_events();
    let did_change = world.set_node_children(node_id, children, conn_id).unwrap();
    let result = if did_change {
        rx.recv().await.unwrap()
    } else {
        WorldDidChangeResponse::default()
    };

    world.assert_internally_consistent();

    result
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

pub fn init_world(children: impl IntoIterator<Item = Node>) -> (World, Connection, NodeId) {
    use tracing_subscriber::{layer::SubscriberExt as _, util::SubscriberInitExt as _};

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::fmt::layer()
                .compact()
                .with_target(false)
                .without_time(),
        )
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let mut world = World::default();
    let conn = world.create_connection();

    let window_id = world.create_node(Node::element("Window").children(children), conn.id);
    world
        .insert_node(InsertNode {
            parent: ROOT_NODE_ID,
            child: window_id,
            offset: weird_core::world::InsertNodeOffset::END,
        })
        .unwrap();

    (world, conn, window_id)
}

#[tokio::test]
async fn test_world_did_change_nothing() {
    let (mut world, conn, window) = init_world([]);

    let diff = render(&mut world, conn.id, window, vec![]).await;

    assert_eq!(diff, WorldDidChangeResponse::default());
}

#[tokio::test]
async fn test_world_did_change_add_to_empty() {
    let (mut world, conn, window) = init_world([]);

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

    let WorldDidChangeResponse {
        inserted,
        removed,
        updated,
    } = diff;
    assert_eq!(removed, vec![]);
    assert_eq!(updated, vec![]);
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
    let (mut world, conn, window) = init_world([
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
    ]);

    let nodes = world
        .nodes()
        .map(|(node_id, node)| (node_id, node.clone()))
        .collect::<HashMap<_, _>>();
    let diff = render(&mut world, conn.id, window, vec![]).await;

    let WorldDidChangeResponse {
        inserted,
        removed,
        updated,
    } = diff;
    assert_eq!(inserted, vec![]);
    assert_eq!(updated, vec![]);

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
            FlatNode::element("Box").id("node2"),
            FlatNode::text("Bar"),
            FlatNode::text("Click me"),
            FlatNode::text("Foo"),
        ]
    );
}

#[tokio::test]
async fn test_world_did_change_replace_all() {
    let (mut world, conn, window) = init_world([
        Node::element("Fizz"),
        Node::element("Buzz"),
        Node::text("foobar"),
    ]);

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

    let WorldDidChangeResponse {
        inserted,
        removed,
        updated,
    } = diff;
    assert_eq!(updated, vec![]);

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

#[tokio::test]
async fn test_world_did_change_reorder_nodes() {
    let (mut world, conn, window) = init_world([
        Node::element("First"),
        Node::element("Foo"),
        Node::element("Bar").id("node-bar"),
        Node::element("Other"),
        Node::text("text"),
        Node::element("Last"),
    ]);

    let foo_id = world
        .nodes()
        .find_map(|(id, node)| {
            if let FlatNode::Element(el) = node
                && el.tag == "Foo"
            {
                Some(id)
            } else {
                None
            }
        })
        .unwrap();
    let bar_id = world
        .nodes()
        .find_map(|(id, node)| {
            if node.get_id() == Some("node-bar") {
                Some(id)
            } else {
                None
            }
        })
        .unwrap();
    let other_id = world
        .nodes()
        .find_map(|(id, node)| {
            if let FlatNode::Element(el) = node
                && el.tag == "Other"
            {
                Some(id)
            } else {
                None
            }
        })
        .unwrap();
    let text_id = world
        .nodes()
        .find_map(|(id, node)| {
            if matches!(node, FlatNode::Text(_)) {
                Some(id)
            } else {
                None
            }
        })
        .unwrap();

    let diff = render(
        &mut world,
        conn.id,
        window,
        vec![
            Node::element("First"),
            Node::element("Other"),
            Node::element("Foo"),
            Node::text("text"),
            Node::element("Bar").id("node-bar"),
            Node::element("Last"),
        ],
    )
    .await;

    let WorldDidChangeResponse {
        inserted,
        removed,
        updated,
    } = diff;
    assert_eq!(updated, vec![]);
    assert_eq!(removed, vec![]);
    assert_eq!(
        inserted,
        vec![
            InsertedNode {
                id: other_id,
                parent_id: window,
                parent_index: 1,
                node: None,
            },
            InsertedNode {
                id: foo_id,
                parent_id: window,
                parent_index: 2,
                node: None,
            },
            InsertedNode {
                id: text_id,
                parent_id: window,
                parent_index: 3,
                node: None,
            },
            InsertedNode {
                id: bar_id,
                parent_id: window,
                parent_index: 4,
                node: None,
            },
        ]
    );
}

#[tokio::test]
async fn test_world_did_change_update_nodes() {
    let (mut world, conn, window) = init_world([
        Node::element("Foo").attr("value", 1),
        Node::element("Bar").attr("value", "A"),
        Node::text("text 1"),
        Node::text("text 2"),
    ]);

    let foo_id = world
        .nodes()
        .find_map(|(id, node)| {
            if let FlatNode::Element(el) = node
                && el.tag == "Foo"
            {
                Some(id)
            } else {
                None
            }
        })
        .unwrap();
    let bar_id = world
        .nodes()
        .find_map(|(id, node)| {
            if let FlatNode::Element(el) = node
                && el.tag == "Bar"
            {
                Some(id)
            } else {
                None
            }
        })
        .unwrap();
    let text_1_id = world
        .nodes()
        .find_map(|(id, node)| {
            if let FlatNode::Text(text) = node
                && text == "text 1"
            {
                Some(id)
            } else {
                None
            }
        })
        .unwrap();
    let text_2_id = world
        .nodes()
        .find_map(|(id, node)| {
            if let FlatNode::Text(text) = node
                && text == "text 2"
            {
                Some(id)
            } else {
                None
            }
        })
        .unwrap();

    let diff = render(
        &mut world,
        conn.id,
        window,
        vec![
            Node::element("Foo").attr("value", "B").attr("added", "C"),
            Node::element("Bar"),
            Node::text("text 1!"),
            Node::text("text 2!"),
        ],
    )
    .await;

    let WorldDidChangeResponse {
        inserted,
        removed,
        updated,
    } = diff;
    assert_eq!(removed, vec![]);
    assert_eq!(inserted, vec![]);
    assert_eq!(
        updated,
        vec![
            UpdatedNode {
                id: foo_id,
                update: NodeUpdate::Element {
                    set_attributes: HashMap::from_iter([
                        ("value".to_string(), serde_json::to_value("B").unwrap()),
                        ("added".to_string(), serde_json::to_value("C").unwrap())
                    ]),
                    clear_attributes: HashSet::new(),
                }
            },
            UpdatedNode {
                id: bar_id,
                update: NodeUpdate::Element {
                    set_attributes: HashMap::new(),
                    clear_attributes: HashSet::from_iter(["value".to_string()]),
                },
            },
            UpdatedNode {
                id: text_1_id,
                update: NodeUpdate::Text {
                    text: "text 1!".to_string()
                },
            },
            UpdatedNode {
                id: text_2_id,
                update: NodeUpdate::Text {
                    text: "text 2!".to_string()
                },
            },
        ]
    );
}

#[tokio::test]
async fn test_world_did_change_complex() {
    let (mut world, conn, window) = init_world([
        Node::element("Foo").attr("value", 1),
        Node::element("Bar").attr("value", "A"),
        Node::text("text 1"),
        Node::text("text 2"),
    ]);

    render(
        &mut world,
        conn.id,
        window,
        vec![
            Node::text("Running 4/5"),
            Node::element("ProgressBar")
                .attr("value", 4)
                .attr("max", 5)
                .children([Node::element("Other"), Node::text("Progress 4")]),
            Node::element("Box")
                .attr("id", "label1")
                .child(Node::text("almost done...")),
            Node::element("Box")
                .attr("id", "label2")
                .child(Node::text("...")),
        ],
    )
    .await;
    render(
        &mut world,
        conn.id,
        window,
        vec![
            Node::text("Running 5/5"),
            Node::element("ProgressBar")
                .attr("value", 5)
                .attr("max", 5)
                .children([Node::element("Other"), Node::text("Progress 5")]),
            Node::element("Box")
                .attr("id", "label2")
                .child(Node::text("almost done...")),
            Node::element("Box")
                .attr("id", "label3")
                .child(Node::text("...")),
            Node::element("Box")
                .attr("id", "label1")
                .child(Node::text("...")),
        ],
    )
    .await;
    render(
        &mut world,
        conn.id,
        window,
        vec![
            Node::text("Hello, a!"),
            Node::element("Input")
                .id("name")
                .attr("value", "a")
                .attr("placeholder", "Your name"),
            Node::element("Button").id("run").child(Node::text("Run")),
            Node::element("Button").id("exit").child(Node::text("Exit")),
        ],
    )
    .await;
}
