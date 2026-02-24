use warp::Filter;
use std::sync::{Arc, Mutex};
use tokio::sync::{broadcast, mpsc};
use futures::{StreamExt, SinkExt};
use crate::zone_state::SharedZoneState;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum GameCommand {
    #[serde(rename = "move")]
    Move { d_x: f32, d_y: f32, d_z: f32, d_h: f32 },
    #[serde(rename = "target")]
    Target { id: u32 },
    #[serde(rename = "attack")]
    Attack { targetId: u32 },
    #[serde(rename = "teleport")]
    Teleport { x: f32, y: f32, z: f32 },
}

pub async fn start_web_server(state: SharedZoneState, tx: broadcast::Sender<String>, cmd_tx: mpsc::UnboundedSender<GameCommand>) {
    let state_filter = warp::any().map(move || state.clone());
    let tx_filter = warp::any().map(move || tx.clone());
    let cmd_tx_filter = warp::any().map(move || cmd_tx.clone());

    // Serve static files from "web" directory
    let static_files = warp::fs::dir("web");
    
    // WebSocket route
    let ws_route = warp::path("ws")
        .and(warp::ws())
        .and(state_filter)
        .and(tx_filter)
        .and(cmd_tx_filter)
        .map(|ws: warp::ws::Ws, state, tx, cmd_tx| {
            ws.on_upgrade(move |socket| handle_socket(socket, state, tx, cmd_tx))
        });

    let routes = static_files.or(ws_route);

    println!("🌐 Web UI available at http://localhost:3030");
    warp::serve(routes).run(([0, 0, 0, 0], 3030)).await;
}

async fn handle_socket(ws: warp::ws::WebSocket, state: SharedZoneState, tx: broadcast::Sender<String>, cmd_tx: mpsc::UnboundedSender<GameCommand>) {
    let (mut ws_tx, mut ws_rx) = ws.split();
    let mut rx = tx.subscribe();

    // Send initial state
    let initial_json = {
        let state_guard = state.lock().unwrap();
        serde_json::to_string(&*state_guard).ok()
    };

    if let Some(json) = initial_json {
        let _ = ws_tx.send(warp::ws::Message::text(json)).await;
    }

    // Forward broadcast messages AND periodically push full state
    let state_for_send = state.clone();
    let mut send_task = tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_millis(500));
        loop {
            tokio::select! {
                result = rx.recv() => {
                    match result {
                        Ok(msg) => {
                            if ws_tx.send(warp::ws::Message::text(msg)).await.is_err() {
                                break;
                            }
                        },
                        Err(_) => break,
                    }
                }
                _ = interval.tick() => {
                    // Periodically push full state so client stays in sync
                    let json = {
                        let guard = state_for_send.lock().unwrap();
                        let spawn_count = guard.spawns.len();
                        if spawn_count > 0 {
                            // println!("WebUI: Sending state with {} spawns", spawn_count);
                        }
                        serde_json::to_string(&*guard).ok()
                    };
                    if let Some(json) = json {
                        if ws_tx.send(warp::ws::Message::text(json)).await.is_err() {
                            break;
                        }
                    }
                }
            }
        }
    });

    // Handle incoming messages
    while let Some(result) = ws_rx.next().await {
        match result {
            Ok(msg) => {
                if msg.is_text() {
                    if let Ok(text) = msg.to_str() {
                        // Attempt to parse GameCommand
                        match serde_json::from_str::<GameCommand>(text) {
                            Ok(cmd) => {
                                println!("WebUI: parsed cmd: {:?}", cmd);
                                if let Err(e) = cmd_tx.send(cmd) {
                                    println!("WebUI: send error: {}", e);
                                }
                            },
                            Err(e) => {
                                println!("WebUI: parse error: {} for: {}", e, text);
                            }
                        }
                    }
                } else if msg.is_close() {
                    break;
                }
            }
            Err(_) => break,
        }
    }
    
    send_task.abort();
}
