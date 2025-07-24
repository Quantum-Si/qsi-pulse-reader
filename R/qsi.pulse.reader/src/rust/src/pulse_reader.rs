use crate::records::{FormattedRecordR, NormalizedPulseR};
use extendr_api::prelude::*;
use qsi_pulse_reader::pulse_reader::headers::ApertureHeader;
use qsi_pulse_reader::pulse_reader::PulseReader as RustPulseReader;

/// Pulses.bin reader
#[extendr]
pub(super) struct PulseReader {
    pulse_reader: RustPulseReader,
    source: String,
    analysis_id: String,
    frame_dur_s: f32,
    run_dur_f: u64,
    run_dur_s: f32,
}

impl PulseReader {
    fn set_df_attributes<T>(&self, df: &mut Dataframe<T>, header: &ApertureHeader) -> Result<()> {
        df.set_attrib("aperture_index", &header.well_id)?;
        df.set_attrib("aperture_x", &header.x)?;
        df.set_attrib("aperture_y", &header.y)?;
        df.set_attrib("aperture_byteloc", &header.byte_loc)?;
        df.set_attrib("source", &self.source)?;
        df.set_attrib("analysis_id", &self.analysis_id)?;
        df.set_attrib("frame_dur_s", &self.frame_dur_s)?;
        df.set_attrib("run_dur_f", &self.run_dur_f)?;
        df.set_attrib("run_dur_s", &self.run_dur_s)?;
        Ok(())
    }
}

#[extendr]
impl PulseReader {
    /// Open a pulses.bin file and return a PulseReader object
    ///
    /// # Arguments
    /// * `file_name` - The name of the pulses.bin file to open
    ///
    /// # Returns
    /// A PulseReader object that can be used to read pulses from the file.
    ///
    /// # Errors
    /// Returns an error if the file cannot be opened or if there is an error reading the file.
    ///
    /// # Examples
    ///
    /// ```R
    /// reader <- PulseReader$new("pulses.bin")
    /// ```
    pub(crate) fn new(file_name: &str) -> Result<Self> {
        let pulse_reader = RustPulseReader::open(file_name.to_string()).map_err(|e| e.to_string())?;
        let source = "pulses.bin".to_string();
        let analysis_id = file_name.split('.').next().unwrap().to_string();
        let frame_dur_s = 1.0 / pulse_reader.fps;
        let run_dur_s = pulse_reader.metadata["duration"].as_f64().unwrap() as f32;
        let run_dur_f = (run_dur_s * pulse_reader.fps).ceil() as u64;

        Ok(PulseReader {
            pulse_reader,
            source,
            analysis_id,
            frame_dur_s,
            run_dur_f,
            run_dur_s,
        })
    }

    /// Get the formatted pulse records for a specific aperture index
    ///
    /// # Arguments
    /// * `aperture_index` - The index of the aperture to get the raw pulse records for
    ///
    /// # Returns
    /// A Data Frame containing the formatted pulse records for the specified aperture index
    ///
    /// # Examples
    /// ```R
    /// reader <- PulseReader$new("pulses.bin")
    /// records <- reader$get_records(12345)
    /// ```
    pub(crate) fn get_all_records(
        &mut self,
        aperture_index: usize,
    ) -> Result<Dataframe<FormattedRecordR>> {
        let (records, header) = self.pulse_reader.get_all_records(aperture_index).unwrap();
        let r_records = records
            .iter()
            .map(|record| FormattedRecordR::from_record(record))
            .collect::<Vec<_>>();
        let mut df = Dataframe::try_from_values(r_records)?;
        self.set_df_attributes(&mut df, &header)?;
        Ok(df)
    }

    /// Get the normalized pulse records for a specific aperture index
    ///
    /// # Arguments
    /// * `aperture_index` - The index of the aperture to get the raw pulse records for
    ///
    /// # Returns
    /// A Data Frame containing the normalized pulse records for the specified aperture index
    ///
    /// # Examples
    /// ```R
    /// reader <- PulseReader$new("pulses.bin")
    /// pulses <- reader$get_normalized_pulses(12345)
    /// ```
    pub(crate) fn get_pulses(
        &mut self,
        aperture_index: usize,
    ) -> Result<Dataframe<NormalizedPulseR>> {
        let (records, header) = self.pulse_reader.get_pulses(aperture_index, None).unwrap();
        let r_records = records
            .iter()
            .map(|record| NormalizedPulseR::from_pulse(record))
            .collect::<Vec<_>>();
        let mut df = Dataframe::try_from_values(r_records)?;
        self.set_df_attributes(&mut df, &header)?;
        Ok(df)
    }
}

extendr_module! {
    mod qsi_pulse_reader;
    impl PulseReader;
}
