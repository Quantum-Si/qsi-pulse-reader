use pyo3::prelude::*;
use pyo3::types::PyDict;
use qsi_pulse_reader::pulse_filter::PulseFilter as RustPulseFilter;

#[pyclass]
#[derive(Clone)]
pub struct PulseFilter {
    pub pulse_filter: RustPulseFilter,
}

#[pymethods]
impl PulseFilter {
    /// Create a new `PulseFilter` instance.
    ///
    /// This method allows you to create a new `PulseFilter` instance with
    /// specified parameters.
    ///
    /// # Parameters
    /// - `min_dur_f`: Minimum pulse duration in frames (optional).
    /// - `min_dur_s`: Minimum pulse duration in seconds (optional).
    /// - `max_dur_s`: Maximum pulse duration in seconds (optional).
    /// - `min_snr`: Minimum pulse signal-to-noise ratio (optional).
    /// - `min_intensity`: Minimum pulse intensity (optional).
    /// - `min_binratio`: Minimum pulse bin ratio (optional).
    /// - `max_binratio`: Maximum pulse bin ratio (optional).
    /// - `start_m`: Start time in minutes (optional).
    /// - `end_m`: End time in minutes (optional).
    /// - `mask_s`: Exclude pulses in the indicated time interval in seconds (optional).
    /// - `recalc_ipd`: Recalculate inter-pulse duration (optional).
    ///
    /// # Returns
    /// - A new `PulseFilter` instance with the specified parameters.
    ///
    /// # Example
    /// ```python
    /// from qsi_pulse_reader import PulseFilter
    /// pulse_filter = PulseFilter(min_dur_f=10, min_snr=5.0)
    /// ```
    #[pyo3(signature = (**kwargs))]
    #[new]
    pub fn new(kwargs: Option<&Bound<'_, PyDict>>) -> PyResult<Self> {
        let mut pulse_filter = RustPulseFilter::default();
        if let Some(kwargs) = kwargs {
            for (key, value) in kwargs.iter() {
                match key.extract::<&str>()? {
                    "min_dur_f" => pulse_filter.min_dur_f = Some(value.extract()?),
                    "min_dur_s" => pulse_filter.min_dur_s = Some(value.extract()?),
                    "max_dur_s" => pulse_filter.max_dur_s = Some(value.extract()?),
                    "min_snr" => pulse_filter.min_snr = Some(value.extract()?),
                    "min_intensity" => pulse_filter.min_intensity = Some(value.extract()?),
                    "min_binratio" => pulse_filter.min_binratio = Some(value.extract()?),
                    "max_binratio" => pulse_filter.max_binratio = Some(value.extract()?),
                    "start_m" => pulse_filter.start_m = Some(value.extract()?),
                    "end_m" => pulse_filter.end_m = Some(value.extract()?),
                    "mask_s" => pulse_filter.mask_s = Some(value.extract()?),
                    "recalc_ipd" => pulse_filter.recalc_ipd = value.extract()?,
                    _ => {}
                }
            }
        }
        Ok(PulseFilter { pulse_filter })
    }
}
