use std::{
    collections::{BTreeMap, BTreeSet, HashMap, HashSet, VecDeque},
    sync::Arc,
};

pub struct World {
    nodes: BTreeMap<NodeId, Arc<FlatNode>>,
    parents: BTreeMap<NodeId, (NodeId, usize)>,
    children: BTreeMap<NodeId, Vec<NodeId>>,
    connection_by_node: BTreeMap<NodeId, ConnectionId>,
    connections: BTreeMap<ConnectionId, ConnectionInner>,
    world_did_change_events: tokio::sync::broadcast::Sender<WorldDidChangeResponse>,
}

impl World {
    pub fn create_connection(&mut self) -> Connection {
        let id = self
            .connections
            .last_key_value()
            .map_or(ConnectionId(0), |(last_id, _)| ConnectionId(last_id.0 + 1));
        let (event_tx, event_rx) = tokio::sync::mpsc::unbounded_channel();
        let inner = ConnectionInner { event_tx };
        let conn = Connection { id, event_rx };
        self.connections.insert(id, inner);
        conn
    }

    pub fn nodes(&self) -> impl Iterator<Item = (NodeId, &FlatNode)> {
        self.nodes.iter().map(|(key, value)| (*key, &**value))
    }

    pub fn create_node(&mut self, node: Node, connection_id: ConnectionId) -> NodeId {
        let new_id = self
            .nodes
            .last_key_value()
            .map(|(last_id, _)| NodeId(last_id.0 + 1))
            .expect("world.nodes is empty");

        let mut next_id = new_id;
        let mut queue: VecDeque<(Node, Option<(NodeId, usize)>)> =
            [(node, None)].into_iter().collect();

        while let Some((node_tree, parent)) = queue.pop_front() {
            let id = next_id;
            next_id = NodeId(next_id.0 + 1);

            self.connection_by_node.insert(id, connection_id);
            match node_tree {
                Node::Text(text) => {
                    self.nodes.insert(id, Arc::new(FlatNode::Text(text)));
                }
                Node::Element(element) => {
                    queue.extend(
                        element
                            .children
                            .into_iter()
                            .enumerate()
                            .map(|(index, child)| (child, Some((id, index)))),
                    );

                    self.nodes
                        .insert(id, Arc::new(FlatNode::Element(element.element)));
                    self.children.insert(id, vec![]);
                }
            };

            if let Some((parent_id, index)) = parent {
                self.parents.insert(id, (parent_id, index));

                let parent_children = self
                    .children
                    .get_mut(&parent_id)
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
        self.parents
            .insert(insert.child, (insert.parent, insert_index));

        let mut queue = VecDeque::from_iter([insert.child]);
        while let Some(id) = queue.pop_front() {
            let (parent_id, parent_index) = self.parents[&id];
            let before_sibling_id = if parent_index + 1 < num_children {
                Some(self.children[&parent_id][parent_index + 1])
            } else {
                None
            };
            event.changes.push(WorldChange::Created(CreatedNodeChange {
                id,
                parent_id,
                before_sibling_id,
                node: self.nodes[&id].clone(),
            }));

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

        let (parent_id, parent_index) = self
            .parents
            .get(&node_id)
            .ok_or(RemoveNodeFailed::NoParentNode)?;

        let parent_children = self
            .children
            .get_mut(parent_id)
            .unwrap_or_else(|| panic!("children not found for parent {parent_id:?}"));
        parent_children.remove(*parent_index);

        let num_receivers = self.world_did_change_events.receiver_count();
        if num_receivers != 0 {
            tracing::debug!(num_receivers, "broadcasting change event");
            let mut event = WorldDidChangeResponse::default();

            event
                .changes
                .push(WorldChange::Deleted(DeletedNodeChange { id: node_id }));

            let _ = self.world_did_change_events.send(event);
        } else {
            tracing::debug!("no listeners, skipping change event");
        }

        Ok(*parent_id)
    }

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
            self.connection_by_node.remove(&node_id);
            let parent = self.parents.remove(&node_id);
            let children = self.children.remove(&node_id);

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
        let node_id = self
            .nodes
            .last_key_value()
            .map(|(last_id, _)| NodeId(last_id.0 + 1))
            .expect("world.nodes is empty");
        let mut queue = VecDeque::from_iter([(Some(node_id), node, parent_id)]);
        while let Some((node_id, node, parent_id)) = queue.pop_front() {
            let node_id = node_id.unwrap_or_else(|| {
                self.nodes
                    .last_key_value()
                    .map(|(last_id, _)| NodeId(last_id.0 + 1))
                    .expect("world.nodes is empty")
            });

            let (node, children) = node.into_flat_and_children();
            let node = Arc::new(node);
            self.nodes.insert(node_id, node.clone());
            self.connection_by_node.insert(node_id, connection_id);

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

        node_id
    }

    pub fn set_node_children(
        &mut self,
        node_id: NodeId,
        children: Vec<Node>,
        connection: ConnectionId,
    ) -> Result<bool, SetNodeChildrenFailed> {
        let mut event = WorldDidChangeResponse::default();

        if !self.nodes.contains_key(&node_id) {
            return Err(SetNodeChildrenFailed::NodeNotFound);
        }

        let mut update_queue = VecDeque::from_iter([(node_id, children)]);

        while let Some((node_id, new_children)) = update_queue.pop_front() {
            let Some(current_child_ids) = self.children.get(&node_id) else {
                return Err(SetNodeChildrenFailed::InvalidNodeType);
            };

            let mut unmatched_children = HashMap::<NodeMatchKey, VecDeque<_>>::new();
            for current_child_id in current_child_ids {
                let current_child = self
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
                    let matched_child = self.nodes.get_mut(&matched_child_id).unwrap();

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
                    self.append_node_inner(child, node_id, connection, &mut event)
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
            self.remove_nodes_inner(unmatched_children, &mut event);

            // Reorder the child nodes. We do this in reverse order because
            // moves events have a `before_sibling_id`, so moving the
            // end nodes first simplifies things.
            for (index, child_id) in new_ordered_child_ids.iter().enumerate().rev() {
                let (parent_id, current_index) = self.parents.get_mut(child_id).unwrap();

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
            self.children.insert(node_id, new_ordered_child_ids);
        }

        if event.is_empty() {
            Ok(false)
        } else {
            let _ = self.world_did_change_events.send(event);
            Ok(true)
        }
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
            let (parent_id, parent_index) = self
                .parents
                .get(&id)
                .unwrap_or_else(|| panic!("node {id:?} does not have a parent"));

            let before_sibling_id = self.children[parent_id].get(parent_index + 1).copied();
            event.changes.push(WorldChange::Created(CreatedNodeChange {
                id,
                parent_id: *parent_id,
                before_sibling_id,
                node: node.clone(),
            }));
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
            .and_then(|connection_id| self.connections.get(connection_id));
        let Some(connection) = connection else {
            return Err(TriggerEventFailed::NoConnectionForNode);
        };

        let target_id = self
            .nodes
            .get(&event.target_node_id)
            .and_then(|node| match &**node {
                FlatNode::Element(element) => element.attributes.get("id"),
                FlatNode::Text(_) => None,
            })
            .and_then(|id| id.as_str())
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

        Ok(())
    }

    pub fn assert_internally_consistent(&self) {
        assert!(self.is_internally_consistent());
    }

    fn is_internally_consistent(&self) -> bool {
        for (node_id, node) in &self.nodes {
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

        true
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
    event_tx: tokio::sync::mpsc::UnboundedSender<Event>,
}

pub struct Connection {
    pub id: ConnectionId,
    event_rx: tokio::sync::mpsc::UnboundedReceiver<Event>,
}

impl Connection {
    pub async fn next_event(&mut self) -> Option<Event> {
        self.event_rx.recv().await
    }
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
    Created(CreatedNodeChange),
    Updated(UpdatedNodeChange),
    Moved(MovedNodeChange),
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

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreatedNodeChange {
    pub id: NodeId,
    pub parent_id: NodeId,
    pub before_sibling_id: Option<NodeId>,
    pub node: Arc<FlatNode>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdatedNodeChange {
    pub id: NodeId,
    #[serde(flatten)]
    pub update: NodeUpdate,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MovedNodeChange {
    pub id: NodeId,
    pub parent_id: NodeId,
    pub before_sibling_id: Option<NodeId>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeletedNodeChange {
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
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    set_attributes: HashMap<String, serde_json::Value>,

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

#[derive(Debug)]
pub enum TriggerEventFailed {
    NoConnectionForNode,
}
