use std::{
    collections::{BTreeMap, BTreeSet, HashMap, HashSet, VecDeque},
    sync::{Arc, Weak, atomic::AtomicU64},
};

use tokio::sync::RwLock;

#[derive(Clone)]
pub struct World {
    state: Arc<RwLock<WorldState>>,
    world_did_change_events: tokio::sync::broadcast::Sender<WorldDidChangeResponse>,
    connection_events: tokio::sync::broadcast::Sender<ConnectionEvent>,
    next_connection_id: Arc<AtomicU64>,
}

impl World {
    pub async fn create_connection(
        &self,
        init_request: InitRequest,
    ) -> Result<(Connection, InitResponse), CreateConnectionError> {
        let weird_protocol_version = WeirdProtocolVersion::CURRENT;
        if init_request.weird_protocol_version != weird_protocol_version {
            return Err(CreateConnectionError::ProtocolVersionMismatch {
                client: init_request.weird_protocol_version,
                current: weird_protocol_version,
            });
        }

        let id = self
            .next_connection_id
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let id = ConnectionId(id);
        let mut state = self.state.write().await;
        let (event_tx, event_rx) = tokio::sync::mpsc::unbounded_channel();
        let inner = ConnectionInner {
            connected: true,
            init_request,
            weird_protocol_version,
            event_tx,
        };
        let conn_entry = state.connections.entry(id).insert_entry(inner);
        let inner = conn_entry.get();

        let _ = self
            .connection_events
            .send(ConnectionEvent::Connected(ConnectionDetails::new(
                id, inner,
            )));

        let conn = Connection {
            id,
            event_rx,
            state: Arc::downgrade(&self.state),
            world_did_change_events: self.world_did_change_events.clone(),
            connection_events: self.connection_events.clone(),
        };

        let response = InitResponse {
            weird_protocol_version,
            connection_id: conn.id,
        };
        Ok((conn, response))
    }

    pub async fn get_nodes(&self) -> BTreeMap<NodeId, Arc<FlatNode>> {
        let state = self.state.read().await;
        state.nodes.clone()
    }

    pub async fn append_node(
        &self,
        node: Node,
        parent_id: NodeId,
        connection_id: ConnectionId,
    ) -> NodeId {
        let mut state = self.state.write().await;
        let mut event = WorldDidChangeResponse::default();

        let node_id = state.append_node_inner(node, parent_id, connection_id, &mut event);

        drop(state);
        if !event.is_empty() {
            let _ = self.world_did_change_events.send(event);
        }

        node_id
    }

    pub async fn remove_node(&self, node_id: NodeId) {
        let mut state = self.state.write().await;

        let mut event = WorldDidChangeResponse::default();
        state.remove_nodes_inner(VecDeque::from_iter([node_id]), &mut event);

        let num_receivers = self.world_did_change_events.receiver_count();
        if num_receivers != 0 {
            tracing::debug!(num_receivers, "broadcasting change event");
            let _ = self.world_did_change_events.send(event);
        } else {
            tracing::debug!("no listeners, skipping change event");
        }
    }

