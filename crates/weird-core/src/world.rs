use std::{
    collections::{BTreeMap, HashMap, VecDeque},
    sync::{Arc, Weak},
};

use tokio::sync::RwLock;

pub struct World {
    nodes: BTreeMap<NodeId, Arc<FlatNode>>,
    parents: BTreeMap<NodeId, NodeId>,
    children: BTreeMap<NodeId, Vec<NodeId>>,
    connection_by_node: BTreeMap<NodeId, ConnectionId>,
    connections: BTreeMap<ConnectionId, Weak<RwLock<ConnectionInner>>>,
    world_did_change_events: tokio::sync::broadcast::Sender<WorldDidChangeResponse>,
}

impl World {
    pub fn create_connection(&mut self) -> Connection {
        let id = self
            .connections
            .last_key_value()
            .map_or(ConnectionId(0), |(last_id, _)| ConnectionId(last_id.0 + 1));
        let inner = Arc::new(RwLock::new(ConnectionInner::default()));
        self.connections.insert(id, Arc::downgrade(&inner));
        Connection { id, _inner: inner }
    }

    pub fn create_node(&mut self, node: Node, connection_id: ConnectionId) -> NodeId {
        let new_id = self
            .nodes
            .last_key_value()
            .map(|(last_id, _)| NodeId(last_id.0 + 1))
            .expect("world.nodes is empty");

        let mut next_id = new_id;
        let mut queue: VecDeque<(Node, Option<NodeId>)> = [(node, None)].into_iter().collect();

        while let Some((node_tree, parent)) = queue.pop_front() {
            let id = next_id;
            next_id = NodeId(next_id.0 + 1);

            self.connection_by_node.insert(id, connection_id);
            match node_tree {
                Node::Text(text) => {
                    self.nodes.insert(id, Arc::new(FlatNode::Text(text)));
                }
                Node::Element(element) => {
                    queue.extend(element.children.into_iter().map(|child| (child, Some(id))));

                    self.nodes
                        .insert(id, Arc::new(FlatNode::Element(element.element)));
                    self.children.insert(id, vec![]);
                }
            };

            if let Some(parent) = parent {
                self.parents.insert(id, parent);

                let parent_children = self
                    .children
                    .get_mut(&parent)
                    .expect("parent element does not have a list of children");
                parent_children.push(id);
            }
        }

        new_id
    }

    pub fn insert_node(&mut self, insert: InsertNode) -> Result<usize, InsertNodeFailed> {
        let mut event = WorldDidChangeResponse::default();

        let insert_index = self.insert_node_for_event(insert, &mut event)?;

        let _ = self.world_did_change_events.send(event);

        Ok(insert_index)
    }

    fn insert_node_for_event(
        &mut self,
        insert: InsertNode,
        event: &mut WorldDidChangeResponse,
    ) -> Result<usize, InsertNodeFailed> {
        if !self.nodes.contains_key(&insert.child) {
            return Err(InsertNodeFailed::NodeNotFound);
        }

        let parent_children = self.children.get_mut(&insert.parent);
        let Some(parent_children) = parent_children else {
            if self.nodes.contains_key(&insert.parent) {
                return Err(InsertNodeFailed::ParentNotFound);
            } else {
                return Err(InsertNodeFailed::InvalidParentNodeType);
            }
        };

        let num_children = parent_children.len();
        let insert_index = match insert.offset {
            InsertNodeOffset::FromStart(offset) => offset,
            InsertNodeOffset::FromEnd(offset) => {
                num_children
                    .checked_sub(offset)
                    .ok_or(InsertNodeFailed::OffsetOutOfBounds {
                        offset: insert.offset,
                        num_children,
                    })?
            }
        };
        if insert_index > num_children {
            return Err(InsertNodeFailed::OffsetOutOfBounds {
                offset: insert.offset,
                num_children,
            });
        }

        parent_children.insert(insert_index, insert.child);
        self.parents.insert(insert.child, insert.parent);

        let mut queue = VecDeque::from_iter([insert.child]);
        while let Some(id) = queue.pop_front() {
            event.inserted.push(InsertedNode {
                id,
                parent: self.parents[&id],
                node: self.nodes[&id].clone(),
            });

            if let Some(children) = self.children.get(&id) {
                queue.extend(children.iter().copied());
            }
        }

        Ok(insert_index)
    }

