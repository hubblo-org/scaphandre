//! Scaphandre is an extensible monitoring agent for energy consumption metrics.
//!
//! It gathers energy consumption data from the system or other data sources thanks to components called *sensors*.
//!
//! Final monitoring data is sent to or exposed for monitoring tools thanks to *exporters*.
#[macro_use]
extern crate log;
pub mod exporters;
pub mod sensors;

#[cfg(target_os = "windows")]
use sensors::msr_rapl;

#[cfg(target_os = "linux")]
use sensors::powercap_rapl;

/// Create a new [`Sensor`] instance with the default sensor available,
/// with its default options.
pub fn get_default_sensor() -> impl sensors::Sensor {
    #[cfg(target_os = "linux")]
    return powercap_rapl::PowercapRAPLSensor::new(
        powercap_rapl::DEFAULT_BUFFER_PER_SOCKET_MAX_KBYTES,
        powercap_rapl::DEFAULT_BUFFER_PER_DOMAIN_MAX_KBYTES,
        false,
    );

    #[cfg(target_os = "windows")]
    return msr_rapl::MsrRAPLSensor::new();
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
