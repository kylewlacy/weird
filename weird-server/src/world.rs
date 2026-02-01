use std::collections::{BTreeMap, HashMap};

pub struct World {
    nodes: BTreeMap<NodeId, WorldNode>,
}

impl World {
    pub fn new() -> Self {
        let root_node = WorldNode {
            parent: None,
            node: Node::Element(Element {
                class: "$root".to_string(),
                attributes: HashMap::new(),
                children: vec![],
            }),
        };

        Self {
            nodes: BTreeMap::from_iter([(ROOT_NODE_ID, root_node)]),
        }
    }

    pub fn create_node(&mut self, node: Node) -> NodeId {
        let id = self
            .nodes
            .last_key_value()
            .map(|(last_id, _)| NodeId(last_id.0 + 1))
            .expect("world.nodes is empty");

        self.nodes.insert(id, WorldNode { node, parent: None });

        id
    }

    pub fn insert_node(&mut self, insert: InsertNode) -> Result<usize, InsertNodeFailed> {
        let parent_node = self
            .nodes
            .get_mut(&insert.parent)
            .ok_or(InsertNodeFailed::ParentNotFound)?;
        let parent_element = match &mut parent_node.node {
            Node::Element(parent_element) => parent_element,
            Node::Text(_) => {
                return Err(InsertNodeFailed::InvalidParentNodeType);
            }
        };

        let num_children = parent_element.children.len();
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
        parent_element.children.insert(insert_index, insert.child);

        let node = self.nodes.get_mut(&insert.child);
        let Some(node) = node else {
            // Node not found! Remove the reference in the parent node we
            // just added, then return an error

            let parent_node = self.nodes.get_mut(&insert.parent).unwrap();
            let parent_element = match &mut parent_node.node {
                Node::Element(parent_element) => parent_element,
                Node::Text(_) => {
                    panic!("parent node isn't valid anymore?");
                }
            };
            parent_element.children.remove(insert_index);

            return Err(InsertNodeFailed::NodeNotFound);
        };

        node.parent = Some(insert.parent);

        Ok(insert_index)
    }

    pub fn remove_node(&mut self, node_id: NodeId) -> Result<NodeId, RemoveNodeFailed> {
        let node = self
            .nodes
            .get_mut(&node_id)
            .ok_or(RemoveNodeFailed::NodeNotFound)?;
        let parent_node_id = node.parent.take().ok_or(RemoveNodeFailed::NoParentNode)?;
        let parent_node = self
            .nodes
            .get_mut(&parent_node_id)
            .unwrap_or_else(|| panic!("tried to remove {node_id:?} but parent node not found"));

        let parent_element = match &mut parent_node.node {
            Node::Element(parent_element) => parent_element,
            Node::Text(_) => {
                panic!("tried to remove {node_id:?} but parent is not an element");
            }
        };
        let child_pos = parent_element
            .children
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
        parent_element.children.remove(child_pos);

        Ok(parent_node_id)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct NodeId(u64);

pub const ROOT_NODE_ID: NodeId = NodeId(0);

pub enum Node {
    Text(String),
    Element(Element),
}

pub struct Element {
    class: String,
    attributes: HashMap<String, String>,
    children: Vec<NodeId>,
}

struct WorldNode {
    parent: Option<NodeId>,
    node: Node,
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

pub enum RemoveNodeFailed {
    NodeNotFound,
    NoParentNode,
}
