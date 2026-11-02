use serde::Serialize;

use super::StoredRequest;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AddProjectArgs {
    pub path: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CheckHeadersGitignoreArgs {
    pub path: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LoadRequestListArgs {
    pub project_path: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LoadRequestArgs {
    pub project_path: String,
    pub id: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveRequestArgs {
    pub project_path: String,
    #[serde(flatten)]
    pub request: StoredRequest,
}
