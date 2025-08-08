use crate::pulse_reader::constants::*;
use anyhow::{Result, anyhow};
use serde::Serialize;
use std::io::Write;

/// The header of a pulses.bin file
///
/// This struct contains the raw header information contained within the first
/// 48 bytes of every pulses.bin file.
#[derive(Serialize, Debug)]
pub struct PulseFileHeader {
    pub magic: u32,
    pub version: u32,
    pub num_reads: u64,
    pub metadata_length: u32,
    pub encoding_record_type: u8,
    pub encoding_record_size: u8,
    pub num_encoding_records: u16,
    pub record_header_size: u32,
    pub record_size: u32,
    pub data_offset: u64,
    pub index_offset: u64,
}

impl PulseFileHeader {
    /// Create a header from a 48-byte buffer
    ///
    /// Creates a new PulseFileHeader from the first 48 bytes of a pulses.bin file.
    pub fn new(buffer: &[u8; FILE_HEADER_SIZE_FULL]) -> Result<Self> {
        Ok(PulseFileHeader {
            magic: u32::from_le_bytes(
                buffer[0..4]
                    .try_into()
                    .map_err(|_| anyhow!("Failed to parse magic number from header"))?,
            ),
            version: u32::from_le_bytes(
                buffer[4..8]
                    .try_into()
                    .map_err(|_| anyhow!("Failed to parse version from header"))?,
            ),
            num_reads: u64::from_le_bytes(
                buffer[8..16]
                    .try_into()
                    .map_err(|_| anyhow!("Failed to parse num_reads from header"))?,
            ),
            metadata_length: u32::from_le_bytes(
                buffer[16..20]
                    .try_into()
                    .map_err(|_| anyhow!("Failed to parse metadata_length from header"))?,
            ),
            encoding_record_type: u8::from_le_bytes(
                buffer[20..21]
                    .try_into()
                    .map_err(|_| anyhow!("Failed to parse encoding_record_type from header"))?,
            ),
            encoding_record_size: u8::from_le_bytes(
                buffer[21..22]
                    .try_into()
                    .map_err(|_| anyhow!("Failed to parse encoding_record_size from header"))?,
            ),
            num_encoding_records: u16::from_le_bytes(
                buffer[22..24]
                    .try_into()
                    .map_err(|_| anyhow!("Failed to parse num_encoding_records from header"))?,
            ),
            record_header_size: u32::from_le_bytes(
                buffer[24..28]
                    .try_into()
                    .map_err(|_| anyhow!("Failed to parse record_header_size from header"))?,
            ),
            record_size: u32::from_le_bytes(
                buffer[28..32]
                    .try_into()
                    .map_err(|_| anyhow!("Failed to parse record_size from header"))?,
            ),
            data_offset: u64::from_le_bytes(
                buffer[32..40]
                    .try_into()
                    .map_err(|_| anyhow!("Failed to parse data_offset from header"))?,
            ),
            index_offset: u64::from_le_bytes(
                buffer[40..48]
                    .try_into()
                    .map_err(|_| anyhow!("Failed to parse index_offset from header"))?,
            ),
        })
    }

    /// Validates whether a pulse file header is valid
    ///
    /// The V4 spec of the binary pulse file type has flexibility for future
    /// revisions that would change how the pulse records are parsed. However,
    /// methods for parsing future revisions of the V4 spec will need to be
    /// implemented later. For now, we validate whether the data contained
    /// within the pulse file header abides our expectations for the current
    /// version of the V4 spec, and failing that, we return an error.
    pub fn validate(&self) -> Result<()> {
        if self.magic != BINARY_PULSE_FILE_MAGIC {
            return Err(anyhow!("Pulse file header is invalid!"));
        }
        if self.encoding_record_type != 1 {
            return Err(anyhow!(
                "Invalid encoding record type: {}",
                self.encoding_record_type
            ));
        } else if self.encoding_record_size != 4 {
            return Err(anyhow!(
                "Invalid encoding record size: {}",
                self.encoding_record_size
            ));
        } else if self.record_size != 16 {
            return Err(anyhow!("Invalid record size: {}", self.record_size));
        } else if self.num_encoding_records != 8 {
            return Err(anyhow!(
                "Invalid number of encoding records: {}",
                self.num_encoding_records
            ));
        }
        Ok(())
    }

    pub fn write_all<W>(&self, writer: &mut W) -> Result<()>
    where
        W: Write,
    {
        let mut buffer = [0u8; FILE_HEADER_SIZE_FULL];
        buffer[0..4].copy_from_slice(&self.magic.to_le_bytes());
        buffer[4..8].copy_from_slice(&self.version.to_le_bytes());
        buffer[8..16].copy_from_slice(&self.num_reads.to_le_bytes());
        buffer[16..20].copy_from_slice(&self.metadata_length.to_le_bytes());
        buffer[20..21].copy_from_slice(&self.encoding_record_type.to_le_bytes());
        buffer[21..22].copy_from_slice(&self.encoding_record_size.to_le_bytes());
        buffer[22..24].copy_from_slice(&self.num_encoding_records.to_le_bytes());
        buffer[24..28].copy_from_slice(&self.record_header_size.to_le_bytes());
        buffer[28..32].copy_from_slice(&self.record_size.to_le_bytes());
        buffer[32..40].copy_from_slice(&self.data_offset.to_le_bytes());
        buffer[40..48].copy_from_slice(&self.index_offset.to_le_bytes());
        writer.write_all(&buffer)?;
        Ok(())
    }
}

