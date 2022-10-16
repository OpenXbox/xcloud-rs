use super::models;
use reqwest;
use std::collections::HashMap;
use std::default::Default;
use uuid;
use xal::cvlib::CorrelationVector;
use xal::models as xal_models;
use xal::request_signer;

type Error = Box<dyn std::error::Error>;
type Result<T> = std::result::Result<T, Error>;

pub struct SmartglassClient {
    session_id: uuid::Uuid,
    request_signer: request_signer::RequestSigner,
    client: reqwest::Client,
    ms_cv: CorrelationVector,
}

impl SmartglassClient {
    pub fn new(
        token: xal_models::response::XSTSResponse,
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
        let client = client_builder
            .user_agent(
                user_agent.unwrap_or("Xbox/2008.0915.0311 CFNetwork/1197 Darwin/20.0.0".to_owned()),
            )
            .default_headers(headers)
            .build()?;

        Ok(Self {
            session_id: session_id.unwrap_or(uuid::Uuid::new_v4()),
            request_signer: request_signer::RequestSigner::default(),
            ms_cv: CorrelationVector::default(),
            client: client,
        })
    }

    fn next_cv(&mut self) -> String {
        self.ms_cv.increment();
        self.ms_cv.to_string()
    }

    pub async fn send_signed(
        &mut self,
        request: &mut reqwest::Request,
    ) -> Result<reqwest::Response> {
        let mut request = request.try_clone().unwrap();

        request
            .headers_mut()
            .insert("MS-CV", self.next_cv().parse()?);
        request = self.request_signer.sign_request(request, None)?;
        Ok(self.client.execute(request).await?)
    }

    pub async fn fetch_operation_status(
        &mut self,
        operation_id: String,
        device_id: String,
    ) -> Result<models::OperationStatusResponse> {
        let url = "https://xccs.xboxlive.com/opStatus";

        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert("x-xbl-contract-version", "3".parse()?);
        headers.insert("x-xbl-opId", operation_id.parse()?);
        headers.insert("x-xbl-deviceId", device_id.parse()?);

        let mut request = self.client.get(url).headers(headers).build()?;
        let resp = self.send_signed(&mut request).await?;

        Ok(serde_json::from_str(&resp.text().await?)?)
    }

    pub async fn get_console_status(
        &mut self,
        console_live_id: String,
    ) -> Result<models::SmartglassConsoleStatus> {
        let url = format!(
            "https://xccs.xboxlive.com/consoles/{live_id}",
            live_id = console_live_id
        );

        let mut request = self.client.get(&url).build()?;
        let resp = self.send_signed(&mut request).await?;

        Ok(serde_json::from_str(&resp.text().await?)?)
    }

    async fn fetch_list(
        &mut self,
        list_name: String,
        query_params: Option<HashMap<String, String>>,
    ) -> Result<reqwest::Response> {
        let url = format!(
            "https://xccs.xboxlive.com/lists/{list_name}",
            list_name = list_name
        );

        let mut req_builder = self.client.get(&url);
        if query_params.is_some() {
            req_builder = req_builder.query(&query_params.unwrap())
        }
        let mut request = req_builder.build()?;
        let resp = self.send_signed(&mut request).await?;

        Ok(resp)
    }

    async fn send_oneshot_command(
        &mut self,
        console_live_id: String,
        command_type: String,
        command: String,
        parameters: Option<Vec<HashMap<String, String>>>,
    ) -> Result<models::CommandResponse> {
        let url = "https://xccs.xboxlive.com/commands";

        let json_body = models::request::OneShotCommandRequest {
            destination: "Xbox".to_owned(),
            command_type: command_type,
            command: command,
            session_id: self.session_id.hyphenated().to_string(),
            source_id: "com.microsoft.smartglass".to_owned(),
            parameters: parameters,
            linked_xbox_id: console_live_id,
        };

        let mut request = self.client.post(url).json(&json_body).build()?;
        let resp = self.send_signed(&mut request).await?;

        Ok(serde_json::from_str(&resp.text().await?)?)
    }

    pub async fn get_console_list(&mut self) -> Result<models::SmartglassConsoleList> {
        let mut query_params: HashMap<String, String> = HashMap::new();
        query_params.insert("queryCurrentDevice".to_owned(), "false".to_owned());
        query_params.insert("includeStorageDevices".to_owned(), "true".to_owned());

        let resp = self
            .fetch_list("devices".to_owned(), Some(query_params))
            .await?;

        Ok(serde_json::from_str(&resp.text().await?)?)
    }

