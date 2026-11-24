use std::{fs, io, path::Path};

use base64::{engine::general_purpose::STANDARD, Engine};
use serde_json::json;

use crate::config::{Config, VersionDisplay};
use crate::protocol::{self, DEFAULT_STATUS_PROTOCOL, MIN_PROTOCOL};

pub(crate) struct Responses {
    config: Config,
    icon: String,
}

impl Responses {
    pub(crate) fn new(config: Config, icon: String) -> io::Result<Self> {
        let responses = Self { config, icon };

        responses.status(i32::MAX, i32::MAX)?;
        responses.disconnect(i32::MAX, &"\0".repeat(16))?;
        Ok(responses)
    }

    pub(crate) fn status(&self, attempts: i32, client: i32) -> io::Result<Vec<u8>> {
        let config = &self.config;
        let protocol = match config.version_display {
            VersionDisplay::Compatible if client >= MIN_PROTOCOL => client,
            VersionDisplay::Compatible => DEFAULT_STATUS_PROTOCOL,
            VersionDisplay::Label => -1,
        };
        let attempt = attempts.to_string();
        let mut motd = config.motd.clone();
        motd.fill(&attempt, None);
        let sample: Vec<_> = config
            .sample
            .iter()
            .enumerate()
            .map(|(i, name)| {
                json!({
                    "id": format!("00000000-0000-0000-0000-{:012x}", i + 1),
                    "name": name.replace("{attempt}", &attempt),
                })
            })
            .collect();

        protocol::json_packet(&json!({
            "version": {
                "name": config.version_name.replace("{attempt}", &attempt),
                "protocol": protocol,
            },
            "players": { "online": attempts, "max": attempts, "sample": sample },
            "description": motd,
            "favicon": self.icon,
        }))
    }

    pub(crate) fn disconnect(&self, attempt: i32, player: &str) -> io::Result<Vec<u8>> {
        let mut message = self.config.disconnect.clone();
        message.fill(&attempt.to_string(), Some(player));
        protocol::json_packet(&message)
    }
}

pub(crate) fn icon(path: &Path) -> io::Result<String> {
    let data = fs::read(path)?;
    Ok(format!("data:image/png;base64,{}", STANDARD.encode(data)))
}