/// A representation of the aperture byte location index from pulses.bin
///
/// This struct wraps the information contained within the aperture byte location
/// index at the end of every pulses.bin file, namely the list of valid apertures
/// and the byte location in pulses.bin of every valid aperture.
pub struct PulseFileIndex {
    pub apertures: Vec<usize>,
    index_map: Vec<u64>,
    min: usize,
}

impl PulseFileIndex {
    /// Creates a new pulse file index from a byte buffer
    ///
    /// Parses a byte buffer containing `num_reads * 12` bytes to create a
    /// PulseFileIndex instance.
    pub fn new(buffer: &[u8], num_reads: usize) -> Result<Self> {
        let min = u32::from_le_bytes(
            buffer[0..4]
                .try_into()
                .map_err(|_| anyhow!("Failed to parse min aperture index from buffer"))?,
        ) as usize;
        let max = u32::from_le_bytes(
            buffer
                [((num_reads - 1) * INDEX_RECORD_SIZE)..((num_reads - 1) * INDEX_RECORD_SIZE + 4)]
                .try_into()
                .map_err(|_| anyhow!("Failed to parse max aperture index from buffer"))?,
        ) as usize;

        let mut apertures = vec![0usize; num_reads];
        let mut index_map = vec![0u64; max - min + 1];
        for idx in 0..num_reads {
            apertures[idx] = u32::from_le_bytes(
                buffer[(idx * INDEX_RECORD_SIZE)..(idx * INDEX_RECORD_SIZE + 4)]
                    .try_into()
                    .map_err(|_| anyhow!("Failed to parse aperture index at position {}", idx))?,
            ) as usize;
            index_map[apertures[idx] - min] = u64::from_le_bytes(
                buffer[(idx * INDEX_RECORD_SIZE + 4)..(idx * INDEX_RECORD_SIZE + 12)]
                    .try_into()
                    .map_err(|_| anyhow!("Failed to parse byte location at position {}", idx))?,
            );
        }
        Ok(PulseFileIndex {
            apertures,
            index_map,
            min,
        })
    }

    /// Attempts to get the byte location of the provided aperture index.
    ///
    /// Will return the byte location of the provided aperture index, or will
    /// return an error if the aperture index is not contained in the index.
    pub fn get(&self, ap: usize) -> Result<u64> {
        // If the aperture index is out of bounds, return an error
        if ap < self.min || ap >= self.min + self.index_map.len() {
            return Err(anyhow!(
                "This file does not contain the provided aperture index: {}",
                ap
            ));
        }
        let offset = self.index_map[ap - self.min];
        // If the offset is zero, that means the aperture is not present, so return an error
        if offset == 0u64 {
            return Err(anyhow!(
                "This file does not contain the provided aperture index: {}",
                ap
            ));
        };
        Ok(offset)
    }
}

/// A struct describing how to interpret raw record fields.
///
/// This struct describes how the data contained within a single pulse record
/// field is to be converted from its original integer type (u16 or i16) to
/// a floating point representation.
#[derive(Serialize)]
pub struct PulseRecordType {
    pub record_type: u8,
    pub bits: u8,
    pub scale: f32,
    pub offset: f32,
}

impl PulseRecordType {
    /// Format a raw record value
    ///
    /// Converts the raw i16 value of a particular field from a raw record
    /// into f32 using the fixed-point format described by the PulseRecordType.
    pub fn format_value(&self, val: i16) -> f32 {
        let fval = if val == V4_NAN_VAL {
            f32::NAN
        } else if val == V4_NEG_INF {
            f32::NEG_INFINITY
        } else if val == V4_POS_INF {
            f32::INFINITY
        } else {
            val as f32
        };

        if fval.is_finite() {
            fval / self.scale + self.offset
        } else {
            fval
        }
    }
}

/// Metadata pertaining to a single aperture
///
/// This struct contains information on the physical location of an aperture,
/// the location of its records in the file, and the number of records corresponding
/// to this aperture in pulses.bin.
pub struct ApertureHeader {
    pub x: u32,
    pub y: u32,
    pub well_id: u32,
    pub num_pulses: u32,
    pub byte_loc: u64,
}

impl ApertureHeader {
    /// Instantiates an aperture header from a buffer of 16 bytes
    pub fn new(buffer: &[u8; 16], byte_loc: u64) -> Result<Self> {
        Ok(ApertureHeader {
            x: u32::from_le_bytes(
                buffer[0..4]
                    .try_into()
                    .map_err(|_| anyhow!("Failed to parse x coordinate from aperture header"))?,
            ),
            y: u32::from_le_bytes(
                buffer[4..8]
                    .try_into()
                    .map_err(|_| anyhow!("Failed to parse y coordinate from aperture header"))?,
            ),
            well_id: u32::from_le_bytes(
                buffer[8..12]
                    .try_into()
                    .map_err(|_| anyhow!("Failed to parse well_id from aperture header"))?,
            ),
            num_pulses: u32::from_le_bytes(
                buffer[12..16]
                    .try_into()
                    .map_err(|_| anyhow!("Failed to parse num_pulses from aperture header"))?,
            ),
            byte_loc,
        })
    }

    pub fn write_all<W>(&self, writer: &mut W) -> Result<()>
    where
        W: Write,
    {
        let mut buffer = [0u8; READ_HEADER_SIZE];
        buffer[0..4].copy_from_slice(&self.x.to_le_bytes());
        buffer[4..8].copy_from_slice(&self.y.to_le_bytes());
        buffer[8..12].copy_from_slice(&self.well_id.to_le_bytes());
        buffer[12..16].copy_from_slice(&self.num_pulses.to_le_bytes());
        writer.write_all(&buffer)?;
        Ok(())
    }
}
