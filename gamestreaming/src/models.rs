use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug, Clone)]
pub enum StreamSetupState {
    WaitingForResources,
    ReadyToConnect,
    Provisioning,
    Provisioned,
}

pub mod common_request {
    use super::{Deserialize, Serialize};

    #[derive(Deserialize, Serialize, Debug, Clone)]
    #[serde(rename_all = "camelCase")]
    pub struct StreamSettings {
        enable_text_to_speech: bool,
        locale: String,
        nano_version: String,
        timezone_offset_minutes: i64,
        use_ice_connection: bool,
    }

    #[derive(Deserialize, Serialize, Debug, Clone)]
    #[serde(rename_all = "camelCase")]
    pub struct StartStreamRequest {
        fallback_region_names: Vec<String>,
        server_id: String,
        settings: StreamSettings,
        system_update_group: String,
        title_id: String,
    }
}

pub mod common_response {
    use super::{Deserialize, Serialize, StreamSetupState};

    #[derive(Deserialize, Serialize, Debug, Clone)]
    #[serde(rename_all = "camelCase")]
    pub struct RegionCloudServer {
        name: String,
        base_uri: String,
        network_test_hostname: String,
        is_default: bool,
        pool_ids: Option<String>,
        system_update_groups: Option<String>,
        fallback_priority: i32,
    }

    #[derive(Deserialize, Serialize, Debug, Clone)]
    #[serde(rename_all = "PascalCase")]
    pub struct CloudEnvironment {
        name: String,
        auth_base_uri: Option<String>,
    }

    #[derive(Deserialize, Serialize, Debug, Clone)]
    #[serde(rename_all = "PascalCase")]
    pub struct ClientCloudSettings {
        environments: Vec<CloudEnvironment>,
    }

    #[derive(Deserialize, Serialize, Debug, Clone)]
    #[serde(rename_all = "camelCase")]
    pub struct OfferingSettings {
        allow_region_selection: bool,
        regions: Vec<RegionCloudServer>,
        client_cloud_settings: ClientCloudSettings,
    }

    #[derive(Deserialize, Serialize, Debug, Clone)]
    #[serde(rename_all = "camelCase")]
    pub struct StreamLoginResponse {
        offering_settings: OfferingSettings,
        market: String,
        gs_token: String,
        token_type: String,
        duration_in_seconds: i32,
    }

    #[derive(Deserialize, Serialize, Debug, Clone)]
    #[serde(rename_all = "camelCase")]
    pub struct StreamSessionResponse {
        session_id: Option<String>,
        session_path: String,
        state: Option<StreamSetupState>,
    }

    #[derive(Deserialize, Serialize, Debug, Clone)]
    #[serde(rename_all = "camelCase")]
    pub struct StreamErrorDetails {
        code: Option<String>,
        message: Option<String>,
    }

    #[derive(Deserialize, Serialize, Debug, Clone)]
    #[serde(rename_all = "camelCase")]
    pub struct StreamStateResponse {
        state: StreamSetupState,
        detailed_session_state: Option<i32>,
        error_details: Option<StreamErrorDetails>,
        transfer_uri: Option<String>,
    }

    #[derive(Deserialize, Serialize, Debug, Clone)]
    #[serde(rename_all = "camelCase")]
    pub struct StreamSRtpData {
        key: String,
    }

    #[derive(Deserialize, Serialize, Debug, Clone)]
    #[serde(rename_all = "camelCase")]
    pub struct StreamServerDetails {
        ip_address: String,
        port: u16,
        ip_v4_address: Option<String>,
        ip_v4_port: u16,
        ip_v6_address: Option<String>,
        ip_v6_port: u16,
        ice_exchange_path: Option<String>,
        stun_server_address: Option<String>,
        srtp: StreamSRtpData,
    }

    #[derive(Deserialize, Serialize, Debug, Clone)]
    #[serde(rename_all = "camelCase")]
    pub struct StreamConfig {
        keep_alive_pulse_in_seconds: i32,
        server_details: StreamServerDetails,
    }

    #[derive(Deserialize, Serialize, Debug, Clone)]
    #[serde(rename_all = "camelCase")]
    pub struct StreamICEConfig {
        candidates: String,
    }
}

pub mod xcloud_response {
    use super::{Deserialize, Serialize};

    #[derive(Deserialize, Serialize, Debug, Clone)]
    #[serde(rename_all = "camelCase")]
    pub struct TitleSupportedTab {
        id: String,
        tab_version: String,
        layout_version: String,
        manifest_version: String,
    }

    #[derive(Deserialize, Serialize, Debug, Clone)]
    #[serde(rename_all = "camelCase")]
    pub struct CloudGameTitleDetails {
        product_id: String,
        xbox_title_id: Option<u32>,
        has_entitlement: bool,
        blocked_by_family_safety: bool,
        supports_in_app_purchases: bool,
        supported_tabs: Option<Vec<TitleSupportedTab>>,
        native_touch: bool,
    }

    #[derive(Deserialize, Serialize, Debug, Clone)]
    #[serde(rename_all = "camelCase")]
    pub struct CloudGameTitle {
        title_id: String,
        details: CloudGameTitleDetails,
    }

    #[derive(Deserialize, Serialize, Debug, Clone)]
    #[serde(rename_all = "camelCase")]
    pub struct TitlesResponse {
        total_items: Option<i32>,
        results: Vec<CloudGameTitle>,
        continuation_token: Option<String>,
    }

    #[derive(Deserialize, Serialize, Debug, Clone)]
    #[serde(rename_all = "camelCase")]
    pub struct TitleWaitTimeResponse {
        estimated_provisioning_time_in_seconds: i32,
        estimated_allocation_time_in_seconds: i32,
        estimated_total_wait_time_in_seconds: i32,
    }
}
