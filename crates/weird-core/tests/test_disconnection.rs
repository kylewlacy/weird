use pretty_assertions::assert_eq;
use weird_core::world::{ElementNodeUpdate, Node, ROOT_NODE_ID};
use weird_test_support::{init_world, updated};

#[tokio::test(flavor = "multi_thread")]
async fn test_disconnect_marks_windows_as_stale() {
    let (world, _conn_1, _window_1) = init_world([]).await;

    let conn_2 = world.create_connection().await;
    let window_2_a = world
        .append_node(
            Node::element("Window").attr("title", "Window A"),
            ROOT_NODE_ID,
            conn_2.id,
        )
        .await;
    let window_2_b = world
        .append_node(
            Node::element("Window").attr("title", "Window B"),
            ROOT_NODE_ID,
            conn_2.id,
        )
        .await;
    let (_, mut change_events_rx) = world.subscribe_to_world_did_change_events().await;

    drop(conn_2);

    let diff = change_events_rx.recv().await.unwrap();
    assert_eq!(
        diff.changes,
        &[
            updated(
                window_2_a,
                ElementNodeUpdate::default().set_attr("stale", true)
            ),
            updated(
                window_2_b,
                ElementNodeUpdate::default().set_attr("stale", true)
            ),
        ]
    );

    world.assert_internally_consistent().await;
}