    pub fn remove_node(&mut self, node_id: NodeId) -> Result<NodeId, RemoveNodeFailed> {
        self.connection_by_node.remove(&node_id);

        if !self.nodes.contains_key(&node_id) {
            return Err(RemoveNodeFailed::NodeNotFound);
        }

        let parent = self
            .parents
            .get(&node_id)
            .ok_or(RemoveNodeFailed::NoParentNode)?;

        let parent_children = self
            .children
            .get_mut(parent)
            .unwrap_or_else(|| panic!("children not found for parent {parent:?}"));

        let child_pos = parent_children
            .iter()
            .enumerate()
            .find_map(|(pos, child_id)| {
                if *child_id == node_id {
                    Some(pos)
                } else {
                    None
                }
            })
            .unwrap_or_else(|| {
                panic!("tried to remove {node_id:?} but parent element did not contain it")
            });
        parent_children.remove(child_pos);

        let num_receivers = self.world_did_change_events.receiver_count();
        if num_receivers != 0 {
            tracing::debug!(num_receivers, "broadcasting change event");
            let mut event = WorldDidChangeResponse::default();

            event.removed.push(node_id);

            let _ = self.world_did_change_events.send(event);
        } else {
            tracing::debug!("no listeners, skipping change event");
        }

        Ok(*parent)
    }

    pub fn set_node_children(
        &mut self,
        node_id: NodeId,
        children: Vec<Node>,
        connection: ConnectionId,
    ) -> Result<(), SetNodeChildrenFailed> {
        let mut event = WorldDidChangeResponse::default();

        if !self.nodes.contains_key(&node_id) {
            return Err(SetNodeChildrenFailed::NodeNotFound);
        }

        let Some(prev_children) = self.children.get(&node_id) else {
            return Err(SetNodeChildrenFailed::InvalidNodeType);
        };

        tracing::info!("removing direct descendents: {:?}", prev_children);
        event.removed.extend(prev_children.iter().copied());

        let mut remove_queue: VecDeque<_> = prev_children.iter().copied().collect();
        while let Some(remove) = remove_queue.pop_front() {
            tracing::info!(?remove, parent = ?self.parents.get(&remove), children = ?self.children.get(&remove), "removing");

            self.nodes.remove(&remove);
            let parent_id = self.parents.remove(&remove);
            let parent_children = parent_id.and_then(|parent_id| self.children.get_mut(&parent_id));

            // Remove the parent node if it still exists. Since we remove
            // nodes outside-in, we'll remove a parent before its children.
            if let Some(parent_children) = parent_children {
                let parent_child_index = parent_children
                    .iter()
                    .enumerate()
                    .find_map(|(i, parent_child)| {
                        if *parent_child == remove {
                            Some(i)
                        } else {
                            None
                        }
                    })
                    .expect("node not found within parent node's children");
                parent_children.remove(parent_child_index);
            }
            let removed_children = self.children.remove(&remove);

            remove_queue.extend(removed_children.into_iter().flatten());
        }

        for child in children {
            let child = self.create_node(child, connection);
            self.insert_node_for_event(
                InsertNode {
                    parent: node_id,
                    child,
                    offset: InsertNodeOffset::END,
                },
                &mut event,
            )
            .unwrap_or_else(|error| panic!("failed to insert node: {error:?}"));
        }

        let _ = self.world_did_change_events.send(event);

        Ok(())
    }

    pub fn initial_client_world_did_change_event(&self) -> WorldDidChangeResponse {
        let mut event = WorldDidChangeResponse::default();

        let root_children = self
            .children
            .get(&ROOT_NODE_ID)
            .expect("root node does not have child list");
        let mut queue: VecDeque<_> = root_children.iter().copied().collect();

        while let Some(id) = queue.pop_front() {
            if let Some(children) = self.children.get(&id) {
                queue.extend(children.iter().copied());
            }

            let node = &self.nodes[&id];

            // Every node has a parent except for the root node, which
            // is excluded from the `DidInsert` event
            let parent = self
                .parents
                .get(&id)
                .unwrap_or_else(|| panic!("node {id:?} does not have a parent"));

            event.inserted.push(InsertedNode {
                id,
                parent: *parent,
                node: node.clone(),
            });
        }

        event
    }

    pub fn subscribe_to_world_did_change_events(
        &self,
    ) -> tokio::sync::broadcast::Receiver<WorldDidChangeResponse> {
        self.world_did_change_events.subscribe()
    }

