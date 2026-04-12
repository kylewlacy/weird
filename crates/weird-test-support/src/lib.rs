use std::sync::Arc;

use weird_core::world::{
    Connection, ConnectionId, CreatedNodeChange, DeletedNodeChange, FlatNode, InitRequest, Node,
    NodeId, NodeUpdate, ROOT_NODE_ID, UpdatedNodeChange, World, WorldChange,
    WorldDidChangeResponse,
};

pub async fn render(
    world: &mut World,
    conn_id: ConnectionId,
    node_id: NodeId,
    children: Vec<Node>,
) -> WorldDidChangeResponse {
    world.assert_internally_consistent().await;

    let (_, mut rx) = world.subscribe_to_world_did_change_events().await;
    let did_change = world
        .set_node_children(node_id, children, conn_id)
        .await
        .unwrap();
    let result = if did_change {
        rx.recv().await.unwrap()
    } else {
        WorldDidChangeResponse::default()
    };

    world.assert_internally_consistent().await;

    result
}

pub async fn node_id(world: &mut World, id: &str) -> NodeId {
    let node_id = world
        .get_nodes()
        .await
        .into_iter()
        .find_map(|(node_id, node)| {
            if node.get_id() == Some(id) {
                Some(node_id)
            } else {
                None
            }
        })
        .unwrap_or_else(|| panic!("node not found with ID '{id}'"));
    tracing::info!(id, ?node_id, "found node");
    node_id
}

pub async fn node_text(world: &mut World, text: &str) -> NodeId {
    let node_id = world
        .get_nodes()
        .await
        .into_iter()
        .find_map(|(node_id, node)| {
            if let FlatNode::Text(node_text) = &*node
                && node_text == text
            {
                Some(node_id)
            } else {
                None
            }
        })
        .unwrap_or_else(|| panic!("node not found with text '{text}'"));
    tracing::info!(text, ?node_id, "found node");
    node_id
}

pub async fn node_tag(world: &mut World, tag: &str) -> NodeId {
    let node_id = world
        .get_nodes()
        .await
        .into_iter()
        .find_map(|(node_id, node)| {
            if let FlatNode::Element(element) = &*node
                && element.tag == tag
            {
                Some(node_id)
            } else {
                None
            }
        })
        .unwrap_or_else(|| panic!("node not found with tag '{tag}'"));
    tracing::info!(tag, ?node_id, "found node");
    node_id
}

pub fn created(
    id: NodeId,
    parent_id: NodeId,
    before_sibling_id: impl Into<Option<NodeId>>,
    node: FlatNode,
) -> WorldChange {
    WorldChange::Created(CreatedNodeChange {
        id,
        parent_id,
        before_sibling_id: before_sibling_id.into(),
        node: Arc::new(node),
    })
}

pub fn updated(id: NodeId, update: impl Into<NodeUpdate>) -> WorldChange {
    WorldChange::Updated(UpdatedNodeChange {
        id,
        update: update.into(),
    })
}

pub fn deleted(id: NodeId) -> WorldChange {
    WorldChange::Deleted(DeletedNodeChange { id })
}

pub async fn init_world(children: impl IntoIterator<Item = Node>) -> (World, Connection, NodeId) {
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

    let world = World::default();
    let (conn, _init_response) = world
        .create_connection(InitRequest::default())
        .await
        .unwrap();

    let window_id = world
        .append_node(
            Node::element("Window").children(children),
            ROOT_NODE_ID,
            conn.id,
        )
        .await;

    (world, conn, window_id)
}
