mod constants;
pub mod headers;
pub mod records;

use crate::pulse_filter::PulseFilter;

use constants::*;
use headers::*;
use records::*;

use std::fs::File;
use std::io::prelude::*;
use std::io::{BufWriter, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

use anyhow::{Result, anyhow};
use serde_json::Value;

const BUFFER_SIZE: usize = 1024 * 1024; // 1MB buffer size for reading and writing

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
    /// # let pulse_file_path = path.join("../example_files/pulses.bin");
    /// let mut pulse_reader = PulseReader::open(pulse_file_path).unwrap();
    /// ```
    pub fn open<P: AsRef<Path>>(file_name: P) -> Result<Self> {
        let mut file = File::open(file_name.as_ref())?;

        // Parse and validate pulse file header
        let mut header_buffer = [0; FILE_HEADER_SIZE_FULL];
        file.read_exact(&mut header_buffer)?;
        let header = PulseFileHeader::new(&header_buffer)?;
        header.validate()?;

        // Parse record types
        let mut record_types: Vec<PulseRecordType> = Vec::new();
        let mut record_buffer = vec![0; 4 * header.num_encoding_records as usize];
        file.read_exact(&mut record_buffer)?;
        for idx in 0..header.num_encoding_records as usize {
            let rec_type = u8::from_le_bytes(
                record_buffer[(4 * idx)..(4 * idx + 1)]
                    .try_into()
                    .map_err(|_| anyhow!("Failed to parse record type byte at index {}", idx))?,
            );
            let rec_bits = u8::from_le_bytes(
                record_buffer[(4 * idx + 1)..(4 * idx + 2)]
                    .try_into()
                    .map_err(|_| anyhow!("Failed to parse record bits byte at index {}", idx))?,
            );
            let rec_off = u16::from_le_bytes(
                record_buffer[(4 * idx + 2)..(4 * idx + 4)]
                    .try_into()
                    .map_err(|_| anyhow!("Failed to parse record offset bytes at index {}", idx))?,
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
        let raw_metadata = String::from_utf8(metadata_buffer)
            .map_err(|e| anyhow!("Failed to parse metadata as UTF-8: {}", e))?;
        let metadata: Value = serde_json::from_str(&raw_metadata)
            .map_err(|e| anyhow!("Failed to parse metadata JSON: {}", e))?;
        let fps = metadata["fps"]
            .as_f64()
            .ok_or_else(|| anyhow!("Missing or invalid 'fps' field in metadata"))?
            as f32;

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
        let index = PulseFileIndex::new(&index_buffer, header.num_reads as usize)?;

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
    /// # let pulse_file_path = path.join("../example_files/pulses.bin");
    /// # let temp_dir = tempdir().unwrap();
    /// # let new_pulse_file_path = temp_dir.path().join("pulses_copy.bin");
    /// # let mut pulse_reader = PulseReader::open(pulse_file_path).unwrap();
    ///
    /// // Copy the first 5 apertures to a new file
    /// let apertures_to_copy = pulse_reader.index.apertures[0..5].to_vec();
    /// pulse_reader.copy_apertures_to_new_file(&apertures_to_copy, &new_pulse_file_path).unwrap();
    /// ```
    pub fn copy_apertures_to_new_file<P: AsRef<Path>>(
        &mut self,
        apertures: &[usize],
        file_name: P,
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
            let byte_loc = self.index.get(*ap)?;
            self.file.seek(SeekFrom::Start(byte_loc))?;
            self.file.read_exact(&mut ap_header_buffer)?;
            let aperture_header = ApertureHeader::new(&ap_header_buffer, byte_loc)?;

            // Store the size of the aperture, then update the offset
            ap_byte_len[idx] = READ_HEADER_SIZE + aperture_header.num_pulses as usize * PULSE_SIZE;
            offset += ap_byte_len[idx];
        }

        // Open the new file with a buffered writer
        let mut new_file = BufWriter::with_capacity(BUFFER_SIZE, File::create(file_name)?);

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
            let byte_loc = self.index.get(*ap)?;
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
        let byte_loc = self.index.get(aperture)?;
        let _ = self.file.seek(SeekFrom::Start(byte_loc))?;

        // Parse aperture header
        let mut buffer = [0; READ_HEADER_SIZE];
        self.file.read_exact(&mut buffer)?;
        let aperture_header = ApertureHeader::new(&buffer, byte_loc)?;

        // Parse raw records
        let mut raw_pulse_records: Vec<RawRecord> = Vec::new();
        let mut pulse_buffer = vec![0; PULSE_SIZE * aperture_header.num_pulses as usize];
        self.file.read_exact(&mut pulse_buffer)?;
        for idx in 0..aperture_header.num_pulses as usize {
            raw_pulse_records.push(RawRecord::new(
                &pulse_buffer[(idx * PULSE_SIZE)..((idx + 1) * PULSE_SIZE)],
            )?);
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
    /// # let pulse_file_path = path.join("../example_files/pulses.bin");
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
        let (raw_records, aperture_header) = self.get_raw_records(aperture)?;
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
    /// # let pulse_file_path = path.join("../example_files/pulses.bin");
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

/// Combine two pulses.bin files into a single file with all pulses from both files
///
/// This function merges multiple PulseReader instances into a single pulses.bin file.
/// It combines the metadata, updates the data offset, and writes all records from each
/// PulseReader to the new file. The resulting file will contain all apertures from the
/// provided PulseReader instances, with updated metadata reflecting the total number of
/// reads and apertures.
/// # Examples
/// ```
/// # use qsi_pulse_reader::pulse_reader::{PulseReader, merge_pulse_files};
/// # use std::path::PathBuf;
/// # use tempfile::tempdir;
/// # let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
/// # let pulse_file_path1 = path.join("../example_files/pulses.bin");
/// # let pulse_file_path2 = path.join("../example_files/pulses.bin");
/// # let temp_dir = tempdir().unwrap();
/// # let new_pulse_file_path = temp_dir.path().join("merged_pulses.bin");
/// let mut pulse_readers = vec![
///     PulseReader::open(pulse_file_path1).unwrap(),
///     PulseReader::open(pulse_file_path2).unwrap(),
/// ];
/// merge_pulse_files(&mut pulse_readers, &new_pulse_file_path).unwrap();
/// ```
pub fn merge_pulse_files<P: AsRef<Path>>(
    pulse_files: &mut [PulseReader],
    new_file_name: P,
) -> Result<()> {
    if pulse_files.is_empty() {
        return Err(anyhow!("No pulse files provided"));
    }

    // Check if the file already exists
    if new_file_name.as_ref().exists() {
        return Err(anyhow!(
            "File {} already exists",
            new_file_name.as_ref().display()
        ));
    }

    // First loop to collect data needed for new metadata/header
    let mut valid_wells: u64 = 0;
    let mut valid_wells_left: u64 = 0;
    let mut valid_wells_right: u64 = 0;
    let mut tot_reads: u64 = 0;
    let mut tot_rows: usize = 0;
    let mut roi_offset_col: Option<u64> = None;
    let mut last_col: Option<u64> = None;
    let mut total_aperture_data_size: u64 = 0;

    for pulse_file in pulse_files.iter_mut() {
        tot_reads += pulse_file.header.num_reads;
        valid_wells += pulse_file.metadata["validWells"]
            .as_u64()
            .expect("validWells missing or invalid");
        valid_wells_left += pulse_file.metadata["validWellsLeft"]
            .as_u64()
            .expect("validWellsLeft missing or invalid");
        valid_wells_right += pulse_file.metadata["validWellsRight"]
            .as_u64()
            .expect("validWellsRight missing or invalid");
        tot_rows += pulse_file.metadata["rows"]
            .as_u64()
            .expect("rows missing or invalid") as usize;

        let run_roi_offset_col = pulse_file.metadata["roi_offset_col"]
            .as_u64()
            .expect("roi_offset_col missing or invalid");
        let run_last_col = run_roi_offset_col
            + pulse_file.metadata["roi_cols"]
                .as_u64()
                .expect("roi_cols missing or invalid");
        if roi_offset_col.is_none() || run_roi_offset_col < roi_offset_col.unwrap() {
            roi_offset_col = Some(run_roi_offset_col);
        }
        if last_col.is_none() || run_last_col > last_col.unwrap() {
            last_col = Some(run_last_col);
        }
        total_aperture_data_size += pulse_file.header.index_offset - pulse_file.header.data_offset;
    }
    if last_col.is_none() || roi_offset_col.is_none() {
        return Err(anyhow!("roi_offset_col or last_col is missing"));
    }
    let roi_cols = last_col.unwrap() - roi_offset_col.unwrap();

    // Update metadata
    let mut new_metadata: Value = pulse_files[0].metadata.clone();
    new_metadata["rows"] = Value::from(tot_rows);
    new_metadata["validWells"] = Value::from(valid_wells);
    new_metadata["validWellsLeft"] = Value::from(valid_wells_left);
    new_metadata["validWellsRight"] = Value::from(valid_wells_right);
    new_metadata["roi_cols"] = Value::from(roi_cols);
    new_metadata["roi_rows"] = Value::from(tot_rows);
    new_metadata["roi_offset_col"] = Value::from(roi_offset_col.unwrap());

    let new_raw_metadata = new_metadata.to_string();
    let new_metadata_len = new_raw_metadata.len() as u32;

    // Find new data offset, rounded up to the next multiple of 16
    // We do this so that each aperture header and pulse record will be aligned to an integer multiple of its length
    let new_data_offset = {
        let offset = (pulse_files[0].header.data_offset as i64
            + (new_metadata_len as i64 - pulse_files[0].header.metadata_length as i64))
            as u64;
        (offset + 15) & !15 // Align to 16-byte boundary
    };

    let new_index_offset = new_data_offset + total_aperture_data_size;

    // Create new header with calculated index offset
    let new_file_header = PulseFileHeader {
        num_reads: tot_reads,
        metadata_length: new_metadata_len,
        data_offset: new_data_offset,
        index_offset: new_index_offset,
        ..pulse_files[0].header
    };

    // Open the new file with a buffered writer and write the header
    let mut new_file: BufWriter<File> =
        BufWriter::with_capacity(BUFFER_SIZE, File::create(new_file_name)?);
    new_file_header.write_all(&mut new_file)?;

    // Copy record info from first pulse file
    let mut record_buffer = vec![0; 4 * new_file_header.num_encoding_records as usize];
    pulse_files[0]
        .file
        .seek(SeekFrom::Start(FILE_HEADER_SIZE_FULL as u64))?; // Skip the header
    pulse_files[0].file.read_exact(&mut record_buffer)?;
    new_file.write_all(&record_buffer)?;

    // Write the new raw metadata
    new_file.write_all(new_raw_metadata.as_bytes())?;

    // Write zeros until we hit position new_data_offset
    let stream_position = new_file.stream_position()?;
    if stream_position > new_data_offset as u64 {
        return Err(anyhow!("Stream position exceeds new data offset"));
    }
    let zero_buffer: Vec<u8> = vec![0; new_data_offset as usize - stream_position as usize];
    new_file.write_all(&zero_buffer)?;

    // Verify that our position matches new_data_offset
    if new_file.stream_position()? != new_data_offset {
        return Err(anyhow!("Stream position mismatch"));
    }

    // Copy aperture records to new file, updating aperture indices and y positions as we go
    let mut row_offset: u32 = 0;
    let mut ap_index_offset: u32 = 0;
    let mut ap_header_buffer = [0u8; READ_HEADER_SIZE];
    let mut ap_buffer = vec![0; BUFFER_SIZE]; // 1MB buffer for pulse records
    for pulse_file in pulse_files.iter_mut() {
        for ap in pulse_file.index.apertures.iter() {
            // Read aperture header
            let byte_loc = pulse_file
                .index
                .get(*ap)
                .expect("Invalid aperture index encountered");
            pulse_file.file.seek(SeekFrom::Start(byte_loc))?;
            pulse_file.file.read_exact(&mut ap_header_buffer)?;

            // Update aperture header's position and index, then write to new file
            let mut aperture_header = ApertureHeader::new(&ap_header_buffer, byte_loc)?;
            aperture_header.y += row_offset;
            aperture_header.well_id += ap_index_offset;
            aperture_header.write_all(&mut new_file)?;

            // Copy pulse records to new file
            let mut remaining_bytes = aperture_header.num_pulses as usize * PULSE_SIZE;
            while remaining_bytes > 0 {
                let slice_len = remaining_bytes.min(ap_buffer.len());
                let ap_buffer_slice = &mut ap_buffer[0..slice_len];
                pulse_file.file.read_exact(ap_buffer_slice)?;
                new_file.write_all(ap_buffer_slice)?;
                remaining_bytes -= slice_len;
            }
        }
        let rows = pulse_file.metadata["rows"]
            .as_u64()
            .expect("rows missing or invalid");
        let cols = pulse_file.metadata["cols"]
            .as_u64()
            .expect("cols missing or invalid");
        row_offset += rows as u32;
        ap_index_offset += (rows * cols) as u32;
    }

    // Verify that we are at the expected location of the index
    if new_file.stream_position()? != new_file_header.index_offset {
        return Err(anyhow!("Stream position mismatch"));
    }

    // Write the index magic
    new_file.write_all(&INDEX_SECTION_MAGIC.to_le_bytes())?;

    // Finally, write the aperture index
    let mut cumulative_offset = new_data_offset as i64;
    ap_index_offset = 0;
    for pulse_file in pulse_files.iter_mut() {
        let byte_offset = cumulative_offset - pulse_file.header.data_offset as i64;
        cumulative_offset +=
            pulse_file.header.index_offset as i64 - pulse_file.header.data_offset as i64;

        for ap in pulse_file.index.apertures.iter() {
            let new_ap = *ap as u32 + ap_index_offset;
            let original_byte_loc =
                pulse_file.index.get(*ap).expect("Invalid aperture index") as i64;
            let new_byte_loc = (original_byte_loc + byte_offset) as u64;
            new_file.write_all(&new_ap.to_le_bytes())?;
            new_file.write_all(&new_byte_loc.to_le_bytes())?;
        }
        let rows = pulse_file.metadata["rows"]
            .as_u64()
            .expect("rows missing or invalid");
        let cols = pulse_file.metadata["cols"]
            .as_u64()
            .expect("cols missing or invalid");
        ap_index_offset += (rows * cols) as u32;
    }
    new_file.flush()?;

    Ok(())
}
