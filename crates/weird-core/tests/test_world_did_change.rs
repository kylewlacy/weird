use pretty_assertions::assert_eq;
use weird_core::world::{
    ElementNodeUpdate, FlatNode, MovedNodeChange, Node, TextNodeUpdate, WorldDidChangeResponse,
};
use weird_test_support::{
    created, deleted, init_world, node_id, node_tag, node_text, render, updated,
};

#[tokio::test(flavor = "multi_thread")]
async fn test_world_did_change_nothing() {
    let (mut world, conn, window) = init_world([]).await;

    let diff = render(&mut world, conn.id, window, vec![]).await;

    assert_eq!(diff, WorldDidChangeResponse::default());
}

#[tokio::test(flavor = "multi_thread")]
async fn test_world_did_change_add_to_empty() {
    let (mut world, conn, window) = init_world([]).await;

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

    let node_1 = node_id(&mut world, "node1").await;
    let node_2 = node_id(&mut world, "node2").await;
    let foo = node_text(&mut world, "Foo").await;
    let bar = node_text(&mut world, "Bar").await;
    let button = node_tag(&mut world, "Button").await;
    let click_me = node_text(&mut world, "Click me").await;
    let hello_world = node_text(&mut world, "hello world").await;

    assert_eq!(
        diff.changes,
        &[
            created(node_1, window, None, FlatNode::element("Box").id("node1")),
            created(node_2, node_1, None, FlatNode::element("Box").id("node2")),
            created(bar, node_1, None, FlatNode::text("Bar")),
            created(foo, node_2, None, FlatNode::text("Foo")),
            created(button, window, None, FlatNode::element("Button")),
            created(click_me, button, None, FlatNode::text("Click me")),
            created(hello_world, window, None, FlatNode::text("hello world")),
        ]
    );
}

#[tokio::test(flavor = "multi_thread")]
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
    ])
    .await;

    let node_1 = node_id(&mut world, "node1").await;
    let node_2 = node_id(&mut world, "node2").await;
    let foo = node_text(&mut world, "Foo").await;
    let bar = node_text(&mut world, "Bar").await;
    let button = node_tag(&mut world, "Button").await;
    let click_me = node_text(&mut world, "Click me").await;
    let hello_world = node_text(&mut world, "hello world").await;

    let diff = render(&mut world, conn.id, window, vec![]).await;

    assert_eq!(
        diff.changes,
        &[
            deleted(node_1),
            deleted(button),
            deleted(hello_world),
            deleted(node_2),
            deleted(bar),
            deleted(click_me),
            deleted(foo),
        ]
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn test_world_did_change_replace_all() {
    let (mut world, conn, window) = init_world([
        Node::element("Fizz"),
        Node::element("Buzz"),
        Node::text("foobar"),
    ])
    .await;

    let fizz = node_tag(&mut world, "Fizz").await;
    let buzz = node_tag(&mut world, "Buzz").await;
    let foobar = node_text(&mut world, "foobar").await;

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

    let foo = node_tag(&mut world, "Foo").await;
    let bar = node_tag(&mut world, "Bar").await;
    let other = node_tag(&mut world, "Other").await;

    assert_eq!(
        diff.changes,
        &[
            created(foo, window, None, FlatNode::element("Foo")),
            created(bar, window, None, FlatNode::element("Bar")),
            created(other, window, None, FlatNode::element("Other")),
            deleted(fizz),
            deleted(buzz),
            deleted(foobar),
        ]
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn test_world_did_change_reorder_nodes() {
    let (mut world, conn, window) = init_world([
        Node::element("First"),
        Node::element("Foo"),
        Node::element("Bar").id("node-bar"),
        Node::element("Other"),
        Node::text("text"),
        Node::element("Last"),
    ])
    .await;

    let foo = node_tag(&mut world, "Foo").await;
    let bar = node_id(&mut world, "node-bar").await;
    let other = node_tag(&mut world, "Other").await;
    let text = node_text(&mut world, "text").await;
    let last = node_tag(&mut world, "Last").await;

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

    println!("{:#?}", diff.changes);

    assert_eq!(
        diff.changes,
        &[
            MovedNodeChange {
                id: bar,
                parent_id: window,
                before_sibling_id: Some(last),
            }
            .into(),
            MovedNodeChange {
                id: text,
                parent_id: window,
                before_sibling_id: Some(bar),
            }
            .into(),
            MovedNodeChange {
                id: foo,
                parent_id: window,
                before_sibling_id: Some(text),
            }
            .into(),
            MovedNodeChange {
                id: other,
                parent_id: window,
                before_sibling_id: Some(foo),
            }
            .into(),
        ]
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn test_world_did_change_update_nodes() {
    let (mut world, conn, window) = init_world([
        Node::element("Foo").attr("value", 1),
        Node::element("Bar").attr("value", "A"),
        Node::text("text 1"),
        Node::text("text 2"),
    ])
    .await;

    let foo = node_tag(&mut world, "Foo").await;
    let bar = node_tag(&mut world, "Bar").await;
    let text_1 = node_text(&mut world, "text 1").await;
    let text_2 = node_text(&mut world, "text 2").await;

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

    assert_eq!(
        diff.changes,
        &[
            updated(
                foo,
                ElementNodeUpdate::default()
                    .set_attr("value", "B")
                    .set_attr("added", "C")
            ),
            updated(bar, ElementNodeUpdate::default().clear_attr("value")),
            updated(text_1, TextNodeUpdate::new("text 1!")),
            updated(text_2, TextNodeUpdate::new("text 2!")),
        ]
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn test_world_did_change_complex() {
    let (mut world, conn, window) = init_world([
        Node::element("Foo").attr("value", 1),
        Node::element("Bar").attr("value", "A"),
        Node::text("text 1"),
        Node::text("text 2"),
    ])
    .await;

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
