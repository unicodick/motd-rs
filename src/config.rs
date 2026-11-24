use std::{fs, io, net::SocketAddr, path::PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub(crate) struct Config {
    pub(crate) listen: SocketAddr,
    pub(crate) version_name: String,
    #[serde(default)]
    pub(crate) version_display: VersionDisplay,
    pub(crate) icon: PathBuf,
    pub(crate) motd: Component,
    #[serde(default)]
    pub(crate) sample: Vec<String>,
    pub(crate) disconnect: Component,
}

impl Config {
    pub(crate) fn load(path: &str) -> io::Result<Self> {
        let data = fs::read(path)?;
        serde_json::from_slice(&data).map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))
    }
}

#[derive(Default, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum VersionDisplay {
    #[default]
    Compatible,
    Label,
}

#[derive(Clone, Deserialize, Serialize)]
pub(crate) struct Component {
    text: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    color: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    bold: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    italic: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    underlined: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    strikethrough: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    obfuscated: Option<bool>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    extra: Vec<Component>,
}

impl Component {
    pub(crate) fn fill(&mut self, attempt: &str, player: Option<&str>) {
        self.text = self.text.replace("{attempt}", attempt);
        if let Some(player) = player {
            self.text = self.text.replace("{player}", player);
        }
        for part in &mut self.extra {
            part.fill(attempt, player);
        }
    }
}
