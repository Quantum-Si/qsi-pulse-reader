#![allow(dead_code)]

pub const FILE_HEADER_SIZE_FULL: usize = 48;
pub const FILE_HEADER_SIZE_PARTIAL: usize = 16;
pub const READ_HEADER_SIZE: usize = 16;
pub const PULSE_SIZE: usize = 16;
pub const INDEX_SECTION_MAGIC: u64 = 724275076598221092; // equivalent to "$INDX$\r\n"
pub const INDEX_RECORD_SIZE: usize = 12;
pub const BINARY_PULSE_FILE_MAGIC: u32 = 1349079889;

pub const NON_PULSE_RECORD_LONG_PULSE_DROPPED: i16 = -2;
pub const NON_PULSE_RECORD_LONG_PULSE_UPDATE: i16 = -3;
pub const NON_PULSE_RECORD_NOP: i16 = -4; // Not currently used
pub const NON_PULSE_RECORD_STEP_UP: i16 = -5;
pub const NON_PULSE_RECORD_STEP_DOWN: i16 = -6;

pub const V4_NAN_VAL: i16 = -32768;
pub const V4_NEG_INF: i16 = -32767;
pub const V4_POS_INF: i16 = 32767;
pub const V4_NORM_HI: i16 = 32766;
pub const V4_NORM_LO: i16 = -32766;
