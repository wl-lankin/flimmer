//! Relay server for Flimmer "watch together": rooms, membership and state fan-out.

pub mod rooms;

use std::collections::HashMap;
use std::future::Future;
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;
use serde_json::{json, Value};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{mpsc, watch};
use tokio::task::JoinSet;
use tokio::time::sleep_until;
use tokio_tungstenite::tungstenite::handshake::server::{ErrorResponse, Request, Response};
use tokio_tungstenite::tungstenite::http::StatusCode;
use tokio_tungstenite::tungstenite::protocol::WebSocketConfig;
use tokio_tungstenite::tungstenite::Message;

use rooms::{ConnId, Hub, JoinError, RateLimiter};

pub const WS_PATH: &str = "/together";
pub const MAX_MESSAGE_BYTES: usize = 2048;
pub const IDLE_TIMEOUT: Duration = Duration::from_secs(90);
pub const RATE_LIMIT: u32 = 30;
const OUTBOX_CAPACITY: usize = 64;
const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(10);

struct Shared {
    hub: Hub,
    senders: HashMap<ConnId, mpsc::Sender<Message>>,
}

type SharedState = Arc<Mutex<Shared>>;

fn lock(state: &SharedState) -> MutexGuard<'_, Shared> {
    state.lock().unwrap_or_else(|e| e.into_inner())
}

pub fn unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// UTC timestamp like 2026-11-10T12:34:56Z, without pulling in a date crate.
fn timestamp() -> String {
    let secs = (unix_ms() / 1000) as i64;
    let (days, rem) = (secs.div_euclid(86_400), secs.rem_euclid(86_400));
    // Howard Hinnant's civil-from-days algorithm.
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!(
        "{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}Z",
        rem / 3600,
        rem % 3600 / 60,
        rem % 60
    )
}

fn log(msg: &str) {
    eprintln!("{} {msg}", timestamp());
}

#[derive(Deserialize)]
#[serde(tag = "t", rename_all = "lowercase")]
enum Incoming {
    Create {
        #[serde(default)]
        name: String,
    },
    Join {
        room: String,
        #[serde(default)]
        name: String,
    },
    State {
        paused: bool,
        time: f64,
        speed: f64,
        #[serde(default)]
        file: String,
        reason: Reason,
    },
    Leave,
    Ping,
}

#[derive(Deserialize, Clone, Copy)]
#[serde(rename_all = "lowercase")]
enum Reason {
    Play,
    Pause,
    Seek,
    Speed,
    Tick,
    Load,
}

impl Reason {
    fn as_str(self) -> &'static str {
        match self {
            Reason::Play => "play",
            Reason::Pause => "pause",
            Reason::Seek => "seek",
            Reason::Speed => "speed",
            Reason::Tick => "tick",
            Reason::Load => "load",
        }
    }
}

fn to_message(value: Value) -> Message {
    Message::text(value.to_string())
}

fn error_msg(code: &str) -> Message {
    to_message(json!({"t": "error", "code": code}))
}

/// Queue a message for one connection; a connection whose outbox is full is too slow and loses it.
fn send_to(s: &Shared, conn: ConnId, msg: Message) {
    if let Some(tx) = s.senders.get(&conn) {
        let _ = tx.try_send(msg);
    }
}

fn broadcast_peers(s: &Shared, room: &str) {
    let peers: Vec<Value> = s
        .hub
        .members(room)
        .iter()
        .map(|m| json!({"id": m.id.to_string(), "name": m.name}))
        .collect();
    let msg = to_message(json!({"t": "peers", "peers": peers}));
    for id in s.hub.member_ids(room) {
        send_to(s, id, msg.clone());
    }
}

fn after_departure(s: &Shared, departure: Option<rooms::Departure>) {
    if let Some(dep) = departure {
        if dep.deleted {
            log(&format!("room {} deleted", dep.room));
        } else {
            broadcast_peers(s, &dep.room);
        }
    }
}

fn leave_room(s: &mut Shared, conn: ConnId) {
    let departure = s.hub.leave(conn);
    after_departure(s, departure);
}

fn handle_text(state: &SharedState, conn: ConnId, text: &str) {
    let mut guard = lock(state);
    let s = &mut *guard;
    let Ok(msg) = serde_json::from_str::<Incoming>(text) else {
        send_to(s, conn, error_msg("bad-request"));
        return;
    };
    match msg {
        Incoming::Create { name } => {
            let (code, departure) = s.hub.create(conn, &name, &mut rand::thread_rng());
            after_departure(s, departure);
            log(&format!("room {code} created"));
            send_to(
                s,
                conn,
                to_message(json!({"t": "created", "room": code, "you": conn.to_string()})),
            );
            broadcast_peers(s, &code);
        }
        Incoming::Join { room, name } => match s.hub.join(conn, &room, &name) {
            Ok((code, departure)) => {
                after_departure(s, departure);
                send_to(
                    s,
                    conn,
                    to_message(json!({"t": "joined", "room": code, "you": conn.to_string()})),
                );
                broadcast_peers(s, &code);
            }
            Err(JoinError::NotFound) => send_to(s, conn, error_msg("room-not-found")),
            Err(JoinError::Full) => send_to(s, conn, error_msg("room-full")),
        },
        Incoming::State {
            paused,
            time,
            speed,
            file,
            reason,
        } => {
            if !time.is_finite() || !speed.is_finite() {
                send_to(s, conn, error_msg("bad-request"));
                return;
            }
            let others = s.hub.others(conn);
            if others.is_empty() {
                return;
            }
            let name = s.hub.name_of(conn).unwrap_or_default().to_string();
            let out = to_message(json!({
                "t": "state", "paused": paused, "time": time, "speed": speed,
                "file": file, "reason": reason.as_str(),
                "from": conn.to_string(), "name": name, "at": unix_ms(),
            }));
            for id in others {
                send_to(s, id, out.clone());
            }
        }
        Incoming::Leave => leave_room(s, conn),
        Incoming::Ping => send_to(s, conn, to_message(json!({"t": "pong", "at": unix_ms()}))),
    }
}

