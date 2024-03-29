fn default_since() -> u32 {
    1
}

#[derive(Clone, Hash, Eq, PartialEq, Debug, serde::Deserialize)]
pub enum ContextType {
    #[serde(rename = "sender")]
    Sender,
    #[serde(rename = "receiver")]
    Receiver,
}

#[derive(Clone, Hash, Eq, PartialEq, Debug, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProtocolType {
    String,
    Int32,
    Uint32,
    Int64,
    Uint64,
    Object,
    NewId,
    Float,
    Fd,
}

#[derive(Clone, Hash, Eq, PartialEq, Debug, serde::Deserialize)]
pub struct Protocol {
    #[serde(rename = "@name")]
    pub name: String,
    pub copyright: String,
    #[serde(rename = "interface")]
    pub interfaces: Vec<Interface>,
}

#[derive(Clone, Hash, Eq, PartialEq, Debug, serde::Deserialize)]
pub struct Interface {
    #[serde(rename = "@name")]
    pub name: String,
    #[serde(default, rename = "request")]
    pub requests: Vec<EventOrRequest>,
    #[serde(default, rename = "event")]
    pub events: Vec<EventOrRequest>,
    #[serde(default, rename = "enum")]
    pub enums: Vec<Enum>,
}

#[derive(Clone, Hash, Eq, PartialEq, Debug, serde::Deserialize)]
pub struct EventOrRequest {
    #[serde(rename = "@name")]
    pub name: String,
    pub description: Description,
    #[serde(rename = "@context-type")]
    pub context_type: Option<ContextType>,
    #[serde(default = "default_since", rename = "@since")]
    pub since: u32,
    #[serde(default, rename = "arg")]
    pub args: Vec<Arg>,
}

#[derive(Clone, Hash, Eq, PartialEq, Debug, serde::Deserialize)]
pub struct Enum {
    #[serde(rename = "@name")]
    pub name: String,
    #[serde(default = "default_since", rename = "@since")]
    pub since: u32,
    #[serde(default, rename = "@bitfield")]
    pub bitfield: bool,
    #[serde(rename = "entry")]
    pub entries: Vec<Entry>,
}

#[derive(Clone, Hash, Eq, PartialEq, Debug, serde::Deserialize)]
pub struct Entry {
    #[serde(rename = "@name")]
    pub name: String,
    #[serde(rename = "@value")]
    pub value: u32,
    #[serde(default = "default_since", rename = "@since")]
    pub since: u32,
    #[serde(rename = "@summary")]
    pub summary: String,
}

#[derive(Clone, Hash, Eq, PartialEq, Debug, serde::Deserialize)]
pub struct Arg {
    #[serde(rename = "@name")]
    pub name: String,
    #[serde(rename = "@type")]
    pub type_: ProtocolType,
    #[serde(rename = "@summary")]
    pub summary: Option<String>,
    #[serde(rename = "@interface")]
    pub interface: Option<String>,
    #[serde(rename = "@interface_arg")]
    pub interface_arg: Option<String>,
    #[serde(rename = "@allows_null")]
    pub allows_null: Option<String>,
    #[serde(rename = "@enum")]
    pub enum_: Option<String>,
}

#[derive(Clone, Hash, Eq, PartialEq, Debug, serde::Deserialize)]
pub struct Description {
    #[serde(rename = "@summary")]
    pub summary: String,
    #[serde(rename = "$text")]
    pub text: String,
}