    pub async fn set_node_children(
        &self,
        node_id: NodeId,
        children: Vec<Node>,
        connection: ConnectionId,
    ) -> Result<bool, SetNodeChildrenFailed> {
        let mut state = self.state.write().await;
        let mut event = WorldDidChangeResponse::default();

        if !state.nodes.contains_key(&node_id) {
            return Err(SetNodeChildrenFailed::NodeNotFound);
        }

        let mut update_queue = VecDeque::from_iter([(node_id, children)]);

        while let Some((node_id, new_children)) = update_queue.pop_front() {
            let Some(current_child_ids) = state.children.get(&node_id) else {
                return Err(SetNodeChildrenFailed::InvalidNodeType);
            };

            let mut unmatched_children = HashMap::<NodeMatchKey, VecDeque<_>>::new();
            for current_child_id in current_child_ids {
                let current_child = state
                    .nodes
                    .get(current_child_id)
                    .unwrap_or_else(|| panic!("node not found: {current_child_id:?}"));
                let key = NodeMatchKey::from(&**current_child);
                unmatched_children
                    .entry(key)
                    .or_default()
                    .push_back(*current_child_id);
            }

            let mut new_ordered_child_ids = vec![];

            for child in new_children {
                let key = NodeMatchKey::from(&child);
                let matched_child_id = unmatched_children
                    .get_mut(&key)
                    .and_then(VecDeque::pop_front);

                let child_id = if let Some(matched_child_id) = matched_child_id {
                    let matched_child = state.nodes.get_mut(&matched_child_id).unwrap();

                    let (child, child_children) = child.into_flat_and_children();

                    if let Some(update) = node_update(matched_child, &child) {
                        event.changes.push(WorldChange::Updated(UpdatedNodeChange {
                            id: matched_child_id,
                            update,
                        }));
                        *matched_child = Arc::new(child);
                    }

                    if let Some(child_children) = child_children {
                        update_queue.push_back((matched_child_id, child_children));
                    }

                    matched_child_id
                } else {
                    state.append_node_inner(child, node_id, connection, &mut event)
                };

                new_ordered_child_ids.push(child_id);
            }

            // Remove all the child nodes we couldn't match
            // (Note: we sort through a `BTreeSet` just so the order is
            // consistent, which is handy for testing)
            let unmatched_children = unmatched_children
                .into_values()
                .flatten()
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect();
            state.remove_nodes_inner(unmatched_children, &mut event);

            // Reorder the child nodes. We do this in reverse order because
            // moves events have a `before_sibling_id`, so moving the
            // end nodes first simplifies things.
            for (index, child_id) in new_ordered_child_ids.iter().enumerate().rev() {
                let (parent_id, current_index) = state.parents.get_mut(child_id).unwrap();

                // Ensure the node isn't being re-parented! This logic
                // assumes that we're visiting every child
                assert_eq!(*parent_id, node_id);

                // Skip the node if it's already at the right index
                if *current_index == index {
                    continue;
                }

                // Add the event
                let before_sibling_id = new_ordered_child_ids.get(index + 1).copied();
                event.changes.push(WorldChange::Moved(MovedNodeChange {
                    id: *child_id,
                    parent_id: node_id,
                    before_sibling_id,
                }));

                // Update the node's parent index
                *current_index = index;
            }

            // Update the child node list
            state.children.insert(node_id, new_ordered_child_ids);
        }

        drop(state);
        if event.is_empty() {
            Ok(false)
        } else {
            let _ = self.world_did_change_events.send(event);
            Ok(true)
        }
    }

    /// Subscribe to future `WorldDidChangeResponse` events, and additionally
    /// get an initial event, which describes changes to get to the
    /// current world state.
    pub async fn subscribe_to_world_did_change_events(
        &self,
    ) -> (
        WorldDidChangeResponse,
        tokio::sync::broadcast::Receiver<WorldDidChangeResponse>,
    ) {
        let state = self.state.read().await;

        let mut initial_event = WorldDidChangeResponse::default();

        let root_children = state
            .children
            .get(&ROOT_NODE_ID)
            .expect("root node does not have child list");
        let mut queue: VecDeque<_> = root_children.iter().copied().collect();

        while let Some(id) = queue.pop_front() {
            if let Some(children) = state.children.get(&id) {
                queue.extend(children.iter().copied());
            }

            let node = &state.nodes[&id];

            // Every node has a parent except for the root node, which
            // is excluded from the `DidInsert` event
            let (parent_id, parent_index) = state
                .parents
                .get(&id)
                .unwrap_or_else(|| panic!("node {id:?} does not have a parent"));

            let before_sibling_id = state.children[parent_id].get(parent_index + 1).copied();
            initial_event
                .changes
                .push(WorldChange::Created(CreatedNodeChange {
                    id,
                    parent_id: *parent_id,
                    before_sibling_id,
                    node: node.clone(),
                }));
        }

        let events_rx = self.world_did_change_events.subscribe();

        (initial_event, events_rx)
    }

    /// Subscribe to future `ConnectionEvent` events, and additionally
    /// get all current connections.
    pub async fn subscribe_to_connection_events(
        &self,
    ) -> (
        Vec<ConnectionDetails>,
        tokio::sync::broadcast::Receiver<ConnectionEvent>,
    ) {
        let state = self.state.read().await;
        let connections = state
            .connections
            .iter()
            .map(|(id, conn)| ConnectionDetails::new(*id, conn))
            .collect();

        let events_rx = self.connection_events.subscribe();

        (connections, events_rx)
    }

