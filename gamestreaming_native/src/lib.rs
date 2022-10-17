#![allow(dead_code)]

pub extern crate byteorder;
pub extern crate pnet;
pub extern crate teredo;
#[macro_use]
pub extern crate bitflags;

pub extern crate webrtc;

pub mod crypto;
pub mod models;
pub mod packets;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
