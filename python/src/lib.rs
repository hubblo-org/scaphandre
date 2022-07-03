#![deny(warnings)]

extern crate pyo3;

use pyo3::create_exception;
use pyo3::exceptions::PyException;
use pyo3::prelude::*;
use scaphandre::sensors;
use scaphandre::sensors::powercap_rapl;
use scaphandre::sensors::units;
use sensors::{powercap_rapl::PowercapRAPLSensor, Sensor};
use std::error::Error;
use std::time::Duration;

create_exception!(scaphandre, PyScaphandreError, PyException);

impl PyScaphandreError {
    fn from_error(err: Box<dyn Error>) -> pyo3::PyErr {
        PyScaphandreError::new_err(err.to_string())
    }
}

#[pyclass]
struct RawScaphandre {
    _scaphandre: powercap_rapl::PowercapRAPLSensor,
    #[pyo3(get)]
    sensor_name: String,
}

#[pymethods]
impl RawScaphandre {
    #[new]
    fn new(
        buffer_per_socket_max_kbytes: u16,
        buffer_per_domain_max_kbytes: u16,
        is_virtual_machine: bool,
    ) -> PyResult<Self> {
        let sensor = PowercapRAPLSensor::new(
            buffer_per_socket_max_kbytes,
            buffer_per_domain_max_kbytes,
            is_virtual_machine,
        );
        Ok(RawScaphandre {
            _scaphandre: sensor,
            sensor_name: "PowercapRAPL".to_string(),
        })
    }

    fn is_compatible(&self) -> bool {
        matches!(PowercapRAPLSensor::check_module(), Ok(_))
    }

    fn get_energy_consumption_measures(&self) -> PyResult<Vec<RawEnergyRecord>> {
        Ok(self
            ._scaphandre
            .generate_topology()
            .map_err(PyScaphandreError::from_error)?
            .record_buffer
            .iter()
            .map(|record| RawEnergyRecord {
                _timestamp: record.timestamp,
                _value: record.value.clone(),
                _unit: record.unit,
            })
            .collect())
    }
}

#[pyclass]
struct RawEnergyRecord {
    _timestamp: Duration,
    _value: String,
    _unit: units::Unit,
}

#[pyfunction]
fn rust_core_version() -> &'static str {
    scaphandre::crate_version()
}

#[pymodule]
// module name need to match project name
fn scaphandre(py: Python, m: &PyModule) -> PyResult<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn")).init();

    m.add_function(pyo3::wrap_pyfunction!(rust_core_version, m)?)?;
    m.add_class::<RawScaphandre>()?;
    m.add_class::<RawEnergyRecord>()?;
    m.add("PyScaphandreError", py.get_type::<PyScaphandreError>())?;
    Ok(())
}
