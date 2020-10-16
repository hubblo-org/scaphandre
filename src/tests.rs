use super::raw_read_from_powercap_sysfs;
#[test]
fn get_from_raw_read_from_powercap_sysfs() {
    let res = raw_read_from_powercap_sysfs().unwrap();
    println!("Result is: \n{}", res);
    assert_eq!(res.contains("Joules"), true);
} 