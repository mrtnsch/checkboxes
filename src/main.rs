use std::collections::HashMap;
use std::sync::{Arc, atomic::AtomicUsize};

use tokio::sync::RwLock;

use model::Client;

use crate::config::CONFIG;
use crate::redis_handler::RedisHandler;

mod config;
mod model;
mod redis_handler;
mod routes;
mod utils;
mod websocket;

type Clients = Arc<RwLock<HashMap<usize, Client>>>;
static NEXT_CLIENT_ID: AtomicUsize = AtomicUsize::new(1);

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let clients: Clients = Arc::new(RwLock::new(HashMap::new()));

    let redis_handler = RedisHandler::new(&CONFIG.redis_url).await?;

    let warp_routes = routes::create_routes(clients, redis_handler);

    println!("Server started at http://localhost:{}", CONFIG.server_port);
    warp::serve(warp_routes)
        .run(([0, 0, 0, 0], CONFIG.server_port))
        .await;

    Ok(())
}
