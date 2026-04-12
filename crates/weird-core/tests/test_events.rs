use pretty_assertions::assert_eq;
use weird_core::world::{Event, Node, TriggerEvent};
use weird_test_support::{deleted, init_world, node_id, node_text, render};

#[tokio::test(flavor = "multi_thread")]
async fn test_event_triggers_listener() {
    let (mut world, mut conn, window) = init_world([]).await;

    render(
        &mut world,
        conn.id,
        window,
        vec![
            Node::element("Button")
                .id("button")
                .child(Node::text("Click me")),
        ],
    )
    .await;

    let button = node_id(&mut world, "button").await;

    world
        .trigger_event(TriggerEvent {
            target_node_id: button,
            event: "click".into(),
            params: serde_json::Value::Object(Default::default()),
        })
        .await
        .unwrap();

    let event = conn.next_event().await.unwrap();
    assert_eq!(
        event,
        Event {
            event: "click".to_string(),
            target_id: Some("button".to_string()),
            target_node_id: button,
            params: serde_json::Value::Object(Default::default()),
        },
    );

    world.assert_internally_consistent().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn test_close_window_event_deletes_window() {
    let (mut world, mut conn, window) = init_world([]).await;

    render(
        &mut world,
        conn.id,
        window,
        vec![
            Node::element("Button")
                .id("button")
                .child(Node::text("Click me")),
        ],
    )
    .await;

    let button = node_id(&mut world, "button").await;
    let text = node_text(&mut world, "Click me").await;

    let (_, mut diff_rx) = world.subscribe_to_world_did_change_events().await;

    world
        .trigger_event(TriggerEvent {
            target_node_id: window,
            event: "close".into(),
            params: serde_json::Value::Object(Default::default()),
        })
        .await
        .unwrap();

    let event = conn.next_event().await;
    assert_eq!(event, None);

    let diff = diff_rx.recv().await.unwrap();
    assert_eq!(
        diff.changes,
        &[deleted(window), deleted(button), deleted(text)]
    );

    world.assert_internally_consistent().await;
}
