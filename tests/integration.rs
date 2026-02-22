use axum::body::Body;
use axum::http::Request;
use serde_json::Value;
use smms::domain::Manifest;
use smms::file_backend::FileBackend;
use std::collections::BTreeMap;
use std::io::Write;
use tower::util::ServiceExt;

fn make_test_state() -> smms::server::AppState {
    let temp = std::env::temp_dir().join("smms_test");
    let _ = std::fs::create_dir_all(&temp);
    std::fs::File::create(temp.join("foo.txt"))
        .and_then(|mut f| f.write_all(b"hello world"))
        .unwrap();
    let backend = FileBackend::new(vec![("test".to_string(), temp)]);
    smms::server::AppState {
        manifest: Manifest {
            version: 1,
            generated_at: "2026-02-22T12:00:00Z".to_string(),
            files: {
                let mut m = BTreeMap::new();
                m.insert(
                    "test/foo.txt".to_string(),
                    blake3::hash(b"hello world").to_hex().to_string(),
                );
                m
            },
            load_order: vec!["mod/ugc_123.mod".to_string()],
        },
        signed_manifest: None,
        files: Some(backend),
    }
}

#[tokio::test]
async fn manifest_returns_200_and_valid_json() {
    let state = make_test_state();
    let app = smms::server::router(state);
    let req = Request::builder()
        .uri("/manifest")
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), 200);
    let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["version"], 1);
    assert!(json["generated_at"].as_str().is_some());
    assert!(json["files"].is_object());
    assert!(json["load_order"].is_array());
}

#[tokio::test]
async fn file_returns_200_for_existing_path() {
    let state = make_test_state();
    let app = smms::server::router(state);
    let req = Request::builder()
        .uri("/file/test/foo.txt")
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), 200);
    let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    assert_eq!(body.as_ref(), b"hello world");
}

#[tokio::test]
async fn file_returns_404_for_missing_path() {
    let state = make_test_state();
    let app = smms::server::router(state);
    let req = Request::builder()
        .uri("/file/nonexistent.txt")
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), 404);
}

#[tokio::test]
async fn file_returns_404_for_traversal_path() {
    let state = make_test_state();
    let app = smms::server::router(state);
    let req = Request::builder()
        .uri("/file/test/../evil.txt")
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), 404);
}

#[test]
fn manifest_validate_hashes_rejects_invalid() {
    let manifest = Manifest {
        version: 1,
        generated_at: "2026-02-22T12:00:00Z".to_string(),
        files: {
            let mut m = BTreeMap::new();
            m.insert("test/foo.txt".to_string(), "not64hexchars".to_string());
            m
        },
        load_order: vec![],
    };
    assert!(manifest.validate_hashes().is_err());
}

#[test]
fn validate_manifest_refuses_empty_files_with_load_order() {
    let manifest = Manifest {
        version: 1,
        generated_at: "2026-02-22T12:00:00Z".to_string(),
        files: BTreeMap::new(),
        load_order: vec!["mod/ugc_1.mod".to_string()],
    };
    assert!(smms::client::validate_manifest_for_fetch(&manifest, false).is_err());
    assert!(smms::client::validate_manifest_for_fetch(&manifest, true).is_ok());
}

#[test]
fn validate_manifest_refuses_invalid_mod_ref() {
    // FIXED: guard against path-traversal style mod refs before client writes descriptor/load order files.
    let manifest = Manifest {
        version: 1,
        generated_at: "2026-02-22T12:00:00Z".to_string(),
        files: BTreeMap::new(),
        load_order: vec!["mod/../../evil.mod".to_string()],
    };
    assert!(smms::client::validate_manifest_for_fetch(&manifest, true).is_err());
}

#[test]
fn file_backend_rejects_traversal_paths() {
    let temp = std::env::temp_dir().join("smms_test_traversal");
    let _ = std::fs::create_dir_all(&temp);
    let backend = FileBackend::new(vec![("test".to_string(), temp.clone())]);
    assert!(backend.resolve_path("test/../evil.txt").is_none());
    assert!(backend.resolve_path("test/foo/../../etc/passwd").is_none());
    assert!(backend.resolve_path("..\\test\\foo.txt").is_none());
}

#[test]
fn file_backend_prefix_boundary_safe() {
    let temp = std::env::temp_dir().join("smms_test_prefix");
    let _ = std::fs::create_dir_all(&temp);
    std::fs::write(temp.join("foo.txt"), b"foo").unwrap();
    std::fs::create_dir_all(temp.join("foobar")).unwrap();
    std::fs::write(temp.join("foobar").join("bar.txt"), b"bar").unwrap();
    let backend = FileBackend::new(vec![("test".to_string(), temp.clone())]);
    assert!(backend.resolve_path("test/foo.txt").is_some());
    assert!(backend.resolve_path("test/foobar/bar.txt").is_some());
    assert!(backend.resolve_path("test/foobar").is_some());
}

#[test]
fn verify_downloaded_hash_rejects_mismatch() {
    let expected = blake3::hash(b"correct content").to_hex().to_string();
    assert!(
        smms::client::verify_downloaded_hash(b"wrong content", &expected, "test/foo.txt").is_err()
    );
    assert!(
        smms::client::verify_downloaded_hash(b"correct content", &expected, "test/foo.txt").is_ok()
    );
}
