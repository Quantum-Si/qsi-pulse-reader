use crate::pulse_reader::constants::*;
use crate::pulse_reader::headers::PulseRecordType;
use anyhow::{Result, anyhow};

/// A single raw record
///
/// This struct represents the raw integer-encoded data corresponding to
/// a single record in pulses.bin.
#[derive(Debug)]
pub struct RawRecord {
    pub frames_since_last: u16,
    pub duration: u16,
    pub m0: i16,
    pub m1: i16,
    pub bk0: i16,
    pub bk1: i16,
    pub std0: i16,
    pub std1: i16,
}

impl RawRecord {
    /// Creates a new raw pulse record from a buffer of exactly 16 bytes
    pub fn new(buffer: &[u8]) -> Result<Self> {
        if buffer.len() != 16 {
            return Err(anyhow!(
                "Buffer must be exactly 16 bytes, got {}",
                buffer.len()
            ));
        }

        Ok(RawRecord {
            frames_since_last: u16::from_le_bytes(
                buffer[0..2]
                    .try_into()
                    .map_err(|_| anyhow!("Failed to parse frames_since_last from raw record"))?,
            ),
            duration: u16::from_le_bytes(
                buffer[2..4]
                    .try_into()
                    .map_err(|_| anyhow!("Failed to parse duration from raw record"))?,
            ),
            m0: i16::from_le_bytes(
                buffer[4..6]
                    .try_into()
                    .map_err(|_| anyhow!("Failed to parse m0 from raw record"))?,
            ),
            m1: i16::from_le_bytes(
                buffer[6..8]
                    .try_into()
                    .map_err(|_| anyhow!("Failed to parse m1 from raw record"))?,
            ),
            bk0: i16::from_le_bytes(
                buffer[8..10]
                    .try_into()
                    .map_err(|_| anyhow!("Failed to parse bk0 from raw record"))?,
            ),
            bk1: i16::from_le_bytes(
                buffer[10..12]
                    .try_into()
                    .map_err(|_| anyhow!("Failed to parse bk1 from raw record"))?,
            ),
            std0: i16::from_le_bytes(
                buffer[12..14]
                    .try_into()
                    .map_err(|_| anyhow!("Failed to parse std0 from raw record"))?,
            ),
            std1: i16::from_le_bytes(
                buffer[14..16]
                    .try_into()
                    .map_err(|_| anyhow!("Failed to parse std1 from raw record"))?,
            ),
        })
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum FormattedRecordType {
    Pulse,
    Padding,
    LongPulseUpdate,
    LongPulseDropped,
    StepUp,
    StepDown,
    Background,
    Unknown,
}

impl FormattedRecordType {
    pub fn to_string(&self) -> String {
        match self {
            FormattedRecordType::Pulse => "pulse".to_string(),
            FormattedRecordType::Padding => "padding".to_string(),
            FormattedRecordType::LongPulseUpdate => "long_pulse_update".to_string(),
            FormattedRecordType::LongPulseDropped => "long_pulse_dropped".to_string(),
            FormattedRecordType::StepUp => "step_up".to_string(),
            FormattedRecordType::StepDown => "step_down".to_string(),
            FormattedRecordType::Background => "background".to_string(),
            FormattedRecordType::Unknown => "unknown".to_string(),
        }
    }
}

/// A single formatted non-pulse record
#[derive(Debug, PartialEq)]
pub struct FormattedRecord {
    pub index: usize,
    pub record_type: FormattedRecordType,
    pub frames_since_last: u16,
    pub duration: u16,
    pub intensity0: f32,
    pub intensity1: f32,
    pub bg0: f32,
    pub bg1: f32,
    pub sd0: f32,
    pub sd1: f32,
    pub long_pulse_num_frames: Option<u32>,
    pub event_frame: Option<u32>,
}

impl FormattedRecord {
    /// Convert a raw record into a formatted record
    pub fn from_raw(
        raw_pulse_record: &RawRecord,
        record_types: &[PulseRecordType],
        index: usize,
    ) -> Self {
        if raw_pulse_record.duration > 0 {
            FormattedRecord {
                index,
                record_type: FormattedRecordType::Pulse,
                frames_since_last: raw_pulse_record.frames_since_last,
                duration: raw_pulse_record.duration,
                intensity0: record_types[2].format_value(raw_pulse_record.m0),
                intensity1: record_types[3].format_value(raw_pulse_record.m1),
                bg0: record_types[4].format_value(raw_pulse_record.bk0),
                bg1: record_types[5].format_value(raw_pulse_record.bk1),
                sd0: record_types[6].format_value(raw_pulse_record.std0),
                sd1: record_types[7].format_value(raw_pulse_record.std1),
                long_pulse_num_frames: None,
                event_frame: None,
            }
        } else if raw_pulse_record.frames_since_last == 65535 {
            FormattedRecord {
                index,
                record_type: FormattedRecordType::Padding,
                frames_since_last: raw_pulse_record.frames_since_last,
                duration: raw_pulse_record.duration,
                intensity0: raw_pulse_record.m0 as f32,
                intensity1: raw_pulse_record.m1 as f32,
                bg0: raw_pulse_record.bk0 as f32,
                bg1: raw_pulse_record.bk1 as f32,
                sd0: raw_pulse_record.std0 as f32,
                sd1: raw_pulse_record.std1 as f32,
                long_pulse_num_frames: None,
                event_frame: None,
            }
        } else if raw_pulse_record.std0 == NON_PULSE_RECORD_LONG_PULSE_UPDATE {
            FormattedRecord {
                index,
                record_type: FormattedRecordType::LongPulseUpdate,
                frames_since_last: raw_pulse_record.frames_since_last,
                duration: raw_pulse_record.duration,
                intensity0: record_types[2].format_value(raw_pulse_record.m0),
                intensity1: record_types[3].format_value(raw_pulse_record.m1),
                bg0: raw_pulse_record.bk0 as f32,
                bg1: raw_pulse_record.bk1 as f32,
                sd0: raw_pulse_record.std0 as f32,
                sd1: raw_pulse_record.std1 as f32,
                long_pulse_num_frames: Some(
                    ((raw_pulse_record.bk1 as u32 & 0xffff) << 16)
                        | (raw_pulse_record.bk0 as u32 & 0xffff),
                ),
                event_frame: None,
            }
        } else if raw_pulse_record.std0 == NON_PULSE_RECORD_LONG_PULSE_DROPPED {
            FormattedRecord {
                index,
                record_type: FormattedRecordType::LongPulseDropped,
                frames_since_last: raw_pulse_record.frames_since_last,
                duration: raw_pulse_record.duration,
                intensity0: record_types[2].format_value(raw_pulse_record.m0),
                intensity1: record_types[3].format_value(raw_pulse_record.m1),
                bg0: raw_pulse_record.bk0 as f32,
                bg1: raw_pulse_record.bk1 as f32,
                sd0: raw_pulse_record.std0 as f32,
                sd1: raw_pulse_record.std1 as f32,
                long_pulse_num_frames: Some(
                    ((raw_pulse_record.bk1 as u32 & 0xffff) << 16)
                        | (raw_pulse_record.bk0 as u32 & 0xffff),
                ),
                event_frame: None,
            }
        } else if raw_pulse_record.std0 == NON_PULSE_RECORD_STEP_UP {
            FormattedRecord {
                index,
                record_type: FormattedRecordType::StepUp,
                frames_since_last: raw_pulse_record.frames_since_last,
                duration: raw_pulse_record.duration,
                intensity0: record_types[2].format_value(raw_pulse_record.m0),
                intensity1: record_types[3].format_value(raw_pulse_record.m1),
                bg0: raw_pulse_record.bk0 as f32,
                bg1: raw_pulse_record.bk1 as f32,
                sd0: raw_pulse_record.std0 as f32,
                sd1: raw_pulse_record.std1 as f32,
                long_pulse_num_frames: None,
                event_frame: Some(
                    ((raw_pulse_record.bk1 as u32 & 0xffff) << 16)
                        | (raw_pulse_record.bk0 as u32 & 0xffff),
                ),
            }
        } else if raw_pulse_record.std0 == NON_PULSE_RECORD_STEP_DOWN {
            FormattedRecord {
                index,
                record_type: FormattedRecordType::StepDown,
                frames_since_last: raw_pulse_record.frames_since_last,
                duration: raw_pulse_record.duration,
                intensity0: record_types[2].format_value(raw_pulse_record.m0),
                intensity1: record_types[3].format_value(raw_pulse_record.m1),
                bg0: raw_pulse_record.bk0 as f32,
                bg1: raw_pulse_record.bk1 as f32,
                sd0: raw_pulse_record.std0 as f32,
                sd1: raw_pulse_record.std1 as f32,
                long_pulse_num_frames: None,
                event_frame: Some(
                    ((raw_pulse_record.bk1 as u32 & 0xffff) << 16)
                        | (raw_pulse_record.bk0 as u32 & 0xffff),
                ),
            }
        } else if raw_pulse_record.m0 == 0 && raw_pulse_record.m1 == 0 {
            FormattedRecord {
                index,
                record_type: FormattedRecordType::Background,
                frames_since_last: raw_pulse_record.frames_since_last,
                duration: raw_pulse_record.duration,
                intensity0: raw_pulse_record.m0 as f32,
                intensity1: raw_pulse_record.m1 as f32,
                bg0: record_types[4].format_value(raw_pulse_record.bk0),
                bg1: record_types[5].format_value(raw_pulse_record.bk1),
                sd0: record_types[6].format_value(raw_pulse_record.std0),
                sd1: record_types[7].format_value(raw_pulse_record.std1),
                long_pulse_num_frames: None,
                event_frame: None,
            }
        } else {
            FormattedRecord {
                index,
                record_type: FormattedRecordType::Unknown,
                frames_since_last: raw_pulse_record.frames_since_last,
                duration: raw_pulse_record.duration,
                intensity0: raw_pulse_record.m0 as f32,
                intensity1: raw_pulse_record.m1 as f32,
                bg0: raw_pulse_record.bk0 as f32,
                bg1: raw_pulse_record.bk1 as f32,
                sd0: raw_pulse_record.std0 as f32,
                sd1: raw_pulse_record.std1 as f32,
                long_pulse_num_frames: None,
                event_frame: None,
            }
        }
    }
}

/// A normalized pulse record.
///
/// A pulse record containing additional information, such as bin ratio and SNR.
#[derive(Debug, Clone)]
pub struct NormalizedPulse {
    pub index: usize,
    pub start_f: u32,
    pub end_f: u32,
    pub dur_f: u32,
    pub dur_s: f32,
    pub ipd_f: u32,
    pub ipd_s: f32,
    pub snr: f32,
    pub intensity: f32,
    pub bin0_intensity: f32,
    pub intensity_display: f32,
    pub binratio: f32,
    pub bg_mean: f32,
    pub bg_std: f32,
    pub bin0_bg_mean: f32,
    pub bin0_bg_std: f32,
}

impl NormalizedPulse {
    fn from_formatted_record(
        pulse_record: &FormattedRecord,
        last_record_end: u32,
        last_pulse_end: Option<u32>,
        fps: f32,
    ) -> Self {
        debug_assert!(
            pulse_record.record_type == FormattedRecordType::Pulse,
            "Expected a pulse record"
        );
        let start_f = last_record_end + pulse_record.frames_since_last as u32;
        let end_f = start_f + pulse_record.duration as u32;
        let ipd_f = match last_pulse_end {
            Some(val) => start_f - val,
            None => 0u32,
        };
        NormalizedPulse {
            index: pulse_record.index,
            start_f,
            end_f,
            dur_f: pulse_record.duration as u32,
            dur_s: pulse_record.duration as f32 / fps,
            ipd_f,
            ipd_s: ipd_f as f32 / fps,
            snr: pulse_record.intensity1 / pulse_record.sd1,
            intensity: pulse_record.intensity1,
            bin0_intensity: pulse_record.intensity0,
            intensity_display: pulse_record.intensity1 + pulse_record.bg1,
            binratio: pulse_record.intensity0 / pulse_record.intensity1,
            bg_mean: pulse_record.bg1,
            bg_std: pulse_record.sd1,
            bin0_bg_mean: pulse_record.bg0,
            bin0_bg_std: pulse_record.sd0,
        }
    }

    /// Converts a buffer of records into a vector of normalized pulse records
    ///
    /// This method takes a collection of all formatted records from a given
    /// aperture and produces a vector of NormalizedPulses. In order
    /// to ensure that the output is correct, all records from a given aperture
    /// must be provided, otherwise several fields will be incorrect, including
    /// index, start_f/end_f, and ipd_f.
    pub fn from_formatted_records(records: &[FormattedRecord], fps: f32) -> Vec<Self> {
        let mut last_record_end = 0u32;
        let mut last_pulse_end: Option<u32> = None;
        let mut norm_records: Vec<Self> = Vec::new();
        for record in records {
            match record.record_type {
                FormattedRecordType::Pulse => {
                    let norm_pulse = NormalizedPulse::from_formatted_record(
                        record,
                        last_record_end,
                        last_pulse_end,
                        fps,
                    );
                    last_record_end = norm_pulse.end_f;
                    last_pulse_end = Some(norm_pulse.end_f);
                    norm_records.push(norm_pulse);
                }
                _ => {
                    last_record_end += record.frames_since_last as u32;
                }
            }
        }
        norm_records
    }
}
