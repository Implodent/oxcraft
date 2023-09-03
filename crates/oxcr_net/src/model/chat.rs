use serde::{Deserialize, Serialize};

use super::item::ItemData;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChatComponent {
    Normal(BasicChatComponent),
    String(ChatStringComponent),
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
    pub extra: Vec<Box<ChatComponent>>,
}

#[derive(Clone, Copy, Default, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChatColor {
    Named(ChatColorNamed),
    ShortCode(ChatShortCode),
    Web(ChatColorWeb),
    #[default]
    Reset,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChatShortCode {
    #[serde(rename = "f")]
    White,
    #[serde(rename = "a")]
    Green,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChatColorNamed {}

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
