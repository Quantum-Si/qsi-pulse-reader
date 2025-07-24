use crate::pulse_reader::records::NormalizedPulse;
use anyhow::{Result, anyhow};

/// A pulse filter for normalized pulse records
///
/// This struct represents a set of pulse filter parameters, and provides
/// a method for classifying whether a NormalizedPulse passes the filter.
/// A value of `None` for one of the fields indicates that stage of the filter
/// is disabled.
#[derive(Clone, Debug)]
pub struct PulseFilter {
    pub min_dur_f: Option<u32>,
    pub min_dur_s: Option<f32>,
    pub max_dur_s: Option<f32>,
    pub min_snr: Option<f32>,
    pub min_intensity: Option<f32>,
    pub min_binratio: Option<f32>,
    pub max_binratio: Option<f32>,
    pub start_m: Option<f32>,
    pub end_m: Option<f32>,
    pub mask_s: Option<(f32, f32)>,
    pub recalc_ipd: bool,
}

impl Default for PulseFilter {
    fn default() -> Self {
        PulseFilter {
            min_dur_f: None,
            min_dur_s: None,
            max_dur_s: None,
            min_snr: None,
            min_intensity: None,
            min_binratio: None,
            max_binratio: None,
            start_m: None,
            end_m: None,
            mask_s: None,
            recalc_ipd: false,
        }
    }
}

impl PulseFilter {
    pub fn new(
        min_dur_f: Option<u32>,
        min_dur_s: Option<f32>,
        max_dur_s: Option<f32>,
        min_snr: Option<f32>,
        min_intensity: Option<f32>,
        min_binratio: Option<f32>,
        max_binratio: Option<f32>,
        start_m: Option<f32>,
        end_m: Option<f32>,
        mask_s: Option<(f32, f32)>,
        recalc_ipd: bool,
    ) -> Self {
        PulseFilter {
            min_dur_f,
            min_dur_s,
            max_dur_s,
            min_snr,
            min_intensity,
            min_binratio,
            max_binratio,
            start_m,
            end_m,
            mask_s,
            recalc_ipd,
        }
    }

    fn evaluate_filter(&self, pulse: &NormalizedPulse, fps: f32) -> bool {
        self.min_dur_f.map_or(true, |min| pulse.dur_f >= min)
            && self.min_dur_s.map_or(true, |min| pulse.dur_s >= min)
            && self.max_dur_s.map_or(true, |max| pulse.dur_s <= max)
            && self.min_snr.map_or(true, |min| pulse.snr >= min)
            && self
                .min_intensity
                .map_or(true, |min| pulse.intensity >= min)
            && self.min_binratio.map_or(true, |min| pulse.binratio >= min)
            && self.max_binratio.map_or(true, |max| pulse.binratio <= max)
            && self.start_m.map_or(true, |start_m| {
                (pulse.start_f as f32) / (60f32 * fps) >= start_m
            })
            && self
                .end_m
                .map_or(true, |end_m| (pulse.end_f as f32) / (60f32 * fps) <= end_m)
            && self.mask_s.map_or(true, |(start_s, end_s)| {
                ((pulse.start_f as f32) / fps > end_s) || ((pulse.end_f as f32) / fps < start_s)
            })
    }

    /// Filters a collection of normalized pulse records
    ///
    /// Filters a collection of pulses, returning a copy of all pulses that pass
    /// the filter. Optionally, will re-calculate the IPD of passing pulses by treating
    /// pulses that fail the filter as if they don't exist, adding their IPD and pulse
    /// duration to the IPD of the next passing pulse.
    ///
    /// # Examples
    ///
    /// ```
    /// # use qsi_pulse_reader::pulse_reader::PulseReader;
    /// use qsi_pulse_reader::pulse_filter::PulseFilter;
    /// # use std::path::PathBuf;
    ///
    /// # let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    /// # let pulse_file_path = path.join("../example_files/pulses.bin").to_string_lossy().to_string();
    /// # let mut pulse_reader = PulseReader::open(pulse_file_path).unwrap();
    /// # let fps = pulse_reader.fps;
    /// # let (mut pulses, ap_header) = pulse_reader.get_pulses(pulse_reader.index.apertures[0], None).unwrap();
    ///
    /// // Create PulseFilter with default options
    /// let pulse_filter: PulseFilter = Default::default();
    ///
    /// // Filter out pulses that fail the filter
    /// let filtered_pulses = pulse_filter.filter_pulses(&pulses, fps).unwrap();
    ///
    /// assert!(filtered_pulses.len() <= pulses.len());
    /// ```
    pub fn filter_pulses(
        &self,
        pulses: &[NormalizedPulse],
        fps: f32,
    ) -> Result<Vec<NormalizedPulse>> {
        let mut filtered_pulses: Vec<NormalizedPulse> = Vec::new();
        let mut last_end_f: u32 = 0;
        for pulse in pulses {
            if pulse.start_f <= last_end_f {
                return Err(anyhow!("The provided pulses aren't sorted!"));
            }
            last_end_f = pulse.end_f;

            if self.evaluate_filter(pulse, fps) {
                if self.recalc_ipd {
                    let ipd_f = if let Some(last_pulse) = filtered_pulses.last() {
                        pulse.start_f - last_pulse.end_f
                    } else {
                        // The first pulse always has an IPD of 0
                        0
                    };
                    filtered_pulses.push(NormalizedPulse {
                        ipd_f,
                        ipd_s: ipd_f as f32 / fps,
                        ..pulse.clone()
                    });
                } else {
                    filtered_pulses.push(pulse.clone());
                }
            }
        }
        Ok(filtered_pulses)
    }
}