    pub async fn trigger_event(&self, event: TriggerEvent) -> Result<(), TriggerEventFailed> {
        let mut state = self.state.write().await;
        let mut change_event = WorldDidChangeResponse::default();

        let connection = state
            .connection_by_node
            .get(&event.target_node_id)
            .and_then(|connection_id| {
                Some((state.connections.get(connection_id)?, *connection_id))
            });
        let Some((connection, connection_id)) = connection else {
            return Err(TriggerEventFailed::NoConnectionForNode);
        };

        let target_node = state.nodes.get(&event.target_node_id);
        if event.event == "close"
            && let Some(target_node) = target_node
            && let FlatNode::Element(el) = &**target_node
            && el.tag == "Window"
            && state
                .parents
                .get(&event.target_node_id)
                .is_some_and(|(parent_id, _)| *parent_id == ROOT_NODE_ID)
        {
            // Window.close event

            // Close the connection by removing it
            state.connections.remove(&connection_id);

            // Remove the window node
            state.remove_nodes_inner(
                VecDeque::from_iter([event.target_node_id]),
                &mut change_event,
            );
        } else {
            let target_id = target_node
                .and_then(|node| node.get_id())
                .map(ToString::to_string);
            let event = Event {
                event: event.event,
                params: event.params,
                target_node_id: event.target_node_id,
                target_id,
            };

            connection
                .event_tx
                .send(event)
                .map_err(|_| TriggerEventFailed::NoConnectionForNode)?;
        }

        drop(state);
        if !change_event.is_empty() {
            let _ = self.world_did_change_events.send(change_event);
        }

        Ok(())
    }

    pub async fn assert_internally_consistent(&self) {
        assert!(self.is_internally_consistent().await);
    }

    async fn is_internally_consistent(&self) -> bool {
        let state = self.state.read().await;
        state.is_internally_consistent()
    }
}

impl Default for World {
    fn default() -> Self {
        Self {
            state: Arc::new(RwLock::new(WorldState::default())),
            world_did_change_events: tokio::sync::broadcast::Sender::new(10),
            connection_events: tokio::sync::broadcast::Sender::new(10),
            next_connection_id: Arc::new(AtomicU64::new(1)),
        }
    }
}

struct WorldState {
    next_node_id: u64,
    nodes: BTreeMap<NodeId, Arc<FlatNode>>,
    parents: BTreeMap<NodeId, (NodeId, usize)>,
    children: BTreeMap<NodeId, Vec<NodeId>>,
    connection_by_node: BTreeMap<NodeId, ConnectionId>,
    nodes_by_connection: BTreeMap<ConnectionId, BTreeSet<NodeId>>,
    connections: BTreeMap<ConnectionId, ConnectionInner>,
}

impl WorldState {
    fn remove_nodes_inner(
        &mut self,
        mut node_queue: VecDeque<NodeId>,
        event: &mut WorldDidChangeResponse,
    ) {
        // Keep track of parent indices, since we'll need to fix the
        // parent and children maps at the end
        let mut parent_indices = HashMap::<NodeId, BTreeSet<usize>>::new();

        while let Some(node_id) = node_queue.pop_front() {
            // Remove the node from each map
            self.nodes.remove(&node_id);
            let conn_id = self.connection_by_node.remove(&node_id);
            let parent = self.parents.remove(&node_id);
            let children = self.children.remove(&node_id);
            if let Some(conn_id) = conn_id
                && let std::collections::btree_map::Entry::Occupied(mut conn_node_entry) =
                    self.nodes_by_connection.entry(conn_id)
            {
                let conn_node_ids = conn_node_entry.get_mut();
                conn_node_ids.remove(&node_id);
                if conn_node_ids.is_empty() {
                    conn_node_entry.remove();

                    // Remove the connection entry if the connection is
                    // disconnected and there are no more nodes
                    if let std::collections::btree_map::Entry::Occupied(conn_entry) =
                        self.connections.entry(conn_id)
                        && !conn_entry.get().connected
                    {
                        conn_entry.remove();
                    }
                }
            }

            // Track the parent node
            if let Some((parent_id, parent_index)) = parent {
                parent_indices
                    .entry(parent_id)
                    .or_default()
                    .insert(parent_index);
            }

            // Update the event
            event
                .changes
                .push(WorldChange::Deleted(DeletedNodeChange { id: node_id }));

            // Add the children to the queue, so each one gets removed too
            node_queue.extend(children.into_iter().flatten());
        }

        // Update the parent and children maps for each parent node that's
        // been updated
        for (parent_id, child_indices) in parent_indices {
            let Some(children) = self.children.get_mut(&parent_id) else {
                // Skip if the parent node was also removed
                continue;
            };

            // Remove each child node that was removed. We iterate starting
            // from the largest index (otherwise the indices would shift)
            for index in child_indices.iter().rev() {
                children.remove(*index);
            }

            // Update each child's parent index
            for (index, child_id) in children.iter().enumerate() {
                self.parents.insert(*child_id, (parent_id, index));
            }
        }
    }

