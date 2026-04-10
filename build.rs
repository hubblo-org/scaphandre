use std::{
    env,
    fs::{self},
    path::Path,
};

#[cfg(all(target_os = "linux", feature = "disks_evaluation"))]
// Build script to parse the CSV file in data/disks, in order to have the CSV records available
// from build time in the binary. For unitary tests, the fixture in tests/fixtures in parsed
// instead. 
fn main() {
    let out_dir = env::var_os("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("csv_records.rs");
    fs::write(
        &dest_path,
        "pub fn get_default_power_model_path() -> PathBuf {
        let cargo_path = env!(\"CARGO_MANIFEST_DIR\");
        
        let power_model_path = Path::new(cargo_path).join(\"data/disks/power_model.csv\");
        power_model_path
            }

        #[cfg(test)]
        pub fn get_test_power_model_path() -> PathBuf {
        let cargo_path = env!(\"CARGO_MANIFEST_DIR\");
        
        let power_model_path = Path::new(cargo_path).join(\"tests/fixtures/disk_power.csv\");
        power_model_path
            }
    
        pub fn parse_csv() -> Reader<File> {
        #[cfg(test)]
        let path = get_test_power_model_path();
        #[cfg(not(test))]
        let path = get_default_power_model_path();
        let records = csv::Reader::from_path(path).unwrap();
        records
            }",
    )
    .unwrap();
    println!("cargo::rerun-if-changed=build.rs")
}
