//! Raw battery-backed cartridge RAM persistence. A .sav is not a CPU save state.
use crate::cartridge::Cartridge;
use std::ffi::OsString;
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

static TEMP_ID: AtomicU64 = AtomicU64::new(0);

pub struct BatterySave {
    path: PathBuf,
    persisted: Vec<u8>,
}

impl BatterySave {
    /// Load all RAM banks before starting the ROM. Reject a wrong-size file
    /// without modifying it or cartridge RAM; callers must not enable saving
    /// after a load error, otherwise they could overwrite an existing save.
    pub fn open(rom_path: &Path, cartridge: &mut Cartridge) -> io::Result<Option<Self>> {
        if !cartridge.has_battery() || cartridge.ram.is_empty() {
            return Ok(None);
        }
        let path = rom_path.with_extension("sav");
        match fs::read(&path) {
            Ok(data) => {
                if data.len() != cartridge.ram.len() {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!(
                            "{} has {} bytes; expected {}",
                            path.display(),
                            data.len(),
                            cartridge.ram.len()
                        ),
                    ));
                }
                cartridge.ram.copy_from_slice(&data);
            }
            Err(err) if err.kind() == io::ErrorKind::NotFound => {}
            Err(err) => return Err(err),
        }
        Ok(Some(Self {
            path,
            persisted: cartridge.ram.clone(),
        }))
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Write only changed RAM. Same-directory temporary file + rename keeps
    /// the previous save intact if a write fails. A failed write is retried
    /// on the next flush because the persisted snapshot is only then updated.
    pub fn flush(&mut self, cartridge: &Cartridge) -> io::Result<bool> {
        if cartridge.ram.len() != self.persisted.len() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "cartridge RAM size changed",
            ));
        }
        if cartridge.ram == self.persisted {
            return Ok(false);
        }
        atomic_write(&self.path, &cartridge.ram)?;
        self.persisted.copy_from_slice(&cartridge.ram);
        Ok(true)
    }
}

fn atomic_write(path: &Path, data: &[u8]) -> io::Result<()> {
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let mut name = OsString::from(".");
    name.push(
        path.file_name()
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "missing save filename"))?,
    );
    name.push(format!(
        ".{}.{}.tmp",
        std::process::id(),
        TEMP_ID.fetch_add(1, Ordering::Relaxed)
    ));
    let temporary = parent.join(name);
    // Never truncate or delete a pre-existing temporary file.
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)?;
    let result = (|| {
        file.write_all(data)?;
        file.sync_all()?;
        drop(file);
        fs::rename(&temporary, path)
    })();
    if result.is_err() {
        let _ = fs::remove_file(temporary);
    }
    result
}
