//! Python bindings for the QSI Pulses.bin reader library
//!
//! Provides the following functionality:
//! - `PulseFile`, a pulses.bin reader
//! - `PulseFilter`, a normalized pulse filter

pub mod pulse_filter;
pub mod pulse_reader;
mod records;
use pulse_filter::PulseFilter;
use pulse_reader::{PulseReader, merge_pulse_files};
use pyo3::prelude::*;

#[pymodule]
fn qsi_pulse_reader(_py: Python, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(merge_pulse_files, m)?)?;
    m.add_class::<PulseReader>()?;
    m.add_class::<PulseFilter>()?;
    Ok(())
}
