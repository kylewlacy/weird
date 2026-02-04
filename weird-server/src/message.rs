use crate::world::{NodeTree, WorldChangeEvent};

#[derive(Debug, facet::Facet)]
#[repr(u8)]
#[facet(untagged)]
pub enum Message {
    #[facet(rename_all = "camelCase")]
    SyncWorld { sync_world: SyncWorldRequest },

    #[facet(rename_all = "camelCase")]
    Show { show: Vec<NodeTree> },
}

#[derive(Debug, facet::Facet)]
#[facet(rename_all = "camelCase")]
pub struct SyncWorldRequest {
    pub request_id: String,
}

#[derive(facet::Facet)]
#[repr(u8)]
#[facet(untagged)]
#[expect(unused)]
pub enum ServerMessage {
    #[facet(rename_all = "camelCase")]
    Event { event: ServerEvent, id: String },
}

#[derive(facet::Facet)]
#[repr(u8)]
#[facet(untagged)]
#[expect(unused)]
pub enum ServerEvent {
    WorldChange(WorldChangeEvent),
}