    pub async fn get_storage_devices(
        &mut self,
        device_id: String,
    ) -> Result<models::StorageDevicesList> {
        let mut query_params: HashMap<String, String> = HashMap::new();
        query_params.insert("deviceId".to_owned(), device_id);

        let resp = self
            .fetch_list("storageDevices".to_owned(), Some(query_params))
            .await?;

        Ok(serde_json::from_str(&resp.text().await?)?)
    }

    pub async fn get_installed_apps(
        &mut self,
        device_id: String,
    ) -> Result<models::InstalledPackagesList> {
        let mut query_params: HashMap<String, String> = HashMap::new();
        query_params.insert("deviceId".to_owned(), device_id);

        let resp = self
            .fetch_list("installedApps".to_owned(), Some(query_params))
            .await?;

        Ok(serde_json::from_str(&resp.text().await?)?)
    }

    pub async fn command_power_wake_up(
        &mut self,
        console_live_id: String,
    ) -> Result<models::CommandResponse> {
        self.send_oneshot_command(
            console_live_id,
            "Power".to_owned(),
            "WakeUp".to_owned(),
            None,
        )
        .await
    }

    pub async fn command_power_turn_off(
        &mut self,
        console_live_id: String,
    ) -> Result<models::CommandResponse> {
        self.send_oneshot_command(
            console_live_id,
            "Power".to_owned(),
            "TurnOff".to_owned(),
            None,
        )
        .await
    }

    pub async fn command_power_reboot(
        &mut self,
        console_live_id: String,
    ) -> Result<models::CommandResponse> {
        self.send_oneshot_command(
            console_live_id,
            "Power".to_owned(),
            "Reboot".to_owned(),
            None,
        )
        .await
    }

    pub async fn command_audio_mute(
        &mut self,
        console_live_id: String,
    ) -> Result<models::CommandResponse> {
        self.send_oneshot_command(console_live_id, "Audio".to_owned(), "Mute".to_owned(), None)
            .await
    }

    pub async fn command_audio_unmute(
        &mut self,
        console_live_id: String,
    ) -> Result<models::CommandResponse> {
        self.send_oneshot_command(
            console_live_id,
            "Audio".to_owned(),
            "Unmute".to_owned(),
            None,
        )
        .await
    }

    pub async fn command_audio_volume(
        &mut self,
        console_live_id: String,
        direction: models::VolumeDirection,
        amount: Option<i32>,
    ) -> Result<models::CommandResponse> {
        let mut parameters: Vec<HashMap<String, String>> = vec![HashMap::new()];
        parameters[0].insert("direction".to_owned(), direction.to_string());
        parameters[0].insert("amount".to_owned(), amount.unwrap_or(1).to_string());

        self.send_oneshot_command(
            console_live_id,
            "Audio".to_owned(),
            "Volume".to_owned(),
            Some(parameters),
        )
        .await
    }

    pub async fn command_config_digital_assistant_remote_control(
        &mut self,
        console_live_id: String,
    ) -> Result<models::CommandResponse> {
        self.send_oneshot_command(
            console_live_id,
            "Config".to_owned(),
            "DigitalAssistantRemoteControl".to_owned(),
            None,
        )
        .await
    }

    pub async fn command_config_remote_access(
        &mut self,
        console_live_id: String,
        enable: bool,
    ) -> Result<models::CommandResponse> {
        let mut parameters: Vec<HashMap<String, String>> = vec![HashMap::new()];
        parameters[0].insert("enabled".to_owned(), enable.to_string().to_lowercase());

        self.send_oneshot_command(
            console_live_id,
            "Config".to_owned(),
            "RemoteAccess".to_owned(),
            Some(parameters),
        )
        .await
    }

    pub async fn command_config_allow_console_streaming(
        &mut self,
        console_live_id: String,
        enable: bool,
    ) -> Result<models::CommandResponse> {
        let mut parameters: Vec<HashMap<String, String>> = vec![HashMap::new()];
        parameters[0].insert("enabled".to_owned(), enable.to_string().to_lowercase());

        self.send_oneshot_command(
            console_live_id,
            "Config".to_owned(),
            "AllowConsoleStreaming".to_owned(),
            Some(parameters),
        )
        .await
    }

    pub async fn command_game_capture_gameclip(
        &mut self,
        console_live_id: String,
    ) -> Result<models::CommandResponse> {
        self.send_oneshot_command(
            console_live_id,
            "Game".to_owned(),
            "CaptureGameClip".to_owned(),
            None,
        )
        .await
    }

