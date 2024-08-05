mod session;
mod server;

use std::sync::{Arc, Mutex};
use actix::prelude::*;
use actix_cors::Cors;
use actix_web::{web, App, HttpServer, HttpRequest, HttpResponse, Error, Responder};
use actix_web_actors::ws;
use redis::{ Commands, RedisResult};
use serde::{Deserialize, Serialize};


struct WsSession {
    app_state: web::Data<AppState>,
}

impl Actor for WsSession {
    type Context = ws::WebsocketContext<Self>;
}

const BITMAP_NAME: &'static str = "mybitmap";

/// Handler for ws::Message message
impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for WsSession {
    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {

        ctx.address().do_send("kraxlersepp");
        match msg {
            Ok(ws::Message::Ping(msg)) => ctx.pong(&msg),
            Ok(ws::Message::Text(text)) => {
                println!("I have been called");
                let state: CheckboxState = serde_json::from_str(&text).unwrap();
                let _: RedisResult<usize> = self.app_state.redis_client.lock().unwrap().setbit(BITMAP_NAME, state.index, state.checked);
                print!("{}", text.to_string());
                ctx.text(text)

            }
            _ => (),
        }
    }
}

async fn index(req: HttpRequest, stream: web::Payload, app_state: web::Data<AppState>) -> Result<HttpResponse, Error> {
    //TODO: This seems be be called only initially
    println!("Index function has been called");
    let resp = ws::start(WsSession { app_state: app_state.clone() }, &req, stream);
    resp
}

async fn get_checkbox_states(app_state: web::Data<AppState>) -> impl Responder {
    let bitmap: Vec<u8> = app_state.redis_client.lock().unwrap().get("mybitmap").unwrap();

    let (true_indices, false_indices) = bitmap_to_tuple(bitmap);
    return web::Json( FullState {
        true_indices,
        false_indices
    })
}


#[derive(Serialize, Deserialize)]
struct CheckboxState {
    index: usize,
    checked: bool,
}

#[derive(Serialize, Deserialize)]
struct FullState {
    true_indices: Vec<usize>,
    false_indices: Vec<usize>
}

struct AppState {
    redis_client: Mutex<redis::Client>,
}


#[actix::main]
async fn main() -> std::io::Result<()> {
    let redis_client = redis::Client::open("redis://127.0.0.1/").unwrap();
    let app_state = web::Data::new(AppState { redis_client: Mutex::new(redis_client) });

    HttpServer::new(move || {
        App::new()
            .wrap(
                Cors::permissive() // This sets permissive CORS
                    .allow_any_origin()
                    .allow_any_method()
                    .allow_any_header()
                    .supports_credentials()
            )
            .app_data(app_state.clone())
            .route("/ws/", web::get().to(index))
            .route("/checkboxes", web::get().to(get_checkbox_states))
    })
        .bind(("127.0.0.1", 8080))?
        .run()
        .await
}

fn bitmap_to_tuple(bitmap: Vec<u8> ) -> (Vec<usize>, Vec<usize>){
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
    return (true_indices, false_indices)
}
