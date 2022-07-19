extern crate pretty_env_logger;

pub mod vehicle;
use rlua::{Value, prelude::LuaError};
use serde::{Deserialize, Serialize};
use vehicle::*;
use std::{io::Write, convert::{TryFrom}, hash::Hash, fmt::{Display, Debug}};
use chrono::Local;
pub use log::{info, warn, error};

pub const VERSION: (u32, u32) = (0, 5);
pub const VERSION_STR: &str = "0.5.0";

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ClientInfoPrivate {
    pub name: String,
    pub secret: String,
    pub client_version: (u32, u32),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ClientInfoPublic {
    pub name: String,
    pub id: u32,
    pub current_vehicle: u32,
    pub ping: u32,
    pub hide_nametag: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ServerInfo {
    pub name: String,
    pub player_count: u8,
    pub client_id: u32,
    pub map: String,
    pub tickrate: u8,
    pub max_vehicles_per_client: u8,
    pub mods: Vec<(String, u32)>,
    pub server_identifier: String,
}

impl ClientInfoPublic {
    pub fn new(id: u32) -> Self {
        Self {
            id,
            ..Default::default()
        }
    }
}

impl Default for ClientInfoPublic {
    fn default() -> Self {
        Self {
            name: String::from("Unknown"),
            id: 0,
            current_vehicle: 0,
            ping: 0,
            hide_nametag: false,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub enum ClientCommand {
    ClientInfo(ClientInfoPrivate),
    VehicleUpdate(VehicleUpdate),
    VehicleData(VehicleData),
    GearboxUpdate(Gearbox),
    RemoveVehicle(u32),
    ResetVehicle(u32),
    Chat(String),
    RequestMods(Vec<String>),
    VehicleMetaUpdate(VehicleMeta),
    VehicleChanged(u32),
    CouplerAttached(CouplerAttached),
    CouplerDetached(CouplerDetached),
    ElectricsUndefinedUpdate(u32, ElectricsUndefined),
    VoiceChatPacket(Vec<u8>),
    // Only used by bridge
    SpatialUpdate([f32; 3], [f32; 3]),
    // Only used by bridge
    StartTalking,
    // Only used by bridge
    EndTalking,
    Ping(u16),
}

#[derive(Debug, Serialize, Deserialize)]
pub enum ServerCommand {
    VehicleUpdate(VehicleUpdate),
    VehicleSpawn(VehicleData),
    RemoveVehicle(u32),
    ResetVehicle(u32),
    Chat(String, Option<u32>),
    TransferFile(String),
    SendLua(String),
    CallEvent(String, Vec<MarshalledLuaValue>),
    PlayerInfoUpdate(ClientInfoPublic),
    VehicleMetaUpdate(VehicleMeta),
    PlayerDisconnected(u32),
    VehicleLuaCommand(u32, String),
    CouplerAttached(CouplerAttached),
    CouplerDetached(CouplerDetached),
    ElectricsUndefinedUpdate(u32, ElectricsUndefined),
    ServerInfo(ServerInfo),
    FilePart(String, Vec<u8>, u32, u32, u32),
    VoiceChatPacket(u32, [f32; 3], Vec<u8>),
    Pong(f64),
}

#[derive(PartialEq, Eq, Hash, Clone, Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MarshalledLuaValue {
    Nil,
    Boolean(bool),
    Integer(i64),
    Number(Float64),
    String(String)
    // Tables were considered, but marshalling it is far too complex.
    // (Hashing, are we pointing to the same table, other pitfalls)
    // Ergo—for sanity—we will not marshall tables. _For Now™_
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Float64(f64);

impl Hash for Float64 {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.to_bits().hash(state)
    }
}

impl PartialEq for Float64 {
    fn eq(&self, other: &Self) -> bool {
        self.0.to_bits() == other.0.to_bits()
    }
}

impl Eq for Float64 {}

pub enum MarshalledLuaValueConversionError {
    UnsupportedType,
    LuaError(LuaError)
}

impl Display for MarshalledLuaValueConversionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use MarshalledLuaValueConversionError as E;
        match self {
            E::UnsupportedType => write!(f, "The value provided is not supported"),
            E::LuaError(e) => write!(f, "{}", e),
        }
    }
}

impl Debug for MarshalledLuaValueConversionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnsupportedType => write!(f, "UnsupportedType"),
            Self::LuaError(arg0) => f.debug_tuple("LuaError").field(arg0).finish(),
        }
    }
}

impl From<LuaError> for MarshalledLuaValueConversionError {
    fn from(e: LuaError) -> Self { Self::LuaError(e) }
}

impl TryFrom<&Value<'_>> for MarshalledLuaValue {
    type Error = MarshalledLuaValueConversionError;

    fn try_from(value: &Value) -> Result<Self, MarshalledLuaValueConversionError> {
        use MarshalledLuaValue as M;
        Ok(match value {
            Value::Nil =>                     {M::Nil},
            Value::Boolean(boolean) => {M::Boolean(boolean.to_owned())},
            Value::Integer(integer) =>  {M::Integer(integer.to_owned())},
            Value::Number(number) =>    {M::Number(Float64(number.to_owned()))},
            Value::String(string) => {M::String(string.to_str()?.to_owned())},
            _ => {Err(MarshalledLuaValueConversionError::UnsupportedType)?}
        })
    }
}

pub fn init_logging()
{
    // pretty_env_logger doesn't appear to print anything without using
    // a filter in the builder.
    let filter = match std::env::var("RUST_LOG")
    {
      Ok(f) => f,
      Err(_e) => "info".to_owned()
    };


    let _ = pretty_env_logger::formatted_builder().
    parse_filters(&filter)
    .default_format()
    .format(|buf, record| {
        let level = { buf.default_styled_level(record.level()) };
        let mut module_path = match record.module_path()
        {
            Some(path) => path,
            None => "unknown"
        };

        // this removes anything past the root so the log stays clean (ex. kissmp_server::voice_Chat -> kissmp_server)
        let c_index = module_path.find(":");
        if c_index.is_some() {
            module_path = &module_path[..c_index.unwrap()];
        }

        writeln!(buf, "[{}] [{}] [{}]: {}", Local::now().format("%H:%M:%S%.3f"), module_path, format_args!("{:>5}", level), record.args())
    })
    .try_init();
}