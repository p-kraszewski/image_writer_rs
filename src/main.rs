mod reader;
mod tools;
mod usb;
mod writer;

use color_eyre::{
    eyre,
    eyre::{eyre, Result},
};
use std::path;

use dialoguer::{theme::ColorfulTheme, Select};
#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};
use usb::detect_pendrives;

const KB: f64 = 1024.0;
const MB: f64 = KB * 1024.0;
const GB: f64 = MB * 1024.0;

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
        }
    };

    let ext = match source_file.extension() {
        None => {
            warn!("Unknown image type. Quitting");
            return Ok(());
        }
        Some(ext) => ext.to_string_lossy().to_ascii_uppercase(),
    };

    let device = match detect_pendrives() {
        Ok(device) => device,
        Err(e) => {
            warn!("{e}");
            return Ok(());
        }
    };

    let (len, reader) = match match ext.as_str() {
        "ISO" | "FS" | "IMG" | "IMA" | "DD" | "BIN" => reader::direct(&source_file),
        "BZ2" => reader::bz2(&source_file),
        "XZ" | "LZMA" => reader::xz(&source_file),
        _ => {
            warn!("unrecognized compression {ext}");
            return Ok(());
        }
    } {
        Ok(decompressor) => decompressor,

        Err(e) => {
            warn!("failed to read image: {e}");
            return Ok(());
        }
    };

    if len % 512 != 0 {
        error!("Image length not multiple of sector size");
        return Ok(());
    }

    let size_txt = match len as f64 {
        mb if mb < GB => format!("{data:.2}MIB", data = mb / MB),
        gb => format!("{data:.2}GiB", data = gb / GB),
    };

    tools::countdown(10, &device.model);

    info!(
        "Copying {size_txt} from {source_file:?} to {dev:?}",
        dev = device.dev
    );

    match writer::copy(len as usize, reader, &device.dev) {
        Ok(_) => {
            info!("copy complete");
        }
        Err(e) => {
            warn!("failed to copy image: {e}");
        }
    }

    Ok(())
}
