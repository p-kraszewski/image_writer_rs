use color_eyre::eyre::{Context, Result, eyre};
use log::debug;
use std::{
    collections::BTreeMap,
    ffi::{OsStr, OsString},
    fs, path,
    path::Path,
};

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct Database {
    pub images: BTreeMap<String, Image>,

    #[serde(skip)]
    pub changed: bool,

    #[serde(skip)]
    pub db_file: path::PathBuf,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct Image {
    pub sha256: String,
    pub length: usize,
}

impl Database {
    fn db<P: AsRef<Path>>(dir: P) -> path::PathBuf { dir.as_ref().join("checksums.yaml") }

    pub fn new<P: AsRef<Path>>(dir: P) -> Self {
        debug!("New checksum database");
        Self {
            images:  BTreeMap::new(),
            changed: false,
            db_file: Self::db(dir),
        }
    }

    pub fn load<P: AsRef<Path>>(dir: P) -> Result<Self> {
        let db_name = Self::db(dir);

        let file = fs::read_to_string(&db_name)?;
        let mut payload = serde_yaml::from_str::<Self>(&file)?;
        debug!("Checksum database loaded from {db_name:?}");
        payload.db_file = db_name;

        Ok(payload)
    }

    pub fn save(&mut self) -> Result<()> {
        if !self.changed {
            debug!("No need to save");
            return Ok(());
        }

        let payload = serde_yaml::to_string(&self)?;
        fs::write(&self.db_file, payload)?;
        debug!("Database saved to {:?}", self.db_file);
        Ok(())
    }

    pub fn get<P: AsRef<OsStr>>(&mut self, name: P) -> Option<([u8; 32], usize)> {
        let name = name.as_ref().to_string_lossy().to_string();
        if let Some(img) = self.images.get_mut(&name) {
            let checksum_bin = hex::decode(&img.sha256).ok()?;
            let mut buf = [0u8; 32];
            buf.copy_from_slice(&checksum_bin);
            debug!("Image found in database");
            Some((buf, img.length))
        } else {
            debug!("Image not found in database");
            None
        }
    }

    pub fn put<P: AsRef<OsStr>>(&mut self, name: P, sha256: [u8; 32], length: usize) {
        let name = name.as_ref().to_string_lossy().to_string();
        let image = Image {
            sha256: hex::encode(sha256),
            length,
        };
        self.images.insert(name, image);
        self.changed = true;
        debug!("Image saved to database");
    }
}
