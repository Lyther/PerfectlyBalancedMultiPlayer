use crate::domain::{Manifest, SignedManifest};
use crate::file_backend::FileBackend;
use axum::{
    extract::{Path, State},
    response::IntoResponse,
    routing::get,
    Json, Router,
};

#[derive(Clone)]
pub struct AppState {
    pub manifest: Manifest,
    pub signed_manifest: Option<SignedManifest>,
    pub files: Option<FileBackend>,
}

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/manifest", get(manifest_handler))
        .route("/file/*path", get(file_handler))
        .with_state(state)
}

async fn manifest_handler(State(state): State<AppState>) -> axum::response::Response {
    if let Some(ref signed) = state.signed_manifest {
        Json(signed.clone()).into_response()
    } else {
        Json(state.manifest.clone()).into_response()
    }
}

async fn file_handler(
    State(state): State<AppState>,
    Path(path): Path<String>,
) -> impl IntoResponse {
    if !state.manifest.files.contains_key(&path) {
        return (axum::http::StatusCode::NOT_FOUND, "file not found").into_response();
    }
    match &state.files {
        Some(backend) => match backend.resolve(&path) {
            Some(fs_path) => match tokio::fs::read(&fs_path).await {
                Ok(blob) => {
                    let body = axum::body::Body::from(blob);
                    (
                        axum::http::StatusCode::OK,
                        [("content-type", "application/octet-stream")],
                        body,
                    )
                        .into_response()
                }
                Err(_) => (axum::http::StatusCode::NOT_FOUND, "file not found").into_response(),
            },
            None => (axum::http::StatusCode::NOT_FOUND, "file not found").into_response(),
        },
        None => (axum::http::StatusCode::NOT_FOUND, "file not found").into_response(),
    }
}
