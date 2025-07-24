mod constants;
pub mod headers;
pub mod records;

use crate::pulse_filter::PulseFilter;

use constants::*;
use headers::*;
use records::*;

use std::path::{Path, PathBuf};
use std::fs::File;
use std::io::prelude::*;
use std::io::{BufWriter, Read, SeekFrom};

use anyhow::{Result, anyhow};
use serde_json::Value;

/// A pulses.bin reader
///
/// This struct is used to parse pulses.bin, extract metadata, and read and
/// format records from apertures.
///
pub struct PulseReader {
    pub file_name: PathBuf,
    file: File,
    pub header: PulseFileHeader,
    pub record_types: Vec<PulseRecordType>,
    pub raw_metadata: String,
    pub fps: f32,
    pub trimmed: bool,
    pub metadata: Value,
    pub index: PulseFileIndex,
}

impl PulseReader {
    /// Attempts to open pulses.bin file for reading
    ///
    /// Opens pulses.bin for reading and reads headers and aperture
    /// byte location index.
    ///
    /// # Examples
    /// ```
    /// use qsi_pulse_reader::pulse_reader::PulseReader;
    /// # use std::path::PathBuf;
    ///
    /// # let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    /// # let pulse_file_path = path.join("../example_files/pulses.bin").to_string_lossy().to_string();
    /// let mut pulse_reader = PulseReader::open(pulse_file_path).unwrap();
    /// ```
    pub fn open<P: AsRef<Path>>(file_name: P) -> Result<Self> {
        let mut file = File::open(file_name.as_ref())?;

        // Parse and validate pulse file header
        let mut header_buffer = [0; FILE_HEADER_SIZE_FULL];
        file.read_exact(&mut header_buffer)?;
        let header = PulseFileHeader::new(&header_buffer);
        header.validate()?;

        // Parse record types
        let mut record_types: Vec<PulseRecordType> = Vec::new();
        let mut record_buffer = vec![0; 4 * header.num_encoding_records as usize];
        file.read_exact(&mut record_buffer)?;
        for idx in 0..header.num_encoding_records as usize {
            let rec_type =
                u8::from_le_bytes(record_buffer[(4 * idx)..(4 * idx + 1)].try_into().unwrap());
            let rec_bits = u8::from_le_bytes(
                record_buffer[(4 * idx + 1)..(4 * idx + 2)]
                    .try_into()
                    .unwrap(),
            );
            let rec_off = u16::from_le_bytes(
                record_buffer[(4 * idx + 2)..(4 * idx + 4)]
                    .try_into()
                    .unwrap(),
            );
            let rec_scale: u16 = 1u16 << rec_bits;
            record_types.push(PulseRecordType {
                record_type: rec_type,
                bits: rec_bits,
                scale: rec_scale as f32,
                offset: rec_off as f32,
            });
        }

        // Parse metadata
        let mut metadata_buffer = vec![0; header.metadata_length as usize];
        file.read_exact(&mut metadata_buffer)?;
        let raw_metadata = String::from_utf8(metadata_buffer).unwrap();
        let metadata: Value = serde_json::from_str(&raw_metadata).unwrap();
        let fps = metadata["fps"].as_f64().unwrap() as f32;

        // Check if the trimmed pulse caller was used
        let trimmed = metadata
            .get("pulseCaller")
            .and_then(|pulse_caller| pulse_caller.get("options").and_then(Value::as_array))
            .map_or(false, |options| {
                options
                    .iter()
                    .any(|value| value.as_str().unwrap_or("") == "trim_boundary_frames")
            });

        // Parse aperture index
        let _ = file.seek(SeekFrom::Start(header.index_offset))?;
        let mut index_magic_buffer = [0; 8];
        file.read_exact(&mut index_magic_buffer)?;
        let index_magic = u64::from_le_bytes(index_magic_buffer);
        if index_magic != INDEX_SECTION_MAGIC {
            return Err(anyhow!("Index magic number mismatch"));
        };

        // Populate aperture index map
        let mut index_buffer = vec![0; INDEX_RECORD_SIZE * header.num_reads as usize];
        file.read_exact(&mut index_buffer)?;
        let index = PulseFileIndex::new(&index_buffer, header.num_reads as usize);

        Ok(PulseReader {
            file_name: file_name.as_ref().to_path_buf(),
            file,
            header,
            record_types,
            raw_metadata,
            metadata,
            fps,
            trimmed,
            index,
        })
    }

