

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
pub struct ServerInfo {
    pub server_id: String,
    pub version: String,
    pub go: String,
    pub host: String,
    pub port: i32,
    pub auth_required: bool,
    pub ssl_required: bool,
    pub max_payload: i64,
}

