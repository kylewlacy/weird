use std::collections::{BTreeMap, HashMap, VecDeque};

pub struct World {
    nodes: BTreeMap<NodeId, Node>,
}

impl World {
    pub fn new() -> Self {
        let root_node = Node::Element(Element {
            parent: None,
            class: Some("World".to_string()),
            properties: ElementProperties::default(),
            children: vec![],
        });

        Self {
            nodes: BTreeMap::from_iter([(ROOT_NODE_ID, root_node)]),
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

            let node = match node_tree.data {
                NodeTreeData::Text(text) => Node::Text(Text { parent, text }),
                NodeTreeData::Element(element) => {
                    let num_children = element.children.len();

                    queue.extend(element.children.into_iter().map(|child| (child, Some(id))));

                    Node::Element(Element {
                        parent,
                        class: node_tree.class,
                        properties: element.properties,
                        children: Vec::with_capacity(num_children),
                    })
                }
            };

            self.nodes.insert(id, node);

            if let Some(parent) = parent {
                let parent_node = &mut self
                    .nodes
                    .get_mut(&parent)
                    .expect("parent element not found");
                match parent_node {
                    Node::Element(element) => {
                        element.children.push(id);
                    }
                    Node::Text(_) => {
                        panic!("parent node is not an element");
                    }
                }
            }
        }

        new_id
    }

    pub fn insert_node(&mut self, insert: InsertNode) -> Result<usize, InsertNodeFailed> {
        let parent_node = self
            .nodes
            .get_mut(&insert.parent)
            .ok_or(InsertNodeFailed::ParentNotFound)?;
        let parent_element = match parent_node {
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
            let parent_element = match parent_node {
                Node::Element(parent_element) => parent_element,
                Node::Text(_) => {
                    panic!("parent node isn't valid anymore?");
                }
            };
            parent_element.children.remove(insert_index);

            return Err(InsertNodeFailed::NodeNotFound);
        };

        *node.parent_mut() = Some(insert.parent);

        Ok(insert_index)
    }

    #[expect(unused)]
    pub fn remove_node(&mut self, node_id: NodeId) -> Result<NodeId, RemoveNodeFailed> {
        let node = self
            .nodes
            .get_mut(&node_id)
            .ok_or(RemoveNodeFailed::NodeNotFound)?;
        let parent_node_id = node
            .parent_mut()
            .take()
            .ok_or(RemoveNodeFailed::NoParentNode)?;
        let mut parent_node = self
            .nodes
            .get_mut(&parent_node_id)
            .unwrap_or_else(|| panic!("tried to remove {node_id:?} but parent node not found"));

        let parent_element = match parent_node {
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

    pub fn initial_sync(&self) -> Vec<SyncChange<'_>> {
        let mut queue = VecDeque::from_iter([ROOT_NODE_ID]);
        let mut changes = vec![];

        while let Some(id) = queue.pop_front() {
            let node = &self.nodes[&id];

            match &node {
                Node::Text(_) => {}
                Node::Element(element) => {
                    queue.extend(element.children.iter().copied());
                }
            }

            changes.push(SyncChange::DidInsert {
                id,
                parent: node.parent(),
                node: FlatNode::from(node),
            });
        }

        changes
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
pub enum FlatNode<'a> {
    Text(#[expect(unused)] &'a str),
    Element(#[expect(unused)] FlatElement<'a>),
}

impl<'a> From<&'a Node> for FlatNode<'a> {
    fn from(node: &'a Node) -> Self {
        match node {
            Node::Text(Text { text, parent: _ }) => Self::Text(text),
            Node::Element(element) => Self::Element(element.into()),
        }
    }
}

#[derive(Debug, facet::Facet)]
#[facet(metadata_container)]
pub struct FlatElement<'a> {
    #[facet(metadata = "tag")]
    class: Option<&'a str>,
    properties: &'a ElementProperties,
}

impl<'a> From<&'a Element> for FlatElement<'a> {
    fn from(element: &'a Element) -> Self {
        let Element {
            class,
            children: _,
            parent: _,
            properties,
        } = element;

        Self {
            class: class.as_deref(),
            properties,
        }
    }
}

#[derive(Debug, Default, facet::Facet)]
#[facet(rename_all = "camelCase")]
pub struct ElementProperties {
    #[facet(flatten)]
    attributes: HashMap<String, String>,
}

enum Node {
    Text(Text),
    Element(Element),
}

impl Node {
    fn parent(&self) -> Option<NodeId> {
        match self {
            Self::Text(text) => text.parent,
            Self::Element(element) => element.parent,
        }
    }

    fn parent_mut(&mut self) -> &mut Option<NodeId> {
        match self {
            Self::Text(text) => &mut text.parent,
            Self::Element(element) => &mut element.parent,
        }
    }
}

pub struct Text {
    parent: Option<NodeId>,
    text: String,
}

pub struct Element {
    parent: Option<NodeId>,
    class: Option<String>,
    children: Vec<NodeId>,
    properties: ElementProperties,
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

#[derive(facet::Facet)]
#[repr(u8)]
pub enum SyncChange<'a> {
    #[expect(unused)]
    DidInsert {
        id: NodeId,
        parent: Option<NodeId>,
        node: FlatNode<'a>,
    },
}
