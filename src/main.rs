mod alpaca_state;
mod astro_math;
pub mod config;
mod consts;
mod telescope_control;
mod util;

use alpaca_state::AlpacaState;
use ascom_alpaca::api::CargoServerInfo;
use ascom_alpaca::Server;
use config::Config;
use std::sync::atomic::AtomicU32;
use telescope_control::StarAdventurer;
use util::*;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let config = confy::load_path("config.toml").expect("Couldn't parse configuration");
    let state = AlpacaState {
        sa: StarAdventurer::new(&config).await,
        sti: AtomicU32::new(0),
    };

    let mut server = Server {
        info: CargoServerInfo!(),
        ..Default::default()
    };
    server.devices.register(state);

    server.start_server().await
}
