use super::models;

pub const IOS_XBOXBETA_PARAMS: models::XalClientParameters = {
    models::XalClientParameters {
        user_agent: "XAL iOS 2020.07.20200714.000",
        app_id: "000000004415494b",
        device_type: models::DeviceType::IOS,
        client_version: "14.0.1",
        title_id: "177887386",
        redirect_uri: "ms-xal-000000004415494b://auth",
        query_display: "ios_phone",
    }
};

pub const IOS_XBOX_PARAMS: models::XalClientParameters = {
    models::XalClientParameters {
        user_agent: "XAL iOS 2021.11.20211021.000",
        app_id: "000000004c12ae6f",
        device_type: models::DeviceType::IOS,
        client_version: "15.6.1",
        title_id: "328178078",
        redirect_uri: "ms-xal-000000004c12ae6f://auth",
        query_display: "ios_phone",
    }
};

pub const ANDROID_XBOXBETA_PARAMS: models::XalClientParameters = {
    models::XalClientParameters {
        user_agent: "XAL Android 2020.07.20200714.000",
        app_id: "000000004415494b",
        device_type: models::DeviceType::ANDROID,
        client_version: "8.0.0",
        title_id: "177887386",
        redirect_uri: "ms-xal-000000004415494b://auth",
        query_display: "android_phone",
    }
};

pub const ANDROID_GAMEPASS_BETA_PARAMS: models::XalClientParameters = {
    models::XalClientParameters {
        user_agent: "XAL Android 2020.07.20200714.000",
        app_id: "000000004c20a908",
        device_type: models::DeviceType::ANDROID,
        client_version: "8.0.0",
        title_id: "1016898439",
        redirect_uri: "ms-xal-public-beta-000000004c20a908://auth",
        query_display: "android_phone",
    }
};
