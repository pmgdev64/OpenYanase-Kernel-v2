// src/tar_writer.rs
const BLOCK_SIZE: usize = 512;

pub struct TarWriter {
    buf: Vec<u8>,
}

impl TarWriter {
    pub fn new() -> Self {
        Self { buf: Vec::new() }
    }

    pub fn add_file(&mut self, name: &str, data: &[u8]) {
        let mut header = [0u8; BLOCK_SIZE];

        let name_bytes = name.as_bytes();
        let n = name_bytes.len().min(99);
        header[0..n].copy_from_slice(&name_bytes[..n]);

        header[100..108].copy_from_slice(b"0000644\0");
        header[108..116].copy_from_slice(b"0000000\0");
        header[116..124].copy_from_slice(b"0000000\0");
        
        let size_octal = format!("{:011o}\0", data.len());
        let size_bytes = size_octal.as_bytes();
        header[124..124 + size_bytes.len().min(12)]
            .copy_from_slice(&size_bytes[..size_bytes.len().min(12)]);
        header[136..148].copy_from_slice(b"00000000000\0");
        header[148..156].copy_from_slice(b"        ");
        header[156] = b'0';
        header[257..263].copy_from_slice(b"ustar\0");
        header[263..265].copy_from_slice(b"00");

        let checksum: u32 = header.iter().map(|&b| b as u32).sum();
        let chksum_octal = format!("{:06o}\0 ", checksum);
        let chksum_bytes = chksum_octal.as_bytes();
        header[148..148 + chksum_bytes.len().min(8)]
            .copy_from_slice(&chksum_bytes[..chksum_bytes.len().min(8)]);

        self.buf.extend_from_slice(&header);
        self.buf.extend_from_slice(data);
        let pad = (BLOCK_SIZE - (data.len() % BLOCK_SIZE)) % BLOCK_SIZE;
        self.buf.extend(std::iter::repeat(0u8).take(pad));
    }

    pub fn finish(mut self) -> Vec<u8> {
        self.buf.extend(std::iter::repeat(0u8).take(BLOCK_SIZE * 2));
        self.buf
    }
}