    fn append_node_inner(
        &mut self,
        node: Node,
        parent_id: NodeId,
        connection_id: ConnectionId,
        event: &mut WorldDidChangeResponse,
    ) -> NodeId {
        let top_node_id = NodeId(self.next_node_id);
        self.next_node_id = self.next_node_id.checked_add(1).unwrap();

        let mut queue = VecDeque::from_iter([(Some(top_node_id), node, parent_id)]);
        while let Some((node_id, node, parent_id)) = queue.pop_front() {
            let node_id = node_id.unwrap_or_else(|| {
                let node_id = NodeId(self.next_node_id);
                self.next_node_id = self.next_node_id.checked_add(1).unwrap();
                node_id
            });

            let (node, children) = node.into_flat_and_children();
            let node = Arc::new(node);
            self.nodes.insert(node_id, node.clone());
            self.connection_by_node.insert(node_id, connection_id);
            self.nodes_by_connection
                .entry(connection_id)
                .or_default()
                .insert(node_id);

            let parent_children = self
                .children
                .get_mut(&parent_id)
                .unwrap_or_else(|| panic!("node {parent_id:?} cannot have children"));
            let parent_index = parent_children.len();

            parent_children.push(node_id);
            self.parents.insert(node_id, (parent_id, parent_index));

            if let Some(children) = children {
                self.children.insert(node_id, vec![]);

                queue.extend(children.into_iter().map(|child| (None, child, node_id)));
            }

            event.changes.push(WorldChange::Created(CreatedNodeChange {
                id: node_id,
                parent_id,
                before_sibling_id: None,
                node,
            }));
        }

        top_node_id
    }

    fn connection_did_close(
        &mut self,
        connection_id: ConnectionId,
        event: &mut WorldDidChangeResponse,
    ) {
        // Mark the connection as disconnected
        if let Some(conn) = self.connections.get_mut(&connection_id) {
            conn.connected = false;
        }

        // Mark any window nodes owned by the connection as stale
        let node_ids = self
            .nodes_by_connection
            .get(&connection_id)
            .into_iter()
            .flatten();
        for node_id in node_ids.clone() {
            let node = self.nodes.get_mut(node_id).unwrap();
            if let FlatNode::Element(el) = &**node
                && el.tag == "Window"
                && el.attributes.get("stale") != Some(&serde_json::Value::Bool(true))
            {
                let node = Arc::make_mut(node);
                let FlatNode::Element(el) = node else {
                    unreachable!();
                };
                el.attributes
                    .insert("stale".into(), serde_json::Value::Bool(true));
                event.changes.push(WorldChange::Updated(UpdatedNodeChange {
                    id: *node_id,
                    update: NodeUpdate::Element(ElementNodeUpdate {
                        set_attributes: HashMap::from_iter([(
                            "stale".into(),
                            serde_json::Value::Bool(true),
                        )]),
                        clear_attributes: HashSet::new(),
                    }),
                }));
            }
        }

        // Remove the connection entry if there are no nodes left
        let mut node_ids = node_ids;
        if node_ids.next().is_none() {
            self.connections.remove(&connection_id);
            self.nodes_by_connection.remove(&connection_id);
        }
    }

