use crate::config::Config;

use super::StarAdventurer;

pub(in crate::telescope_control) async fn create_sa(config: Option<Config>) -> StarAdventurer {
    let config = config.unwrap_or_else(|| confy::load_path("test_config.toml").unwrap());
    StarAdventurer::new(&config).await
}
