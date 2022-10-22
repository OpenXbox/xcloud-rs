use serde::{Deserialize, Serialize};
use std::clone::Clone;
use std::collections::HashMap;
use std::fmt;

#[derive(Deserialize, Serialize, Debug, Clone, Copy)]
pub enum ConsoleType {
    XboxOne,
    XboxOneS,
    XboxOneSDigital,
    XboxOneX,
    XboxSeriesS,
    XboxSeriesX,
}

#[derive(Deserialize, Serialize, Debug, Clone, Copy)]
pub enum PowerState {
    Unknown,
    On,
    Off,
    ConnectedStandby,
    SystemUpdate,
}

#[derive(Deserialize, Serialize, Debug, Clone, Copy)]
pub enum PlaybackState {
    Unknown,
    Playing,
    Paused,
    Stopped,
}

#[derive(Deserialize, Serialize, Debug, Clone, Copy)]
pub enum ErrorCode {
    OK,
    CurrentConsoleNotFound,
    RemoteManagementDisabled,
    XboxDataNotFound,
    XboxNotPaired,
}

#[derive(Deserialize, Serialize, Debug, Clone, Copy)]
pub enum OpStatus {
    Paused,
    OffConsoleError,
    Pending,
    TimedOut,
    Error,
    Succeeded,
}

#[derive(Deserialize, Serialize, Debug, Clone, Copy)]
pub enum VolumeDirection {
    Up,
    Down,
}

impl fmt::Display for VolumeDirection {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Deserialize, Serialize, Debug, Clone, Copy)]
pub enum InputKeyType {
    Guide,
    Menu,
    View,
    A,
    B,
    X,
    Y,
    Up,
    Down,
    Left,
    Right,
}

impl fmt::Display for InputKeyType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Deserialize, Serialize, Debug, Clone, Copy)]
pub enum MediaCommand {
    Pause,
    Play,
    Previous,
    Next,
}

impl fmt::Display for MediaCommand {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

pub mod request {
    use super::{Deserialize, HashMap, Serialize};

    #[derive(Deserialize, Serialize, Debug, Clone)]
    #[serde(rename_all = "camelCase")]
    pub struct OneShotCommandRequest {
        pub destination: String,
        #[serde(alias = "type")]
        pub command_type: String,
        pub command: String,
        pub session_id: String,
        pub source_id: String,
        pub parameters: Option<Vec<HashMap<String, String>>>,
        pub linked_xbox_id: String,
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SmartglassApiStatus {
    pub error_code: String,
    pub error_message: Option<String>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct StorageDevice {
    pub storage_device_id: String,
    pub storage_device_name: String,
    pub is_default: bool,
    pub total_space_bytes: f32,
    pub free_space_bytes: f32,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SmartglassConsole {
    pub id: String,
    pub name: String,
    pub console_type: ConsoleType,
    pub power_state: PowerState,
    pub console_streaming_enabled: bool,
    pub digital_assistant_remote_control_enabled: bool,
    pub remote_management_enabled: bool,
    pub storage_devices: Option<Vec<StorageDevice>>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SmartglassConsoleList {
    pub agent_user_id: Option<String>,
    pub result: Vec<SmartglassConsole>,
    pub status: SmartglassApiStatus,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SmartglassConsoleStatus {
    pub power_state: PowerState,
    pub console_streaming_enabled: bool,
    pub digital_assistant_remote_control_enabled: bool,
    pub remote_management_enabled: bool,

    pub focus_app_aumid: String,
    pub is_tv_configured: bool,
    pub login_state: Option<String>,
    pub playback_state: PlaybackState,

    pub storage_devices: Option<Vec<StorageDevice>>,
    pub status: SmartglassApiStatus,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct InstalledPackage {
    pub one_store_product_id: Option<String>,
    pub title_id: i32,
    pub aumid: Option<String>,
    pub last_active_time: Option<String>,
    pub is_game: bool,
    pub name: Option<String>,
    pub content_type: String,
    pub instance_id: String,
    pub storage_device_id: String,
    pub unique_id: String,
    pub legacy_product_id: Option<String>,
    pub version: i32,
    pub size_in_bytes: i32,
    pub install_time: String,
    pub update_time: Option<String>,
    pub parent_id: Option<String>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct InstalledPackagesList {
    pub result: Vec<InstalledPackage>,
    pub status: SmartglassApiStatus,
    pub agent_user_id: Option<String>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct StorageDevicesList {
    pub device_id: String,
    pub result: Vec<StorageDevice>,
    pub status: SmartglassApiStatus,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct OpStatusNode {
    pub operation_status: OpStatus,
    pub op_id: String,
    pub originating_session_id: String,
    pub command: String,
    pub succeeded: bool,
    pub console_status_code: Option<i32>,
    pub xccs_error_code: Option<ErrorCode>,
    pub h_result: Option<i32>,
    pub message: Option<String>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct OperationStatusResponse {
    pub op_status_list: Vec<OpStatusNode>,
    pub status: SmartglassApiStatus,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CommandDestination {
    pub id: String,
    pub name: String,
    pub power_state: PowerState,
    pub remote_management_enabled: bool,
    pub console_streaming_enabled: bool,
    pub console_type: ConsoleType,
    pub wireless_warning: Option<String>,
    pub out_of_home_warning: Option<String>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CommandResponse {
    pub result: Option<String>,
    pub ui_text: Option<String>,
    pub destination: CommandDestination,
    pub user_info: Option<String>,
    pub op_id: String,
    pub status: SmartglassApiStatus,
}
