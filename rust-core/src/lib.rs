//! QSI Pulses.bin reader library
//!
//! Provides the following functionality:
//! - `PulseReader`, a pulses.bin reader
//! - `PulseFilter`, a normalized pulse filter

pub mod pulse_filter;
pub mod pulse_reader;

#[cfg(test)]
mod tests {
    use crate::pulse_filter::PulseFilter;
    use crate::pulse_reader::records::NormalizedPulse;
    use crate::pulse_reader::{PulseReader, merge_pulse_files};
    use anyhow::Result;
    use std::path::PathBuf;
    use std::vec;
    use tempfile::tempdir;

    fn get_pulse_reader() -> Result<PulseReader> {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        PulseReader::open(
            path.join("../example_files/pulses.bin")
                .to_string_lossy()
                .to_string(),
        )
    }

    #[test]
    fn test_pulse_reader() -> Result<()> {
        let mut pulse_reader = get_pulse_reader()?;
        let apertures = pulse_reader.index.apertures.clone();
        for ap in apertures {
            let (records, ap_header) = pulse_reader.get_all_records(ap)?;
            assert!(ap_header.num_pulses as usize == records.len());

            let (pulses, _ap_header) = pulse_reader.get_pulses(ap, None)?;
            assert!(records.len() >= pulses.len());
        }
        Ok(())
    }

    fn check_pulse_filter(
        pulse_reader: &mut PulseReader,
        pulse_filter: &PulseFilter,
        condition: impl Fn(&NormalizedPulse, &PulseFilter) -> bool,
    ) -> Result<()> {
        let apertures = pulse_reader.index.apertures.clone();
        for ap in apertures {
            let (pulses, _ap_header) = pulse_reader.get_pulses(ap, Some(pulse_filter))?;
            for pulse in pulses {
                assert!(condition(&pulse, pulse_filter));
            }
        }
        Ok(())
    }

    #[test]
    fn test_pulse_filter() -> Result<()> {
        let mut pulse_reader = get_pulse_reader()?;
        let fps = pulse_reader.fps;

        let mut pulse_filter = PulseFilter::default();
        pulse_filter.min_dur_f = Some(10);
        check_pulse_filter(&mut pulse_reader, &pulse_filter, |pulse, filter| {
            pulse.dur_f >= filter.min_dur_f.unwrap()
        })?;

        let mut pulse_filter = PulseFilter::default();
        pulse_filter.min_dur_s = Some(1.0);
        pulse_filter.max_dur_s = Some(2.0);
        check_pulse_filter(&mut pulse_reader, &pulse_filter, |pulse, filter| {
            pulse.dur_s >= filter.min_dur_s.unwrap() && pulse.dur_s <= filter.max_dur_s.unwrap()
        })?;

        let mut pulse_filter = PulseFilter::default();
        pulse_filter.min_snr = Some(6.0);
        check_pulse_filter(&mut pulse_reader, &pulse_filter, |pulse, filter| {
            pulse.snr >= filter.min_snr.unwrap()
        })?;

        let mut pulse_filter = PulseFilter::default();
        pulse_filter.min_intensity = Some(100.0);
        check_pulse_filter(&mut pulse_reader, &pulse_filter, |pulse, filter| {
            pulse.intensity >= filter.min_intensity.unwrap()
        })?;

        let mut pulse_filter = PulseFilter::default();
        pulse_filter.min_binratio = Some(0.1);
        pulse_filter.max_binratio = Some(0.8);
        check_pulse_filter(&mut pulse_reader, &pulse_filter, |pulse, filter| {
            pulse.binratio >= filter.min_binratio.unwrap()
                && pulse.binratio <= filter.max_binratio.unwrap()
        })?;

        let mut pulse_filter = PulseFilter::default();
        pulse_filter.start_m = Some(120.0);
        pulse_filter.end_m = Some(240.0);
        check_pulse_filter(
            &mut pulse_reader,
            &pulse_filter,
            |pulse: &NormalizedPulse, filter| {
                pulse.start_f as f32 / (60.0 * fps) >= filter.start_m.unwrap()
                    && pulse.end_f as f32 / (60.0 * fps) <= filter.end_m.unwrap()
            },
        )?;

        let mut pulse_filter = PulseFilter::default();
        pulse_filter.mask_s = Some((3600.0, 7200.0));
        check_pulse_filter(&mut pulse_reader, &pulse_filter, |pulse, filter| {
            let (start, end) = filter.mask_s.unwrap();
            pulse.start_f as f32 / fps <= start || pulse.end_f as f32 / fps >= end
        })?;

        let mut pulse_filter = PulseFilter::default();
        pulse_filter.min_dur_f = Some(10);
        pulse_filter.recalc_ipd = true;
        let apertures = pulse_reader.index.apertures.clone();
        for ap in apertures {
            let (pulses, _ap_header) = pulse_reader.get_pulses(ap, Some(&pulse_filter))?;
            for window in pulses.windows(2) {
                let (pulse_a, pulse_b) = (&window[0], &window[1]);
                assert!(pulse_a.end_f + pulse_b.ipd_f == pulse_b.start_f);
            }
        }
        Ok(())
    }

    #[test]
    fn test_copy_apertures_to_new_file() -> Result<()> {
        let mut pulse_reader = get_pulse_reader()?;
        let apertures = pulse_reader.index.apertures[0..5].to_vec();
        let temp_dir =
            tempdir().map_err(|e| anyhow::anyhow!("Failed to create temp directory: {}", e))?;
        let new_file_path = temp_dir
            .path()
            .join("new_pulses.bin")
            .to_string_lossy()
            .to_string();

        pulse_reader.copy_apertures_to_new_file(&apertures, &new_file_path)?;

        let mut new_pulse_reader = PulseReader::open(new_file_path)?;
        assert_eq!(new_pulse_reader.index.apertures.len(), apertures.len());
        for ap in apertures {
            let (orig_records, _ap_header) = pulse_reader.get_all_records(ap)?;
            let (new_records, _new_ap_header) = new_pulse_reader.get_all_records(ap)?;
            assert_eq!(orig_records.len(), new_records.len());
            for (orig, new) in orig_records.iter().zip(new_records.iter()) {
                assert_eq!(orig, new);
            }
        }
        Ok(())
    }

    #[test]
    fn test_merge_pulse_files() {
        let mut pulse_readers = vec![get_pulse_reader().unwrap(), get_pulse_reader().unwrap()];
        let temp_dir = tempdir().unwrap();
        let new_file_path = temp_dir
            .path()
            .join("merged_pulses.bin")
            .to_string_lossy()
            .to_string();

        merge_pulse_files(&mut pulse_readers, &new_file_path).unwrap();

        let mut merged_reader = PulseReader::open(new_file_path).unwrap();
        assert_eq!(
            merged_reader.index.apertures.len(),
            pulse_readers[0].index.apertures.len() * 2
        );
        let apertures = pulse_readers[0].index.apertures.clone();
        for ap in apertures {
            let (orig_records, _ap_header) = pulse_readers[0].get_all_records(ap).unwrap();
            let (merged_records, _new_ap_header) = merged_reader.get_all_records(ap).unwrap();
            assert_eq!(orig_records.len(), merged_records.len());
            for (orig, merged) in orig_records.iter().zip(merged_records.iter()) {
                assert_eq!(orig, merged);
            }
        }
    }
}
