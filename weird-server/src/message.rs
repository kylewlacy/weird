use crate::world::{NodeTree, SyncChange};

#[derive(Debug, facet::Facet)]
#[repr(u8)]
#[facet(untagged)]
pub enum Message {
    #[facet(rename_all = "camelCase")]
    SyncWorld { sync_world: SyncWorldRequest },

    #[facet(rename_all = "camelCase")]
    Show { show: NodeTree },
}

#[derive(Debug, facet::Facet)]
#[facet(rename_all = "camelCase")]
pub struct SyncWorldRequest {
    pub request_id: String,
}

#[derive(facet::Facet)]
#[repr(u8)]
#[facet(untagged)]
pub enum ServerMessage {
    #[facet(rename_all = "camelCase")]
    #[expect(unused)]
    SyncWorld { sync_world: SyncWorldResponse },
}

#[derive(facet::Facet)]
#[facet(rename_all = "camelCase")]
pub struct SyncWorldResponse {
    pub request_id: String,
    pub changes: Vec<SyncChange>,
}
