use color_eyre::eyre::{Context, Result};
use std::{
    fs, io,
    io::{BufReader, Read, Seek, SeekFrom},
    path,
};

pub fn direct(source: &path::Path) -> Result<(u64, Box<dyn Read>)> {
    let f = fs::File::open(source).context("failed to open file")?;
    let mut reader = BufReader::new(f);

    let len = reader
        .seek(SeekFrom::End(0))
        .context("failed to seek in file")?;
    reader
        .seek(SeekFrom::Start(0))
        .context("failed to seek in file")?;

    Ok((len, Box::new(reader)))
}

pub fn bz2(source: &path::Path) -> Result<(u64, Box<dyn Read>)> {
    use bzip2::bufread::MultiBzDecoder;

    let len = {
        let file = fs::File::open(&source).context("failed to open file")?;
        let reader = BufReader::new(file);
        let mut bz_reader = MultiBzDecoder::new(reader);
        let mut null = io::empty();
        io::copy(&mut bz_reader, &mut null).context("failed to measure output")?
    };

    let file = fs::File::open(source).context("failed to open file")?;
    let reader = BufReader::new(file);
    let bz_reader = MultiBzDecoder::new(reader);

    Ok((len, Box::new(bz_reader)))
}

pub fn xz(source: &path::Path) -> Result<(u64, Box<dyn Read>)> {
    use liblzma::bufread::XzDecoder;

    let len = {
        let file = fs::File::open(&source).context("failed to open file")?;
        let reader = BufReader::new(file);
        let mut bz_reader = XzDecoder::new(reader);
        let mut null = io::empty();
        io::copy(&mut bz_reader, &mut null).context("failed to measure output")?
    };

    let file = fs::File::open(source).context("failed to open file")?;
    let reader = BufReader::new(file);
    let bz_reader = XzDecoder::new(reader);

    Ok((len, Box::new(bz_reader)))
}
