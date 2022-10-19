use std::{io::Cursor, str::FromStr};

use webrtc::sdp::SessionDescription;

// Wrapper around webrtc crate type
// Used for de/serializing
pub struct SdpSessionDescription(pub SessionDescription);

impl FromStr for SdpSessionDescription {
    type Err = webrtc::sdp::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut cursor = Cursor::new(s);
        Ok(SdpSessionDescription(SessionDescription::unmarshal(
            &mut cursor,
        )?))
    }
}

impl ToString for SdpSessionDescription {
    fn to_string(&self) -> String {
        SessionDescription::marshal(&self.0)
    }
}
