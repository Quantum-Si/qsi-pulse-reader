use extendr_api::prelude::*;
use qsi_pulse_reader::pulse_reader::records::{FormattedRecord, NormalizedPulse};

#[derive(IntoDataFrameRow)]
pub(crate) struct FormattedRecordR {
    index: usize,
    record_type: String,
    frames_since_last: u16,
    duration: u16,
    intensity0: f32,
    intensity1: f32,
    bg0: f32,
    bg1: f32,
    sd0: f32,
    sd1: f32,
    long_pulse_num_frames: Option<u32>,
    event_frame: Option<u32>,
}

impl FormattedRecordR {
    pub(crate) fn from_record(record: &FormattedRecord) -> Self {
        FormattedRecordR {
            index: record.index,
            record_type: record.record_type.to_string(),
            frames_since_last: record.frames_since_last,
            duration: record.duration,
            intensity0: record.intensity0,
            intensity1: record.intensity1,
            bg0: record.bg0,
            bg1: record.bg1,
            sd0: record.sd0,
            sd1: record.sd1,
            long_pulse_num_frames: record.long_pulse_num_frames,
            event_frame: record.event_frame,
        }
    }
}

#[derive(IntoDataFrameRow)]
pub(crate) struct NormalizedPulseR {
    index: usize,
    start_f: u32,
    end_f: u32,
    dur_f: u32,
    dur_s: f32,
    ipd_f: u32,
    ipd_s: f32,
    snr: f32,
    intensity: f32,
    bin0_intensity: f32,
    intensity_display: f32,
    binratio: f32,
    bg_mean: f32,
    bg_std: f32,
    bin0_bg_mean: f32,
    bin0_bg_std: f32,
}

impl NormalizedPulseR {
    pub(crate) fn from_pulse(record: &NormalizedPulse) -> Self {
        NormalizedPulseR {
            index: record.index,
            start_f: record.start_f,
            end_f: record.end_f,
            dur_f: record.dur_f,
            dur_s: record.dur_s,
            ipd_f: record.ipd_f,
            ipd_s: record.ipd_s,
            snr: record.snr,
            intensity: record.intensity,
            bin0_intensity: record.bin0_intensity,
            intensity_display: record.intensity_display,
            binratio: record.binratio,
            bg_mean: record.bg_mean,
            bg_std: record.bg_std,
            bin0_bg_mean: record.bin0_bg_mean,
            bin0_bg_std: record.bin0_bg_std,
        }
    }
}
