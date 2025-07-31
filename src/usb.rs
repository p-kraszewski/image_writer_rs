use color_eyre::eyre::{eyre, Result};
use dialoguer::{theme::ColorfulTheme, Select};
#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};
use std::{ffi::OsStr, fmt, fs, path};

const GIB: u64 = 1024 * 1024 * 1024;
const GB: u64 = 1000 * 1000 * 1000;

#[derive(Debug, Clone)]
pub struct Device {
    pub dev:    path::PathBuf,
    pub model:  String,
    pub vendor: String,
    pub size:   usize,
}

impl fmt::Display for Device {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(
            format!(
                "{dev}: {vendor} {model} ({sizei:.1} GiB / {size:.1} GB)",
                dev = self.dev.to_string_lossy(),
                vendor = self.vendor,
                model = self.model,
                size = self.size as f64 / GB as f64,
                sizei = self.size as f64 / GIB as f64,
            )
            .as_str(),
        )
    }
}

fn get_str(dev: &path::Path, val: &str) -> Result<String> {
    let dev_path = path::Path::new(dev);
    let file = dev_path.join(val);
    let data = fs::read_to_string(file)?;
    Ok(data.trim().to_string())
}

fn get_int(dev: &path::Path, val: &str, radix: u32) -> Result<u64> {
    let dat = get_str(dev, val)?;

    Ok(u64::from_str_radix(&dat, radix)?)
}

pub fn check_device(sys: &path::Path) -> Result<Device> {
    let base_name = sys.file_name().and_then(OsStr::to_str).unwrap();
    let sys_path = path::Path::new("/sys/block").join(base_name);
    let vendor = get_str(&sys_path, "device/vendor")?;
    let model = get_str(&sys_path, "device/model")?;
    let size = get_int(&sys_path, "size", 10)? as usize * 512;
    if size == 0 {
        return Err(eyre!("Probably an empty card reader"));
    }

    let dev = Device {
        dev: sys.to_path_buf(),
        model,
        vendor,
        size,
    };

    Ok(dev)
}

pub fn detect_pendrives() -> Result<Device> {
    let mut devices = Vec::new();
    for entry in fs::read_dir("/dev/disk/by-id")? {
        match entry {
            Ok(entry) => {
                let dev = entry.file_name().to_string_lossy().to_string();
                if dev.starts_with("usb-") && dev.ends_with("0:0") {
                    match fs::canonicalize(entry.path()) {
                        Ok(path) => {
                            if !path
                                .file_name()
                                .unwrap()
                                .to_string_lossy()
                                .starts_with("sr")
                            {
                                match check_device(&path) {
                                    Ok(device) => {
                                        devices.push(device);
                                    }
                                    Err(e) => {
                                        debug!("Skipped device {path:?}: {e}");
                                    }
                                }
                            }
                        }

                        Err(e) => {
                            error!("Failed to dereference device: {e}");
                        }
                    }
                }
            }
            Err(e) => {
                debug!("Failed to iterate over entry {e}");
            }
        }
    }

    if devices.is_empty() {
        return Err(eyre!("No devices found"));
    }

    let device = if devices.len() > 1 {
        info!("Multiple devices detected");
        let selection = Select::with_theme(&ColorfulTheme::default())
            .default(0)
            .with_prompt("Select device to overwrite [q to abort]:")
            .items(&devices)
            .interact_opt()?;
        if let Some(selection) = selection {
            &devices[selection]
        } else {
            return Err(eyre!("No device selected"));
        }
    } else {
        &devices[0]
    };

    Ok(device.clone())
}
