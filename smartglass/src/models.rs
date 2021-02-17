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
    error_code: String,
    error_message: Option<String>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct StorageDevice {
    storage_device_id: String,
    storage_device_name: String,
    is_default: bool,
    total_space_bytes: f32,
    free_space_bytes: f32,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SmartglassConsole {
    id: String,
    name: String,
    console_type: ConsoleType,
    power_state: PowerState,
    console_streaming_enabled: bool,
    digital_assistant_remote_control_enabled: bool,
    remote_management_enabled: bool,
    storage_devices: Option<Vec<StorageDevice>>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SmartglassConsoleList {
    agent_user_id: Option<String>,
    result: Vec<SmartglassConsole>,
    status: SmartglassApiStatus,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SmartglassConsoleStatus {
    power_state: PowerState,
    console_streaming_enabled: bool,
    digital_assistant_remote_control_enabled: bool,
    remote_management_enabled: bool,

    focus_app_aumid: String,
    is_tv_configured: bool,
    login_state: Option<String>,
    playback_state: PlaybackState,

    storage_devices: Option<Vec<StorageDevice>>,
    status: SmartglassApiStatus,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct InstalledPackage {
    one_store_product_id: Option<String>,
    title_id: i32,
    aumid: Option<String>,
    last_active_time: Option<String>,
    is_game: bool,
    name: Option<String>,
    content_type: String,
    instance_id: String,
    storage_device_id: String,
    unique_id: String,
    legacy_product_id: Option<String>,
    version: i32,
    size_in_bytes: i32,
    install_time: String,
    update_time: Option<String>,
    parent_id: Option<String>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct InstalledPackagesList {
    result: Vec<InstalledPackage>,
    status: SmartglassApiStatus,
    agent_user_id: Option<String>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct StorageDevicesList {
    device_id: String,
    result: Vec<StorageDevice>,
    status: SmartglassApiStatus,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct OpStatusNode {
    operation_status: OpStatus,
    op_id: String,
    originating_session_id: String,
    command: String,
    succeeded: bool,
    console_status_code: Option<i32>,
    xccs_error_code: Option<ErrorCode>,
    h_result: Option<i32>,
    message: Option<String>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct OperationStatusResponse {
    op_status_list: Vec<OpStatusNode>,
    status: SmartglassApiStatus,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CommandDestination {
    id: String,
    name: String,
    power_state: PowerState,
    remote_management_enabled: bool,
    console_streaming_enabled: bool,
    console_type: ConsoleType,
    wireless_warning: Option<String>,
    out_of_home_warning: Option<String>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CommandResponse {
    result: Option<String>,
    ui_text: Option<String>,
    destination: CommandDestination,
    user_info: Option<String>,
    op_id: String,
    status: SmartglassApiStatus,
}
