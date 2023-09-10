mod alpaca_state;
mod astro_math;
pub mod config;
mod telescope_control;
mod util;

use ascom_alpaca::api::CargoServerInfo;
use ascom_alpaca::Server;
use config::Config;
use net_literals::addr;
use telescope_control::StarAdventurer;
use util::*;

#[tokio::main]
async fn main() -> eyre::Result<std::convert::Infallible> {
    tracing_subscriber::fmt::init();

    let config = confy::load_path("config.toml").expect("Couldn't parse configuration");
    let sa = StarAdventurer::new(&config).await;

    let mut server = Server {
        info: CargoServerInfo!(),
        listen_addr: addr!("127.0.0.1:8000"),
        ..Default::default()
    };
    server.devices.register(sa);

    server.start().await
}