    pub async fn command_game_capture_screenshot(
        &mut self,
        console_live_id: String,
    ) -> Result<models::CommandResponse> {
        self.send_oneshot_command(
            console_live_id,
            "Game".to_owned(),
            "CaptureScreenshot".to_owned(),
            None,
        )
        .await
    }

    pub async fn command_game_invite_party_to_game(
        &mut self,
        console_live_id: String,
    ) -> Result<models::CommandResponse> {
        self.send_oneshot_command(
            console_live_id,
            "Game".to_owned(),
            "InvitePartyToGame".to_owned(),
            None,
        )
        .await
    }

    pub async fn command_game_invite_to_party(
        &mut self,
        console_live_id: String,
    ) -> Result<models::CommandResponse> {
        self.send_oneshot_command(
            console_live_id,
            "Game".to_owned(),
            "InviteToParty".to_owned(),
            None,
        )
        .await
    }

    pub async fn command_game_kick_from_party(
        &mut self,
        console_live_id: String,
    ) -> Result<models::CommandResponse> {
        self.send_oneshot_command(
            console_live_id,
            "Game".to_owned(),
            "KickFromParty".to_owned(),
            None,
        )
        .await
    }

    pub async fn command_game_leave_party(
        &mut self,
        console_live_id: String,
    ) -> Result<models::CommandResponse> {
        self.send_oneshot_command(
            console_live_id,
            "Game".to_owned(),
            "LeaveParty".to_owned(),
            None,
        )
        .await
    }

    pub async fn command_game_set_online_status(
        &mut self,
        console_live_id: String,
    ) -> Result<models::CommandResponse> {
        self.send_oneshot_command(
            console_live_id,
            "Game".to_owned(),
            "SetOnlineStatus".to_owned(),
            None,
        )
        .await
    }

    pub async fn command_game_start_a_party(
        &mut self,
        console_live_id: String,
    ) -> Result<models::CommandResponse> {
        self.send_oneshot_command(
            console_live_id,
            "Game".to_owned(),
            "StartAParty".to_owned(),
            None,
        )
        .await
    }

    pub async fn command_game_start_broadcasting(
        &mut self,
        console_live_id: String,
    ) -> Result<models::CommandResponse> {
        self.send_oneshot_command(
            console_live_id,
            "Game".to_owned(),
            "StartBroadcasting".to_owned(),
            None,
        )
        .await
    }

    pub async fn command_game_stop_broadcasting(
        &mut self,
        console_live_id: String,
    ) -> Result<models::CommandResponse> {
        self.send_oneshot_command(
            console_live_id,
            "Game".to_owned(),
            "StopBroadcasting".to_owned(),
            None,
        )
        .await
    }

    pub async fn command_gamestreaming_start_management_service(
        &mut self,
        console_live_id: String,
    ) -> Result<models::CommandResponse> {
        self.send_oneshot_command(
            console_live_id,
            "GameStreaming".to_owned(),
            "StartStreamingManagementService".to_owned(),
            None,
        )
        .await
    }

    pub async fn command_gamestreaming_stop_streaming(
        &mut self,
        console_live_id: String,
    ) -> Result<models::CommandResponse> {
        self.send_oneshot_command(
            console_live_id,
            "GameStreaming".to_owned(),
            "StopStreaming".to_owned(),
            None,
        )
        .await
    }

    pub async fn command_marketplace_redeem_code(
        &mut self,
        console_live_id: String,
    ) -> Result<models::CommandResponse> {
        self.send_oneshot_command(
            console_live_id,
            "Marketplace".to_owned(),
            "RedeemCode".to_owned(),
            None,
        )
        .await
    }

    pub async fn command_marketplace_search(
        &mut self,
        console_live_id: String,
    ) -> Result<models::CommandResponse> {
        self.send_oneshot_command(
            console_live_id,
            "Marketplace".to_owned(),
            "Search".to_owned(),
            None,
        )
        .await
    }

    pub async fn command_marketplace_search_store(
        &mut self,
        console_live_id: String,
    ) -> Result<models::CommandResponse> {
        self.send_oneshot_command(
            console_live_id,
            "Marketplace".to_owned(),
            "SearchTheStore".to_owned(),
            None,
        )
        .await
    }

    pub async fn command_marketplace_show_title(
        &mut self,
        console_live_id: String,
    ) -> Result<models::CommandResponse> {
        self.send_oneshot_command(
            console_live_id,
            "Marketplace".to_owned(),
            "ShowTitle".to_owned(),
            None,
        )
        .await
    }

