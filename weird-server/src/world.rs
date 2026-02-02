use std::{
    collections::{BTreeMap, HashMap, VecDeque},
    sync::Arc,
};

pub struct World {
    nodes: BTreeMap<NodeId, Arc<Node>>,
    parents: BTreeMap<NodeId, NodeId>,
    children: BTreeMap<NodeId, Vec<NodeId>>,
    change_sender: tokio::sync::broadcast::Sender<Vec<SyncChange>>,
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
            change_sender: tokio::sync::broadcast::Sender::new(10),
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

        let mut changes = vec![];
        let mut inserted_queue = VecDeque::from_iter([insert.child]);
        while let Some(id) = inserted_queue.pop_front() {
            changes.push(SyncChange::DidInsert {
                id,
                parent: self.parents[&id],
                node: self.nodes[&id].clone(),
            });

            if let Some(children) = self.children.get(&id) {
                inserted_queue.extend(children.iter().copied());
            }
        }

        let _ = self.change_sender.send(changes);

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

        Ok(*parent)
    }

    pub fn initial_sync(&self) -> Vec<SyncChange> {
        let root_children = self
            .children
            .get(&ROOT_NODE_ID)
            .expect("root node does not have child list");
        let mut queue: VecDeque<_> = root_children.iter().copied().collect();
        let mut changes = vec![];

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

            changes.push(SyncChange::DidInsert {
                id,
                parent: *parent,
                node: node.clone(),
            });
        }

        changes
    }

    pub fn subscribe_to_sync_changes(&self) -> tokio::sync::broadcast::Receiver<Vec<SyncChange>> {
        self.change_sender.subscribe()
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

pub enum RemoveNodeFailed {
    NodeNotFound,
    NoParentNode,
}

#[derive(Clone, facet::Facet)]
#[repr(u8)]
pub enum SyncChange {
    #[expect(unused)]
    DidInsert {
        id: NodeId,
        parent: NodeId,
        node: Arc<Node>,
    },
}
