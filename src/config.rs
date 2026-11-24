use std::{fs, io, net::SocketAddr, path::PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Deserialize)]
pub(crate) struct Config {
    pub(crate) listen: SocketAddr,
    pub(crate) version_name: String,
    #[serde(default)]
    pub(crate) version_display: VersionDisplay,
    pub(crate) icon: PathBuf,
    pub(crate) motd: Component,
    pub(crate) motd_sprites: Option<Component>,
    pub(crate) motd_banner: Option<Component>,
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
#[serde(untagged)]
pub(crate) enum Component {
    Plain(String),
    Styled(Styled),
}

#[derive(Clone, Deserialize, Serialize)]
pub(crate) struct Styled {
    #[serde(flatten)]
    content: Content,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    color: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    shadow_color: Option<Value>,
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

#[derive(Clone, Deserialize, Serialize)]
#[serde(untagged)]
enum Content {
    Text {
        text: String,
    },
    Sprite {
        sprite: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        atlas: Option<String>,
    },
    Head {
        player: Value,
        #[serde(skip_serializing_if = "Option::is_none")]
        hat: Option<bool>,
    },
}

impl Component {
    pub(crate) fn fill(&mut self, attempt: &str, player: Option<&str>) {
        let fill = |text: &mut String| {
            *text = text.replace("{attempt}", attempt);
            if let Some(player) = player {
                *text = text.replace("{player}", player);
            }
        };
        match self {
            Self::Plain(text) => fill(text),
            Self::Styled(part) => {
                if let Content::Text { text } = &mut part.content {
                    fill(text);
                }
                for child in &mut part.extra {
                    child.fill(attempt, player);
                }
            }
        }
    }
}
