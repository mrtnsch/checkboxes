use std::sync::atomic::Ordering;

use futures::{FutureExt, StreamExt};
use serde_json::to_string;
use tokio::sync::mpsc;
use tokio_stream::wrappers::UnboundedReceiverStream;
use warp::ws::{Message, WebSocket};

use crate::{Clients, NEXT_CLIENT_ID};
use crate::model::Client;
use crate::redis_handler::RedisHandler;

pub async fn broadcast_message(message: String, sender_id: usize, clients: &Clients) {
    for (&uid, client) in clients.read().await.iter() {
        if uid != sender_id {
            if let Err(_disconnected) = client.sender.send(Ok(Message::text(message.clone()))) {
                // The tx is disconnected, our client disconnected, so don't try to send
            }
        }
    }
}

pub async fn handle_connection(ws: WebSocket, clients: Clients, redis: RedisHandler) {
    let client_id = NEXT_CLIENT_ID.fetch_add(1, Ordering::Relaxed);
    let (client_ws_sender, mut client_ws_rcv) = ws.split();
    let (client_sender, client_rcv) = mpsc::unbounded_channel();

    let client_rcv = UnboundedReceiverStream::new(client_rcv);

    tokio::task::spawn(client_rcv.forward(client_ws_sender).map(|result| {
        if let Err(e) = result {
            eprintln!("error sending websocket msg: {}", e);
        }
    }));

    clients.write().await.insert(
        client_id,
        Client {
            sender: client_sender,
        },
    );

    println!("New client connected: {}", client_id);

    // Send initial state to the new client
    send_initial_state(client_id, &clients, &redis).await;

    while let Some(result) = client_ws_rcv.next().await {
        let msg = match result {
            Ok(msg) => msg,
            Err(e) => {
                eprintln!("error receiving ws message for id: {}): {}", client_id, e);
                break;
            }
        };
        handle_message(client_id, msg, &clients, &redis).await;
    }

    deregister_client(clients, &client_id).await;
}

async fn deregister_client(clients: Clients, client_id: &usize) {
    clients.write().await.remove(&client_id);
    println!("Client disconnected: {}", client_id);
}

async fn send_initial_state(client_id: usize, clients: &Clients, redis: &RedisHandler) {
    let checkbox_state = redis.get_initial_state().await.unwrap();
    if let Some(client) = clients.read().await.get(&client_id) {
        let _ = client.sender.send(Ok(Message::text(to_string(&checkbox_state).unwrap())));
    }
}

async fn handle_message(client_id: usize, msg: Message, clients: &Clients, redis: &RedisHandler) {
    let message = match msg.to_str() {
        Ok(v) => v,
        Err(_) => return,
    };
    println!("received message: {}", message);

    if message.starts_with("checkbox:") {
        let checkbox_info = message.trim_start_matches("checkbox:");
        let mut parts = checkbox_info.split(':');
        if let (Some(checkbox_id), Some(state)) = (parts.next(), parts.next()) {
            redis.update_checkbox(checkbox_id.parse().unwrap(), state).await.expect("TODO: panic message");
            broadcast_message(format!("Checkbox updated: {}", checkbox_info), client_id, clients).await;
        }
    }
}