    fn is_internally_consistent(&self) -> bool {
        let mut expected_nodes_by_connection = BTreeMap::<_, BTreeSet<_>>::new();

        for (node_id, node) in &self.nodes {
            if *node_id != ROOT_NODE_ID {
                let Some(conn_id) = self.connection_by_node.get(node_id) else {
                    tracing::warn!(?node_id, "node does not belong to a connection");
                    return false;
                };

                expected_nodes_by_connection
                    .entry(*conn_id)
                    .or_default()
                    .insert(*node_id);
            }

            if !self.connection_by_node.contains_key(node_id) && *node_id != ROOT_NODE_ID {
                tracing::warn!(?node_id, "node does not belong to a connection");
                return false;
            }

            if !self.parents.contains_key(node_id) && *node_id != ROOT_NODE_ID {
                tracing::warn!(?node_id, "node does not have a parent");
                return false;
            }

            match &**node {
                FlatNode::Text(_) => {
                    if self.children.contains_key(node_id) {
                        tracing::warn!(?node_id, "text node has children");
                        return false;
                    }
                }
                FlatNode::Element(_) => {}
            }
        }

        let mut expected_parents = BTreeMap::new();
        for (node_id, children_ids) in &self.children {
            if !self.nodes.contains_key(node_id) {
                tracing::warn!(?node_id, "parent node does not exist");
                return false;
            }

            for (index, child_id) in children_ids.iter().enumerate() {
                if !self.nodes.contains_key(child_id) {
                    tracing::warn!(?child_id, "child node does not exist");
                    return false;
                }

                expected_parents.insert(*child_id, (*node_id, index));
            }
        }

        if self.parents != expected_parents {
            tracing::warn!(parents = ?self.parents, children = ?self.children, "parent and child maps don't align");
            return false;
        }

        if self.nodes_by_connection != expected_nodes_by_connection {
            tracing::warn!(nodes_by_connection = ?self.nodes_by_connection, connection_by_node = ?self.connection_by_node, "nodes_by_connection and connection_by_node maps don't align");
            return false;
        }

        true
    }
}

impl Default for WorldState {
    fn default() -> Self {
        let root_node = FlatNode::Element(FlatElement::new("World"));

        Self {
            next_node_id: ROOT_NODE_ID.0 + 1,
            nodes: BTreeMap::from_iter([(ROOT_NODE_ID, Arc::new(root_node))]),
            parents: BTreeMap::new(),
            children: BTreeMap::from_iter([(ROOT_NODE_ID, vec![])]),
            connections: BTreeMap::new(),
            connection_by_node: BTreeMap::new(),
            nodes_by_connection: BTreeMap::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum NodeMatchKey {
    Text,
    Element {
        tag: String,
        id: Option<serde_json::Value>,
    },
}

impl<'a> From<&'a FlatNode> for NodeMatchKey {
    fn from(value: &'a FlatNode) -> Self {
        match value {
            FlatNode::Text(_) => NodeMatchKey::Text,
            FlatNode::Element(element) => NodeMatchKey::Element {
                tag: element.tag.clone(),
                id: element.attributes.get("id").cloned(),
            },
        }
    }
}

impl<'a> From<&'a Node> for NodeMatchKey {
    fn from(value: &'a Node) -> Self {
        match value {
            Node::Text(_) => NodeMatchKey::Text,
            Node::Element(element) => NodeMatchKey::Element {
                tag: element.element.tag.clone(),
                id: element.element.attributes.get("id").cloned(),
            },
        }
    }
}

fn node_update(current: &FlatNode, updated: &FlatNode) -> Option<NodeUpdate> {
    match (current, updated) {
        (FlatNode::Element(current), FlatNode::Element(updated)) => {
            let mut set_attributes = HashMap::new();
            let mut clear_attributes = HashSet::new();

            for (key, current_value) in &current.attributes {
                let updated_value = updated.attributes.get(key);
                match updated_value {
                    Some(updated_value) if updated_value == current_value => {
                        // Do nothing, value already matches
                    }
                    Some(updated_value) => {
                        // Updated value does not match, so set it to update it
                        set_attributes.insert(key.clone(), updated_value.clone());
                    }
                    None => {
                        // Updated value does not exist, so clear the
                        // attribute
                        clear_attributes.insert(key.clone());
                    }
                }
            }
            for (key, updated_value) in &updated.attributes {
                if !current.attributes.contains_key(key) {
                    // Updated value exists but current value doesn't so
                    // insert it
                    set_attributes.insert(key.clone(), updated_value.clone());
                }
            }

            if set_attributes.is_empty() && clear_attributes.is_empty() {
                None
            } else {
                Some(NodeUpdate::Element(ElementNodeUpdate {
                    set_attributes,
                    clear_attributes,
                }))
            }
        }
        (FlatNode::Text(current), FlatNode::Text(updated)) => {
            if current == updated {
                None
            } else {
                Some(NodeUpdate::Text(TextNodeUpdate::new(updated)))
            }
        }
        (FlatNode::Text(_), FlatNode::Element(_)) | (FlatNode::Element(_), FlatNode::Text(_)) => {
            panic!("tried to compute node update between different node types");
        }
    }
}

struct ConnectionInner {
    connected: bool,
    init_request: InitRequest,
    weird_protocol_version: WeirdProtocolVersion,
    event_tx: tokio::sync::mpsc::UnboundedSender<Event>,
}

pub struct Connection {
    pub id: ConnectionId,
    event_rx: tokio::sync::mpsc::UnboundedReceiver<Event>,
    state: Weak<RwLock<WorldState>>,
    world_did_change_events: tokio::sync::broadcast::Sender<WorldDidChangeResponse>,
    connection_events: tokio::sync::broadcast::Sender<ConnectionEvent>,
}

impl Connection {
    pub async fn next_event(&mut self) -> Option<Event> {
        self.event_rx.recv().await
    }
}

impl Drop for Connection {
    fn drop(&mut self) {
        let Some(state) = self.state.upgrade() else {
            return;
        };

        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                let _ =
                    self.connection_events
                        .send(ConnectionEvent::Disconnected(DisconnectedEvent {
                            connection_id: self.id,
                        }));

                let mut state = state.write().await;
                let mut event = WorldDidChangeResponse::default();
                state.connection_did_close(self.id, &mut event);
                if !event.is_empty() {
                    let _ = self.world_did_change_events.send(event);
                }
            })
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum WeirdProtocolVersion {
    #[serde(rename = "0.1.0")]
    V0_1_0,
}

impl WeirdProtocolVersion {
    pub const CURRENT: Self = Self::V0_1_0;
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InitRequest {
    pub weird_protocol_version: WeirdProtocolVersion,
    pub client: Option<String>,
}

impl Default for InitRequest {
    fn default() -> Self {
        Self {
            weird_protocol_version: WeirdProtocolVersion::CURRENT,
            client: None,
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InitResponse {
    weird_protocol_version: WeirdProtocolVersion,
    connection_id: ConnectionId,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NodeId(u64);

impl serde::Serialize for NodeId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.0.to_string())
    }
}

impl<'de> serde::Deserialize<'de> for NodeId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s: &str = serde::Deserialize::deserialize(deserializer)?;
        let id = s.parse().map_err(serde::de::Error::custom)?;
        Ok(Self(id))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ConnectionId(u64);

impl std::fmt::Display for ConnectionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl serde::Serialize for ConnectionId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.0.to_string())
    }
}

impl<'de> serde::Deserialize<'de> for ConnectionId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s: &str = serde::Deserialize::deserialize(deserializer)?;
        let id = s.parse().map_err(serde::de::Error::custom)?;
        Ok(Self(id))
    }
}

