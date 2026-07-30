use std::io::{self, Read, Seek, SeekFrom};
use std::path::Path;
use std::fs;

const HAP_MAGIC: u32 = 0x48415001;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Compression {
    None = 0,
    Deflate = 1,
}

impl Compression {
    fn from_u8(v: u8) -> Self {
        match v {
            1 => Self::Deflate,
            _ => Self::None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct HapEntry {
    pub path: String,
    pub offset: u64,
    pub compressed_size: u64,
    pub original_size: u64,
    pub compression: Compression,
    pub encrypted: bool,
    pub crc32: u32,
}

#[derive(Debug)]
#[allow(dead_code)]
pub struct HapHeader {
    pub format_version: u16,
    pub flags: u16,
    pub entry_count: u32,
    pub dir_offset: u32,
    pub dir_size: u32,
    pub data_offset: u32,
    pub data_size: u64,
    pub sha256: [u8; 32],
}

pub struct HapReader<R: Read + Seek> {
    reader: R,
    pub header: HapHeader,
    pub entries: Vec<HapEntry>,
}

impl<R: Read + Seek> HapReader<R> {
    pub fn open(mut reader: R) -> io::Result<Self> {
        let header = Self::read_header(&mut reader)?;
        let entries = Self::read_directory(&mut reader, &header)?;
        Ok(Self { reader, header, entries })
    }

    fn read_header(r: &mut R) -> io::Result<HapHeader> {
        let mut buf = [0u8; 64];
        r.seek(SeekFrom::Start(0))?;
        r.read_exact(&mut buf)?;

        let magic = u32::from_le_bytes([buf[0], buf[1], buf[2], buf[3]]);
        if magic != HAP_MAGIC {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "not a valid HAP file"));
        }

        Ok(HapHeader {
            format_version: u16::from_le_bytes([buf[4], buf[5]]),
            flags: u16::from_le_bytes([buf[6], buf[7]]),
            entry_count: u32::from_le_bytes([buf[8], buf[9], buf[10], buf[11]]),
            dir_offset: u32::from_le_bytes([buf[12], buf[13], buf[14], buf[15]]),
            dir_size: u32::from_le_bytes([buf[16], buf[17], buf[18], buf[19]]),
            data_offset: u32::from_le_bytes([buf[20], buf[21], buf[22], buf[23]]),
            data_size: u64::from_le_bytes(buf[24..32].try_into().unwrap()),
            sha256: buf[32..64].try_into().unwrap(),
        })
    }

    fn read_directory(r: &mut R, header: &HapHeader) -> io::Result<Vec<HapEntry>> {
        r.seek(SeekFrom::Start(header.dir_offset as u64))?;
        let mut entries = Vec::with_capacity(header.entry_count as usize);

        for _ in 0..header.entry_count {
            let mut len_buf = [0u8; 2];
            r.read_exact(&mut len_buf)?;
            let path_len = u16::from_le_bytes(len_buf) as usize;

            let mut path_buf = vec![0u8; path_len];
            r.read_exact(&mut path_buf)?;
            let path = String::from_utf8(path_buf)
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

            let mut meta = [0u8; 30];
            r.read_exact(&mut meta)?;

            entries.push(HapEntry {
                path,
                offset: u64::from_le_bytes(meta[0..8].try_into().unwrap()),
                compressed_size: u64::from_le_bytes(meta[8..16].try_into().unwrap()),
                original_size: u64::from_le_bytes(meta[16..24].try_into().unwrap()),
                compression: Compression::from_u8(meta[24]),
                encrypted: meta[25] != 0,
                crc32: u32::from_le_bytes(meta[26..30].try_into().unwrap()),
            });
        }
        Ok(entries)
    }

    pub fn read_entry(&mut self, entry: &HapEntry) -> io::Result<Vec<u8>> {
        if entry.encrypted {
            return Err(io::Error::new(io::ErrorKind::PermissionDenied, "encrypted entries not supported"));
        }

        self.reader.seek(SeekFrom::Start(
            self.header.data_offset as u64 + entry.offset,
        ))?;
        let mut raw = vec![0u8; entry.compressed_size as usize];
        self.reader.read_exact(&mut raw)?;

        let data = match entry.compression {
            Compression::None => raw,
            Compression::Deflate => {
                let mut decoder = flate2::read::DeflateDecoder::new(&raw[..]);
                let mut out = Vec::with_capacity(entry.original_size as usize);
                decoder.read_to_end(&mut out)?;
                out
            }
        };

        let actual_crc = crc32fast::hash(&data);
        if actual_crc != entry.crc32 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("CRC-32 mismatch: expected {:#010X}, got {:#010X}", entry.crc32, actual_crc),
            ));
        }

        Ok(data)
    }

    pub fn read_file(&mut self, path: &str) -> io::Result<Vec<u8>> {
        let entry = self.entries.iter()
            .find(|e| e.path == path)
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, format!("file not found: {path}")))?
            .clone();
        self.read_entry(&entry)
    }

    pub fn is_signed(&self) -> bool {
        self.header.flags & 0x01 != 0
    }
}

impl HapReader<io::BufReader<fs::File>> {
    pub fn open_file(path: &Path) -> io::Result<Self> {
        let file = fs::File::open(path)?;
        Self::open(io::BufReader::new(file))
    }
}

pub fn is_hap_format(path: &Path) -> io::Result<bool> {
    let mut f = fs::File::open(path)?;
    let mut magic = [0u8; 4];
    if f.read_exact(&mut magic).is_err() {
        return Ok(false);
    }
    Ok(u32::from_le_bytes(magic) == HAP_MAGIC)
}
