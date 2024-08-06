use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct Config {
    pub redis_url: String,
    pub redis_bitmap_name: String,
    pub number_of_checkboxes: usize,
    pub server_port: u16,
}

impl Config {
    pub fn from_env() -> Result<Self, envy::Error> {
        dotenv::dotenv().ok();
        envy::from_env::<Config>()
    }
}

lazy_static::lazy_static! {
    pub static ref CONFIG: Config = Config::from_env().expect("Failed to load configuration");
}
