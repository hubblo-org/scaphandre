use std::convert;
use std::io;
use std::{error::Error, fmt};

#[derive(Debug)]
pub enum PowercapReadError {
    IoError(io::Error),
}
impl Error for PowercapReadError {
}

impl fmt::Display for PowercapReadError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Couldn't read from powercap sysfs !")
    }
}
impl convert::From<io::Error> for PowercapReadError {
    fn from(error: io::Error) -> Self {
        PowercapReadError::IoError(error)
    }
}