    /// Create a new pulses.bin file with a subset of the apertures in this one
    ///
    /// This function creates a new pulses.bin file with only records from the specified list of
    /// apertures. All other apertures will be omitted from the created pulses.bin file. This is
    /// useful for creating smaller pulses.bin files for testing purposes.
    ///
    /// # Examples
    /// ```
    /// # use qsi_pulse_reader::pulse_reader::PulseReader;
    /// # use std::path::PathBuf;
    /// # use tempfile::tempdir;
    ///
    /// # let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    /// # let pulse_file_path = path.join("../example_files/pulses.bin").to_string_lossy().to_string();
    /// # let temp_dir = tempdir().unwrap();
    /// # let new_pulse_file_path = temp_dir.path().join("pulses_copy.bin").to_string_lossy().to_string();
    /// # let mut pulse_reader = PulseReader::open(pulse_file_path).unwrap();
    ///
    /// // Copy the first 5 apertures to a new file
    /// let apertures_to_copy = pulse_reader.index.apertures[0..5].to_vec();
    /// pulse_reader.copy_apertures_to_new_file(&apertures_to_copy, &new_pulse_file_path).unwrap();
    /// ```

    pub fn copy_apertures_to_new_file(
        &mut self,
        apertures: &[usize],
        file_name: &str,
    ) -> Result<()> {
        // The index logic requires apertures to be sorted
        let mut apertures = apertures.to_vec();
        apertures.sort();

        // Initialize the offset to the beginning of the pulse record data
        let mut offset = self.header.data_offset as usize;

        // To determine the size of each aperture on disk, we will first need to parse the header
        // for that aperture. Allocate memory for an aperture header.
        let mut ap_header_buffer = [0u8; READ_HEADER_SIZE];

        // The index tells us where each aperture's records begin on disk. Allocate memory to store
        // the new byte location for each aperture.
        let mut new_ap_byte_loc: Vec<usize> = vec![0; apertures.len()];

        // The total size in bytes of every aperture's header + records
        let mut ap_byte_len: Vec<usize> = vec![0; apertures.len()];

        for (idx, ap) in apertures.iter().enumerate() {
            // The byte loc of the aperture in the new file
            new_ap_byte_loc[idx] = offset;

            // Read and parse the aperture header
            let byte_loc = self.index.get(*ap).unwrap();
            self.file.seek(SeekFrom::Start(byte_loc))?;
            self.file.read_exact(&mut ap_header_buffer)?;
            let aperture_header = ApertureHeader::new(&ap_header_buffer, byte_loc);

            // Store the size of the aperture, then update the offset
            ap_byte_len[idx] = READ_HEADER_SIZE + aperture_header.num_pulses as usize * PULSE_SIZE;
            offset += ap_byte_len[idx];
        }

        // Open the new file with a buffered writer
        let buffer_size = 1024 * 1024; // 1MB buffer
        let mut new_file = BufWriter::with_capacity(buffer_size, File::create(file_name)?);

        // Create a new header with updated num_reads and index_offset, then write to new file
        let new_file_header = PulseFileHeader {
            num_reads: apertures.len() as u64,
            index_offset: offset as u64,
            ..self.header
        };
        new_file_header.write_all(&mut new_file)?;

        // Seek to the position immediately after the initial header, which contains the metadata
        // and pulse record info, and copy that data to the new file
        self.file
            .seek(SeekFrom::Start(FILE_HEADER_SIZE_FULL as u64))?;
        let mut remaining_header_buffer: Vec<u8> =
            vec![0; self.header.data_offset as usize - FILE_HEADER_SIZE_FULL];
        self.file.read_exact(&mut remaining_header_buffer)?;
        new_file.write_all(&remaining_header_buffer)?;

        // Allocate enough memory for the largest aperture, then loop over apertures and copy their
        // header and records one-by-one to the new file
        let max_byte_len = *ap_byte_len.iter().max().unwrap_or(&0);
        let mut ap_buffer: Vec<u8> = vec![0; max_byte_len];
        for (idx, ap) in apertures.iter().enumerate() {
            let ap_buffer_slice = &mut ap_buffer[0..ap_byte_len[idx]];
            let byte_loc = self.index.get(*ap).unwrap();
            self.file.seek(SeekFrom::Start(byte_loc))?;
            self.file.read_exact(ap_buffer_slice)?;
            new_file.write_all(ap_buffer_slice)?;
        }

        // Write the index magic integer
        new_file.write_all(&INDEX_SECTION_MAGIC.to_le_bytes())?;

        // Write the index
        for (ap, new_byte_loc) in apertures.iter().zip(new_ap_byte_loc) {
            // This may look inefficient, but since we're using a buffered writer, it's actually
            // more performant than collecting the whole index in memory and writing all at once.
            new_file.write_all(&(*ap as u32).to_le_bytes())?;
            new_file.write_all(&(new_byte_loc as u64).to_le_bytes())?;
        }
        new_file.flush()?;
        Ok(())
    }

