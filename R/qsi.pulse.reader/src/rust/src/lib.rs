//! R bindings for the QSI Pulses.bin reader library
//!
//! Provides the following functionality:
//! - `PulseFile`, a pulses.bin reader

mod pulse_reader;
mod records;

extendr_api::extendr_module! {
    mod qsi_pulse_reader;
    use pulse_reader;
}
