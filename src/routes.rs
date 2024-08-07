use warp::Filter;
use warp::http::StatusCode;

use crate::Clients;
use crate::redis_handler::RedisHandler;
use crate::websocket::handle_connection;

pub fn create_routes(
    clients: Clients,
    redis_handler: RedisHandler,
) -> impl Filter<Extract=impl warp::Reply, Error=warp::Rejection> + Clone {
    let ws_route = warp::path("ws")
        .and(warp::ws())
        .and(with_clients(clients.clone()))
        .and(with_redis_handler(redis_handler.clone()))
        .map(|ws: warp::ws::Ws, clients, redis_handler| {
            ws.on_upgrade(move |socket| handle_connection(socket, clients, redis_handler))
        });

    let static_files = warp::path::end().and(warp::fs::dir("static"));
    let health_route = warp::path("health").map(|| StatusCode::OK);

    ws_route.or(static_files).or(health_route)
}

fn with_clients(clients: Clients) -> impl Filter<Extract=(Clients,), Error=std::convert::Infallible> + Clone {
    warp::any().map(move || clients.clone())
}

fn with_redis_handler(redis_handler: RedisHandler) -> impl Filter<Extract=(RedisHandler,), Error=std::convert::Infallible> + Clone {
    warp::any().map(move || redis_handler.clone())
}