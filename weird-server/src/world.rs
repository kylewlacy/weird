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
        let root_node = Node::Element(Element {
            class: Some("World".to_string()),
            properties: ElementProperties::default(),
        });

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

            match node_tree.data {
                NodeTreeData::Text(text) => {
                    self.nodes.insert(id, Arc::new(Node::Text(text)));
                }
                NodeTreeData::Element(element) => {
                    queue.extend(element.children.into_iter().map(|child| (child, Some(id))));

                    self.nodes.insert(
                        id,
                        Arc::new(Node::Element(Element {
                            class: node_tree.class,
                            properties: element.properties,
                        })),
                    );
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

        // Broadcast a message for node changes, but only if there's a listener.
        // NOTE: This only makes sense if we can guarantee that someone else
        // can't subscribe while this function is running! This is only valid
        // here because we have `&mut` access to the sender and we never clone
        // it, meaning a listener can't subscribe while where in this function.
        // The assert helps catch if one of our assumptions breaks though.
        assert!(self.world_did_change_events.strong_count() == 1);
        assert!(self.world_did_change_events.weak_count() == 0);

        let num_receivers = self.world_did_change_events.receiver_count();
        if num_receivers != 0 {
            tracing::debug!(num_receivers, "broadcasting change event");
            let mut event = WorldDidChangeEvent::default();

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

            let _ = self.world_did_change_events.send(event);
        } else {
            tracing::debug!("no listeners, skipping change event");
        }

        Ok(insert_index)
    }

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

    pub fn node_children(&self, node_id: NodeId) -> Option<&[NodeId]> {
        self.children.get(&node_id).map(|children| &**children)
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, facet::Facet)]
#[facet(transparent)]
pub struct NodeId(u64);

pub const ROOT_NODE_ID: NodeId = NodeId(0);

#[derive(Debug, facet::Facet)]
#[facet(metadata_container)]
pub struct NodeTree {
    #[facet(metadata = "tag")]
    class: Option<String>,
    data: NodeTreeData,
}

impl Into<NodeTree> for ElementTree {
    fn into(self) -> NodeTree {
        NodeTree {
            class: self.class,
            data: NodeTreeData::Element(self.data),
        }
    }
}

#[derive(Debug, facet::Facet)]
#[repr(u8)]
#[facet(untagged)]
pub enum NodeTreeData {
    Text(String),
    Element(ElementTreeData),
}

#[derive(Debug, facet::Facet)]
#[facet(metadata_container)]
pub struct ElementTree {
    #[facet(metadata = "tag")]
    class: Option<String>,
    #[facet(flatten)]
    data: ElementTreeData,
}

impl ElementTree {
    pub fn new(
        class: impl Into<String>,
        properties: ElementProperties,
        children: Vec<NodeTree>,
    ) -> Self {
        Self {
            class: Some(class.into()),
            data: ElementTreeData {
                children,
                properties,
            },
        }
    }
}

#[derive(Debug, facet::Facet)]
#[facet(rename_all = "camelCase")]
pub struct ElementTreeData {
    #[facet(default)]
    children: Vec<NodeTree>,
    #[facet(flatten)]
    properties: ElementProperties,
}

#[derive(Debug, facet::Facet)]
#[repr(u8)]
#[facet(untagged)]
pub enum Node {
    Text(#[expect(unused)] String),
    Element(#[expect(unused)] Element),
}

#[derive(Debug, facet::Facet)]
#[facet(metadata_container)]
pub struct Element {
    #[facet(metadata = "tag")]
    class: Option<String>,
    properties: ElementProperties,
}

#[derive(Debug, Default, facet::Facet)]
#[facet(rename_all = "camelCase")]
pub struct ElementProperties {
    #[facet(flatten)]
    attributes: HashMap<String, String>,
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

#[derive(Default, Clone, facet::Facet)]
#[facet(rename_all = "camelCase")]
pub struct WorldDidChangeEvent {
    inserted: Vec<InsertedNode>,
    removed: Vec<NodeId>,
}

#[derive(Clone, facet::Facet)]
#[facet(rename_all = "camelCase")]
pub struct InsertedNode {
    id: NodeId,
    parent: NodeId,
    node: Arc<Node>,
}
