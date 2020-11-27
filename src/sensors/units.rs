use std::fmt;

// !!!!!!!!!!!!!!!!! Unit !!!!!!!!!!!!!!!!!!!!!!!
#[derive(Debug)]
pub enum Unit {
    Joule,
    MilliJoule,
    MicroJoule,
    MegaWatt,
    KiloWatt ,
    Watt,
    MilliWatt,
    MicroWatt,
    Jiffries // time unit of usage, relative to the CPU
}

impl Unit {
    pub fn to(measure: f64, source_unit: &Unit, dest_unit: &Unit) -> Result<f64, String>{
        let energy_order = [
            Unit::Joule, Unit::MilliJoule, Unit::MicroJoule
        ];
        let power_order = [
            Unit::MegaWatt, Unit::KiloWatt, Unit::Watt,
            Unit::MicroWatt, Unit::MilliWatt
        ];
        let pos_source_energy = energy_order.iter().position(|x| x == source_unit);
        let pos_dest_energy = energy_order.iter().position(|x| x == dest_unit);
        let pos_source_power = power_order.iter().position(|x| x == source_unit);
        let pos_dest_power = power_order.iter().position(|x| x == dest_unit);
        if pos_source_energy.is_some() && pos_dest_energy.is_some() {
           let pos_source = pos_source_energy.unwrap();
           let pos_dest = pos_dest_energy.unwrap(); 
            Ok(measure * Unit::get_mult(pos_source, pos_dest))
        } else if pos_source_power.is_some() && pos_dest_power.is_some() {
           let pos_source = pos_source_power.unwrap();
           let pos_dest = pos_dest_power.unwrap(); 
            Ok(measure * Unit::get_mult(pos_source, pos_dest))
        } else {
            panic!("Impossible conversion asked from energy value to power value (without time dimension).");
        }
    }

    fn get_mult(pos_source: usize, pos_dest: usize) -> f64{
        let mut mult: f64 = 1.0;
        if pos_dest > pos_source {
            // source < dest
            for _ in 0..(pos_dest - pos_source) {
               mult *= 1000.0; 
            }
        } else if pos_dest < pos_source {
            // source > dest
            for _ in 0..(pos_source - pos_dest) {
               mult /= 1000.0; 
            }   
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
            Unit::Jiffries => write!(f, "Jiffries")
        }
    }
}

impl Eq for Unit {}
impl PartialEq for Unit{
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
    fn kw_equals_1000w(){
        let value = 1.0;
        let source = Unit::KiloWatt;
        let dest = Unit::Watt;
        assert_eq!(Unit::to(value, &source, &dest).unwrap(), 1000.0);
    }
    #[test]
    fn kw_equals_0001megawatt(){
        let value = 1.0;
        let source = Unit::KiloWatt;
        let dest = Unit::MegaWatt;
        assert_eq!(Unit::to(value, &source, &dest).unwrap(), 0.001);
    }

    #[test]
    fn joule_equals_1000000microjoules() {
        let value = 1.0;
        let source = Unit::Joule;
        let dest = Unit::MicroJoule;
        assert_eq!(Unit::to(value, &source, &dest).unwrap(), 1000000.0);
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
