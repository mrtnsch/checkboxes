use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use warp::ws::Message;

// Client structure
pub struct Client {
    pub sender: mpsc::UnboundedSender<Result<Message, warp::Error>>,
}

#[derive(Serialize, Deserialize)]
pub struct CheckboxState {
    pub true_indices: Vec<usize>,
    pub false_indices: Vec<usize>,
    pub is_initial: bool,
}