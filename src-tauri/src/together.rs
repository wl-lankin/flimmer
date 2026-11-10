//! Watch together: one WebSocket to the relay (server/), held in Rust so the page never needs
//! network rights. Everything the relay sends goes to the page as a `together` event.

use std::sync::Mutex;

use futures_util::{SinkExt, StreamExt};
use tauri::{AppHandle, Emitter, State};
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::Message;

/// The relay; FLIMMER_TOGETHER_URL overrides it (local testing).
const DEFAULT_URL: &str = "ws://91.98.161.167/together";

#[derive(Default)]
pub struct Link(Mutex<Option<mpsc::UnboundedSender<String>>>);

fn url() -> String {
    std::env::var("FLIMMER_TOGETHER_URL").unwrap_or_else(|_| DEFAULT_URL.to_string())
}

/// Connects (closing any earlier link) and creates or joins a room.
#[tauri::command]
pub async fn together_open(
    app: AppHandle,
    link: State<'_, Link>,
    action: String,
    room: String,
    name: String,
) -> Result<(), String> {
    let first = match action.as_str() {
        "create" => serde_json::json!({ "t": "create", "name": name }),
        "join" => serde_json::json!({ "t": "join", "room": room, "name": name }),
        _ => return Err("unknown action".into()),
    };
    let (socket, _) = tokio_tungstenite::connect_async(url())
        .await
        .map_err(|e| e.to_string())?;
    let (mut sink, mut stream) = socket.split();
    let (tx, mut rx) = mpsc::unbounded_channel::<String>();
    tx.send(first.to_string()).map_err(|e| e.to_string())?;
    // Replacing the sender drops the old one, which ends the old link's writer.
    *link.0.lock().unwrap() = Some(tx);

    tauri::async_runtime::spawn(async move {
        while let Some(text) = rx.recv().await {
            if sink.send(Message::Text(text.into())).await.is_err() {
                break;
            }
        }
        let _ = sink.close().await;
    });
    tauri::async_runtime::spawn(async move {
        while let Some(Ok(msg)) = stream.next().await {
            if let Message::Text(text) = msg {
                if let Ok(value) = serde_json::from_str::<serde_json::Value>(&text) {
                    let _ = app.emit("together", value);
                }
            }
        }
        let _ = app.emit("together", serde_json::json!({ "t": "closed" }));
    });
    Ok(())
}

/// Sends one protocol message (`state`, `ping`) to the relay.
#[tauri::command]
pub fn together_send(link: State<'_, Link>, msg: serde_json::Value) -> Result<(), String> {
    let kind = msg.get("t").and_then(|t| t.as_str()).unwrap_or_default();
    if !matches!(kind, "state" | "ping") {
        return Err("only state and ping go through here".into());
    }
    let guard = link.0.lock().unwrap();
    let tx = guard.as_ref().ok_or("not connected")?;
    tx.send(msg.to_string()).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn together_close(link: State<'_, Link>) {
    if let Some(tx) = link.0.lock().unwrap().take() {
        let _ = tx.send(serde_json::json!({ "t": "leave" }).to_string());
    }
}