    pub async fn command_media_command(
        &mut self,
        console_live_id: String,
        media_command: models::MediaCommand,
    ) -> Result<models::CommandResponse> {
        self.send_oneshot_command(
            console_live_id,
            "Media".to_owned(),
            media_command.to_string(),
            None,
        )
        .await
    }

    pub async fn command_shell_activate_app_with_uri(
        &mut self,
        console_live_id: String,
    ) -> Result<models::CommandResponse> {
        self.send_oneshot_command(
            console_live_id,
            "Shell".to_owned(),
            "ActivateApplicationWithUri".to_owned(),
            None,
        )
        .await
    }

    pub async fn command_shell_activate_app_with_aumid(
        &mut self,
        console_live_id: String,
    ) -> Result<models::CommandResponse> {
        self.send_oneshot_command(
            console_live_id,
            "Shell".to_owned(),
            "ActivateApplicationWithAumid".to_owned(),
            None,
        )
        .await
    }

    pub async fn command_shell_activate_app_with_onestore_product_id(
        &mut self,
        console_live_id: String,
        one_store_product_id: String,
    ) -> Result<models::CommandResponse> {
        let mut parameters: Vec<HashMap<String, String>> = vec![HashMap::new()];
        parameters[0].insert("oneStoreProductId".to_owned(), one_store_product_id);

        self.send_oneshot_command(
            console_live_id,
            "Shell".to_owned(),
            "ActivationApplicationWithOneStoreProductId".to_owned(),
            Some(parameters),
        )
        .await
    }

    pub async fn command_shell_allow_remote_management(
        &mut self,
        console_live_id: String,
    ) -> Result<models::CommandResponse> {
        self.send_oneshot_command(
            console_live_id,
            "Shell".to_owned(),
            "AllowRemoteManagement".to_owned(),
            None,
        )
        .await
    }

    pub async fn command_shell_change_view(
        &mut self,
        console_live_id: String,
    ) -> Result<models::CommandResponse> {
        self.send_oneshot_command(
            console_live_id,
            "Shell".to_owned(),
            "ChangeView".to_owned(),
            None,
        )
        .await
    }

    pub async fn command_shell_check_for_package_updates(
        &mut self,
        console_live_id: String,
    ) -> Result<models::CommandResponse> {
        self.send_oneshot_command(
            console_live_id,
            "Shell".to_owned(),
            "CheckForPackageUpdates".to_owned(),
            None,
        )
        .await
    }

    pub async fn command_shell_copy_packages(
        &mut self,
        console_live_id: String,
    ) -> Result<models::CommandResponse> {
        self.send_oneshot_command(
            console_live_id,
            "Shell".to_owned(),
            "CopyPackages".to_owned(),
            None,
        )
        .await
    }

    pub async fn command_shell_move_packages(
        &mut self,
        console_live_id: String,
    ) -> Result<models::CommandResponse> {
        self.send_oneshot_command(
            console_live_id,
            "Shell".to_owned(),
            "MovePackages".to_owned(),
            None,
        )
        .await
    }

    pub async fn command_shell_install_packages(
        &mut self,
        console_live_id: String,
        big_cat_ids: Vec<String>,
    ) -> Result<models::CommandResponse> {
        let mut parameters: Vec<HashMap<String, String>> = vec![HashMap::new()];
        parameters[0].insert("bigCatIdList".to_owned(), big_cat_ids.join(","));

        self.send_oneshot_command(
            console_live_id,
            "Shell".to_owned(),
            "InstallPackages".to_owned(),
            Some(parameters),
        )
        .await
    }

    pub async fn command_shell_uninstall_package(
        &mut self,
        console_live_id: String,
        instance_id: String,
    ) -> Result<models::CommandResponse> {
        let mut parameters: Vec<HashMap<String, String>> = vec![HashMap::new()];
        parameters[0].insert("instanceId".to_owned(), instance_id);

        self.send_oneshot_command(
            console_live_id,
            "Shell".to_owned(),
            "UninstallPackage".to_owned(),
            Some(parameters),
        )
        .await
    }

    pub async fn command_shell_update_packages(
        &mut self,
        console_live_id: String,
    ) -> Result<models::CommandResponse> {
        self.send_oneshot_command(
            console_live_id,
            "Shell".to_owned(),
            "UpdatePackages".to_owned(),
            None,
        )
        .await
    }

