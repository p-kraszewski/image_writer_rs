use crate::reader;
use color_eyre::eyre::{Context, Result, eyre};
use dialoguer::{Select, theme::ColorfulTheme};
use log::warn;
use sha2::Digest;
use std::{
    fs, io,
    io::{BufReader, Read},
    path,
};

pub fn by_ext(ext: &str) -> Result<Box<dyn Decompressor>> {
    match ext {
        "ISO" | "FS" | "IMG" | "IMA" | "DD" | "BIN" | "RAW" => Ok(reader::Direct::init()),
        "BZ2" | "BZIP2" => Ok(reader::BZ2::init()),
        "GZ" | "GZIP" => Ok(reader::GZIP::init()),
        "XZ" | "LZMA" | "PIXZ" => Ok(reader::XZ::init()),
        "ZST" | "ZSTD" => Ok(reader::ZSTD::init()),
        "LZ4" => Ok(reader::LZ4::init()),
        _ => Err(eyre!("unrecognized compression {ext}")),
    }
}

pub trait Decompressor
where
    Self: 'static + Send,
{
    fn init() -> Box<dyn Decompressor>
    where
        Self: Default,
    {
        Box::new(Self::default())
    }
    fn open_reader(&self, path: &path::Path) -> Result<Box<dyn Read>>;
    fn get_size_sum(&self, path: &path::Path) -> Result<(usize, [u8; 32])> {
        let mut reader = self.open_reader(path)?;

        let mut boot = [0u8; 512];
        reader
            .read_exact(&mut boot)
            .context("failed to read boot")?;

        if boot[510] != 0x55 || boot[511] != 0xAA {
            warn!("This doesn't look like a hybrid image (lacks MBR signature)");
            let selection = Select::with_theme(&ColorfulTheme::default())
                .default(0)
                .with_prompt("What to do:")
                .items(&["Abort", "Continue"])
                .interact()?;
            if selection == 0 {
                return Err(eyre!("Bad image format"));
            }
        }

        let mut file_sum = sha2::Sha256::new();
        file_sum.update(&boot);

        let size = io::copy(&mut reader, &mut file_sum).context("failed to measure output")?;
        let checksum = file_sum.finalize();
        let mut bin_checksum = [0u8; 32];
        bin_checksum.copy_from_slice(&checksum);
        Ok((512 + size as usize, bin_checksum))
    }
    fn get_name(&self) -> &str;
}

#[derive(Debug, Clone, Default)]
pub struct Direct {}

impl Decompressor for Direct {
    fn open_reader(&self, path: &path::Path) -> Result<Box<dyn Read>> {
        let f = fs::File::open(path).context("failed to open file")?;
        let raw_reader = BufReader::new(f);
        Ok(Box::new(raw_reader))
    }

    fn get_name(&self) -> &str { "uncompressed" }
}

#[derive(Debug, Clone, Default)]
pub struct BZ2 {}
impl Decompressor for BZ2 {
    fn open_reader(&self, path: &path::Path) -> Result<Box<dyn Read>> {
        let file = fs::File::open(path).context("failed to open file")?;
        let reader = BufReader::new(file);
        let decompress_reader = bzip2::bufread::MultiBzDecoder::new(reader);
        Ok(Box::new(decompress_reader))
    }

    fn get_name(&self) -> &str { "compressed with BZIP2" }
}

#[derive(Debug, Clone, Default)]
pub struct XZ {}

impl Decompressor for XZ {
    fn open_reader(&self, path: &path::Path) -> Result<Box<dyn Read>> {
        let file = fs::File::open(path).context("failed to open file")?;
        let reader = BufReader::new(file);
        let decompress_reader = liblzma::bufread::XzDecoder::new(reader);
        Ok(Box::new(decompress_reader))
    }

    fn get_name(&self) -> &str { "compressed with XZ/LZMA" }
}

#[derive(Debug, Clone, Default)]
pub struct GZIP {}

impl Decompressor for GZIP {
    fn open_reader(&self, path: &path::Path) -> Result<Box<dyn Read>> {
        let file = fs::File::open(path).context("failed to open file")?;
        let reader = BufReader::new(file);
        let decompress_reader = flate2::bufread::MultiGzDecoder::new(reader);
        Ok(Box::new(decompress_reader))
    }

    fn get_name(&self) -> &str { "compressed with GZIP" }
}

#[derive(Debug, Clone, Default)]
pub struct ZSTD {}

impl Decompressor for ZSTD {
    fn open_reader(&self, path: &path::Path) -> Result<Box<dyn Read>> {
        let file = fs::File::open(path).context("failed to open file")?;
        let decompress_reader = zstd::Decoder::new(file).context("failed to decompress")?;
        Ok(Box::new(decompress_reader))
    }

    fn get_name(&self) -> &str { "compressed with ZSTD" }
}

#[derive(Debug, Clone, Default)]
pub struct LZ4 {}

impl Decompressor for LZ4 {
    fn open_reader(&self, path: &path::Path) -> Result<Box<dyn Read>> {
        let file = fs::File::open(path).context("failed to open file")?;
        let reader = BufReader::new(file);
        let decompress_reader = lz4_flex::frame::FrameDecoder::new(reader);
        Ok(Box::new(decompress_reader))
    }

    fn get_name(&self) -> &str { "compressed with LZ4" }
}