    pub async fn trigger_event(&self, event: TriggerEvent) -> Result<(), TriggerEventFailed> {
        let connection = self
            .connection_by_node
            .get(&event.target_node_id)
            .and_then(|connection_id| self.connections.get(connection_id)?.upgrade());
        let Some(connection) = connection else {
            return Err(TriggerEventFailed::NoConnectionForNode);
        };

        let mut connection = connection.write().await;
        connection.event_queue.push_back(event);
        let _ = connection.event_listener.send(());

        Ok(())
    }

    pub async fn take_next_event(&self, connection: ConnectionId) -> Option<Event> {
        let event = self.take_next_trigger_event(connection).await?;
        let target_id = self
            .nodes
            .get(&event.target_node_id)
            .and_then(|node| match &**node {
                FlatNode::Element(element) => element.attributes.get("id"),
                FlatNode::Text(_) => None,
            })
            .and_then(|id| id.as_str())
            .map(ToString::to_string);
        Some(Event {
            event: event.event,
            params: event.params,
            target_node_id: event.target_node_id,
            target_id,
        })
    }

    async fn take_next_trigger_event(&self, connection: ConnectionId) -> Option<TriggerEvent> {
        loop {
            // Get the connection if it still exists
            let connection = self.connections.get(&connection)?.upgrade()?;
            let mut connection = connection.write().await;

            // Pop an event if there's one currently queued
            if let Some(event) = connection.event_queue.pop_front() {
                return Some(event);
            }

            // Otherwise, subscribe...
            let mut listener = connection.event_listener.subscribe();
            drop(connection);

            // ...and wait for the next event, then try again
            listener.recv().await.ok()?;
        }
    }
}

impl Default for World {
    fn default() -> Self {
        let root_node = FlatNode::Element(FlatElement::new("World"));

        Self {
            nodes: BTreeMap::from_iter([(ROOT_NODE_ID, Arc::new(root_node))]),
            parents: BTreeMap::new(),
            children: BTreeMap::from_iter([(ROOT_NODE_ID, vec![])]),
            connections: BTreeMap::new(),
            connection_by_node: BTreeMap::new(),
            world_did_change_events: tokio::sync::broadcast::Sender::new(10),
        }
    }
}

struct ConnectionInner {
    event_queue: VecDeque<TriggerEvent>,
    event_listener: tokio::sync::broadcast::Sender<()>,
}

impl Default for ConnectionInner {
    fn default() -> Self {
        Self {
            event_queue: VecDeque::new(),
            event_listener: tokio::sync::broadcast::Sender::new(10),
        }
    }
}

pub struct Connection {
    pub id: ConnectionId,
    _inner: Arc<RwLock<ConnectionInner>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
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

#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
pub enum FlatNode {
    Text(String),
    Element(FlatElement),
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct FlatElement {
    tag: String,
    #[serde(default)]
    attributes: HashMap<String, serde_json::Value>,
}

impl FlatElement {
    pub fn new(tag: impl Into<String>) -> Self {
        Self {
            tag: tag.into(),
            attributes: HashMap::new(),
        }
    }
}

pub struct InsertNode {
    pub parent: NodeId,
    pub child: NodeId,
    pub offset: InsertNodeOffset,
}

#[derive(Debug, Clone, Copy)]
pub enum InsertNodeOffset {
    FromStart(usize),
    FromEnd(usize),
}

impl InsertNodeOffset {
    pub const BEGINNING: Self = InsertNodeOffset::FromStart(0);
    pub const END: Self = InsertNodeOffset::FromEnd(0);
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
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
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TriggerEvent {
    target_node_id: NodeId,
    event: String,
    params: serde_json::Value,
}

#[derive(Debug)]
pub enum InsertNodeFailed {
    NodeNotFound,
    ParentNotFound,
    OffsetOutOfBounds {
        offset: InsertNodeOffset,
        num_children: usize,
    },
    InvalidParentNodeType,
}

#[derive(Debug)]
pub enum RemoveNodeFailed {
    NodeNotFound,
    NoParentNode,
}

#[derive(Debug)]
pub enum SetNodeChildrenFailed {
    NodeNotFound,
    InvalidNodeType,
}

#[derive(Default, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorldDidChangeResponse {
    inserted: Vec<InsertedNode>,
    removed: Vec<NodeId>,
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InsertedNode {
    id: NodeId,
    parent: NodeId,
    node: Arc<FlatNode>,
}

#[derive(Debug)]
pub enum TriggerEventFailed {
    NoConnectionForNode,
}
