use anyhow::Result;
use numpy::convert::IntoPyArray;
use pyo3::prelude::*;
use pyo3::types::PyDict;
use qsi_pulse_reader::pulse_reader::headers::PulseRecordType;
use qsi_pulse_reader::pulse_reader::records::{FormattedRecord, NormalizedPulse};

// Helper utility for converting a vec of structs into a dict of vecs
fn field_collect<P, T, F>(structs: &[P], f: F) -> Vec<T>
where
    F: Fn(&P) -> T,
{
    structs.iter().map(f).collect::<Vec<T>>()
}

/// Converts a vector of pulses or records into a Python dictionary
///
/// This trait is provided as a convenient way for converting a collection
/// of records or pulses into a Python dictionary that can be passed to
/// a Python process and then converted into a Pandas dataframe.
pub trait ToPyDict {
    fn to_pydict(
        &self,
        py: Python,
        aperture_index: Option<usize>,
        record_types: Option<&[PulseRecordType]>,
    ) -> Result<Py<PyDict>>;
}

impl ToPyDict for Vec<NormalizedPulse> {
    fn to_pydict(
        &self,
        py: Python,
        aperture_index: Option<usize>,
        _record_types: Option<&[PulseRecordType]>,
    ) -> Result<Py<PyDict>> {
        let pydict = PyDict::new(py);
        pydict.set_item(
            "index",
            field_collect(self, |p| p.index as u64).into_pyarray(py),
        )?;
        if let Some(ap) = aperture_index {
            pydict.set_item(
                "aperture_index",
                vec![ap as u64; self.len()].into_pyarray(py),
            )?;
        }
        pydict.set_item(
            "start_f",
            field_collect(self, |p| p.start_f).into_pyarray(py),
        )?;
        pydict.set_item("end_f", field_collect(self, |p| p.end_f).into_pyarray(py))?;
        pydict.set_item("dur_f", field_collect(self, |p| p.dur_f).into_pyarray(py))?;
        pydict.set_item("dur_s", field_collect(self, |p| p.dur_s).into_pyarray(py))?;
        pydict.set_item("ipd_f", field_collect(self, |p| p.ipd_f).into_pyarray(py))?;
        pydict.set_item("ipd_s", field_collect(self, |p| p.ipd_s).into_pyarray(py))?;
        pydict.set_item("snr", field_collect(self, |p| p.snr).into_pyarray(py))?;
        pydict.set_item(
            "intensity",
            field_collect(self, |p| p.intensity).into_pyarray(py),
        )?;
        pydict.set_item(
            "bin0_intensity",
            field_collect(self, |p| p.bin0_intensity).into_pyarray(py),
        )?;
        pydict.set_item(
            "intensity_display",
            field_collect(self, |p| p.intensity_display).into_pyarray(py),
        )?;
        pydict.set_item(
            "binratio",
            field_collect(self, |p| p.binratio).into_pyarray(py),
        )?;
        pydict.set_item(
            "bg_mean",
            field_collect(self, |p| p.bg_mean).into_pyarray(py),
        )?;
        pydict.set_item("bg_std", field_collect(self, |p| p.bg_std).into_pyarray(py))?;
        pydict.set_item(
            "bin0_bg_mean",
            field_collect(self, |p| p.bin0_bg_mean).into_pyarray(py),
        )?;
        pydict.set_item(
            "bin0_bg_std",
            field_collect(self, |p| p.bin0_bg_std).into_pyarray(py),
        )?;
        Ok(pydict.into())
    }
}

impl ToPyDict for Vec<FormattedRecord> {
    fn to_pydict(
        &self,
        py: Python,
        _aperture_index: Option<usize>,
        _record_types: Option<&[PulseRecordType]>,
    ) -> Result<Py<PyDict>> {
        let mut ipd_f = vec![0u16; self.len()];
        let mut dur_f = vec![0u16; self.len()];
        let mut bin0_intensity = vec![0f32; self.len()];
        let mut bin1_intensity = vec![0f32; self.len()];
        let mut bin0_bg_mean = vec![0f32; self.len()];
        let mut bin1_bg_mean = vec![0f32; self.len()];
        let mut bin0_bg_std = vec![0f32; self.len()];
        let mut bin1_bg_std = vec![0f32; self.len()];
        let mut start_f = vec![0u32; self.len()];
        let mut record_type: Vec<String> = Vec::new();
        let mut long_pulse_duration_f = vec![0f64; self.len()];
        let mut event_f = vec![0f64; self.len()];

        let mut last_record_end = 0u32;
        for (idx, record) in self.iter().enumerate() {
            ipd_f[idx] = record.frames_since_last;
            dur_f[idx] = record.duration;
            bin0_intensity[idx] = record.intensity0;
            bin1_intensity[idx] = record.intensity1;
            bin0_bg_mean[idx] = record.bg0;
            bin1_bg_mean[idx] = record.bg1;
            bin0_bg_std[idx] = record.sd0;
            bin1_bg_std[idx] = record.sd1;
            start_f[idx] = last_record_end + record.frames_since_last as u32;
            record_type.push(record.record_type.to_string());
            long_pulse_duration_f[idx] =
                record.long_pulse_num_frames.map_or(f64::NAN, |v| v as f64);
            event_f[idx] = record.event_frame.map_or(f64::NAN, |v| v as f64);
            last_record_end += record.frames_since_last as u32;
        }
        let pydict = PyDict::new(py);
        pydict.set_item("ipd_f", ipd_f.into_pyarray(py))?;
        pydict.set_item("dur_f", dur_f.into_pyarray(py))?;
        pydict.set_item("bin0_intensity", bin0_intensity.into_pyarray(py))?;
        pydict.set_item("bin1_intensity", bin1_intensity.into_pyarray(py))?;
        pydict.set_item("bin0_bg_mean", bin0_bg_mean.into_pyarray(py))?;
        pydict.set_item("bin1_bg_mean", bin1_bg_mean.into_pyarray(py))?;
        pydict.set_item("bin0_bg_std", bin0_bg_std.into_pyarray(py))?;
        pydict.set_item("bin1_bg_std", bin1_bg_std.into_pyarray(py))?;
        pydict.set_item("start_f", start_f.into_pyarray(py))?;
        pydict.set_item("record_type", record_type)?;
        pydict.set_item(
            "long_pulse_duration_f",
            long_pulse_duration_f.into_pyarray(py),
        )?;
        pydict.set_item("event_f", event_f.into_pyarray(py))?;
        Ok(pydict.into())
    }
}
