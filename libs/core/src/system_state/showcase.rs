#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(all(feature = "gen-binds", not(feature = "salvo")), derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(all(feature = "gen-binds", not(feature = "salvo")), ts(export))]
pub struct GithubUser {
    pub login: String,
    pub avatar_url: String,
}

/// Code the user types on github.com/login/device to authorize the app.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(all(feature = "gen-binds", not(feature = "salvo")), derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(all(feature = "gen-binds", not(feature = "salvo")), ts(export))]
pub struct GithubDeviceCode {
    pub user_code: String,
    pub verification_uri: String,
    pub expires_in: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(all(feature = "gen-binds", not(feature = "salvo")), derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(all(feature = "gen-binds", not(feature = "salvo")), ts(export))]
pub struct ShowcaseSubmission {
    pub name: String,
    pub description: String,
    pub tags: Vec<String>,
}
