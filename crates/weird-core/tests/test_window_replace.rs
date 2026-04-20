use pretty_assertions::assert_eq;
use weird_core::world::{FlatNode, InitRequest, Node, ROOT_NODE_ID};
use weird_test_support::{append_node, created, deleted, init_world};

#[tokio::test(flavor = "multi_thread")]
async fn test_window_replace() {
    let (mut world, _conn, _window) = init_world([]).await;

    let (conn_1, _) = world
        .create_connection(
            InitRequest {
                app: Some("app 1".to_string()),
                ..Default::default()
            },
            weird_core::world::ConnectionSource::Other,
        )
        .await
        .unwrap();

    let (diff, window_1) = append_node(
        &mut world,
        conn_1.id,
        ROOT_NODE_ID,
        Node::element("Window").id("window1").attr("replace", true),
    )
    .await;

    assert_eq!(
        diff.changes,
        &[created(
            window_1,
            ROOT_NODE_ID,
            None,
            FlatNode::element("Window")
                .id("window1")
                .attr("replace", true)
        )]
    );

    drop(conn_1);

    let (conn_2, _) = world
        .create_connection(
            InitRequest {
                app: Some("app 1".to_string()),
                ..Default::default()
            },
            weird_core::world::ConnectionSource::Other,
        )
        .await
        .unwrap();

    let (diff, window_2) = append_node(
        &mut world,
        conn_2.id,
        ROOT_NODE_ID,
        Node::element("Window").id("window2").attr("replace", true),
    )
    .await;

    assert_eq!(
        diff.changes,
        &[
            deleted(window_1),
            created(
                window_2,
                ROOT_NODE_ID,
                None,
                FlatNode::element("Window")
                    .id("window2")
                    .attr("replace", true)
            ),
        ]
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn test_window_replace_does_not_replace_if_app_is_different() {
    let (mut world, _conn, _window) = init_world([]).await;

    let (conn_1, _) = world
        .create_connection(
            InitRequest {
                app: Some("app 1".to_string()),
                ..Default::default()
            },
            weird_core::world::ConnectionSource::Other,
        )
        .await
        .unwrap();

    let (diff, window_1) = append_node(
        &mut world,
        conn_1.id,
        ROOT_NODE_ID,
        Node::element("Window").id("window1").attr("replace", true),
    )
    .await;

    assert_eq!(
        diff.changes,
        &[created(
            window_1,
            ROOT_NODE_ID,
            None,
            FlatNode::element("Window")
                .id("window1")
                .attr("replace", true)
        )]
    );

    drop(conn_1);

    let (conn_2, _) = world
        .create_connection(
            InitRequest {
                app: Some("app 2".to_string()),
                ..Default::default()
            },
            weird_core::world::ConnectionSource::Other,
        )
        .await
        .unwrap();

    let (diff, window_2) = append_node(
        &mut world,
        conn_2.id,
        ROOT_NODE_ID,
        Node::element("Window").id("window2").attr("replace", true),
    )
    .await;

    assert_eq!(
        diff.changes,
        &[created(
            window_2,
            ROOT_NODE_ID,
            None,
            FlatNode::element("Window")
                .id("window2")
                .attr("replace", true)
        ),]
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn test_window_replace_does_not_replace_if_not_set_originally() {
    let (mut world, _conn, _window) = init_world([]).await;

    let (conn_1, _) = world
        .create_connection(
            InitRequest {
                app: Some("app 1".to_string()),
                ..Default::default()
            },
            weird_core::world::ConnectionSource::Other,
        )
        .await
        .unwrap();

    let (diff, window_1) = append_node(
        &mut world,
        conn_1.id,
        ROOT_NODE_ID,
        Node::element("Window").id("window1").attr("replace", false),
    )
    .await;

    assert_eq!(
        diff.changes,
        &[created(
            window_1,
            ROOT_NODE_ID,
            None,
            FlatNode::element("Window")
                .id("window1")
                .attr("replace", false)
        )]
    );

    drop(conn_1);

    let (conn_2, _) = world
        .create_connection(
            InitRequest {
                app: Some("app 1".to_string()),
                ..Default::default()
            },
            weird_core::world::ConnectionSource::Other,
        )
        .await
        .unwrap();

    let (diff, window_2) = append_node(
        &mut world,
        conn_2.id,
        ROOT_NODE_ID,
        Node::element("Window").id("window2").attr("replace", true),
    )
    .await;

    assert_eq!(
        diff.changes,
        &[created(
            window_2,
            ROOT_NODE_ID,
            None,
            FlatNode::element("Window")
                .id("window2")
                .attr("replace", true)
        ),]
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn test_window_replace_does_not_replace_if_not_set_now() {
    let (mut world, _conn, _window) = init_world([]).await;

    let (conn_1, _) = world
        .create_connection(
            InitRequest {
                app: Some("app 1".to_string()),
                ..Default::default()
            },
            weird_core::world::ConnectionSource::Other,
        )
        .await
        .unwrap();

    let (diff, window_1) = append_node(
        &mut world,
        conn_1.id,
        ROOT_NODE_ID,
        Node::element("Window").id("window1").attr("replace", true),
    )
    .await;

    assert_eq!(
        diff.changes,
        &[created(
            window_1,
            ROOT_NODE_ID,
            None,
            FlatNode::element("Window")
                .id("window1")
                .attr("replace", true)
        )]
    );

    drop(conn_1);

    let (conn_2, _) = world
        .create_connection(
            InitRequest {
                app: Some("app 2".to_string()),
                ..Default::default()
            },
            weird_core::world::ConnectionSource::Other,
        )
        .await
        .unwrap();

    let (diff, window_2) = append_node(
        &mut world,
        conn_2.id,
        ROOT_NODE_ID,
        Node::element("Window").id("window2").attr("replace", false),
    )
    .await;

    assert_eq!(
        diff.changes,
        &[created(
            window_2,
            ROOT_NODE_ID,
            None,
            FlatNode::element("Window")
                .id("window2")
                .attr("replace", false)
        ),]
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn test_window_replace_does_not_replace_if_prior_still_connected() {
    let (mut world, _conn, _window) = init_world([]).await;

    let (conn_1, _) = world
        .create_connection(
            InitRequest {
                app: Some("app 1".to_string()),
                ..Default::default()
            },
            weird_core::world::ConnectionSource::Other,
        )
        .await
        .unwrap();

    let (diff, window_1) = append_node(
        &mut world,
        conn_1.id,
        ROOT_NODE_ID,
        Node::element("Window").id("window1").attr("replace", true),
    )
    .await;

    assert_eq!(
        diff.changes,
        &[created(
            window_1,
            ROOT_NODE_ID,
            None,
            FlatNode::element("Window")
                .id("window1")
                .attr("replace", true)
        )]
    );

    let (conn_2, _) = world
        .create_connection(
            InitRequest {
                app: Some("app 1".to_string()),
                ..Default::default()
            },
            weird_core::world::ConnectionSource::Other,
        )
        .await
        .unwrap();

    let (diff, window_2) = append_node(
        &mut world,
        conn_2.id,
        ROOT_NODE_ID,
        Node::element("Window").id("window2").attr("replace", true),
    )
    .await;

    assert_eq!(
        diff.changes,
        &[created(
            window_2,
            ROOT_NODE_ID,
            None,
            FlatNode::element("Window")
                .id("window2")
                .attr("replace", true)
        ),]
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn test_window_replace_multiple() {
    let (mut world, _conn, _window) = init_world([]).await;

    let (conn_1, _) = world
        .create_connection(
            InitRequest {
                app: Some("app 1".to_string()),
                ..Default::default()
            },
            weird_core::world::ConnectionSource::Other,
        )
        .await
        .unwrap();

    let (diff, window_1) = append_node(
        &mut world,
        conn_1.id,
        ROOT_NODE_ID,
        Node::element("Window").id("window1").attr("replace", true),
    )
    .await;

    assert_eq!(
        diff.changes,
        &[created(
            window_1,
            ROOT_NODE_ID,
            None,
            FlatNode::element("Window")
                .id("window1")
                .attr("replace", true)
        )]
    );

    let (conn_2, _) = world
        .create_connection(
            InitRequest {
                app: Some("app 1".to_string()),
                ..Default::default()
            },
            weird_core::world::ConnectionSource::Other,
        )
        .await
        .unwrap();

    let (diff, window_2) = append_node(
        &mut world,
        conn_2.id,
        ROOT_NODE_ID,
        Node::element("Window").id("window2").attr("replace", true),
    )
    .await;

    assert_eq!(
        diff.changes,
        &[created(
            window_2,
            ROOT_NODE_ID,
            None,
            FlatNode::element("Window")
                .id("window2")
                .attr("replace", true)
        ),]
    );

    drop(conn_1);
    drop(conn_2);

    let (conn_3, _) = world
        .create_connection(
            InitRequest {
                app: Some("app 1".to_string()),
                ..Default::default()
            },
            weird_core::world::ConnectionSource::Other,
        )
        .await
        .unwrap();

    let (diff, window_3) = append_node(
        &mut world,
        conn_3.id,
        ROOT_NODE_ID,
        Node::element("Window").id("window3").attr("replace", true),
    )
    .await;

    assert_eq!(
        diff.changes,
        &[
            deleted(window_1),
            deleted(window_2),
            created(
                window_3,
                ROOT_NODE_ID,
                None,
                FlatNode::element("Window")
                    .id("window3")
                    .attr("replace", true)
            ),
        ]
    );
}
