use serde::Serialize;

use super::StoredRequest;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AddProjectArgs {
    pub(crate) path: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CheckHeadersGitignoreArgs {
    pub(crate) path: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LoadRequestListArgs {
    pub(crate) project_path: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LoadRequestArgs {
    pub(crate) project_path: String,
    pub(crate) id: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SaveRequestArgs {
    pub(crate) project_path: String,
    #[serde(flatten)]
    pub(crate) request: StoredRequest,
}
