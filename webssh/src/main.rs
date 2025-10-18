use std::{net::TcpStream, sync::{Arc, atomic::{AtomicBool, Ordering}}, time::{Duration, Instant}};

use axum::{
    extract::{Path, Query, State, ws::{Message, WebSocket, WebSocketUpgrade}},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use base64::Engine as _;
use dashmap::DashMap;
use rand::{distributions::Alphanumeric, Rng};
use serde::{Deserialize, Serialize};
use ssh2::Session as SshSession;
use tokio::{sync::mpsc, task::JoinHandle, time};
use tracing::{error, info, warn};
use uuid::Uuid;

#[derive(Clone)]
struct AppState {
    sessions: Arc<DashMap<String, Arc<SessionEntry>>>,
    known_hosts: Arc<DashMap<String, String>>, // host:port -> fingerprint
}

#[derive(Debug, Serialize, Deserialize)]
struct CreateSessionReq {
    host: String,
    #[serde(default = "default_port")] 
    port: u16,
    username: String,
    #[serde(default)]
    password: String,
    #[serde(default = "default_cols")]
    cols: u32,
    #[serde(default = "default_rows")]
    rows: u32,
}

fn default_port() -> u16 { 22 }
fn default_cols() -> u32 { 120 }
fn default_rows() -> u32 { 32 }

#[derive(Debug, Serialize)]
struct HostKeyInfo {
    algorithm: String,
    fingerprint: String,
    trusted: bool,
    mismatch: bool,
}

#[derive(Debug, Serialize)]
struct CreateSessionResp {
    sessionId: String,
    token: String,
    hostKey: HostKeyInfo,
    pendingAccept: bool,
}

#[derive(Debug)]
struct SessionEntry {
    id: String,
    token: String,
    host: String,
    port: u16,
    username: String,
    created_at: Instant,
    last_activity: parking_lot::Mutex<Instant>,
    accepted: AtomicBool,
    hostkey_fp: String,
    pty_cols: u32,
    pty_rows: u32,
    tcp: parking_lot::Mutex<Option<TcpStream>>, // holds until ws attaches
    ssh: parking_lot::Mutex<Option<SshSession>>,
    bridge: parking_lot::Mutex<Option<Bridge>>, // filled when ws connects
}

struct Bridge {
    // Thread that owns channel + session nonblocking loop
    to_ssh_tx: std::sync::mpsc::Sender<ToSsh>,
    from_ssh_rx: Option<mpsc::Receiver<FromSsh>>, // handed to the single ws consumer
    handle: Option<JoinHandle<()>>,
}

enum ToSsh {
    Write(Vec<u8>),
    Resize(u32, u32),
    Close,
}

#[derive(Debug)]
enum FromSsh {
    Stdout(Vec<u8>),
    Stderr(Vec<u8>),
    ExitStatus(i32),
    Error(String),
}

#[tokio::main]
async fn main() {
    init_tracing();

    let state = AppState {
        sessions: Arc::new(DashMap::new()),
        known_hosts: Arc::new(DashMap::new()),
    };

    // Spawn cleanup task
    {
        let sessions = state.sessions.clone();
        tokio::spawn(async move {
            let mut ticker = time::interval(Duration::from_secs(30));
            let idle = Duration::from_secs(60 * 30);
            loop {
                ticker.tick().await;
                let now = Instant::now();
                let ids: Vec<String> = sessions
                    .iter()
                    .filter_map(|kv| {
                        let v = kv.value();
                        let la = v.last_activity.lock();
                        if now.duration_since(*la) > idle {
                            Some(v.id.clone())
                        } else {
                            None
                        }
                    })
                    .collect();
                for id in ids {
                    sessions.remove(&id);
                }
            }
        });
    }

    let app = Router::new()
        .route("/api/session", post(create_session))
        .route("/api/session/:id/accept-hostkey", post(accept_hostkey))
        .route("/api/ws", get(ws_handler))
        .with_state(state);

    let addr = std::env::var("WEBSSH_ADDR").unwrap_or_else(|_| "0.0.0.0:8080".into());
    info!(%addr, "starting webssh server");
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

fn init_tracing() {
    use tracing_subscriber::{fmt, EnvFilter};
    use tracing_subscriber::prelude::*;
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    tracing_subscriber::registry()
        .with(filter)
        .with(fmt::layer())
        .init();
}

fn compute_hostkey_fingerprint(sess: &SshSession) -> anyhow::Result<(String, String)> {
    use sha2::{Digest, Sha256};
    let (key, _typ) = sess.host_key().ok_or_else(|| anyhow::anyhow!("no host key"))?;
    let mut hasher = Sha256::new();
    hasher.update(key);
    let digest = hasher.finalize();
    let fp = base64::engine::general_purpose::STANDARD_NO_PAD.encode(digest);
    Ok(("SHA256".into(), fp))
}

#[derive(Debug, Deserialize)]
struct AcceptReq { token: String }

async fn accept_hostkey(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<AcceptReq>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let Some(entry) = state.sessions.get(&id).map(|e| e.clone()) else {
        return Err((StatusCode::NOT_FOUND, "session not found".into()));
    };
    if entry.token != req.token {
        return Err((StatusCode::UNAUTHORIZED, "invalid token".into()));
    }
    entry.accepted.store(true, Ordering::SeqCst);
    // Persist known host
    let key = format!("{}:{}", entry.host, entry.port);
    state.known_hosts.insert(key, entry.hostkey_fp.clone());
    Ok((StatusCode::NO_CONTENT, ()))
}

async fn create_session(
    State(state): State<AppState>,
    Json(req): Json<CreateSessionReq>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    if req.host.trim().is_empty() || req.username.trim().is_empty() {
        return Err((StatusCode::BAD_REQUEST, "host and username required".into()));
    }

    let host = req.host.clone();
    let port = req.port;
    let username = req.username.clone();
    let password = req.password.clone();
    let cols = req.cols;
    let rows = req.rows;

    let known_hosts = state.known_hosts.clone();

    // Perform blocking SSH connect + auth
    let connect_res = tokio::task::spawn_blocking(move || {
        let addr = format!("{}:{}", host, port);
        let tcp = TcpStream::connect(&addr)?;
        tcp.set_nodelay(true).ok();
        let mut sess = SshSession::new()?;
        sess.set_compress(true);
        sess.set_tcp_stream(tcp.try_clone()?);
        sess.handshake()?;
        sess.set_keepalive(true, 60);
        // Host key
        let (alg, fp) = compute_hostkey_fingerprint(&sess)?;
        // TOFU check
        let key = addr.clone();
        let known = known_hosts.get(&key).map(|e| e.clone());
        if let Some(existing) = known {
            if existing.value().as_str() != fp {
                return Ok::<_, anyhow::Error>((sess, tcp, alg, fp, true));
            }
        }
        // Auth with password (may be empty)
        sess.userauth_password(&username, &password)?;
        if !sess.authenticated() {
            return Err(anyhow::anyhow!("authentication failed"));
        }
        Ok::<_, anyhow::Error>((sess, tcp, alg, fp, false))
    }).await.map_err(internal_error)?;

    let (sess, tcp, alg, fp, mismatch) = match connect_res {
        Ok(v) => v,
        Err(e) => {
            error!(error = %e, "connection/auth error");
            return Err((StatusCode::BAD_GATEWAY, format!("ssh error: {}", e)));
        }
    };
    if mismatch {
        let body = serde_json::json!({
            "error": "host key mismatch",
            "hostKey": {
                "algorithm": alg,
                "fingerprint": fp,
                "trusted": true,
                "mismatch": true
            }
        }).to_string();
        return Err((StatusCode::CONFLICT, body));
    }

    let id = Uuid::new_v4().to_string();
    let token: String = rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(48)
        .map(char::from)
        .collect();

    let key = format!("{}:{}", req.host, req.port);
    let trusted = state.known_hosts.get(&key).is_some();
    let pending_accept = !trusted;
    let hostkey = HostKeyInfo {
        algorithm: alg,
        fingerprint: fp.clone(),
        trusted,
        mismatch,
    };

    // Store session entry
    let entry = Arc::new(SessionEntry {
        id: id.clone(),
        token: token.clone(),
        host: req.host,
        port: req.port,
        username: req.username,
        created_at: Instant::now(),
        last_activity: parking_lot::Mutex::new(Instant::now()),
        accepted: AtomicBool::new(trusted),
        hostkey_fp: fp,
        pty_cols: cols,
        pty_rows: rows,
        tcp: parking_lot::Mutex::new(Some(tcp)),
        ssh: parking_lot::Mutex::new(Some(sess)),
        bridge: parking_lot::Mutex::new(None),
    });

    state.sessions.insert(id.clone(), entry);

    let resp = CreateSessionResp {
        sessionId: id,
        token,
        hostKey: hostkey,
        pendingAccept: pending_accept,
    };

    Ok((StatusCode::OK, Json(resp)))
}

#[derive(Debug, Deserialize)]
struct WsQuery {
    sessionId: String,
    token: String,
}

async fn ws_handler(
    State(state): State<AppState>,
    Query(q): Query<WsQuery>,
    ws: WebSocketUpgrade,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let Some(entry) = state.sessions.get(&q.sessionId).map(|e| e.clone()) else {
        return Err((StatusCode::NOT_FOUND, "session not found".into()));
    };
    if entry.token != q.token {
        return Err((StatusCode::UNAUTHORIZED, "invalid token".into()));
    }
    if !entry.accepted.load(Ordering::SeqCst) {
        return Err((StatusCode::PRECONDITION_REQUIRED, "hostkey not yet accepted".into()));
    }

    Ok(ws
        .max_message_size(1024 * 1024 * 8)
        .max_frame_size(1024 * 1024 * 8)
        .on_upgrade(move |socket| handle_ws(socket, entry)))
}

async fn handle_ws(socket: WebSocket, entry: Arc<SessionEntry>) {
    // Setup bridge if not present
    if entry.bridge.lock().is_none() {
        if let Err(e) = setup_bridge(&entry) {
            error!(%e, "failed to setup ssh bridge");
            return;
        }
    }

    // take receiver for this ws
    let mut bridge_guard = entry.bridge.lock();
    let Some(bridge) = bridge_guard.as_mut() else { return; };
    let Some(mut from_ssh_rx) = bridge.from_ssh_rx.take() else {
        warn!("from_ssh_rx already taken; concurrent ws not supported");
        return;
    };
    let to_ssh_tx = bridge.to_ssh_tx.clone();
    drop(bridge_guard);

    let (mut ws_sender, mut ws_receiver) = socket.split();

    // WS heartbeat
    let mut ping_int = time::interval(Duration::from_secs(20));
    let mut last_pong = Instant::now();
    let mut last_activity = Instant::now();

    // Rate limiting for inbound stdin
    let mut tokens: i64 = 1024 * 1024; // 1MiB burst
    let rate_per_sec: i64 = 1024 * 1024; // 1MiB/s
    let mut last_refill = Instant::now();

    // Main loop: multiplex WS in, SSH out, and periodic tasks
    loop {
        tokio::select! {
            // SSH -> WS
            Some(msg) = from_ssh_rx.recv() => {
                match msg {
                    FromSsh::Stdout(data) => {
                        let payload = base64::engine::general_purpose::STANDARD.encode(&data);
                        if ws_sender.send(Message::Text(serde_json::json!({"type":"stdout","data": payload}).to_string())).await.is_err() {
                            break;
                        }
                        *entry.last_activity.lock() = Instant::now();
                    }
                    FromSsh::Stderr(data) => {
                        let payload = base64::engine::general_purpose::STANDARD.encode(&data);
                        if ws_sender.send(Message::Text(serde_json::json!({"type":"stderr","data": payload}).to_string())).await.is_err() {
                            break;
                        }
                        *entry.last_activity.lock() = Instant::now();
                    }
                    FromSsh::ExitStatus(code) => {
                        let _ = ws_sender.send(Message::Text(serde_json::json!({"type":"exit","code": code}).to_string())).await;
                        break;
                    }
                    FromSsh::Error(err) => {
                        let _ = ws_sender.send(Message::Text(serde_json::json!({"type":"error","message": err}).to_string())).await;
                        break;
                    }
                }
            }
            // WS ticker: ping + idle timeout + refill
            _ = ping_int.tick() => {
                // refill tokens
                let elapsed = last_refill.elapsed().as_secs_f64();
                let add = (elapsed * rate_per_sec as f64) as i64;
                tokens = (tokens + add).min(rate_per_sec * 2);
                last_refill = Instant::now();

                if ws_sender.send(Message::Ping(Vec::new())).await.is_err() {
                    break;
                }
                // idle timeout
                if last_activity.elapsed() > Duration::from_secs(60 * 10) {
                    let _ = ws_sender.send(Message::Close(None)).await;
                    break;
                }
            }
            // WS -> SSH
            Some(Ok(msg)) = ws_receiver.recv() => {
                match msg {
                    Message::Text(text) => {
                        last_activity = Instant::now();
                        *entry.last_activity.lock() = Instant::now();
                        if let Err(e) = handle_client_text(text, &to_ssh_tx, &mut tokens) {
                            warn!(%e, "handle_client_text failed");
                        }
                    }
                    Message::Binary(_) => {}
                    Message::Ping(p) => {
                        last_activity = Instant::now();
                        *entry.last_activity.lock() = Instant::now();
                        let _ = ws_sender.send(Message::Pong(p)).await;
                    }
                    Message::Pong(_) => {
                        last_pong = Instant::now();
                        *entry.last_activity.lock() = Instant::now();
                    }
                    Message::Close(_) => {
                        break;
                    }
                }
            }
            else => break,
        }
        if last_pong.elapsed() > Duration::from_secs(60) {
            break;
        }
    }

    // signal close to ssh thread
    let _ = to_ssh_tx.send(ToSsh::Close);
}

fn handle_client_text(text: String, to_ssh_tx: &std::sync::mpsc::Sender<ToSsh>, tokens: &mut i64) -> anyhow::Result<()> {
    #[derive(Deserialize)]
    struct Msg { r#type: String, data: Option<String>, cols: Option<u32>, rows: Option<u32> }
    let m: Msg = serde_json::from_str(&text)?;
    match m.r#type.as_str() {
        "stdin" => {
            if let Some(data) = m.data {
                let bytes = base64::engine::general_purpose::STANDARD.decode(data)?;
                if (*tokens as usize) < bytes.len() {
                    // drop silently to rate-limit
                    return Ok(());
                }
                *tokens -= bytes.len() as i64;
                let _ = to_ssh_tx.send(ToSsh::Write(bytes));
            }
        }
        "resize" => {
            if let (Some(c), Some(r)) = (m.cols, m.rows) {
                let _ = to_ssh_tx.send(ToSsh::Resize(c, r));
            }
        }
        "ping" => {
            // no-op; ws layer handles ping/pong
        }
        _ => {}
    }
    Ok(())
}

fn setup_bridge(entry: &Arc<SessionEntry>) -> anyhow::Result<()> {
    // create channels
    let (to_ssh_tx, to_ssh_rx) = std::sync::mpsc::channel::<ToSsh>();
    let (from_ssh_tx, from_ssh_rx) = mpsc::channel::<FromSsh>(64);

    // Take ownership of session and TCP
    let mut ssh_opt = entry.ssh.lock();
    let sess = ssh_opt.take().ok_or_else(|| anyhow::anyhow!("ssh session missing"))?;

    let tcp_opt = entry.tcp.lock().take();
    let tcp = tcp_opt.ok_or_else(|| anyhow::anyhow!("tcp missing"))?;

    // Spawn blocking thread for SSH IO
    let id = entry.id.clone();
    let host_clone = entry.host.clone();

    let init_cols = entry.pty_cols;
    let init_rows = entry.pty_rows;
    let handle = tokio::task::spawn_blocking(move || {
        if let Err(e) = ssh_io_thread(id, host_clone, sess, tcp, init_cols, init_rows, to_ssh_rx, from_ssh_tx) {
            error!(error = %e, "ssh io thread error");
        }
    });

    entry.bridge.lock().replace(Bridge { to_ssh_tx, from_ssh_rx: Some(from_ssh_rx), handle: Some(handle) });
    Ok(())
}

fn ssh_io_thread(
    session_id: String,
    host: String,
    mut sess: SshSession,
    _tcp: TcpStream,
    init_cols: u32,
    init_rows: u32,
    to_ssh_rx: std::sync::mpsc::Receiver<ToSsh>,
    from_ssh_tx: mpsc::Sender<FromSsh>,
) -> anyhow::Result<()> {
    // configure non-blocking
    sess.set_blocking(false);
    // keepalive
    sess.set_keepalive(true, 60);

    // open channel, request pty and shell
    let mut chan = sess.channel_session()?;
    chan.request_pty("xterm-256color", None, Some((init_cols, init_rows, 0, 0)))?;
    chan.shell()?;

    let mut last_keepalive = Instant::now();

    let mut buf = [0u8; 16 * 1024];
    let mut ebuf = [0u8; 8 * 1024];

    loop {
        // process input commands
        for _ in 0..8 {
            match to_ssh_rx.try_recv() {
                Ok(ToSsh::Write(mut data)) => {
                    let mut written = 0;
                    while written < data.len() {
                        match chan.write(&data[written..]) {
                            Ok(n) => written += n,
                            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                                std::thread::sleep(Duration::from_millis(2));
                            }
                            Err(e) => {
                                let _ = from_ssh_tx.blocking_send(FromSsh::Error(format!("write error: {}", e)));
                                return Ok(());
                            }
                        }
                    }
                }
                Ok(ToSsh::Resize(cols, rows)) => {
                    let _ = chan.request_pty_size(cols, rows, None, None);
                }
                Ok(ToSsh::Close) => {
                    let _ = chan.close();
                    let _ = chan.wait_close();
                    return Ok(());
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => break,
                Err(std::sync::mpsc::TryRecvError::Disconnected) => return Ok(()),
            }
        }

        // read stdout
        match chan.read(&mut buf) {
            Ok(0) => {
                if chan.eof() {
                    let code = chan.exit_status().unwrap_or_default();
                    let _ = from_ssh_tx.blocking_send(FromSsh::ExitStatus(code));
                    break;
                }
            }
            Ok(n) => {
                let _ = from_ssh_tx.try_send(FromSsh::Stdout(buf[..n].to_vec()));
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
            Err(e) => {
                let _ = from_ssh_tx.blocking_send(FromSsh::Error(format!("read stdout error: {}", e)));
                break;
            }
        }

        // read stderr
        match chan.stderr().read(&mut ebuf) {
            Ok(0) => {}
            Ok(n) => {
                let _ = from_ssh_tx.try_send(FromSsh::Stderr(ebuf[..n].to_vec()));
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
            Err(e) => {
                let _ = from_ssh_tx.blocking_send(FromSsh::Error(format!("read stderr error: {}", e)));
                break;
            }
        }

        // keepalive send every 30s
        if last_keepalive.elapsed() > Duration::from_secs(30) {
            let _ = sess.keepalive_send();
            last_keepalive = Instant::now();
        }

        std::thread::sleep(Duration::from_millis(5));
    }

    let _ = chan.close();
    let _ = chan.wait_close();
    Ok(())
}

fn internal_error<E: std::fmt::Display>(err: E) -> (StatusCode, String) {
    (StatusCode::INTERNAL_SERVER_ERROR, format!("internal error: {}", err))
}
