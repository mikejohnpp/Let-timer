use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use let_timer_core::{Command, Database, MediaType, NewMedia};
use let_timer_http::policy::{PolicyError, validate_remote};
use let_timer_http::{AppState, build_router};
use serde_json::Value;
use std::sync::{Arc, Mutex};
use tower::ServiceExt;

#[tokio::test]
async fn sec_1_rpc_rejects_import_file_with_403_and_creates_no_file() {
    let temp_media_dir = std::env::temp_dir().join(format!(
        "let_timer_sec1_media_{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&temp_media_dir).unwrap();
    // Safety check: point media dir to empty temp folder
    unsafe {
        std::env::set_var("LET_TIMER_MEDIA_DIR", &temp_media_dir);
    }

    let db = Arc::new(Mutex::new(Database::open_in_memory().unwrap()));
    let state = Arc::new(AppState {
        db: Arc::clone(&db),
        is_loopback: true,
        command_handler: Arc::new(|cmd, conn, ctx, notify, events| {
            let_timer_daemon::handle_command(cmd, conn, ctx, notify, events)
        }),
        timer_notify: None,
        events: None,
    });

    let app = build_router(state);

    let cmd = Command::CreateMedia {
        media: NewMedia {
            name: "stolen".into(),
            description: None,
            url: String::new(),
            media_type: MediaType::Image,
            content: None,
        },
        import_file: Some("/etc/hostname".into()),
    };

    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/rpc")
        .header("content-type", "application/json")
        .header("x-let-timer-protocol", "1")
        .body(Body::from(serde_json::to_string(&cmd).unwrap()))
        .unwrap();

    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);

    let body = res.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["error"]["code"], "forbidden_remote_command");

    // Assert NO file was written to the media directory
    let entries: Vec<_> = std::fs::read_dir(&temp_media_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .collect();
    assert_eq!(
        entries.len(),
        0,
        "No files should appear in media directory"
    );

    let _ = std::fs::remove_dir_all(temp_media_dir);
}

#[tokio::test]
async fn sec_1_rpc_allows_create_media_without_import_file() {
    let db = Arc::new(Mutex::new(Database::open_in_memory().unwrap()));
    let state = Arc::new(AppState {
        db: Arc::clone(&db),
        is_loopback: true,
        command_handler: Arc::new(|cmd, conn, ctx, notify, events| {
            let_timer_daemon::handle_command(cmd, conn, ctx, notify, events)
        }),
        timer_notify: None,
        events: None,
    });

    let app = build_router(state);

    let cmd = Command::CreateMedia {
        media: NewMedia {
            name: "remote-image".into(),
            description: None,
            url: "https://example.com/image.png".into(),
            media_type: MediaType::Image,
            content: None,
        },
        import_file: None,
    };

    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/rpc")
        .header("content-type", "application/json")
        .header("x-let-timer-protocol", "1")
        .body(Body::from(serde_json::to_string(&cmd).unwrap()))
        .unwrap();

    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let body = res.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["Media"]["name"], "remote-image");
    assert_eq!(json["Media"]["url"], "https://example.com/image.png");
}

#[tokio::test]
async fn sec_1_rest_create_media_succeeds_without_filesystem_path() {
    let db = Arc::new(Mutex::new(Database::open_in_memory().unwrap()));
    let state = Arc::new(AppState {
        db: Arc::clone(&db),
        is_loopback: true,
        command_handler: Arc::new(|cmd, conn, ctx, notify, events| {
            let_timer_daemon::handle_command(cmd, conn, ctx, notify, events)
        }),
        timer_notify: None,
        events: None,
    });

    let app = build_router(state);

    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/medias")
        .header("content-type", "application/json")
        .body(Body::from(
            serde_json::json!({
                "name": "rest-image",
                "description": "via rest",
                "url": "https://example.com/via-rest.png",
                "media_type": "image"
            })
            .to_string(),
        ))
        .unwrap();

    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::CREATED);

    let body = res.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["name"], "rest-image");
    assert_eq!(json["url"], "https://example.com/via-rest.png");
}

#[test]
fn sec_1_validate_remote_exhaustive_policy_guard() {
    let safe_cmd = Command::Ping;
    assert_eq!(validate_remote(&safe_cmd), Ok(()));

    let unsafe_cmd = Command::CreateMedia {
        media: NewMedia {
            name: "test".into(),
            description: None,
            url: String::new(),
            media_type: MediaType::Music,
            content: None,
        },
        import_file: Some("/etc/shadow".into()),
    };
    assert!(matches!(
        validate_remote(&unsafe_cmd),
        Err(PolicyError::ForbiddenRemoteCommand(_))
    ));
}

#[tokio::test]
async fn sec_3a_cors_untrusted_origin_rejected() {
    let db = Arc::new(Mutex::new(Database::open_in_memory().unwrap()));
    let state = Arc::new(AppState {
        db: Arc::clone(&db),
        is_loopback: true,
        command_handler: Arc::new(|cmd, conn, ctx, notify, events| {
            let_timer_daemon::handle_command(cmd, conn, ctx, notify, events)
        }),
        timer_notify: None,
        events: None,
    });

    let app = build_router(state);

    let req = Request::builder()
        .method("OPTIONS")
        .uri("/api/v1/health")
        .header("origin", "https://evil.example")
        .header("access-control-request-method", "GET")
        .body(Body::empty())
        .unwrap();

    let res = app.oneshot(req).await.unwrap();
    assert!(
        res.headers().get("access-control-allow-origin").is_none(),
        "untrusted origin should not receive Access-Control-Allow-Origin header"
    );
}

#[tokio::test]
async fn sec_3a_cors_allowed_origin_succeeds() {
    unsafe {
        std::env::set_var(
            "LET_TIMER_CORS_ORIGINS",
            "https://app.example.com,http://localhost:3000",
        );
    }

    let db = Arc::new(Mutex::new(Database::open_in_memory().unwrap()));
    let state = Arc::new(AppState {
        db: Arc::clone(&db),
        is_loopback: true,
        command_handler: Arc::new(|cmd, conn, ctx, notify, events| {
            let_timer_daemon::handle_command(cmd, conn, ctx, notify, events)
        }),
        timer_notify: None,
        events: None,
    });

    let app = build_router(state);

    let req = Request::builder()
        .method("OPTIONS")
        .uri("/api/v1/health")
        .header("origin", "https://app.example.com")
        .header("access-control-request-method", "GET")
        .body(Body::empty())
        .unwrap();

    let res = app.oneshot(req).await.unwrap();
    assert_eq!(
        res.headers()
            .get("access-control-allow-origin")
            .and_then(|v| v.to_str().ok()),
        Some("https://app.example.com")
    );

    unsafe {
        std::env::remove_var("LET_TIMER_CORS_ORIGINS");
    }
}
