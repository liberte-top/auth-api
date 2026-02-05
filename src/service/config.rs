use std::{env, sync::Arc};

use crate::config::Config;

pub trait ConfigService: Send + Sync {
    fn port(&self) -> u16;
    fn config(&self) -> &Config;
}

pub struct ConfigServiceImpl {
    config: Arc<Config>,
}

impl ConfigServiceImpl {
    pub fn new() -> Self {
        let port = env::var("PORT")
            .ok()
            .and_then(|value| value.parse::<u16>().ok())
            .unwrap_or(3333);

        Self {
            config: Arc::new(Config { port }),
        }
    }
}

impl ConfigService for ConfigServiceImpl {
    fn port(&self) -> u16 {
        self.config.port
    }

    fn config(&self) -> &Config {
        &self.config
    }
}
