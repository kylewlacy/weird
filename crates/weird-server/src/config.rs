use std::collections::HashSet;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Config {
    #[serde(default = "default_allow_origins")]
    pub allow_origins: HashSet<String>,
}

fn default_allow_origins() -> HashSet<String> {
    HashSet::from_iter(["http://localhost:5173".to_string()])
}
