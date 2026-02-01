#[derive(Debug, facet::Facet)]
#[repr(u8)]
#[facet(untagged)]
pub enum Message {
    Show { show: ShowMessage },
}

#[derive(Debug, facet::Facet)]
#[facet(rename_all = "PascalCase")]
pub struct ShowMessage {
    pub text: String,
}
