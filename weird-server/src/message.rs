use crate::world::SyncChange;

#[derive(Debug, facet::Facet)]
#[repr(u8)]
#[facet(untagged)]
pub enum Message {
    #[facet(rename_all = "camelCase")]
    Show { show: ShowMessage },

    #[facet(rename_all = "camelCase")]
    SyncWorld { sync_world: SyncWorldRequest },
}

#[derive(Debug, facet::Facet)]
#[facet(rename_all = "PascalCase")]
pub struct ShowMessage {
    pub text: String,
}

#[derive(Debug, facet::Facet)]
#[facet(rename_all = "camelCase")]
pub struct SyncWorldRequest {
    pub request_id: String,
}

#[derive(facet::Facet)]
#[repr(u8)]
#[facet(untagged)]
pub enum ServerMessage<'a> {
    #[facet(rename_all = "camelCase")]
    #[expect(unused)]
    SyncWorld { sync_world: SyncWorldResponse<'a> },
}

#[derive(facet::Facet)]
#[facet(rename_all = "camelCase")]
pub struct SyncWorldResponse<'a> {
    pub request_id: String,
    pub changes: Vec<SyncChange<'a>>,
}
