use super::models;
use reqwest;
use serde::de::DeserializeOwned;
use xal::extensions::JsonExDeserializeMiddleware;
use xal::extensions::SigningReqwestBuilder;
use std::collections::HashMap;
use std::default::Default;
use uuid;
use xal::cvlib::CorrelationVector;

type Error = Box<dyn std::error::Error>;
type Result<T> = std::result::Result<T, Error>;

pub struct SmartglassClient {
    session_id: uuid::Uuid,
    request_signer: xal::RequestSigner,
    client: reqwest::Client,
    ms_cv: CorrelationVector,
}

impl SmartglassClient {
    pub fn new(
        token: xal::response::XSTSToken,
        session_id: Option<uuid::Uuid>,
        user_agent: Option<String>,
    ) -> Result<Self> {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            reqwest::header::AUTHORIZATION,
            token.authorization_header_value().parse()?,
        );
        headers.insert("skillplatform", "RemoteManagement".parse()?);
        headers.insert("x-xbl-contract-version", "4".parse()?);

        let client_builder = reqwest::ClientBuilder::new();
        let client =
            client_builder
                .user_agent(user_agent.unwrap_or_else(|| {
                    "Xbox/2008.0915.0311 CFNetwork/1197 Darwin/20.0.0".to_owned()
                }))
                .default_headers(headers)
                .build()?;

