use std::io::{Read, Write};
pub const MAGIC: [u8; 4] = *b"AHUB";
pub const VERSION: u8 = 1;
pub const PCM_S16LE: u8 = 1;
pub const HEADER_LEN: usize = 12;
pub const MAX_FRAME_BYTES: usize = 16 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AudioFormat {
    pub sample_rate: u32,
    pub channels: u8,
    pub codec: u8,
}
impl AudioFormat {
    pub const PCM_48K_MONO: Self = Self {
        sample_rate: 48_000,
        channels: 1,
        codec: PCM_S16LE,
    };
}

pub fn write_header(mut out: impl Write, format: AudioFormat) -> std::io::Result<()> {
    out.write_all(&MAGIC)?;
    out.write_all(&[VERSION, format.codec, format.channels, 0])?;
    out.write_all(&format.sample_rate.to_be_bytes())
}
pub fn read_header(mut input: impl Read) -> std::io::Result<AudioFormat> {
    let mut header = [0; HEADER_LEN];
    input.read_exact(&mut header)?;
    if header[..4] != MAGIC || header[4] != VERSION || header[5] != PCM_S16LE || header[6] != 1 {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "unsupported Android Hub audio stream",
        ));
    }
    let sample_rate = u32::from_be_bytes(header[8..12].try_into().unwrap());
    if !(8_000..=48_000).contains(&sample_rate) {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "invalid sample rate",
        ));
    }
    Ok(AudioFormat {
        sample_rate,
        channels: 1,
        codec: PCM_S16LE,
    })
}
pub fn read_frame(mut input: impl Read) -> std::io::Result<Vec<u8>> {
    let mut size = [0; 4];
    input.read_exact(&mut size)?;
    let len = u32::from_be_bytes(size) as usize;
    if len == 0 || len > MAX_FRAME_BYTES || len % 2 != 0 {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "invalid audio frame length",
        ));
    }
    let mut frame = vec![0; len];
    input.read_exact(&mut frame)?;
    Ok(frame)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn round_trip_header() {
        let mut bytes = Vec::new();
        write_header(&mut bytes, AudioFormat::PCM_48K_MONO).unwrap();
        assert_eq!(
            read_header(bytes.as_slice()).unwrap(),
            AudioFormat::PCM_48K_MONO
        );
    }
    #[test]
    fn rejects_huge_frame() {
        assert!(read_frame([0xff, 0xff, 0xff, 0xff].as_slice()).is_err());
    }
}
