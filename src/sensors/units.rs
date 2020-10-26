use crate::sensors::utils::type_of;

// !!!!!!!!!!!!!!!!! Unit !!!!!!!!!!!!!!!!!!!!!!!
#[derive(Debug)]
pub enum EnergyUnit {
    Joule,
    MilliJoule,
    MicroJoule,
}

impl EnergyUnit {
    fn to(measure: f64, source_unit: &EnergyUnit, dest_unit: &EnergyUnit) -> Result<f64, String>{
        let order = [EnergyUnit::Joule, EnergyUnit::MilliJoule, EnergyUnit::MicroJoule];
        let pos_source = order.iter().position(|x| x == source_unit).unwrap();
        let pos_dest = order.iter().position(|x| x == dest_unit).unwrap();
        println!("{} {}", pos_source, pos_dest);
        let mut mult: f64 = 1.0;
        if pos_dest > pos_source {
            // source < dest
            for _ in 0..(pos_dest - pos_source) {
               mult = mult * 1000.0; 
            }
        } else if pos_dest < pos_source {
            println!("COUCOU");
            // source > dest
            for _ in 0..(pos_source - pos_dest) {
               mult = mult / 1000.0; 
            }   
        }
        Ok(measure * mult)
    }
}

impl Eq for EnergyUnit {}
impl PartialEq for EnergyUnit{
    fn eq(&self, other: &Self) -> bool {
        format!("{:?}", self) == format!("{:?}", other)
    }
}


#[derive(Debug)]
pub enum PowerUnit {
    MegaWatt,
    KiloWatt ,
    Watt,
    MilliWatt,
    MicroWatt
}

impl PowerUnit {
    fn to(measure: f64, source_unit: &PowerUnit, dest_unit: &PowerUnit) -> Result<f64, String>{
        let order = [PowerUnit::MegaWatt, PowerUnit::KiloWatt, PowerUnit::Watt, PowerUnit::MilliWatt, PowerUnit::MicroWatt];
        let pos_source = order.iter().position(|x| x == source_unit).unwrap();
        let pos_dest = order.iter().position(|x| x == dest_unit).unwrap();
        println!("{} {}", pos_source, pos_dest);
        let mut mult: f64 = 1.0;
        if pos_dest > pos_source {
            // source < dest
            for _ in 0..(pos_dest - pos_source) {
               mult = mult * 1000.0; 
            }
        } else if pos_dest < pos_source {
            println!("COUCOU");
            // source > dest
            for _ in 0..(pos_source - pos_dest) {
               mult = mult / 1000.0; 
            }   
        }
        Ok(measure * mult)
    }

}

impl Eq for PowerUnit {}
impl PartialEq for PowerUnit{
    fn eq(&self, other: &Self) -> bool {
        format!("{:?}", self) == format!("{:?}", other)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn kw_equals_1000w(){
        let value = 1.0;
        let source = PowerUnit::KiloWatt;
        let dest = PowerUnit::Watt;
        assert_eq!(PowerUnit::to(value, &source, &dest).unwrap(), 1000.0);
    }
    #[test]
    fn kw_equals_0001megawatt(){
        let value = 1.0;
        let source = PowerUnit::KiloWatt;
        let dest = PowerUnit::MegaWatt;
        assert_eq!(PowerUnit::to(value, &source, &dest).unwrap(), 0.001);
    }

    #[test]
    fn joule_equals_1000000microjoules() {
        let value = 1.0;
        let source = EnergyUnit::Joule;
        let dest = EnergyUnit::MicroJoule;
        assert_eq!(EnergyUnit::to(value, &source, &dest).unwrap(), 1000000.0);
    }
}