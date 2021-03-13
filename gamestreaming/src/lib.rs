pub extern crate pnet;
pub extern crate teredo;
pub extern crate byteorder;
#[macro_use]
pub extern crate bitflags;

pub mod crypto;
pub mod models;
pub mod packets;
pub mod webrtc;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
