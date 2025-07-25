use color_eyre::eyre::{Context, Result};
#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};
use sha2::Digest;
use std::{
    fs,
    io::{Read, Seek, Write},
    os::unix::fs::OpenOptionsExt,
    path,
};

const BUF_SIZE: usize = 1024 * 1024;
const PAGE_SIZE: usize = 4096;

pub fn copy(len: usize, mut source: Box<dyn Read>, target: &path::Path) -> Result<()> {
    let mut buf = [0u8; BUF_SIZE + PAGE_SIZE];
    let page_shift = (PAGE_SIZE - ((buf.as_ptr() as usize) & (PAGE_SIZE - 1))) % PAGE_SIZE;
    let aligned_buf = &mut buf[page_shift .. page_shift + BUF_SIZE];

    let bar = indicatif::ProgressBar::new(len as u64)
        .with_message("Writing")
        .with_style(indicatif::ProgressStyle::with_template(
            "{wide_bar} {msg} {bytes_per_sec}, ETA:{eta}",
        )?);

    let mut out = fs::OpenOptions::new()
        .write(true)
        .read(true)
        .create(false)
        .custom_flags(libc::O_DIRECT)
        .open(target)
        .context("failed to open output device {target}")?;

    let mut file_sum = sha2::Sha256::new();
    let mut data_left = len;

    while data_left > 0 {
        let read_block_size = data_left.clamp(0, BUF_SIZE);

        if let Err(e) = source.read_exact(&mut aligned_buf[.. read_block_size]) {
            warn!("failed to read image: {e}");
            bar.finish_and_clear();
            return Err(e.into());
        };

        if let Err(e) = out.write_all(&aligned_buf[.. read_block_size]) {
            warn!("failed to write image: {e}");
            bar.finish_and_clear();
            return Err(e.into());
        }

        file_sum.update(&aligned_buf[.. read_block_size]);

        data_left -= read_block_size;
        bar.inc(read_block_size as u64);
    }

    bar.set_position(0);
    bar.set_message("Verifying");

    out.flush().context("failed to flush output file")?;
    out.rewind().context("failed to rewind file")?;

    let source_sum = file_sum.finalize();

    let mut file_sum = sha2::Sha256::new();
    data_left = len;

    while data_left > 0 {
        let read_block_size = data_left.clamp(0, BUF_SIZE);

        if let Err(e) = out.read_exact(&mut aligned_buf[.. read_block_size]) {
            warn!("failed to read target for verification: {e}");
            bar.finish_and_clear();
            return Err(e.into());
        }
        file_sum.update(&aligned_buf[.. read_block_size]);
        data_left -= read_block_size;

        bar.inc(read_block_size as u64);
    }

    bar.finish_and_clear();

    let device_sum = file_sum.finalize();

    if source_sum.eq(&device_sum) {
        info!("Verifying successful");
    } else {
        warn!("Failed to verify image");
    }

    Ok(())
}
