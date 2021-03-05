pub extern crate pnet;
pub extern crate teredo;

pub mod crypto;
pub mod models;
pub mod webrtc;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
