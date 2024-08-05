use std::collections::HashMap;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};
use tokio::sync::{mpsc, RwLock};
use warp::ws::{Message, WebSocket};
use warp::Filter;
use redis::AsyncCommands;
use futures::{FutureExt, StreamExt};
use serde::{Deserialize, Serialize};
use serde_json::to_string;
use tokio_stream::wrappers::UnboundedReceiverStream;

// Client structure
struct Client {
    sender: mpsc::UnboundedSender<Result<Message, warp::Error>>,
}

type Clients = Arc<RwLock<HashMap<usize, Client>>>;
type RedisClient = Arc<redis::Client>;

static NEXT_CLIENT_ID: AtomicUsize = AtomicUsize::new(1);
const REDIS_BITMAP_NAME: &'static str = "mybitmap";
const NUMBER_OF_CHECKBOXES: usize = 1000;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let clients: Clients = Arc::new(RwLock::new(HashMap::new()));

    // Set up Redis client
    let redis_client = redis::Client::open("redis://127.0.0.1/")?;
    let redis_client = Arc::new(redis_client);

    // WebSocket route
    let ws_route = warp::path("ws")
        .and(warp::ws())
        .and(with_clients(clients.clone()))
        .and(with_redis(redis_client.clone()))
        .map(|ws: warp::ws::Ws, clients, redis| {
            ws.on_upgrade(move |socket| handle_connection(socket, clients, redis))
        });

    // Serve static files
    let static_files = warp::path("static").and(warp::fs::dir("static"));

    let routes = ws_route.or(static_files);

    println!("Server started at http://localhost:3030");
    warp::serve(routes).run(([127, 0, 0, 1], 3030)).await;

    Ok(())
}

fn with_clients(clients: Clients) -> impl Filter<Extract=(Clients,), Error=std::convert::Infallible> + Clone {
    warp::any().map(move || clients.clone())
}

fn with_redis(redis: RedisClient) -> impl Filter<Extract=(RedisClient,), Error=std::convert::Infallible> + Clone {
    warp::any().map(move || redis.clone())
}

async fn handle_connection(ws: WebSocket, clients: Clients, redis: RedisClient) {
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

    clients.write().await.remove(&client_id);
    println!("Client disconnected: {}", client_id);
}


async fn send_initial_state(client_id: usize, clients: &Clients, redis: &RedisClient) {
    let mut conn = redis.get_async_connection().await.unwrap();

    let bitmap: Vec<u8> = conn.get(REDIS_BITMAP_NAME).await.unwrap();

    let (true_indices, false_indices) = bitmap_to_tuple(bitmap);

    let checkbox_state = CheckboxState { true_indices, false_indices, is_initial: true };
    if let Some(client) = clients.read().await.get(&client_id) {
        let _ = client.sender.send(Ok(Message::text(to_string(&checkbox_state).unwrap())));
    }
}

async fn handle_message(client_id: usize, msg: Message, clients: &Clients, redis: &RedisClient) {
    let message = match msg.to_str() {
        Ok(v) => v,
        Err(_) => return,
    };
    println!("received message: {}", message);

    if message.starts_with("checkbox:") {
        let checkbox_info = message.trim_start_matches("checkbox:");
        let mut parts = checkbox_info.split(':');
        if let (Some(checkbox_id), Some(state)) = (parts.next(), parts.next()) {
            update_checkbox_state(checkbox_id, state, redis).await;
            broadcast_message(format!("Checkbox updated: {}", checkbox_info), client_id, clients).await;
        }
    }
}

async fn update_checkbox_state(checkbox_id: &str, state: &str, redis: &RedisClient) {
    let mut conn = redis.get_async_connection().await.unwrap();
    let checkbox_id = checkbox_id.parse().unwrap();
    if checkbox_id > NUMBER_OF_CHECKBOXES {
        //Trying to set the value of a checkbox beyond the defined limit --> ignore and return
        return; };
    let _: () = conn.setbit(REDIS_BITMAP_NAME, checkbox_id, state.parse().unwrap()).await.unwrap();
}

async fn broadcast_message(message: String, sender_id: usize, clients: &Clients) {
    for (&uid, client) in clients.read().await.iter() {
        if uid != sender_id {
            if let Err(_disconnected) = client.sender.send(Ok(Message::text(message.clone()))) {
                // The tx is disconnected, our client disconnected, so don't try to send
            }
        }
    }
}

fn bitmap_to_tuple(bitmap: Vec<u8>) -> (Vec<usize>, Vec<usize>) {
    // Create two vectors to store the indices of true and false bits
    let mut true_indices: Vec<usize> = Vec::new();
    let mut false_indices: Vec<usize> = Vec::new();

    // Convert each byte to a binary representation and store indices
    for (byte_index, byte) in bitmap.iter().enumerate() {
        for bit_index in 0..8 {
            // Calculate the bit's position in the overall bitmap
            let bit_position = byte_index * 8 + bit_index;
            // Extract the bit value and classify it
            if (byte >> (7 - bit_index)) & 1 == 1 {
                true_indices.push(bit_position);
            } else {
                false_indices.push(bit_position);
            }
        }
    }
    return (true_indices, false_indices);
}

#[derive(Serialize, Deserialize)]
struct CheckboxState {
    true_indices: Vec<usize>,
    false_indices: Vec<usize>,
    is_initial: bool,
}