use redis::aio::MultiplexedConnection;
use redis::AsyncCommands;

use crate::config::CONFIG;
use crate::model::CheckboxState;
use crate::utils::bitmap_to_tuple;

#[derive(Clone)]
pub struct RedisHandler {
    connection: MultiplexedConnection,
}

impl RedisHandler {
    pub async fn new(redis_url: &str) -> Result<Self, redis::RedisError> {
        let client = redis::Client::open(redis_url)?;
        let conn = client.get_multiplexed_tokio_connection().await.unwrap();
        Ok(Self {
            connection: conn
        })
    }

    pub async fn get_initial_state(&self) -> Result<CheckboxState, redis::RedisError> {
        let mut conn = self.connection.clone();
        let bitmap: Vec<u8> = conn.get(&CONFIG.redis_bitmap_name).await?;
        let (true_indices, false_indices) = bitmap_to_tuple(bitmap);

        Ok(CheckboxState {
            true_indices,
            false_indices,
            is_initial: true,
        })
    }

    pub async fn update_checkbox(&self, checkbox_id: usize, state: &str) -> Result<(), redis::RedisError> {
        if checkbox_id >= CONFIG.number_of_checkboxes {
            return Err(redis::RedisError::from((redis::ErrorKind::InvalidClientConfig, "Checkbox ID out of range")));
        }
        let mut conn = self.connection.clone();

        conn.setbit(&CONFIG.redis_bitmap_name, checkbox_id, state.parse().unwrap()).await?;
        Ok(())
    }
}