pub const ROOT_NODE_ID: NodeId = NodeId(0);

#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
pub enum Node {
    Text(String),
    Element(Element),
}

impl Node {
    pub fn text(text: impl Into<String>) -> Self {
        Self::Text(text.into())
    }

    pub fn element(tag: impl Into<String>) -> Self {
        Self::Element(Element::new(tag))
    }

    pub fn children(self, nodes: impl IntoIterator<Item = Node>) -> Self {
        let Self::Element(element) = self else {
            panic!("called .children() on non-element node");
        };
        Self::Element(element.children(nodes))
    }

    pub fn child(self, node: Node) -> Self {
        let Self::Element(element) = self else {
            panic!("called .child() on non-element node");
        };
        Self::Element(element.child(node))
    }

    pub fn attrs(self, attrs: impl IntoIterator<Item = (String, serde_json::Value)>) -> Self {
        let Self::Element(element) = self else {
            panic!("called .attrs() on non-element node");
        };
        Self::Element(element.attrs(attrs))
    }

    pub fn attr(self, name: impl Into<String>, value: impl serde::Serialize) -> Self {
        let Self::Element(element) = self else {
            panic!("called .attr() on non-element node");
        };
        Self::Element(element.attr(name, value))
    }

    pub fn id(self, id: impl Into<String>) -> Self {
        let Self::Element(element) = self else {
            panic!("called .id() on non-element node");
        };
        Self::Element(element.id(id))
    }

    fn into_flat_and_children(self) -> (FlatNode, Option<Vec<Node>>) {
        match self {
            Self::Text(text) => (FlatNode::Text(text), None),
            Self::Element(element) => (FlatNode::Element(element.element), Some(element.children)),
        }
    }
}