fn ws_config() -> WebSocketConfig {
    WebSocketConfig::default()
        .max_message_size(Some(MAX_MESSAGE_BYTES))
        .max_frame_size(Some(MAX_MESSAGE_BYTES))
}

// Signature is fixed by the tungstenite handshake callback trait.
#[allow(clippy::result_large_err)]
fn check_path(req: &Request, resp: Response) -> Result<Response, ErrorResponse> {
    if req.uri().path() == WS_PATH {
        Ok(resp)
    } else {
        let mut not_found = ErrorResponse::new(Some("not found".to_string()));
        *not_found.status_mut() = StatusCode::NOT_FOUND;
        Err(not_found)
    }
}

async fn handle_connection(
    stream: TcpStream,
    conn: ConnId,
    state: SharedState,
    mut shutdown: watch::Receiver<bool>,
) {
    let accepted = tokio::time::timeout(
        HANDSHAKE_TIMEOUT,
        tokio_tungstenite::accept_hdr_async_with_config(stream, check_path, Some(ws_config())),
    )
    .await;
    let Ok(Ok(ws)) = accepted else {
        return;
    };
    log(&format!("conn {conn} connected"));

    let (tx, mut rx) = mpsc::channel::<Message>(OUTBOX_CAPACITY);
    lock(&state).senders.insert(conn, tx);
    let (mut sink, mut stream) = ws.split();
    let mut limiter = RateLimiter::new(RATE_LIMIT, Duration::from_secs(1));
    let mut deadline = tokio::time::Instant::now() + IDLE_TIMEOUT;

    loop {
        tokio::select! {
            incoming = stream.next() => {
                let Some(Ok(frame)) = incoming else { break };
                deadline = tokio::time::Instant::now() + IDLE_TIMEOUT;
                match frame {
                    Message::Text(text) if limiter.allow(Instant::now()) => {
                        handle_text(&state, conn, text.as_str());
                    }
                    Message::Ping(payload) => {
                        if sink.send(Message::Pong(payload)).await.is_err() {
                            break;
                        }
                    }
                    Message::Close(_) => break,
                    Message::Binary(_) if limiter.allow(Instant::now()) => {
                        let _ = sink.send(error_msg("bad-request")).await;
                    }
                    _ => {}
                }
            }
            outgoing = rx.recv() => {
                let Some(msg) = outgoing else { break };
                if sink.send(msg).await.is_err() {
                    break;
                }
            }
            _ = sleep_until(deadline) => {
                log(&format!("conn {conn} idle timeout"));
                break;
            }
            _ = shutdown.changed() => {
                let _ = sink.send(Message::Close(None)).await;
                break;
            }
        }
    }

    {
        let mut guard = lock(&state);
        guard.senders.remove(&conn);
        leave_room(&mut guard, conn);
    }
    log(&format!("conn {conn} disconnected"));
}

/// Accepts connections until `shutdown` resolves, then closes all connections.
pub async fn run<F: Future<Output = ()>>(listener: TcpListener, shutdown: F) {
    let state: SharedState = Arc::new(Mutex::new(Shared {
        hub: Hub::new(),
        senders: HashMap::new(),
    }));
    let (stop_tx, stop_rx) = watch::channel(false);
    let mut tasks = JoinSet::new();
    let mut next_id: ConnId = 1;
    tokio::pin!(shutdown);

    loop {
        tokio::select! {
            accepted = listener.accept() => match accepted {
                Ok((stream, _)) => {
                    let conn = next_id;
                    next_id += 1;
                    tasks.spawn(handle_connection(stream, conn, state.clone(), stop_rx.clone()));
                }
                Err(e) => {
                    log(&format!("accept failed: {e}"));
                    tokio::time::sleep(Duration::from_millis(100)).await;
                }
            },
            Some(_) = tasks.join_next(), if !tasks.is_empty() => {}
            _ = &mut shutdown => break,
        }
    }

    log("shutting down");
    let _ = stop_tx.send(true);
    let drain = async { while tasks.join_next().await.is_some() {} };
    let _ = tokio::time::timeout(Duration::from_secs(3), drain).await;
}
