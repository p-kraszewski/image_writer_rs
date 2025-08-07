mod reader;
mod tools;
mod usb;

use crate::{reader::*, tools::*, usb::*};
use color_eyre::eyre::{Context, Result};
#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};
use sha2::Digest;
use std::{
    fs,
    io::{Read, Seek, Write},
    os::unix::fs::OpenOptionsExt,
    path,
    sync::mpsc,
};

const KB: f64 = 1024.0;
const MB: f64 = KB * 1024.0;
const GB: f64 = MB * 1024.0;

const BUFFERS: usize = 4;

enum ReaderResult {
    Ready,
    Done,
    Error,
    Block(AlignedBuffer),
}

fn main() -> Result<()> {
    color_eyre::install()?;

    env_logger::builder()
        .filter_level(log::LevelFilter::Debug)
        .init();

    let source_file = match std::env::args().nth(1) {
        Some(v) => path::PathBuf::from(v),
        None => {
            error!("No disk image path given, aborting");
            return Ok(());
        },
    };

    let ext = match source_file.extension() {
        None => {
            error!("Unknown image type. Quitting");
            return Ok(());
        },
        Some(ext) => ext.to_string_lossy().to_ascii_uppercase(),
    };

    let device = match detect_pendrives() {
        Ok(device) => device,
        Err(e) => {
            error!("Detecting pendrives failed: {}", eyre_unroll(e));
            return Ok(());
        },
    };

    let reader = match by_ext(&ext) {
        Ok(reader) => reader,
        Err(e) => {
            error!("Detecting decompressor failed: {}", eyre_unroll(e));
            return Ok(());
        },
    };

    info!(
        "Calculating length and checksum of {source_file:?}, {comp}.",
        comp = reader.get_name()
    );

    let (len, source_sum) = match reader.get_size_sum(&source_file) {
        Ok((len, csum)) => (len, csum),
        Err(e) => {
            error!("Failed to analyze file: {}", eyre_unroll(e));
            return Ok(());
        },
    };

    if len % 512 != 0 {
        error!("Image length not multiple of sector size");
        return Ok(());
    }

    if len > device.size {
        error!("Image won't fit on media");
        return Ok(());
    }

    info!(
        "Decompressed file SHA256: {sha}",
        sha = source_sum
            .iter()
            .map(|byte| format!("{byte:02X}"))
            .collect::<Vec<_>>()
            .join("")
    );

    let size_txt = match len as f64 {
        mb if mb < GB => format!("{data:.2}MiB", data = mb / MB),
        gb => format!("{data:.2}GiB", data = gb / GB),
    };

    countdown(10, &device.model);

    info!(
        "Copying {size_txt} from {source_file:?} to {dev:?}",
        dev = device.dev,
    );

    let (wrtx, wrrx) = mpsc::sync_channel(BUFFERS);
    let (rdtx, rdrx) = mpsc::sync_channel(BUFFERS);

    for _ in 0..BUFFERS {
        wrtx.send(AlignedBuffer::new())?;
    }

    let bar = indicatif::ProgressBar::new(len as u64)
        .with_message("Writing")
        .with_style(indicatif::ProgressStyle::with_template(
            "{wide_bar} {msg} {bytes_per_sec}, ETA:{eta}",
        )?);

    let mut out = match fs::OpenOptions::new()
        .write(true)
        .read(true)
        .create(false)
        .custom_flags(libc::O_DIRECT)
        .open(&device.dev)
    {
        Err(e) => {
            error!(
                "Failed to open output device {target:?}: {e}",
                target = device.dev,
            );
            return Ok(());
        },
        Ok(out) => out,
    };

    let read_thread = std::thread::spawn(move || -> Result<()> {
        let mut data_left = len;
        let reader = match by_ext(&ext) {
            Ok(reader) => reader,
            Err(e) => {
                rdtx.send(ReaderResult::Error)
                    .expect("failed to send error");
                return Err(e);
            },
        };
        let mut decompressor = match reader.open_reader(&source_file) {
            Ok(d) => d,
            Err(e) => {
                rdtx.send(ReaderResult::Error)
                    .expect("failed to send error");
                return Err(e);
            },
        };

        rdtx.send(ReaderResult::Ready)
            .expect("failed to send ready");

        while data_left > 0 {
            let read_block_size = data_left.clamp(0, BUF_SIZE);

            let mut buf = match wrrx.recv() {
                Ok(buf) => buf,
                Err(e) => {
                    rdtx.send(ReaderResult::Error)
                        .expect("failed to send error");
                    return Err(e.into());
                },
            };

            let aligned_buf = buf.get_aligned_buf();

            if let Err(e) = decompressor.read_exact(&mut aligned_buf[..read_block_size]) {
                rdtx.send(ReaderResult::Error)
                    .expect("failed to send error");
                return Err(e.into());
            };

            buf.used = read_block_size;
            rdtx.send(ReaderResult::Block(buf))
                .expect("failed to send data");
            data_left -= read_block_size;
        }
        rdtx.send(ReaderResult::Done).expect("failed to send done");
        for _ in 0..BUFFERS {
            wrrx.recv().expect("failed to flush buffers");
        }
        Ok(())
    });

    match rdrx.recv()? {
        ReaderResult::Ready => (),
        ReaderResult::Error => {
            let result = read_thread.join().unwrap().expect_err("unexpected success");
            bar.finish_and_clear();
            error!("Reading thread failed: {}", eyre_unroll(result));
            return Ok(());
        },
        _ => {
            bar.finish_and_clear();
            error!("Unexpected read thread finish");
            _ = read_thread.join().unwrap();
            return Ok(());
        },
    }

    loop {
        match rdrx.recv()? {
            ReaderResult::Done => break,
            ReaderResult::Ready => {
                error!("Unexpected ready");
                continue;
            },
            ReaderResult::Error => {
                let result = read_thread.join().unwrap().expect_err("unexpected success");
                bar.finish_and_clear();
                error!("Reading thread failed: {}", eyre_unroll(result));
                return Ok(());
            },
            ReaderResult::Block(mut buf) => {
                let read_block_size = buf.used;
                let aligned_buf = buf.get_aligned_buf();

                if let Err(e) = out.write_all(&aligned_buf[..read_block_size]) {
                    bar.finish_and_clear();
                    error!("failed to write image: {e}");
                    return Ok(());
                }
                bar.inc(read_block_size as u64);
                // After last block bails out, as thread closes
                wrtx.send(buf).expect("failed to send back buffer");
            },
        }
    }

    if let Err(e) = read_thread.join().unwrap() {
        error!("Reading thread failed: {}", eyre_unroll(e));
        return Ok(());
    }

    bar.set_position(0);
    bar.set_message("Verifying");

    out.flush().context("failed to flush output file")?;
    out.rewind().context("failed to rewind file")?;

    let mut file_sum = sha2::Sha256::new();
    let mut data_left = len;

    let mut read_buf = AlignedBuffer::new();
    let read_buf = read_buf.get_aligned_buf();

    while data_left > 0 {
        let read_block_size = data_left.clamp(0, BUF_SIZE);

        if let Err(e) = out.read_exact(&mut read_buf[..read_block_size]) {
            warn!("failed to read target for verification: {e}");
            bar.finish_and_clear();
            return Ok(());
        }
        file_sum.update(&read_buf[..read_block_size]);
        data_left -= read_block_size;

        bar.inc(read_block_size as u64);
    }

    bar.finish_and_clear();

    let device_sum = file_sum.finalize();
    let mut bin_checksum = [0u8; 32];
    bin_checksum.copy_from_slice(&device_sum);

    if source_sum.eq(&bin_checksum) {
        info!("Target verification successful");
    } else {
        error!("Target verification failed");
    }

    Ok(())
}
