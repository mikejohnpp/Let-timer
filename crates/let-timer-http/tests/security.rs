use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use let_timer_core::{Command, Database, MediaType, NewMedia};
use let_timer_http::policy::{PolicyError, validate_remote};
use let_timer_http::{AppState, build_router};
use serde_json::Value;
use std::sync::{Arc, Mutex};
use tower::ServiceExt;

static ENV_MUTEX: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

#[tokio::test]
async fn sec_1_rpc_rejects_import_file_with_403_and_creates_no_file() {
    let _env_guard = ENV_MUTEX.lock().await;
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
        .header("x-let-timer-client", "cli")
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
        .header("x-let-timer-client", "cli")
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
        .header("x-let-timer-client", "cli")
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
    let _env_guard = ENV_MUTEX.lock().await;
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
    let _env_guard = ENV_MUTEX.lock().await;
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
    let _env_guard = ENV_MUTEX.lock().await;
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

#[tokio::test]
async fn sec_2_0_differential_test() {
    let _env_guard = ENV_MUTEX.lock().await;
    let temp_dir = std::env::temp_dir().join(format!(
        "let_timer_sec2_diff_{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let media_root = temp_dir.join("media");
    std::fs::create_dir_all(&media_root).unwrap();
    std::fs::write(media_root.join("sample.png"), b"fake png data").unwrap();

    unsafe {
        std::env::set_var("LET_TIMER_MEDIA_DIR", &media_root);
    }

    let db = Arc::new(Mutex::new(Database::open_in_memory().unwrap()));
    // Create at least one active API token in DB
    {
        let conn = db.lock().unwrap();
        let repo = let_timer_core::TokenRepository::new(conn.conn());
        let _ = repo.create("admin").unwrap();
    }

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

    // (a) Media file with no token
    let req_media = Request::builder()
        .method("GET")
        .uri("/api/v1/media/files/sample.png")
        .body(Body::empty())
        .unwrap();
    let res_media = app.clone().oneshot(req_media).await.unwrap();
    let status_media = res_media.status();

    // (b) /metrics with no token
    let req_metrics = Request::builder()
        .method("GET")
        .uri("/metrics")
        .body(Body::empty())
        .unwrap();
    let res_metrics = app.clone().oneshot(req_metrics).await.unwrap();
    let status_metrics = res_metrics.status();

    // (c) /swagger-ui/ with no token
    let req_swagger = Request::builder()
        .method("GET")
        .uri("/swagger-ui/")
        .body(Body::empty())
        .unwrap();
    let res_swagger = app.clone().oneshot(req_swagger).await.unwrap();
    let status_swagger = res_swagger.status();

    // (d) SPA route with no token
    let req_spa = Request::builder()
        .method("GET")
        .uri("/some/spa/route")
        .body(Body::empty())
        .unwrap();
    let res_spa = app.clone().oneshot(req_spa).await.unwrap();
    let status_spa = res_spa.status();

    println!("=== SEC-2.0 Differential Test Results ===");
    println!("(a) Media file status: {}", status_media);
    println!("(b) /metrics status: {}", status_metrics);
    println!("(c) /swagger-ui/ status: {}", status_swagger);
    println!("(d) SPA route status: {}", status_spa);

    unsafe {
        std::env::remove_var("LET_TIMER_MEDIA_DIR");
    }
    let _ = std::fs::remove_dir_all(temp_dir);
}

#[tokio::test]
async fn sec_2_unauthenticated_requests_rejected_when_token_exists() {
    let db = Arc::new(Mutex::new(Database::open_in_memory().unwrap()));
    {
        let conn = db.lock().unwrap();
        let repo = let_timer_core::TokenRepository::new(conn.conn());
        let _ = repo.create("admin").unwrap();
    }

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

    // 1. /api/v1/health is PUBLIC => 200 OK
    let req_health = Request::builder()
        .method("GET")
        .uri("/api/v1/health")
        .body(Body::empty())
        .unwrap();
    let res_health = app.clone().oneshot(req_health).await.unwrap();
    assert_eq!(res_health.status(), StatusCode::OK);

    // 2. Protected routes with no token => 401 UNAUTHORIZED
    for path in [
        "/api/v1/tasks",
        "/metrics",
        "/swagger-ui/",
        "/api/v1/openapi.json",
        "/api/v1/media/files/test.png",
    ] {
        let req = Request::builder()
            .method("GET")
            .uri(path)
            .body(Body::empty())
            .unwrap();
        let res = app.clone().oneshot(req).await.unwrap();
        assert_eq!(
            res.status(),
            StatusCode::UNAUTHORIZED,
            "Expected 401 for unauthenticated request to {path}"
        );
    }
}

#[tokio::test]
async fn sec_2_signed_media_url_access_and_range_headers() {
    let _env_guard = ENV_MUTEX.lock().await;
    let temp_dir = std::env::temp_dir().join(format!(
        "let_timer_sec2_media_{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&temp_dir).unwrap();
    let file_content = b"0123456789abcdefghijklmnopqrstuvwxyz";
    std::fs::write(temp_dir.join("track.mp3"), file_content).unwrap();

    unsafe {
        std::env::set_var("LET_TIMER_MEDIA_DIR", &temp_dir);
    }

    let db = Arc::new(Mutex::new(Database::open_in_memory().unwrap()));
    let secret = {
        let conn = db.lock().unwrap();
        let repo = let_timer_core::SettingsRepository::new(conn.conn());
        repo.get_or_create_media_signing_secret().unwrap()
    };
    {
        let conn = db.lock().unwrap();
        let token_repo = let_timer_core::TokenRepository::new(conn.conn());
        let _ = token_repo.create("admin").unwrap();
    }

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

    let now_unix = chrono::Utc::now().timestamp() as u64;
    let exp = now_unix + 3600;
    let sig = let_timer_http::media_auth::sign_media_path(&secret, "track.mp3", exp);

    // 1. Valid signature: 200 OK with security headers and no ACAO: *
    let req_valid = Request::builder()
        .method("GET")
        .uri(format!("/api/v1/media/files/track.mp3?exp={exp}&sig={sig}"))
        .body(Body::empty())
        .unwrap();
    let res_valid = app.clone().oneshot(req_valid).await.unwrap();
    assert_eq!(res_valid.status(), StatusCode::OK);

    let headers = res_valid.headers();
    assert_eq!(headers.get("x-content-type-options").unwrap(), "nosniff");
    assert_eq!(
        headers.get("content-security-policy").unwrap(),
        "sandbox; default-src 'none'"
    );
    assert_eq!(
        headers.get("cross-origin-resource-policy").unwrap(),
        "same-origin"
    );
    let cc = headers.get("cache-control").unwrap().to_str().unwrap();
    assert!(
        cc.starts_with("private, max-age="),
        "cache-control was: {cc}"
    );
    assert!(
        headers.get("access-control-allow-origin").is_none(),
        "ACAO: * must be omitted from media responses"
    );

    // 2. Range request: Range: bytes=0-9 with valid signature => 206 Partial Content
    let req_range = Request::builder()
        .method("GET")
        .uri(format!("/api/v1/media/files/track.mp3?exp={exp}&sig={sig}"))
        .header("range", "bytes=0-9")
        .body(Body::empty())
        .unwrap();
    let res_range = app.clone().oneshot(req_range).await.unwrap();
    assert_eq!(res_range.status(), StatusCode::PARTIAL_CONTENT);
    let range_body = res_range.into_body().collect().await.unwrap().to_bytes();
    assert_eq!(range_body.as_ref(), &file_content[0..10]);

    // 3. Expired signature => 401 UNAUTHORIZED
    let expired_exp = now_unix - 10;
    let expired_sig =
        let_timer_http::media_auth::sign_media_path(&secret, "track.mp3", expired_exp);
    let req_expired = Request::builder()
        .method("GET")
        .uri(format!(
            "/api/v1/media/files/track.mp3?exp={expired_exp}&sig={expired_sig}"
        ))
        .body(Body::empty())
        .unwrap();
    let res_expired = app.clone().oneshot(req_expired).await.unwrap();
    assert_eq!(res_expired.status(), StatusCode::UNAUTHORIZED);

    // 4. Tampered signature => 401 UNAUTHORIZED
    let req_tampered = Request::builder()
        .method("GET")
        .uri(format!(
            "/api/v1/media/files/track.mp3?exp={exp}&sig=0123456789abcdef"
        ))
        .body(Body::empty())
        .unwrap();
    let res_tampered = app.clone().oneshot(req_tampered).await.unwrap();
    assert_eq!(res_tampered.status(), StatusCode::UNAUTHORIZED);

    // 5. Tampered path => 401 UNAUTHORIZED
    let req_wrong_path = Request::builder()
        .method("GET")
        .uri(format!("/api/v1/media/files/other.mp3?exp={exp}&sig={sig}"))
        .body(Body::empty())
        .unwrap();
    let res_wrong_path = app.clone().oneshot(req_wrong_path).await.unwrap();
    assert_eq!(res_wrong_path.status(), StatusCode::UNAUTHORIZED);

    // 6. Tampered exp => 401 UNAUTHORIZED
    let req_wrong_exp = Request::builder()
        .method("GET")
        .uri(format!(
            "/api/v1/media/files/track.mp3?exp={}&sig={sig}",
            exp + 100
        ))
        .body(Body::empty())
        .unwrap();
    let res_wrong_exp = app.clone().oneshot(req_wrong_exp).await.unwrap();
    assert_eq!(res_wrong_exp.status(), StatusCode::UNAUTHORIZED);

    unsafe {
        std::env::remove_var("LET_TIMER_MEDIA_DIR");
    }
    let _ = std::fs::remove_dir_all(temp_dir);
}

#[tokio::test]
async fn sec_2_post_media_url_endpoint_generates_valid_signed_url() {
    let _env_guard = ENV_MUTEX.lock().await;
    let temp_dir = std::env::temp_dir().join(format!(
        "let_timer_sec2_post_url_{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&temp_dir).unwrap();
    std::fs::write(temp_dir.join("wallpaper.jpg"), b"fake jpeg bytes").unwrap();

    unsafe {
        std::env::set_var("LET_TIMER_MEDIA_DIR", &temp_dir);
    }

    let db = Arc::new(Mutex::new(Database::open_in_memory().unwrap()));
    let token = {
        let conn = db.lock().unwrap();
        let token_repo = let_timer_core::TokenRepository::new(conn.conn());
        token_repo.create("admin").unwrap()
    };
    let media_id = {
        let conn = db.lock().unwrap();
        let media_repo = let_timer_core::MediaRepository::new(conn.conn());
        let m = media_repo
            .create(&NewMedia {
                name: "wallpaper".into(),
                description: None,
                url: "/api/v1/media/files/wallpaper.jpg".into(),
                media_type: MediaType::Image,
                content: None,
            })
            .unwrap();
        m.id
    };

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

    // Call POST /api/v1/media/{id}/url with auth token
    let req_url = Request::builder()
        .method("POST")
        .uri(format!("/api/v1/media/{media_id}/url?ttl=1800"))
        .header("authorization", format!("Bearer {}", token.token))
        .body(Body::empty())
        .unwrap();
    let res_url = app.clone().oneshot(req_url).await.unwrap();
    assert_eq!(res_url.status(), StatusCode::OK);

    let body = res_url.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&body).unwrap();
    let signed_url = json["url"].as_str().unwrap();
    assert!(signed_url.contains("?exp="));
    assert!(signed_url.contains("&sig="));

    // Request the signed URL without any authorization headers
    let req_file = Request::builder()
        .method("GET")
        .uri(signed_url)
        .body(Body::empty())
        .unwrap();
    let res_file = app.clone().oneshot(req_file).await.unwrap();
    assert_eq!(res_file.status(), StatusCode::OK);

    unsafe {
        std::env::remove_var("LET_TIMER_MEDIA_DIR");
    }
    let _ = std::fs::remove_dir_all(temp_dir);
}

#[tokio::test]
async fn sec_2_route_enumeration_public_allowlist_coverage() {
    assert_eq!(
        let_timer_http::router::PUBLIC_ROUTES,
        &["/api/v1/health"],
        "Only /api/v1/health must be in the public allow-list"
    );

    let db = Arc::new(Mutex::new(Database::open_in_memory().unwrap()));
    {
        let conn = db.lock().unwrap();
        let token_repo = let_timer_core::TokenRepository::new(conn.conn());
        let _ = token_repo.create("admin").unwrap();
    }

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

    let routes = [
        ("GET", "/api/v1/health", true),
        ("GET", "/metrics", false),
        ("GET", "/swagger-ui/", false),
        ("GET", "/api/v1/openapi.json", false),
        ("POST", "/api/v1/rpc", false),
        ("GET", "/api/v1/events", false),
        ("GET", "/api/v1/auth/tokens", false),
        ("GET", "/api/v1/auth/me", false),
        ("GET", "/api/v1/tasks", false),
        ("GET", "/api/v1/tasks/current", false),
        ("GET", "/api/v1/tasks/start-candidates", false),
        ("GET", "/api/v1/timer", false),
        ("GET", "/api/v1/workspaces", false),
        ("GET", "/api/v1/medias", false),
        ("POST", "/api/v1/media/upload", false),
        ("POST", "/api/v1/media/1/url", false),
        ("GET", "/api/v1/media-lists", false),
        ("GET", "/api/v1/settings/timezone", false),
        ("GET", "/api/v1/media/files/unauthorized.png", false),
    ];

    for (method, path, is_public) in routes {
        let req = Request::builder()
            .method(method)
            .uri(path)
            .header("x-let-timer-protocol", "1")
            .body(Body::empty())
            .unwrap();

        let res = app.clone().oneshot(req).await.unwrap();
        if is_public {
            assert_eq!(
                res.status(),
                StatusCode::OK,
                "Expected 200 OK for public route {method} {path}"
            );
        } else {
            assert_eq!(
                res.status(),
                StatusCode::UNAUTHORIZED,
                "Expected 401 UNAUTHORIZED for protected route {method} {path}"
            );
        }
    }
}

#[tokio::test]
async fn sec_3b_host_header_validation_blocks_dns_rebinding() {
    let _env_guard = ENV_MUTEX.lock().await;

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

    // 1. Host: attacker.example => 421 Misdirected Request
    let req_attacker = Request::builder()
        .method("GET")
        .uri("/api/v1/tasks")
        .header("host", "attacker.example")
        .body(Body::empty())
        .unwrap();
    let res_attacker = app.clone().oneshot(req_attacker).await.unwrap();
    assert_eq!(res_attacker.status(), StatusCode::MISDIRECTED_REQUEST);

    // 2. Host: attacker.example:8080 => 421 Misdirected Request
    let req_attacker_port = Request::builder()
        .method("GET")
        .uri("/api/v1/tasks")
        .header("host", "attacker.example:8080")
        .body(Body::empty())
        .unwrap();
    let res_attacker_port = app.clone().oneshot(req_attacker_port).await.unwrap();
    assert_eq!(res_attacker_port.status(), StatusCode::MISDIRECTED_REQUEST);

    // 3. Loopback hosts: localhost, 127.0.0.1, [::1] => allowed past host check
    for valid_host in [
        "localhost:8080",
        "127.0.0.1:8080",
        "[::1]:8080",
        "localhost",
    ] {
        let req = Request::builder()
            .method("GET")
            .uri("/api/v1/tasks")
            .header("host", valid_host)
            .body(Body::empty())
            .unwrap();
        let res = app.clone().oneshot(req).await.unwrap();
        assert_ne!(
            res.status(),
            StatusCode::MISDIRECTED_REQUEST,
            "Host {valid_host} should be permitted"
        );
    }
}

#[tokio::test]
async fn sec_3b_foreign_origin_rejected_on_mutating_requests() {
    let _env_guard = ENV_MUTEX.lock().await;

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

    // POST with foreign Origin => 403 Forbidden
    let req_foreign = Request::builder()
        .method("POST")
        .uri("/api/v1/tasks")
        .header("host", "127.0.0.1:8080")
        .header("origin", "https://evil-attacker.example")
        .header("content-type", "application/json")
        .header("x-let-timer-client", "web")
        .body(Body::from(r#"{"name":"malicious"}"#))
        .unwrap();
    let res_foreign = app.clone().oneshot(req_foreign).await.unwrap();
    assert_eq!(res_foreign.status(), StatusCode::FORBIDDEN);

    // POST with same-origin Origin => allowed past origin check
    let req_same = Request::builder()
        .method("POST")
        .uri("/api/v1/tasks")
        .header("host", "127.0.0.1:8080")
        .header("origin", "http://127.0.0.1:8080")
        .header("content-type", "application/json")
        .header("x-let-timer-client", "web")
        .body(Body::from(r#"{"name":"legitimate"}"#))
        .unwrap();
    let res_same = app.clone().oneshot(req_same).await.unwrap();
    assert_ne!(
        res_same.status(),
        StatusCode::FORBIDDEN,
        "Same-origin request should not be rejected with 403"
    );
}

#[tokio::test]
async fn sec_3b_simple_request_csrf_content_type_rejected() {
    let _env_guard = ENV_MUTEX.lock().await;

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

    // Form POST with text/plain => 415 Unsupported Media Type
    let req_text = Request::builder()
        .method("POST")
        .uri("/api/v1/tasks")
        .header("host", "127.0.0.1:8080")
        .header("content-type", "text/plain")
        .header("x-let-timer-client", "web")
        .body(Body::from("name=csrf"))
        .unwrap();
    let res_text = app.clone().oneshot(req_text).await.unwrap();
    assert_eq!(res_text.status(), StatusCode::UNSUPPORTED_MEDIA_TYPE);

    // Form POST with application/x-www-form-urlencoded => 415
    let req_form = Request::builder()
        .method("POST")
        .uri("/api/v1/tasks")
        .header("host", "127.0.0.1:8080")
        .header("content-type", "application/x-www-form-urlencoded")
        .header("x-let-timer-client", "web")
        .body(Body::from("name=csrf"))
        .unwrap();
    let res_form = app.clone().oneshot(req_form).await.unwrap();
    assert_eq!(res_form.status(), StatusCode::UNSUPPORTED_MEDIA_TYPE);
}

#[tokio::test]
async fn sec_3b_mode_a_requires_client_header_on_mutating_requests() {
    let _env_guard = ENV_MUTEX.lock().await;

    let db = Arc::new(Mutex::new(Database::open_in_memory().unwrap()));
    // No tokens created in DB + is_loopback => Mode A
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

    // 1. Mutating POST without X-Let-Timer-Client in Mode A => 403 Forbidden
    let req_no_header = Request::builder()
        .method("POST")
        .uri("/api/v1/tasks")
        .header("host", "127.0.0.1:8080")
        .header("content-type", "application/json")
        .body(Body::from(r#"{"name":"test"}"#))
        .unwrap();
    let res_no_header = app.clone().oneshot(req_no_header).await.unwrap();
    assert_eq!(res_no_header.status(), StatusCode::FORBIDDEN);

    // 2. Mutating POST with X-Let-Timer-Client: web in Mode A => accepted
    let req_web = Request::builder()
        .method("POST")
        .uri("/api/v1/tasks")
        .header("host", "127.0.0.1:8080")
        .header("content-type", "application/json")
        .header("x-let-timer-client", "web")
        .body(Body::from(r#"{"name":"task-from-web"}"#))
        .unwrap();
    let res_web = app.clone().oneshot(req_web).await.unwrap();
    assert_eq!(res_web.status(), StatusCode::CREATED);

    // 3. Mutating POST with X-Let-Timer-Client: cli in Mode A => accepted
    let req_cli = Request::builder()
        .method("POST")
        .uri("/api/v1/tasks")
        .header("host", "127.0.0.1:8080")
        .header("content-type", "application/json")
        .header("x-let-timer-client", "cli")
        .body(Body::from(r#"{"name":"task-from-cli"}"#))
        .unwrap();
    let res_cli = app.clone().oneshot(req_cli).await.unwrap();
    assert_eq!(res_cli.status(), StatusCode::CREATED);
}

// ─── SEC-5: Upload and Media Serving Safety Tests ─────────────────────────────

fn make_multipart_body(
    boundary: &str,
    filename: &str,
    file_bytes: &[u8],
    media_type: &str,
) -> Vec<u8> {
    let mut body = Vec::new();
    // 1. file field
    body.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
    body.extend_from_slice(
        format!("Content-Disposition: form-data; name=\"file\"; filename=\"{filename}\"\r\n")
            .as_bytes(),
    );
    body.extend_from_slice(b"Content-Type: application/octet-stream\r\n\r\n");
    body.extend_from_slice(file_bytes);
    body.extend_from_slice(b"\r\n");

    // 2. media_type field
    body.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
    body.extend_from_slice(b"Content-Disposition: form-data; name=\"media_type\"\r\n\r\n");
    body.extend_from_slice(media_type.as_bytes());
    body.extend_from_slice(b"\r\n");

    // End boundary
    body.extend_from_slice(format!("--{boundary}--\r\n").as_bytes());
    body
}

#[tokio::test]
async fn sec_5_oversized_json_body_returns_413() {
    let _env_guard = ENV_MUTEX.lock().await;

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

    // Payload > 1 MiB (e.g. 1.2 MiB)
    let large_description = "A".repeat(1_200_000);
    let large_json = serde_json::json!({
        "name": "Oversized task",
        "description": large_description
    })
    .to_string();

    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/tasks")
        .header("host", "127.0.0.1:8080")
        .header("content-type", "application/json")
        .header("x-let-timer-client", "cli")
        .body(Body::from(large_json))
        .unwrap();

    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::PAYLOAD_TOO_LARGE);
}

#[tokio::test]
async fn sec_5_upload_rejects_disallowed_extensions() {
    let _env_guard = ENV_MUTEX.lock().await;

    let temp_dir = std::env::temp_dir().join(format!(
        "sec5_upload_disallowed_{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&temp_dir).unwrap();
    unsafe {
        std::env::set_var("LET_TIMER_MEDIA_DIR", &temp_dir);
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

    let disallowed_files = [
        (
            "evil.html",
            "image",
            b"<html><body>hello</body></html>".as_slice(),
        ),
        (
            "evil.svg",
            "image",
            b"<svg xmlns='http://www.w3.org/2000/svg'></svg>".as_slice(),
        ),
        ("evil.js", "poem", b"console.log('pwned');".as_slice()),
        (
            "evil.exe",
            "music",
            b"MZ\x90\x00\x03\x00\x00\x00".as_slice(),
        ),
    ];

    for (filename, media_type, content) in disallowed_files {
        let boundary = "boundary123";
        let body = make_multipart_body(boundary, filename, content, media_type);

        let req = Request::builder()
            .method("POST")
            .uri("/api/v1/media/upload")
            .header("host", "127.0.0.1:8080")
            .header(
                "content-type",
                format!("multipart/form-data; boundary={boundary}"),
            )
            .header("x-let-timer-client", "cli")
            .body(Body::from(body))
            .unwrap();

        let res = app.clone().oneshot(req).await.unwrap();
        assert_eq!(res.status(), StatusCode::UNSUPPORTED_MEDIA_TYPE);
    }

    // Verify nothing stored in media directory
    let entries: Vec<_> = std::fs::read_dir(&temp_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .collect();
    assert!(
        entries.is_empty(),
        "Expected media dir to be empty, but found {:?}",
        entries
    );

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[tokio::test]
async fn sec_5_upload_rejects_magic_bytes_mismatch() {
    let _env_guard = ENV_MUTEX.lock().await;

    let temp_dir = std::env::temp_dir().join(format!(
        "sec5_upload_magic_{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&temp_dir).unwrap();
    unsafe {
        std::env::set_var("LET_TIMER_MEDIA_DIR", &temp_dir);
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

    // Named .png but containing HTML
    let boundary = "boundary456";
    let body = make_multipart_body(
        boundary,
        "avatar.png",
        b"<!DOCTYPE html><html><script>alert(1)</script></html>",
        "image",
    );

    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/media/upload")
        .header("host", "127.0.0.1:8080")
        .header(
            "content-type",
            format!("multipart/form-data; boundary={boundary}"),
        )
        .header("x-let-timer-client", "cli")
        .body(Body::from(body))
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::UNSUPPORTED_MEDIA_TYPE);

    // Verify nothing stored
    let entries: Vec<_> = std::fs::read_dir(&temp_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .collect();
    assert!(entries.is_empty());

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[tokio::test]
async fn sec_5_upload_rejects_oversized_file() {
    let _env_guard = ENV_MUTEX.lock().await;

    let temp_dir = std::env::temp_dir().join(format!(
        "sec5_upload_oversized_{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&temp_dir).unwrap();
    unsafe {
        std::env::set_var("LET_TIMER_MEDIA_DIR", &temp_dir);
        std::env::set_var("LET_TIMER_MAX_UPLOAD_MB", "1");
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

    // Poem limit is 1MB. Send 1.2MB text.
    let poem_content = "Lines of a long poem.\n".repeat(60_000);
    let boundary = "boundary789";
    let body = make_multipart_body(boundary, "poem.txt", poem_content.as_bytes(), "poem");

    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/media/upload")
        .header("host", "127.0.0.1:8080")
        .header(
            "content-type",
            format!("multipart/form-data; boundary={boundary}"),
        )
        .header("x-let-timer-client", "cli")
        .body(Body::from(body))
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::PAYLOAD_TOO_LARGE);

    // Verify temp files cleaned up
    let entries: Vec<_> = std::fs::read_dir(&temp_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .collect();
    assert!(entries.is_empty());

    let _ = std::fs::remove_dir_all(&temp_dir);
    unsafe {
        std::env::remove_var("LET_TIMER_MAX_UPLOAD_MB");
    }
}

#[tokio::test]
async fn sec_5_upload_valid_files_succeeds_and_serves_security_headers() {
    let _env_guard = ENV_MUTEX.lock().await;

    let temp_dir = std::env::temp_dir().join(format!(
        "sec5_upload_valid_{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&temp_dir).unwrap();
    unsafe {
        std::env::set_var("LET_TIMER_MEDIA_DIR", &temp_dir);
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

    // 1. Valid PNG upload
    let valid_png_bytes = [
        0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, b'I', b'H', b'D',
        b'R',
    ];
    let boundary = "boundaryPng";
    let body = make_multipart_body(boundary, "picture.png", &valid_png_bytes, "image");

    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/media/upload")
        .header("host", "127.0.0.1:8080")
        .header(
            "content-type",
            format!("multipart/form-data; boundary={boundary}"),
        )
        .header("x-let-timer-client", "cli")
        .body(Body::from(body))
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::CREATED);

    let res_body = res.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&res_body).unwrap();
    let signed_url = json["url"].as_str().expect("url must exist");
    assert!(signed_url.starts_with("/api/v1/media/files/images/"));

    // 2. Fetch the uploaded media using signed URL and verify serving headers
    let req_serve = Request::builder()
        .method("GET")
        .uri(signed_url)
        .header("host", "127.0.0.1:8080")
        .body(Body::empty())
        .unwrap();

    let res_serve = app.clone().oneshot(req_serve).await.unwrap();
    assert_eq!(res_serve.status(), StatusCode::OK);

    let headers = res_serve.headers();
    assert_eq!(headers.get("x-content-type-options").unwrap(), "nosniff");
    assert_eq!(
        headers.get("content-security-policy").unwrap(),
        "sandbox; default-src 'none'"
    );
    assert_eq!(
        headers.get("cross-origin-resource-policy").unwrap(),
        "same-origin"
    );
    assert_eq!(headers.get("content-type").unwrap(), "image/png");
    assert!(headers.get("access-control-allow-origin").is_none());

    // 3. Valid MP3 upload
    let valid_mp3_bytes = b"ID3\x03\x00\x00\x00\x00\x00\x00AudioData...";
    let boundary_mp3 = "boundaryMp3";
    let body_mp3 = make_multipart_body(boundary_mp3, "track.mp3", valid_mp3_bytes, "music");

    let req_mp3 = Request::builder()
        .method("POST")
        .uri("/api/v1/media/upload")
        .header("host", "127.0.0.1:8080")
        .header(
            "content-type",
            format!("multipart/form-data; boundary={boundary_mp3}"),
        )
        .header("x-let-timer-client", "cli")
        .body(Body::from(body_mp3))
        .unwrap();

    let res_mp3 = app.clone().oneshot(req_mp3).await.unwrap();
    assert_eq!(res_mp3.status(), StatusCode::CREATED);
    let body_mp3_res = res_mp3.into_body().collect().await.unwrap().to_bytes();
    let json_mp3: Value = serde_json::from_slice(&body_mp3_res).unwrap();
    let mp3_url = json_mp3["url"].as_str().unwrap();

    let req_serve_mp3 = Request::builder()
        .method("GET")
        .uri(mp3_url)
        .header("host", "127.0.0.1:8080")
        .body(Body::empty())
        .unwrap();
    let res_serve_mp3 = app.clone().oneshot(req_serve_mp3).await.unwrap();
    assert_eq!(res_serve_mp3.status(), StatusCode::OK);
    assert_eq!(
        res_serve_mp3.headers().get("content-type").unwrap(),
        "audio/mpeg"
    );

    // 4. Valid Poem upload
    let valid_poem = b"Two roads diverged in a yellow wood,\nAnd sorry I could not travel both";
    let boundary_poem = "boundaryPoem";
    let body_poem = make_multipart_body(boundary_poem, "road.txt", valid_poem, "poem");

    let req_poem = Request::builder()
        .method("POST")
        .uri("/api/v1/media/upload")
        .header("host", "127.0.0.1:8080")
        .header(
            "content-type",
            format!("multipart/form-data; boundary={boundary_poem}"),
        )
        .header("x-let-timer-client", "cli")
        .body(Body::from(body_poem))
        .unwrap();

    let res_poem = app.clone().oneshot(req_poem).await.unwrap();
    assert_eq!(res_poem.status(), StatusCode::CREATED);
    let body_poem_res = res_poem.into_body().collect().await.unwrap().to_bytes();
    let json_poem: Value = serde_json::from_slice(&body_poem_res).unwrap();
    let poem_url = json_poem["url"].as_str().unwrap();

    let req_serve_poem = Request::builder()
        .method("GET")
        .uri(poem_url)
        .header("host", "127.0.0.1:8080")
        .body(Body::empty())
        .unwrap();
    let res_serve_poem = app.clone().oneshot(req_serve_poem).await.unwrap();
    assert_eq!(res_serve_poem.status(), StatusCode::OK);
    assert_eq!(
        res_serve_poem.headers().get("content-type").unwrap(),
        "text/plain; charset=utf-8"
    );

    let _ = std::fs::remove_dir_all(&temp_dir);
}
