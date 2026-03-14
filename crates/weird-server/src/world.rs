use std::{
    collections::{BTreeMap, HashMap, VecDeque},
    sync::Arc,
};

pub struct World {
    nodes: BTreeMap<NodeId, Arc<Node>>,
    parents: BTreeMap<NodeId, NodeId>,
    children: BTreeMap<NodeId, Vec<NodeId>>,
    world_did_change_events: tokio::sync::broadcast::Sender<WorldDidChangeEvent>,
}

impl World {
    pub fn new() -> Self {
        let root_node = Node::Element(Element::new("World"));

        Self {
            nodes: BTreeMap::from_iter([(ROOT_NODE_ID, Arc::new(root_node))]),
            parents: BTreeMap::new(),
            children: BTreeMap::from_iter([(ROOT_NODE_ID, vec![])]),
            world_did_change_events: tokio::sync::broadcast::Sender::new(10),
        }
    }

    pub fn create_node(&mut self, node: NodeTree) -> NodeId {
        let new_id = self
            .nodes
            .last_key_value()
            .map(|(last_id, _)| NodeId(last_id.0 + 1))
            .expect("world.nodes is empty");

        let mut next_id = new_id;
        let mut queue: VecDeque<(NodeTree, Option<NodeId>)> = [(node, None)].into_iter().collect();

        while let Some((node_tree, parent)) = queue.pop_front() {
            let id = next_id;
            next_id = NodeId(next_id.0 + 1);

            match node_tree {
                NodeTree::Text(text) => {
                    self.nodes.insert(id, Arc::new(Node::Text(text)));
                }
                NodeTree::Element(element) => {
                    queue.extend(element.children.into_iter().map(|child| (child, Some(id))));

                    self.nodes
                        .insert(id, Arc::new(Node::Element(element.element)));
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
        let mut event = WorldDidChangeEvent::default();

        let insert_index = self.insert_node_for_event(insert, &mut event)?;

        let _ = self.world_did_change_events.send(event);

        Ok(insert_index)
    }

    fn insert_node_for_event(
        &mut self,
        insert: InsertNode,
        event: &mut WorldDidChangeEvent,
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

    #[expect(unused)]
    pub fn remove_node(&mut self, node_id: NodeId) -> Result<NodeId, RemoveNodeFailed> {
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
            let mut event = WorldDidChangeEvent::default();

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
        children: Vec<NodeTree>,
    ) -> Result<(), SetNodeChildrenFailed> {
        let mut event = WorldDidChangeEvent::default();

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
            let child = self.create_node(child);
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

    pub fn initial_client_world_did_change_event(&self) -> WorldDidChangeEvent {
        let mut event = WorldDidChangeEvent::default();

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
    ) -> tokio::sync::broadcast::Receiver<WorldDidChangeEvent> {
        self.world_did_change_events.subscribe()
    }
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

pub const ROOT_NODE_ID: NodeId = NodeId(0);

#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
pub enum NodeTree {
    Text(String),
    Element(ElementTree),
}

impl Into<NodeTree> for ElementTree {
    fn into(self) -> NodeTree {
        NodeTree::Element(self)
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct ElementTree {
    #[serde(default)]
    children: Vec<NodeTree>,
    #[serde(flatten)]
    element: Element,
}

impl ElementTree {
    pub fn new(tag: impl Into<String>) -> Self {
        Self {
            element: Element::new(tag),
            children: vec![],
        }
    }

    pub fn children(mut self, children: impl IntoIterator<Item = NodeTree>) -> Self {
        self.children.extend(children);
        self
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
pub enum Node {
    Text(String),
    Element(Element),
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct Element {
    tag: String,
    #[serde(default)]
    attributes: HashMap<String, serde_json::Value>,
}

impl Element {
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
    #[expect(unused)]
    pub const BEGINNING: Self = InsertNodeOffset::FromStart(0);
    pub const END: Self = InsertNodeOffset::FromEnd(0);
}

#[derive(Debug)]
pub enum InsertNodeFailed {
    NodeNotFound,
    ParentNotFound,
    OffsetOutOfBounds {
        #[expect(unused)]
        offset: InsertNodeOffset,
        #[expect(unused)]
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
pub struct WorldDidChangeEvent {
    inserted: Vec<InsertedNode>,
    removed: Vec<NodeId>,
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InsertedNode {
    id: NodeId,
    parent: NodeId,
    node: Arc<Node>,
}