impl From<Element> for Node {
    fn from(value: Element) -> Self {
        Self::Element(value)
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct Element {
    #[serde(default)]
    children: Vec<Node>,
    #[serde(flatten)]
    element: FlatElement,
}

impl Element {
    pub fn new(tag: impl Into<String>) -> Self {
        Self {
            element: FlatElement::new(tag),
            children: vec![],
        }
    }

    pub fn children(mut self, nodes: impl IntoIterator<Item = Node>) -> Self {
        self.children.extend(nodes);
        self
    }

    pub fn child(mut self, node: Node) -> Self {
        self.children.push(node);
        self
    }

    pub fn attrs(mut self, attrs: impl IntoIterator<Item = (String, serde_json::Value)>) -> Self {
        self.element.attributes.extend(attrs);
        self
    }

    pub fn attr(mut self, name: impl Into<String>, value: impl serde::Serialize) -> Self {
        let name = name.into();
        let value = serde_json::to_value(value).unwrap_or_else(|error| {
            panic!(
                "failed to serialize attribute '{name}' for tag '{}': {error}",
                self.element.tag
            )
        });
        self.element.attributes.insert(name, value);
        self
    }

    pub fn id(self, id: impl Into<String>) -> Self {
        self.attr("id", id.into())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
pub enum FlatNode {
    Text(String),
    Element(FlatElement),
}

impl FlatNode {
    pub fn text(text: impl Into<String>) -> Self {
        Self::Text(text.into())
    }

    pub fn element(tag: impl Into<String>) -> Self {
        Self::Element(FlatElement::new(tag))
    }

    pub fn attrs(self, attrs: impl IntoIterator<Item = (String, serde_json::Value)>) -> Self {
        let Self::Element(element) = self else {
            panic!("called .attrs() on non-element node");
        };
        Self::Element(element.attrs(attrs))
    }

    pub fn attr(self, name: impl Into<String>, value: impl serde::Serialize) -> Self {
        let Self::Element(element) = self else {
            panic!("called .attr() on non-element node");
        };
        Self::Element(element.attr(name, value))
    }

    pub fn id(self, id: impl Into<String>) -> Self {
        let Self::Element(element) = self else {
            panic!("called .id() on non-element node");
        };
        Self::Element(element.id(id))
    }

    pub fn get_id(&self) -> Option<&str> {
        let Self::Element(element) = self else {
            return None;
        };
        element.attributes.get("id")?.as_str()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct FlatElement {
    pub tag: String,
    #[serde(default)]
    pub attributes: HashMap<String, serde_json::Value>,
}

impl FlatElement {
    pub fn new(tag: impl Into<String>) -> Self {
        Self {
            tag: tag.into(),
            attributes: HashMap::new(),
        }
    }

    pub fn attrs(mut self, attrs: impl IntoIterator<Item = (String, serde_json::Value)>) -> Self {
        self.attributes.extend(attrs);
        self
    }

    pub fn attr(mut self, name: impl Into<String>, value: impl serde::Serialize) -> Self {
        let name = name.into();
        let value = serde_json::to_value(value).unwrap_or_else(|error| {
            panic!(
                "failed to serialize attribute '{name}' for tag '{}': {error}",
                self.tag
            )
        });
        self.attributes.insert(name, value);
        self
    }

    pub fn id(self, id: impl Into<String>) -> Self {
        self.attr("id", id.into())
    }
}

#[derive(Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Event {
    pub target_id: Option<String>,
    pub target_node_id: NodeId,
    pub event: String,
    pub params: serde_json::Value,
}

impl Event {
    pub fn is(&self, target_id: &str, event: &str) -> bool {
        self.event == event && self.target_id.as_deref() == Some(target_id)
    }

    pub fn param<T>(&self, param: &str) -> serde_json::Result<T>
    where
        T: serde::de::DeserializeOwned,
    {
        let params = self
            .params
            .as_object()
            .ok_or_else(|| serde::de::Error::custom("expected event params to be an object"))?;
        let param_value = params
            .get(param)
            .cloned()
            .unwrap_or(serde_json::Value::Null);
        let value = serde_json::from_value(param_value)?;
        Ok(value)
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TriggerEvent {
    pub target_node_id: NodeId,
    pub event: String,
    pub params: serde_json::Value,
}

#[derive(Debug)]
pub enum SetNodeChildrenFailed {
    NodeNotFound,
    InvalidNodeType,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorldDidChangeResponse {
    pub changes: Vec<WorldChange>,
}

impl WorldDidChangeResponse {
    pub fn is_empty(&self) -> bool {
        self.changes.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WorldChange {
    /// Create a new node, parented under an existing node.
    Created(CreatedNodeChange),
    /// Update the content of an existing node.
    Updated(UpdatedNodeChange),
    /// Move an existing node somewhere else in the tree.
    Moved(MovedNodeChange),
    /// Delete an existing node. Each descendent will also include a deletion
    /// change (either before or after its ancestors' deletions).
    Deleted(DeletedNodeChange),
}

impl From<CreatedNodeChange> for WorldChange {
    fn from(value: CreatedNodeChange) -> Self {
        Self::Created(value)
    }
}

impl From<UpdatedNodeChange> for WorldChange {
    fn from(value: UpdatedNodeChange) -> Self {
        Self::Updated(value)
    }
}

impl From<MovedNodeChange> for WorldChange {
    fn from(value: MovedNodeChange) -> Self {
        Self::Moved(value)
    }
}

impl From<DeletedNodeChange> for WorldChange {
    fn from(value: DeletedNodeChange) -> Self {
        Self::Deleted(value)
    }
}

/// Create a new node in the world.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreatedNodeChange {
    /// The ID of the new node.
    pub id: NodeId,

    /// The ID of the parent node for the new node.
    pub parent_id: NodeId,

    /// The child node within the parent that this node comes before, or
    /// `None` if the new node was added to the end of the parent node.
    pub before_sibling_id: Option<NodeId>,

    /// The content of the new node. Every node starts without any children--
    /// child nodes are created with further changes.
    pub node: Arc<FlatNode>,
}

/// Update the content of an existing node.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdatedNodeChange {
    /// The ID of the node to update.
    pub id: NodeId,

    #[serde(flatten)]
    pub update: NodeUpdate,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MovedNodeChange {
    /// The ID of the node to move.
    pub id: NodeId,

    /// The ID of new parent for the node.
    pub parent_id: NodeId,

    /// The child node within the parent that this node comes before, or
    /// `None` if the node was moved to the end of the parent node.
    pub before_sibling_id: Option<NodeId>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeletedNodeChange {
    /// The ID of the node to delete.
    pub id: NodeId,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
pub enum NodeUpdate {
    #[serde(rename_all = "camelCase")]
    Text(TextNodeUpdate),
    #[serde(rename_all = "camelCase")]
    Element(ElementNodeUpdate),
}

impl From<TextNodeUpdate> for NodeUpdate {
    fn from(value: TextNodeUpdate) -> Self {
        Self::Text(value)
    }
}

impl From<ElementNodeUpdate> for NodeUpdate {
    fn from(value: ElementNodeUpdate) -> Self {
        Self::Element(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TextNodeUpdate {
    /// The new text content of the node.
    text: String,
}

impl TextNodeUpdate {
    pub fn new(text: impl Into<String>) -> Self {
        Self { text: text.into() }
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ElementNodeUpdate {
    /// Attributes to add or replace in the element.
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    set_attributes: HashMap<String, serde_json::Value>,

    /// Attributes to remove from the element.
    #[serde(skip_serializing_if = "HashSet::is_empty")]
    clear_attributes: HashSet<String>,
}

impl ElementNodeUpdate {
    pub fn set_attr(mut self, name: impl Into<String>, value: impl serde::Serialize) -> Self {
        let name = name.into();
        let value = serde_json::to_value(value)
            .unwrap_or_else(|error| panic!("failed to serialize attribute '{name}': {error}"));
        self.set_attributes
            .insert(name, serde_json::to_value(value).unwrap());
        self
    }

    pub fn clear_attr(mut self, name: impl Into<String>) -> Self {
        self.clear_attributes.insert(name.into());
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ConnectionEvent {
    Connected(ConnectionDetails),
    Disconnected(DisconnectedEvent),
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionDetails {
    connection_id: ConnectionId,
    connected: bool,
    weird_protocol_version: WeirdProtocolVersion,
    client: Option<String>,
}

impl ConnectionDetails {
    fn new(connection_id: ConnectionId, conn: &ConnectionInner) -> Self {
        Self {
            connection_id,
            connected: conn.connected,
            client: conn.init_request.client.clone(),
            weird_protocol_version: conn.weird_protocol_version,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DisconnectedEvent {
    connection_id: ConnectionId,
}

#[derive(Debug, thiserror::Error)]
pub enum CreateConnectionError {
    #[error("protocol version mismatch: client expects {client:?}, current is {current:?}")]
    ProtocolVersionMismatch {
        client: WeirdProtocolVersion,
        current: WeirdProtocolVersion,
    },
}

#[derive(Debug)]
pub enum TriggerEventFailed {
    NoConnectionForNode,
}