    /// Extract header and raw (unformatted) records for the given aperture index
    fn get_raw_records(&mut self, aperture: usize) -> Result<(Vec<RawRecord>, ApertureHeader)> {
        // Seek to beginning of records for given aperture
        let byte_loc = self.index.get(aperture).unwrap();
        let _ = self.file.seek(SeekFrom::Start(byte_loc))?;

        // Parse aperture header
        let mut buffer = [0; READ_HEADER_SIZE];
        self.file.read_exact(&mut buffer)?;
        let aperture_header = ApertureHeader::new(&buffer, byte_loc);

        // Parse raw records
        let mut raw_pulse_records: Vec<RawRecord> = Vec::new();
        let mut pulse_buffer = vec![0; PULSE_SIZE * aperture_header.num_pulses as usize];
        self.file.read_exact(&mut pulse_buffer)?;
        for idx in 0..aperture_header.num_pulses as usize {
            raw_pulse_records.push(RawRecord::new(
                &pulse_buffer[(idx * PULSE_SIZE)..((idx + 1) * PULSE_SIZE)],
            ));
        }
        Ok((raw_pulse_records, aperture_header))
    }

    /// Extract header and all formatted records for the given aperture index
    ///
    /// Parses and returns a vector of FormattedRecords, each representing a single
    /// formatted record from the given aperture, as well as an ApertureHeader.
    ///
    /// # Examples
    /// ```
    /// # use qsi_pulse_reader::pulse_reader::PulseReader;
    /// # use std::path::PathBuf;
    ///
    /// # let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    /// # let pulse_file_path = path.join("../example_files/pulses.bin").to_string_lossy().to_string();
    /// # let mut pulse_reader = PulseReader::open(pulse_file_path).unwrap();
    /// let ap = pulse_reader.index.apertures[0];
    ///
    /// let (records, aperture_header) = pulse_reader.get_all_records(ap).unwrap();
    ///
    /// assert!(aperture_header.well_id as usize == ap);
    /// assert!(aperture_header.num_pulses as usize == records.len());
    /// ```
    pub fn get_all_records(
        &mut self,
        aperture: usize,
    ) -> Result<(Vec<FormattedRecord>, ApertureHeader)> {
        let (raw_records, aperture_header) = self.get_raw_records(aperture).unwrap();
        let records: Vec<FormattedRecord> = raw_records
            .into_iter()
            .enumerate()
            .map(|(idx, raw_record)| {
                FormattedRecord::from_raw(&raw_record, &self.record_types, idx)
            })
            .collect();
        Ok((records, aperture_header))
    }

    /// Extract header and normalized pulses for the given aperture index
    ///
    /// Parses and returns a vector of NormalizedPulses, each representing a
    /// single normalized pulse from the given aperture, as well as an ApertureHeader.
    /// This excludes non-pulse records, such as background records.
    /// If a pulse filter is provided, it will be applied to the pulses.
    ///
    /// # Examples
    /// ```
    /// # use qsi_pulse_reader::pulse_reader::PulseReader;
    /// # use std::path::PathBuf;
    ///
    /// # let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    /// # let pulse_file_path = path.join("../example_files/pulses.bin").to_string_lossy().to_string();
    /// # let mut pulse_reader = PulseReader::open(pulse_file_path).unwrap();
    /// let ap = pulse_reader.index.apertures[0];
    ///
    /// let (pulses, aperture_header) = pulse_reader.get_pulses(ap, None).unwrap();
    ///
    /// assert!(aperture_header.well_id as usize == ap);
    ///
    /// // The header indicates the total number of records in the aperture, including non-pulse records.
    /// assert!(aperture_header.num_pulses as usize >= pulses.len());
    /// ```
    pub fn get_pulses(
        &mut self,
        aperture: usize,
        pulse_filter: Option<&PulseFilter>,
    ) -> Result<(Vec<NormalizedPulse>, ApertureHeader)> {
        let (records, aperture_header) = self.get_all_records(aperture)?;
        let pulse_records = NormalizedPulse::from_formatted_records(&records, self.fps);
        if let Some(filter) = pulse_filter {
            Ok((
                filter.filter_pulses(&pulse_records, self.fps)?,
                aperture_header,
            ))
        } else {
            Ok((pulse_records, aperture_header))
        }
    }
}