        Ok(Self {
            session_id: session_id.unwrap_or_else(uuid::Uuid::new_v4),
            request_signer: xal::RequestSigner::default(),
            ms_cv: CorrelationVector::default(),
            client,
        })
    }

    fn next_cv(&mut self) -> String {
        self.ms_cv.increment();
        self.ms_cv.to_string()
    }

    pub async fn fetch_operation_status(
        &mut self,
        operation_id: &str,
        device_id: &str,
    ) -> Result<models::OperationStatusResponse> {
        let url = "https://xccs.xboxlive.com/opStatus";

        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert("x-xbl-contract-version", "3".parse()?);
        headers.insert("x-xbl-opId", operation_id.parse()?);
        headers.insert("x-xbl-deviceId", device_id.parse()?);

        self.client
            .get(url)
            .headers(headers)
            .header("MS-CV", self.next_cv())
            .sign(&mut self.request_signer, None)
            .await?
            .send()
            .await?
            .json_ex::<models::OperationStatusResponse>()
            .await
            .map_err(|err| err.into())
    }

    pub async fn get_console_status(
        &mut self,
        console_live_id: &str,
    ) -> Result<models::SmartglassConsoleStatus> {
        let url = format!(
            "https://xccs.xboxlive.com/consoles/{live_id}",
            live_id = console_live_id
        );

        self.client
            .get(&url)
            .header("MS-CV", self.next_cv())
            .sign(&mut self.request_signer, None)
            .await?
            .send()
            .await?
            .json::<models::SmartglassConsoleStatus>()
            .await
            .map_err(|err| err.into())
    }

    async fn fetch_list<T>(
        &mut self,
        list_name: &str,
        query_params: Option<HashMap<String, String>>,
    ) -> Result<T>
    where
        T: DeserializeOwned,
    {
        let url = format!(
            "https://xccs.xboxlive.com/lists/{list_name}",
            list_name = list_name
        );

        let mut req_builder = self.client.get(&url);
        if query_params.is_some() {
            req_builder = req_builder.query(&query_params.unwrap())
        }
        req_builder
            .header("MS-CV", self.next_cv())
            .sign(&mut self.request_signer, None)
            .await?
            .send()
            .await?
            .json::<T>()
            .await
            .map_err(|err| err.into())
    }

    async fn send_oneshot_command(
        &mut self,
        console_live_id: &str,
        command_type: &str,
        command: &str,
        parameters: Option<Vec<HashMap<String, String>>>,
    ) -> Result<models::CommandResponse> {
        let url = "https://xccs.xboxlive.com/commands";

        let json_body = models::request::OneShotCommandRequest {
            destination: "Xbox".to_owned(),
            command_type: command_type.to_owned(),
            command: command.to_owned(),
            session_id: self.session_id.hyphenated().to_string(),
            source_id: "com.microsoft.smartglass".to_owned(),
            parameters,
            linked_xbox_id: console_live_id.to_owned(),
        };

        self.client
            .post(url)
            .header("MS-CV", self.next_cv())
            .json(&json_body)
            .sign(&mut self.request_signer, None)
            .await?
            .send()
            .await?
            .json::<models::CommandResponse>()
            .await
            .map_err(|err| err.into())
    }

    pub async fn get_console_list(&mut self) -> Result<models::SmartglassConsoleList> {
        let mut query_params: HashMap<String, String> = HashMap::new();
        query_params.insert("queryCurrentDevice".to_owned(), "false".to_owned());
        query_params.insert("includeStorageDevices".to_owned(), "true".to_owned());

        self.fetch_list("devices", Some(query_params))
            .await
    }

    pub async fn get_storage_devices(
        &mut self,
        device_id: &str,
    ) -> Result<models::StorageDevicesList> {
        let mut query_params: HashMap<String, String> = HashMap::new();
        query_params.insert("deviceId".to_owned(), device_id.to_owned());

        self.fetch_list("storageDevices", Some(query_params))
            .await
    }

    pub async fn get_installed_apps(
        &mut self,
        device_id: &str,
    ) -> Result<models::InstalledPackagesList> {
        let mut query_params: HashMap<String, String> = HashMap::new();
        query_params.insert("deviceId".to_owned(), device_id.to_owned());

        self.fetch_list("installedApps", Some(query_params))
            .await
    }

    pub async fn command_power_wake_up(
        &mut self,
        console_live_id: &str,
    ) -> Result<models::CommandResponse> {
        self.send_oneshot_command(
            console_live_id,
            "Power",
            "WakeUp",
            None,
        )
        .await
    }

    pub async fn command_power_turn_off(
        &mut self,
        console_live_id: &str,
    ) -> Result<models::CommandResponse> {
        self.send_oneshot_command(
            console_live_id,
            "Power",
            "TurnOff",
            None,
        )
        .await
    }

    pub async fn command_power_reboot(
        &mut self,
        console_live_id: &str,
    ) -> Result<models::CommandResponse> {
        self.send_oneshot_command(
            console_live_id,
            "Power",
            "Reboot",
            None,
        )
        .await
    }

    pub async fn command_audio_mute(
        &mut self,
        console_live_id: &str,
    ) -> Result<models::CommandResponse> {
        self.send_oneshot_command(
            console_live_id,
            "Audio",
            "Mute",
            None
        )
        .await
    }

    pub async fn command_audio_unmute(
        &mut self,
        console_live_id: &str,
    ) -> Result<models::CommandResponse> {
        self.send_oneshot_command(
            console_live_id,
            "Audio",
            "Unmute",
            None,
        )
        .await
    }

    pub async fn command_audio_volume(
        &mut self,
        console_live_id: &str,
        direction: models::VolumeDirection,
        amount: Option<i32>,
    ) -> Result<models::CommandResponse> {
        let mut parameters: Vec<HashMap<String, String>> = vec![HashMap::new()];
        parameters[0].insert("direction".to_owned(), direction.to_string());
        parameters[0].insert("amount".to_owned(), amount.unwrap_or(1).to_string());

        self.send_oneshot_command(
            console_live_id,
            "Audio",
            "Volume",
            Some(parameters),
        )
        .await
    }

    pub async fn command_config_digital_assistant_remote_control(
        &mut self,
        console_live_id: &str,
    ) -> Result<models::CommandResponse> {
        self.send_oneshot_command(
            console_live_id,
            "Config",
            "DigitalAssistantRemoteControl",
            None,
        )
        .await
    }

    pub async fn command_config_remote_access(
        &mut self,
        console_live_id: &str,
        enable: bool,
    ) -> Result<models::CommandResponse> {
        let mut parameters: Vec<HashMap<String, String>> = vec![HashMap::new()];
        parameters[0].insert("enabled".to_owned(), enable.to_string().to_lowercase());

        self.send_oneshot_command(
            console_live_id,
            "Config",
            "RemoteAccess",
            Some(parameters),
        )
        .await
    }

    pub async fn command_config_allow_console_streaming(
        &mut self,
        console_live_id: &str,
        enable: bool,
    ) -> Result<models::CommandResponse> {
        let mut parameters: Vec<HashMap<String, String>> = vec![HashMap::new()];
        parameters[0].insert("enabled".to_owned(), enable.to_string().to_lowercase());

        self.send_oneshot_command(
            console_live_id,
            "Config",
            "AllowConsoleStreaming",
            Some(parameters),
        )
        .await
    }

    pub async fn command_game_capture_gameclip(
        &mut self,
        console_live_id: &str,
    ) -> Result<models::CommandResponse> {
        self.send_oneshot_command(
            console_live_id,
            "Game",
            "CaptureGameClip",
            None,
        )
        .await
    }

    pub async fn command_game_capture_screenshot(
        &mut self,
        console_live_id: &str,
    ) -> Result<models::CommandResponse> {
        self.send_oneshot_command(
            console_live_id,
            "Game",
            "CaptureScreenshot",
            None,
        )
        .await
    }

    pub async fn command_game_invite_party_to_game(
        &mut self,
        console_live_id: &str,
    ) -> Result<models::CommandResponse> {
        self.send_oneshot_command(
            console_live_id,
            "Game",
            "InvitePartyToGame",
            None,
        )
        .await
    }

    pub async fn command_game_invite_to_party(
        &mut self,
        console_live_id: &str,
    ) -> Result<models::CommandResponse> {
        self.send_oneshot_command(
            console_live_id,
            "Game",
            "InviteToParty",
            None,
        )
        .await
    }

    pub async fn command_game_kick_from_party(
        &mut self,
        console_live_id: &str,
    ) -> Result<models::CommandResponse> {
        self.send_oneshot_command(
            console_live_id,
            "Game",
            "KickFromParty",
            None,
        )
        .await
    }

    pub async fn command_game_leave_party(
        &mut self,
        console_live_id: &str,
    ) -> Result<models::CommandResponse> {
        self.send_oneshot_command(
            console_live_id,
            "Game",
            "LeaveParty",
            None,
        )
        .await
    }

    pub async fn command_game_set_online_status(
        &mut self,
        console_live_id: &str,
    ) -> Result<models::CommandResponse> {
        self.send_oneshot_command(
            console_live_id,
            "Game",
            "SetOnlineStatus",
            None,
        )
        .await
    }

    pub async fn command_game_start_a_party(
        &mut self,
        console_live_id: &str,
    ) -> Result<models::CommandResponse> {
        self.send_oneshot_command(
            console_live_id,
            "Game",
            "StartAParty",
            None,
        )
        .await
    }

    pub async fn command_game_start_broadcasting(
        &mut self,
        console_live_id: &str,
    ) -> Result<models::CommandResponse> {
        self.send_oneshot_command(
            console_live_id,
            "Game",
            "StartBroadcasting",
            None,
        )
        .await
    }

    pub async fn command_game_stop_broadcasting(
        &mut self,
        console_live_id: &str,
    ) -> Result<models::CommandResponse> {
        self.send_oneshot_command(
            console_live_id,
            "Game",
            "StopBroadcasting",
            None,
        )
        .await
    }

    pub async fn command_gamestreaming_start_management_service(
        &mut self,
        console_live_id: &str,
    ) -> Result<models::CommandResponse> {
        self.send_oneshot_command(
            console_live_id,
            "GameStreaming",
            "StartStreamingManagementService",
            None,
        )
        .await
    }

    pub async fn command_gamestreaming_stop_streaming(
        &mut self,
        console_live_id: &str,
    ) -> Result<models::CommandResponse> {
        self.send_oneshot_command(
            console_live_id,
            "GameStreaming",
            "StopStreaming",
            None,
        )
        .await
    }

    pub async fn command_marketplace_redeem_code(
        &mut self,
        console_live_id: &str,
    ) -> Result<models::CommandResponse> {
        self.send_oneshot_command(
            console_live_id,
            "Marketplace",
            "RedeemCode",
            None,
        )
        .await
    }

    pub async fn command_marketplace_search(
        &mut self,
        console_live_id: &str,
    ) -> Result<models::CommandResponse> {
        self.send_oneshot_command(
            console_live_id,
            "Marketplace",
            "Search",
            None,
        )
        .await
    }

    pub async fn command_marketplace_search_store(
        &mut self,
        console_live_id: &str,
    ) -> Result<models::CommandResponse> {
        self.send_oneshot_command(
            console_live_id,
            "Marketplace",
            "SearchTheStore",
            None,
        )
        .await
    }

    pub async fn command_marketplace_show_title(
        &mut self,
        console_live_id: &str,
    ) -> Result<models::CommandResponse> {
        self.send_oneshot_command(
            console_live_id,
            "Marketplace",
            "ShowTitle",
            None,
        )
        .await
    }

    pub async fn command_media_command(
        &mut self,
        console_live_id: &str,
        media_command: models::MediaCommand,
    ) -> Result<models::CommandResponse> {
        self.send_oneshot_command(
            console_live_id,
            "Media",
            &media_command.to_string(),
            None,
        )
        .await
    }

    pub async fn command_shell_activate_app_with_uri(
        &mut self,
        console_live_id: &str,
    ) -> Result<models::CommandResponse> {
        self.send_oneshot_command(
            console_live_id,
            "Shell",
            "ActivateApplicationWithUri",
            None,
        )
        .await
    }

    pub async fn command_shell_activate_app_with_aumid(
        &mut self,
        console_live_id: &str,
    ) -> Result<models::CommandResponse> {
        self.send_oneshot_command(
            console_live_id,
            "Shell",
            "ActivateApplicationWithAumid",
            None,
        )
        .await
    }

    pub async fn command_shell_activate_app_with_onestore_product_id(
        &mut self,
        console_live_id: &str,
        one_store_product_id: String,
    ) -> Result<models::CommandResponse> {
        let mut parameters: Vec<HashMap<String, String>> = vec![HashMap::new()];
        parameters[0].insert("oneStoreProductId".to_owned(), one_store_product_id);

        self.send_oneshot_command(
            console_live_id,
            "Shell",
            "ActivationApplicationWithOneStoreProductId",
            Some(parameters),
        )
        .await
    }

    pub async fn command_shell_allow_remote_management(
        &mut self,
        console_live_id: &str,
    ) -> Result<models::CommandResponse> {
        self.send_oneshot_command(
            console_live_id,
            "Shell",
            "AllowRemoteManagement",
            None,
        )
        .await
    }

    pub async fn command_shell_change_view(
        &mut self,
        console_live_id: &str,
    ) -> Result<models::CommandResponse> {
        self.send_oneshot_command(
            console_live_id,
            "Shell",
            "ChangeView",
            None,
        )
        .await
    }

    pub async fn command_shell_check_for_package_updates(
        &mut self,
        console_live_id: &str,
    ) -> Result<models::CommandResponse> {
        self.send_oneshot_command(
            console_live_id,
            "Shell",
            "CheckForPackageUpdates",
            None,
        )
        .await
    }

    pub async fn command_shell_copy_packages(
        &mut self,
        console_live_id: &str,
    ) -> Result<models::CommandResponse> {
        self.send_oneshot_command(
            console_live_id,
            "Shell",
            "CopyPackages",
            None,
        )
        .await
    }

    pub async fn command_shell_move_packages(
        &mut self,
        console_live_id: &str,
    ) -> Result<models::CommandResponse> {
        self.send_oneshot_command(
            console_live_id,
            "Shell",
            "MovePackages",
            None,
        )
        .await
    }

    pub async fn command_shell_install_packages(
        &mut self,
        console_live_id: &str,
        big_cat_ids: Vec<String>,
    ) -> Result<models::CommandResponse> {
        let mut parameters: Vec<HashMap<String, String>> = vec![HashMap::new()];
        parameters[0].insert("bigCatIdList".to_owned(), big_cat_ids.join(","));

        self.send_oneshot_command(
            console_live_id,
            "Shell",
            "InstallPackages",
            Some(parameters),
        )
        .await
    }

    pub async fn command_shell_uninstall_package(
        &mut self,
        console_live_id: &str,
        instance_id: &str,
    ) -> Result<models::CommandResponse> {
        let mut parameters: Vec<HashMap<String, String>> = vec![HashMap::new()];
        parameters[0].insert("instanceId".to_owned(), instance_id.to_owned());

        self.send_oneshot_command(
            console_live_id,
            "Shell",
            "UninstallPackage",
            Some(parameters),
        )
        .await
    }

    pub async fn command_shell_update_packages(
        &mut self,
        console_live_id: &str,
    ) -> Result<models::CommandResponse> {
        self.send_oneshot_command(
            console_live_id,
            "Shell",
            "UpdatePackages",
            None,
        )
        .await
    }

    pub async fn command_shell_eject_disk(
        &mut self,
        console_live_id: &str,
    ) -> Result<models::CommandResponse> {
        self.send_oneshot_command(
            console_live_id,
            "Shell",
            "EjectDisk",
            None,
        )
        .await
    }

    pub async fn command_shell_go_back(
        &mut self,
        console_live_id: &str,
    ) -> Result<models::CommandResponse> {
        self.send_oneshot_command(
            console_live_id,
            "Shell",
            "GoBack",
            None,
        )
        .await
    }

    pub async fn command_shell_go_home(
        &mut self,
        console_live_id: &str,
    ) -> Result<models::CommandResponse> {
        self.send_oneshot_command(
            console_live_id,
            "Shell",
            "GoHome",
            None,
        )
        .await
    }

    pub async fn command_shell_pair_controller(
        &mut self,
        console_live_id: &str,
    ) -> Result<models::CommandResponse> {
        self.send_oneshot_command(
            console_live_id,
            "Shell",
            "PairController",
            None,
        )
        .await
    }

    pub async fn command_shell_send_text_message(
        &mut self,
        console_live_id: &str,
    ) -> Result<models::CommandResponse> {
        self.send_oneshot_command(
            console_live_id,
            "Shell",
            "SendTextMessage",
            None,
        )
        .await
    }

    pub async fn command_shell_show_guide_tab(
        &mut self,
        console_live_id: &str,
    ) -> Result<models::CommandResponse> {
        self.send_oneshot_command(
            console_live_id,
            "Shell",
            "ShowGuideTab",
            None,
        )
        .await
    }

    pub async fn command_shell_sign_in(
        &mut self,
        console_live_id: &str,
    ) -> Result<models::CommandResponse> {
        self.send_oneshot_command(
            console_live_id,
            "Shell",
            "SignIn",
            None,
        )
        .await
    }

    pub async fn command_shell_sign_out(
        &mut self,
        console_live_id: &str,
    ) -> Result<models::CommandResponse> {
        self.send_oneshot_command(
            console_live_id,
            "Shell",
            "SignOut",
            None,
        )
        .await
    }

    pub async fn command_shell_launch_game(
        &mut self,
        console_live_id: &str,
    ) -> Result<models::CommandResponse> {
        self.send_oneshot_command(
            console_live_id,
            "Shell",
            "LaunchGame",
            None,
        )
        .await
    }

    pub async fn command_shell_terminate_application(
        &mut self,
        console_live_id: &str,
    ) -> Result<models::CommandResponse> {
        self.send_oneshot_command(
            console_live_id,
            "Shell",
            "TerminateApplication",
            None,
        )
        .await
    }

    pub async fn command_shell_keyinput(
        &mut self,
        console_live_id: &str,
        key_type: models::InputKeyType,
    ) -> Result<models::CommandResponse> {
        let mut parameters: Vec<HashMap<String, String>> = vec![HashMap::new()];
        parameters[0].insert("keyType".to_owned(), key_type.to_string());

        self.send_oneshot_command(
            console_live_id,
            "Shell",
            "InjectKey",
            Some(parameters),
        )
        .await
    }

    pub async fn command_shell_textinput(
        &mut self,
        console_live_id: &str,
        text_input: String,
    ) -> Result<models::CommandResponse> {
        let mut parameters: Vec<HashMap<String, String>> = vec![HashMap::new()];
        parameters[0].insert("replacementString".to_owned(), text_input);

        self.send_oneshot_command(
            console_live_id,
            "Shell",
            "InjectString",
            Some(parameters),
        )
        .await
    }

    pub async fn command_tv_show_guide(
        &mut self,
        console_live_id: &str,
    ) -> Result<models::CommandResponse> {
        self.send_oneshot_command(
            console_live_id,
            "TV",
            "ShowGuide",
            None,
        )
        .await
    }

    pub async fn command_tv_watch_channel(
        &mut self,
        console_live_id: &str,
    ) -> Result<models::CommandResponse> {
        self.send_oneshot_command(
            console_live_id,
            "TV",
            "WatchChannel",
            None,
        )
        .await
    }
}
