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

//  Copyright 2020 The scaphandre authors.
//
//  Licensed under the Apache License, Version 2.0 (the "License");
//  you may not use this file except in compliance with the License.
//  You may obtain a copy of the License at
//
//      http://www.apache.org/licenses/LICENSE-2.0
//
//  Unless required by applicable law or agreed to in writing, software
//  distributed under the License is distributed on an "AS IS" BASIS,
//  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
//  See the License for the specific language governing permissions and
//  limitations under the License.
