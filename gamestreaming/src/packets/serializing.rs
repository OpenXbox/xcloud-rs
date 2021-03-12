use std::io::{Read, Write, Seek};

type Error = Box<dyn std::error::Error>;
type Result<T> = std::result::Result<T, Error>;

pub trait Serialize {
    fn serialize(writer: dyn Write) -> usize;
}

pub trait Deserialize: Sized {
    fn deserialize<T: Read + Seek>(reader: &mut T) -> Result<Self>;
}