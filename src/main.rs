mod alpaca_state;
mod astro_math;
pub mod config;
mod consts;
mod telescope_control;
mod util;

use ascom_alpaca::api::CargoServerInfo;
use ascom_alpaca::Server;
use config::Config;
use telescope_control::StarAdventurer;
use util::*;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let config = confy::load_path("config.toml").expect("Couldn't parse configuration");
    let sa = StarAdventurer::new(&config).await;

    let mut server = Server {
        info: CargoServerInfo!(),
        ..Default::default()
    };
    server.devices.register(sa);

    server
        .listen_addr
        .set_ip(std::net::Ipv4Addr::LOCALHOST.into());
    server.start_server().await
}
