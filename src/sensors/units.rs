use std::{cmp::Ordering, fmt};

// !!!!!!!!!!!!!!!!! Unit !!!!!!!!!!!!!!!!!!!!!!!
#[derive(Debug)]
pub enum Unit {
    Joule,
    MilliJoule,
    MicroJoule,
    MegaWatt,
    KiloWatt,
    Watt,
    MilliWatt,
    MicroWatt,
    Percentage,
}

impl Unit {
    /// Converts either an energy measurement (Joule, MilliJoule or MicroJoule) to another energy Unit (Joule, MilliJoule or MicroJoule)
    /// or a power measurement (MilliWatt, MicroWatt, Watt, KiloWatt) to another power Unit
    pub fn to(measure: f64, source_unit: &Unit, dest_unit: &Unit) -> Result<f64, String> {
        let energy_order = [Unit::Joule, Unit::MilliJoule, Unit::MicroJoule];
        let power_order = [
            Unit::MegaWatt,
            Unit::KiloWatt,
            Unit::Watt,
            Unit::MilliWatt,
            Unit::MicroWatt,
        ];
        let pos_source_energy = energy_order.iter().position(|x| x == source_unit);
        let pos_dest_energy = energy_order.iter().position(|x| x == dest_unit);
        let pos_source_power = power_order.iter().position(|x| x == source_unit);
        let pos_dest_power = power_order.iter().position(|x| x == dest_unit);
        if let (Some(pos_source), Some(pos_dest)) = (pos_source_energy, pos_dest_energy) {
            Ok(measure * Unit::get_mult(pos_source, pos_dest))
        } else if let (Some(pos_source), Some(pos_dest)) = (pos_source_power, pos_dest_power) {
            Ok(measure * Unit::get_mult(pos_source, pos_dest))
        } else {
            panic!("Impossible conversion asked from energy value to power value (without time dimension).");
        }
    }

    /// Helper func to compute the multiplicative factor needed for a conversion
    fn get_mult(pos_source: usize, pos_dest: usize) -> f64 {
        let mut mult: f64 = 1.0;
        match pos_dest.cmp(&pos_source) {
            Ordering::Greater => mult *= 1000.0_f64.powf((pos_dest - pos_source) as f64),
            Ordering::Less => mult /= 1000.0_f64.powf((pos_source - pos_dest) as f64),
            Ordering::Equal => (),
        }
        mult
    }
}

impl fmt::Display for Unit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Unit::Joule => write!(f, "Joules"),
            Unit::MilliJoule => write!(f, "MilliJoules"),
            Unit::MicroJoule => write!(f, "MicroJoules"),
            Unit::MilliWatt => write!(f, "MilliWatts"),
            Unit::MicroWatt => write!(f, "MicroWatts"),
            Unit::Watt => write!(f, "Watts"),
            Unit::KiloWatt => write!(f, "KiloWatts"),
            Unit::MegaWatt => write!(f, "MegaWatts"),
            Unit::Percentage => write!(f, "Percentage"),
        }
    }
}

impl Eq for Unit {}
impl PartialEq for Unit {
    fn eq(&self, other: &Self) -> bool {
        format!("{:?}", self) == format!("{:?}", other)
    }
}

impl Copy for Unit {}

impl Clone for Unit {
    fn clone(&self) -> Self {
        *self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn kw_equals_1000w() {
        let value = 1.0;
        let source = Unit::KiloWatt;
        let dest = Unit::Watt;
        assert_eq!(Unit::to(value, &source, &dest).unwrap(), 1000.0);
    }
    #[test]
    fn kw_equals_0001megawatt() {
        let value = 1.0;
        let source = Unit::KiloWatt;
        let dest = Unit::MegaWatt;
        assert_eq!(Unit::to(value, &source, &dest).unwrap(), 0.001);
    }

    #[test]
    fn kw_to_milliwatt() {
        let value = 2.0;
        let source = Unit::KiloWatt;
        let dest = Unit::MilliWatt;
        assert_eq!(Unit::to(value, &source, &dest).unwrap(), 2000000.0);
    }

    #[test]
    fn milliwatt_to_watt() {
        let value = 6.0;
        let source = Unit::MilliWatt;
        let dest = Unit::Watt;
        assert_eq!(Unit::to(value, &source, &dest).unwrap(), 0.006);
    }

    #[test]
    fn megawatt_to_microwatt() {
        let value = 12.0;
        let source = Unit::MegaWatt;
        let dest = Unit::MicroWatt;
        assert_eq!(Unit::to(value, &source, &dest).unwrap(), 12000000000000.0);
    }

    #[test]
    fn joule_equals_1000000microjoules() {
        let value = 1.0;
        let source = Unit::Joule;
        let dest = Unit::MicroJoule;
        assert_eq!(Unit::to(value, &source, &dest).unwrap(), 1000000.0);
    }

    #[test]
    fn joule_to_milijoules() {
        let value = 2.0;
        let source = Unit::Joule;
        let dest = Unit::MilliJoule;
        assert_eq!(Unit::to(value, &source, &dest).unwrap(), 2000.0);
    }

    #[test]
    fn milijoule_to_joules() {
        let value = 4000.0;
        let source = Unit::MilliJoule;
        let dest = Unit::Joule;
        assert_eq!(Unit::to(value, &source, &dest).unwrap(), 4.0);
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