    pub async fn command_shell_eject_disk(
        &mut self,
        console_live_id: String,
    ) -> Result<models::CommandResponse> {
        self.send_oneshot_command(
            console_live_id,
            "Shell".to_owned(),
            "EjectDisk".to_owned(),
            None,
        )
        .await
    }

    pub async fn command_shell_go_back(
        &mut self,
        console_live_id: String,
    ) -> Result<models::CommandResponse> {
        self.send_oneshot_command(
            console_live_id,
            "Shell".to_owned(),
            "GoBack".to_owned(),
            None,
        )
        .await
    }

    pub async fn command_shell_go_home(
        &mut self,
        console_live_id: String,
    ) -> Result<models::CommandResponse> {
        self.send_oneshot_command(
            console_live_id,
            "Shell".to_owned(),
            "GoHome".to_owned(),
            None,
        )
        .await
    }

    pub async fn command_shell_pair_controller(
        &mut self,
        console_live_id: String,
    ) -> Result<models::CommandResponse> {
        self.send_oneshot_command(
            console_live_id,
            "Shell".to_owned(),
            "PairController".to_owned(),
            None,
        )
        .await
    }

    pub async fn command_shell_send_text_message(
        &mut self,
        console_live_id: String,
    ) -> Result<models::CommandResponse> {
        self.send_oneshot_command(
            console_live_id,
            "Shell".to_owned(),
            "SendTextMessage".to_owned(),
            None,
        )
        .await
    }

    pub async fn command_shell_show_guide_tab(
        &mut self,
        console_live_id: String,
    ) -> Result<models::CommandResponse> {
        self.send_oneshot_command(
            console_live_id,
            "Shell".to_owned(),
            "ShowGuideTab".to_owned(),
            None,
        )
        .await
    }

    pub async fn command_shell_sign_in(
        &mut self,
        console_live_id: String,
    ) -> Result<models::CommandResponse> {
        self.send_oneshot_command(
            console_live_id,
            "Shell".to_owned(),
            "SignIn".to_owned(),
            None,
        )
        .await
    }

    pub async fn command_shell_sign_out(
        &mut self,
        console_live_id: String,
    ) -> Result<models::CommandResponse> {
        self.send_oneshot_command(
            console_live_id,
            "Shell".to_owned(),
            "SignOut".to_owned(),
            None,
        )
        .await
    }

    pub async fn command_shell_launch_game(
        &mut self,
        console_live_id: String,
    ) -> Result<models::CommandResponse> {
        self.send_oneshot_command(
            console_live_id,
            "Shell".to_owned(),
            "LaunchGame".to_owned(),
            None,
        )
        .await
    }

    pub async fn command_shell_terminate_application(
        &mut self,
        console_live_id: String,
    ) -> Result<models::CommandResponse> {
        self.send_oneshot_command(
            console_live_id,
            "Shell".to_owned(),
            "TerminateApplication".to_owned(),
            None,
        )
        .await
    }

    pub async fn command_shell_keyinput(
        &mut self,
        console_live_id: String,
        key_type: models::InputKeyType,
    ) -> Result<models::CommandResponse> {
        let mut parameters: Vec<HashMap<String, String>> = vec![HashMap::new()];
        parameters[0].insert("keyType".to_owned(), key_type.to_string());

        self.send_oneshot_command(
            console_live_id,
            "Shell".to_owned(),
            "InjectKey".to_owned(),
            Some(parameters),
        )
        .await
    }

    pub async fn command_shell_textinput(
        &mut self,
        console_live_id: String,
        text_input: String,
    ) -> Result<models::CommandResponse> {
        let mut parameters: Vec<HashMap<String, String>> = vec![HashMap::new()];
        parameters[0].insert("replacementString".to_owned(), text_input);

        self.send_oneshot_command(
            console_live_id,
            "Shell".to_owned(),
            "InjectString".to_owned(),
            Some(parameters),
        )
        .await
    }

    pub async fn command_tv_show_guide(
        &mut self,
        console_live_id: String,
    ) -> Result<models::CommandResponse> {
        self.send_oneshot_command(
            console_live_id,
            "TV".to_owned(),
            "ShowGuide".to_owned(),
            None,
        )
        .await
    }

    pub async fn command_tv_watch_channel(
        &mut self,
        console_live_id: String,
    ) -> Result<models::CommandResponse> {
        self.send_oneshot_command(
            console_live_id,
            "TV".to_owned(),
            "WatchChannel".to_owned(),
            None,
        )
        .await
    }
}
