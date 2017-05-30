
#[derive(Serialize, Deserialize, Clone)]
pub struct ConnectOption {
    pub verbose: bool,
    pub pedantic: bool,
    pub ssl_required: bool,
    pub auth_token: String,
    pub user: String,
    pub pass: String,
    pub name: String,
    pub lang: String,
    pub version: String,
}

impl ConnectOption {
    pub fn new() -> ConnectOption {
        Self::new_internal(None, None, None, None)
    }
    /// Create connection parameter with additional parameters
    ///
    /// for suppress "+OK" message, pass verbose=true
    pub fn new_with_param(user: &str, pass: &str, verbose: bool, appname: &str) -> ConnectOption {
        Self::new_internal(Some(user), Some(pass), Some(verbose), Some(appname))
    }
    fn new_internal(user: Option<&str>, pass: Option<&str>, verbose: Option<bool>, appname: Option<&str>) -> ConnectOption {
        ConnectOption {
            verbose: verbose.unwrap_or(false),
            pedantic: false,
            ssl_required: false,
            auth_token: String::from(""),
            user: user.unwrap_or_default().to_owned(),
            pass: pass.unwrap_or_default().to_owned(),
            name: appname.unwrap_or("simple-rust-nats-client").to_owned(),
            lang: String::from("Rust"),
            version: String::from("0.0.1"),
        }
    }
}

