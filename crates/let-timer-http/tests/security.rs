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

#[tokio::test]
async fn sec_4_web_dir_traversal_and_symlinks() {
    let temp_dir = std::env::temp_dir().join(format!(
        "let_timer_sec4_web_{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let web_root = temp_dir.join("web");
    let outside = temp_dir.join("outside");
    std::fs::create_dir_all(&web_root).unwrap();
    std::fs::create_dir_all(&outside).unwrap();

    std::fs::write(web_root.join("index.html"), b"<html>SPA Shell</html>").unwrap();
    std::fs::write(web_root.join("style.css"), b"body { color: red; }").unwrap();

    let secret = outside.join("secret.txt");
    std::fs::write(&secret, b"super secret file content").unwrap();

    let symlink_path = web_root.join("leak_link.txt");
    #[cfg(unix)]
    std::os::unix::fs::symlink(&secret, &symlink_path).unwrap();

    unsafe {
        std::env::set_var("LET_TIMER_WEB_DIR", &web_root);
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

    // 1. Normal file request works
    let req = Request::builder()
        .method("GET")
        .uri("/style.css")
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = res.into_body().collect().await.unwrap().to_bytes();
    assert_eq!(&body[..], b"body { color: red; }");

    // 2. Traversal attempt with literal `..` is rejected (404)
    let req = Request::builder()
        .method("GET")
        .uri("/../outside/secret.txt")
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);

    // 3. Encoded traversal sequence is rejected (404)
    let req = Request::builder()
        .method("GET")
        .uri("/%2e%2e/%2e%2e/etc/passwd")
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);

    let req = Request::builder()
        .method("GET")
        .uri("/..%2fsecret")
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);

    // 4. Symlink pointing outside is rejected (404)
    let req = Request::builder()
        .method("GET")
        .uri("/leak_link.txt")
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);

    // 5. Normal client SPA route falls back to index.html
    let req = Request::builder()
        .method("GET")
        .uri("/settings")
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = res.into_body().collect().await.unwrap().to_bytes();
    assert_eq!(&body[..], b"<html>SPA Shell</html>");

    unsafe {
        std::env::remove_var("LET_TIMER_WEB_DIR");
    }
    let _ = std::fs::remove_dir_all(temp_dir);
}

#[tokio::test]
async fn sec_4_media_symlink_outside_rejected() {
    let temp_dir = std::env::temp_dir().join(format!(
        "let_timer_sec4_media_{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let media_root = temp_dir.join("media");
    let outside = temp_dir.join("outside");
    std::fs::create_dir_all(&media_root).unwrap();
    std::fs::create_dir_all(&outside).unwrap();

    let secret = outside.join("media_secret.mp3");
    std::fs::write(&secret, b"secret audio").unwrap();

    let symlink_path = media_root.join("leak_audio.mp3");
    #[cfg(unix)]
    std::os::unix::fs::symlink(&secret, &symlink_path).unwrap();

    unsafe {
        std::env::set_var("LET_TIMER_MEDIA_DIR", &media_root);
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
        .method("GET")
        .uri("/api/v1/media/files/leak_audio.mp3")
        .body(Body::empty())
        .unwrap();
    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);

    unsafe {
        std::env::remove_var("LET_TIMER_MEDIA_DIR");
    }
    let _ = std::fs::remove_dir_all(temp_dir);
}

#[tokio::test]
async fn sec_4_raw_socket_traversal_request_rejected() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

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
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let server_task = tokio::spawn(async move {
        let _ = axum::serve(listener, app).await;
    });

    let mut stream = tokio::net::TcpStream::connect(addr).await.unwrap();
    stream
        .write_all(b"GET /../x HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n")
        .await
        .unwrap();

    let mut response = Vec::new();
    stream.read_to_end(&mut response).await.unwrap();
    let resp_str = String::from_utf8_lossy(&response);

    assert!(
        resp_str.contains("404 Not Found") || resp_str.contains("400 Bad Request"),
        "Raw traversal request must be rejected with 404 or 400, got: {}",
        resp_str
    );

    server_task.abort();
}
