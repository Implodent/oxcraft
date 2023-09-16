use serde::{Deserialize, Serialize};

use super::item::ItemData;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ChatComponent {
    Normal(BasicChatComponent),
    String(ChatStringComponent),
    Multi(Vec<ChatComponent>),
}

impl Default for ChatComponent {
    fn default() -> Self {
        Self::String(ChatStringComponent::default())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ChatStringComponent {
    pub text: String,
    #[serde(flatten)]
    pub basic: BasicChatComponent,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct BasicChatComponent {
    pub bold: bool,
    pub italic: bool,
    pub underlined: bool,
    pub strikethrough: bool,
    pub obfuscated: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub font: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<ChatColor>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub insertion: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub click_event: Option<ChatClickEvent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hover_event: Option<Box<ChatHoverEvent>>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub extra: Vec<ChatComponent>,
}

#[derive(Clone, Copy, Default, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ChatColor {
    Named(ChatColorNamed),
    ShortCode(ChatShortCode),
    Web(ChatColorWeb),
    #[default]
    #[serde(rename = "reset")]
    Reset,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChatShortCode {
    #[serde(rename = "f")]
    White,
    #[serde(rename = "a")]
    Green,
    #[serde(rename = "0")]
    Black,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChatColorNamed {
    Black,
    DarkBlue,
    DarkGren,
    DarkCyan,
    DarkRed,
    Purple,
    Gold,
    Gray,
    DarkGray,
    Blue,
    Green,
    Aqua,
    Red,
    LightPurple,
    Yellow,
    White,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChatColorWeb {
    Hex(u32),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "action", content = "value", rename_all = "snake_case")]
pub enum ChatClickEvent {
    OpenUrl(String),
    RunCommand(String),
    SuggestCommand(String),
    ChangePage(u8),
    CopyToClipboard(String),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "action", content = "value", rename_all = "snake_case")]
pub enum ChatHoverEvent {
    ShowText(ChatComponent),
    ShowItem(ItemData),
}

#[macro_export]
macro_rules! chat {
    // "hello"(bold = true, italic = true)
    (string {
        $($field:ident: $value:expr,)*
        $str:expr
    }) => {
        $crate::model::chat::ChatComponent::String($crate::model::chat::ChatStringComponent {
            text: String::from($str),
            basic: $crate::model::chat::BasicChatComponent {
                $($field: $value,)*
                ..Default::default()
            }
        })
    };
    (multi { $($what:tt),+ }) => {
        $crate::model::chat::ChatComponent::Multi(vec![$(chat!($what),)*])
    };
}
