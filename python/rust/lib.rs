//! Python bindings for the QSI Pulses.bin reader library
//!
//! Provides the following functionality:
//! - `PulseFile`, a pulses.bin reader
//! - `PulseFilter`, a normalized pulse filter

mod pulse_filter;
mod pulse_reader;
mod records;
use pulse_filter::PulseFilter;
use pulse_reader::PulseReader;
use pyo3::prelude::*;

#[pymodule]
fn qsi_pulse_reader(_py: Python, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PulseReader>()?;
    m.add_class::<PulseFilter>()?;
    Ok(())
}
