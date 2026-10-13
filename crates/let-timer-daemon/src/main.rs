use std::sync::{Arc, Mutex};

use chrono::Utc;
use let_timer_core::{
    Command, CompositeNotifier, Database, EventBus, IpcServer, RequestContext, Response,
};
use let_timer_daemon::{Scheduler, handle_command};
use tokio::sync::Notify;

fn invoke_command(
    cmd: Command,
    db: &Arc<Mutex<Database>>,
    notify: &Arc<Notify>,
    events: &Arc<EventBus>,
) -> Response {
    let now = Utc::now();
    let db_guard = db.lock().unwrap_or_else(|e| e.into_inner());
    let ctx = RequestContext::from_settings(db_guard.conn(), now, None, None);
    handle_command(cmd, db_guard.conn(), &ctx, Some(notify), Some(events))
}

fn init_logging() {
    let filter = std::env::var("LET_TIMER_LOG")
        .or_else(|_| std::env::var("RUST_LOG"))
        .unwrap_or_else(|_| "info".to_string());
    let filter_layer = tracing_subscriber::EnvFilter::new(filter);

    let format_json = std::env::var("LET_TIMER_LOG_FORMAT")
        .map(|v| v.eq_ignore_ascii_case("json"))
        .unwrap_or(false);

    if format_json {
        let _ = tracing_subscriber::fmt()
            .with_env_filter(filter_layer)
            .json()
            .try_init();
    } else {
        let _ = tracing_subscriber::fmt()
            .with_env_filter(filter_layer)
            .try_init();
    }
}

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "let-timer-daemon", about = "Background daemon for Let-timer")]
struct Opts {
    /// Bind address for HTTP server (e.g. 127.0.0.1:8765 or 0.0.0.0:8765)
    #[arg(long, visible_alias = "listen", env = "LET_TIMER_LISTEN")]
    http: Option<String>,

    #[command(subcommand)]
    cmd: Option<DaemonSubcommand>,
}

#[derive(Subcommand)]
enum DaemonSubcommand {
    /// Token management commands
    Token {
        #[command(subcommand)]
        action: TokenAction,
    },
}

#[derive(Subcommand)]
enum TokenAction {
    /// Create a new API token
    Create { name: String },
    /// Rotate an existing API token
    Rotate { name: String },
    /// Revoke an API token
    Revoke { name: String },
    /// List all API tokens
    List,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let opts = Opts::parse();

    // 1. Handle standalone token subcommands without running the daemon server
    if let Some(DaemonSubcommand::Token { action }) = opts.cmd {
        let database = Database::open_default()?;
        let repo = let_timer_core::TokenRepository::new(database.conn());

        match action {
            TokenAction::Create { name } => {
                let created = repo.create(&name)?;
                println!("Created token '{}':\n{}", created.name, created.token);
            }
            TokenAction::Rotate { name } => {
                let created = repo.rotate(&name)?;
                println!("Rotated token '{}':\n{}", created.name, created.token);
            }
            TokenAction::Revoke { name } => {
                let revoked = repo.revoke(&name)?;
                if revoked {
                    println!("Revoked token '{}'", name);
                } else {
                    eprintln!("Token '{}' not found or already revoked", name);
                    std::process::exit(1);
                }
            }
            TokenAction::List => {
                let tokens = repo.list_all()?;
                if tokens.is_empty() {
                    println!("No tokens found.");
                } else {
                    for t in tokens {
                        let status = if t.revoked_at.is_some() {
                            "revoked"
                        } else {
                            "active"
                        };
                        println!("#{}: {} ({})", t.id, t.name, status);
                    }
                }
            }
        }
        return Ok(());
    }

    init_logging();
    let path = let_timer_core::ipc::socket_path();
    let path = path.to_string_lossy().into_owned();
    let database = Arc::new(Mutex::new(Database::open_default()?));

    // Bootstrap token initialization if env is present
    if let Ok(bootstrap_token) = std::env::var("LET_TIMER_BOOTSTRAP_TOKEN")
        && !bootstrap_token.trim().is_empty()
    {
        let db = database.lock().unwrap_or_else(|e| e.into_inner());
        let repo = let_timer_core::TokenRepository::new(db.conn());
        let _ = repo.create_with_raw_token("bootstrap", bootstrap_token.trim());
        tracing::info!("bootstrap token configured");
    }

    let notify_signal = Arc::new(Notify::new());
    let event_bus = Arc::new(EventBus::default());
    let notifier = Arc::new(CompositeNotifier::from_env_or_default(true));

    let scheduler = Scheduler::new(Arc::clone(&database), notifier, Arc::clone(&notify_signal))
        .with_events(Arc::clone(&event_bus));
    let scheduler_handle = tokio::spawn(async move {
        scheduler.run().await;
    });

    let db_clone = Arc::clone(&database);
    let notify_clone = Arc::clone(&notify_signal);
    let events_clone = Arc::clone(&event_bus);
    let server = IpcServer::new(&path, move |cmd| {
        invoke_command(cmd, &db_clone, &notify_clone, &events_clone)
    })
    .await?;

    // Optional HTTP server (from CLI flag --http/--listen or env LET_TIMER_LISTEN / LET_TIMER_HTTP / LET_TIMER_BIND)
    let bind_addr = opts
        .http
        .or_else(|| std::env::var("LET_TIMER_LISTEN").ok())
        .or_else(|| std::env::var("LET_TIMER_HTTP").ok())
        .or_else(|| std::env::var("LET_TIMER_BIND").ok());

    let http_handle = if let Some(bind_addr) = bind_addr {
        let is_loopback = bind_addr.starts_with("127.")
            || bind_addr.starts_with("localhost:")
            || bind_addr.starts_with("[::1]");
        let app_state = Arc::new(let_timer_http::AppState {
            db: Arc::clone(&database),
            is_loopback,
            command_handler: Arc::new(move |cmd, conn, ctx, notify, events| {
                let_timer_daemon::handle_command(cmd, conn, ctx, notify, events)
            }),
            timer_notify: Some(Arc::clone(&notify_signal)),
            events: Some(Arc::clone(&event_bus)),
        });

        let app = let_timer_http::build_router(app_state);
        tracing::info!("starting HTTP server on {}", bind_addr);
        let listener = tokio::net::TcpListener::bind(&bind_addr).await?;
        Some(tokio::spawn(async move {
            if let Err(e) = axum::serve(listener, app).await {
                tracing::error!("HTTP server error: {}", e);
            }
        }))
    } else {
        None
    };

    tokio::select! {
        res = server.run() => {
            if let Err(e) = res {
                tracing::error!("IPC server error: {}", e);
            }
        }
        _ = tokio::signal::ctrl_c() => {
            tracing::info!("received ctrl-c signal");
        }
    }

    if let Some(h) = http_handle {
        h.abort();
    }
    scheduler_handle.abort();
    tracing::info!("daemon exited cleanly");
    Ok(())
}
