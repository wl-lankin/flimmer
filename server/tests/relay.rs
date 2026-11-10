use futures_util::{SinkExt, StreamExt};
use serde_json::{json, Value};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::oneshot;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{connect_async, MaybeTlsStream, WebSocketStream};

type Client = WebSocketStream<MaybeTlsStream<TcpStream>>;

async fn start() -> (String, oneshot::Sender<()>, tokio::task::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let (tx, rx) = oneshot::channel();
    let handle = tokio::spawn(flimmer_together::run(listener, async {
        let _ = rx.await;
    }));
    (format!("ws://{addr}"), tx, handle)
}

async fn connect(base: &str) -> Client {
    connect_async(format!("{base}/together")).await.unwrap().0
}

async fn send(c: &mut Client, v: Value) {
    c.send(Message::text(v.to_string())).await.unwrap();
}

async fn recv(c: &mut Client) -> Value {
    let msg = tokio::time::timeout(std::time::Duration::from_secs(5), c.next())
        .await
        .expect("timed out")
        .expect("closed")
        .expect("ws error");
    serde_json::from_str(msg.to_text().unwrap()).unwrap()
}

#[tokio::test]
async fn create_join_relay_leave() {
    let (base, stop, handle) = start().await;
    let mut a = connect(&base).await;
    let mut b = connect(&base).await;

    send(&mut a, json!({"t": "create", "name": "  Anna "})).await;
    let created = recv(&mut a).await;
    assert_eq!(created["t"], "created");
    let room = created["room"].as_str().unwrap().to_string();
    assert_eq!(room.len(), 6);
    let a_id = created["you"].as_str().unwrap().to_string();
    assert_eq!(recv(&mut a).await["t"], "peers");

    send(
        &mut b,
        json!({"t": "join", "room": room.to_lowercase(), "name": "Ben"}),
    )
    .await;
    let joined = recv(&mut b).await;
    assert_eq!(joined["t"], "joined");
    assert_eq!(joined["room"], room);
    let peers_b = recv(&mut b).await;
    assert_eq!(peers_b["peers"].as_array().unwrap().len(), 2);
    let peers_a = recv(&mut a).await;
    assert_eq!(peers_a["peers"].as_array().unwrap().len(), 2);

    send(
        &mut a,
        json!({"t": "state", "paused": false, "time": 12.5, "speed": 1.0, "file": "x.mkv", "reason": "play"}),
    )
    .await;
    let st = recv(&mut b).await;
    assert_eq!(st["t"], "state");
    assert_eq!(st["time"], 12.5);
    assert_eq!(st["reason"], "play");
    assert_eq!(st["from"], a_id);
    assert_eq!(st["name"], "Anna");
    assert!(st["at"].as_u64().unwrap() > 0);

    send(&mut b, json!({"t": "ping"})).await;
    assert_eq!(recv(&mut b).await["t"], "pong");

    send(&mut b, json!({"t": "nonsense"})).await;
    let err = recv(&mut b).await;
    assert_eq!(err["code"], "bad-request");

    send(&mut b, json!({"t": "leave"})).await;
    let peers = recv(&mut a).await;
    assert_eq!(peers["t"], "peers");
    assert_eq!(peers["peers"].as_array().unwrap().len(), 1);

    send(&mut b, json!({"t": "join", "room": "ZZZZZZ"})).await;
    assert_eq!(recv(&mut b).await["code"], "room-not-found");

    let _ = stop.send(());
    handle.await.unwrap();
}

#[tokio::test]
async fn disconnect_updates_peers_and_wrong_path_is_404() {
    let (base, stop, handle) = start().await;
    assert!(connect_async(format!("{base}/other")).await.is_err());

    let mut a = connect(&base).await;
    let mut b = connect(&base).await;
    send(&mut a, json!({"t": "create", "name": "A"})).await;
    let room = recv(&mut a).await["room"].as_str().unwrap().to_string();
    recv(&mut a).await;
    send(&mut b, json!({"t": "join", "room": room, "name": "B"})).await;
    recv(&mut b).await;
    recv(&mut b).await;
    recv(&mut a).await;

    drop(b);
    let peers = recv(&mut a).await;
    assert_eq!(peers["peers"].as_array().unwrap().len(), 1);

    let _ = stop.send(());
    handle.await.unwrap();
}

#[tokio::test]
async fn oversized_frame_closes_connection() {
    let (base, stop, handle) = start().await;
    let mut a = connect(&base).await;
    a.send(Message::text("x".repeat(4096))).await.unwrap();
    let end = tokio::time::timeout(std::time::Duration::from_secs(5), async {
        loop {
            match a.next().await {
                None | Some(Err(_)) | Some(Ok(Message::Close(_))) => break,
                Some(Ok(_)) => {}
            }
        }
    })
    .await;
    assert!(end.is_ok());
    let _ = stop.send(());
    handle.await.unwrap();
}
