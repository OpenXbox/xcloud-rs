#[derive(Debug, Clone, PartialEq)]
pub enum QosPacketType {
    ServerHandshake = 1,
    ClientHandshake = 2,
    Control = 3,
    Data = 4,
    ServerPolicy = 5,
    ClientPolicy = 6,
}

#[derive(Debug, Clone, PartialEq)]
pub enum QosControlFlags {
    Reinitialize = 0x01
}

#[derive(Debug, Clone, PartialEq)]
pub struct QosServerPolicy {
    pub schema_version: u32,
    pub policy_length: u32,
    pub fragment_count: u32,
    pub offset: u32,
    pub fragment_size: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct QosServerHandshake {
    pub protocol_version: u32,
    pub min_supported_client_version: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct QosClientPolicy {
    pub schema_version: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct QosClientHandshake {
    pub protocol_version: u32,
    pub initial_frame_id: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct QosControl {
    pub flags: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct QosData {
    pub flags: u32,
    pub frame_id: u32,
}


#[derive(Debug, Clone, PartialEq)]
pub enum QosPacket {
    ServerHandshake(QosServerHandshake),
    ClientHandshake(QosClientHandshake),
    Control(QosControl),
    Data(QosData),
    ServerPolicy(QosServerPolicy),
    ClientPolicy(QosClientPolicy),
}