pub extern crate webrtc_rs_stun as stun;
pub extern crate webrtc_rs_srtp as srtp;
pub extern crate webrtc_rs_rtp as rtp;

pub mod models;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
