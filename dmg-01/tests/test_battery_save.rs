use dmg_01::{battery_save::BatterySave, cartridge::Cartridge};
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

static ID: AtomicU64 = AtomicU64::new(0);
struct Directory(PathBuf);
impl Directory {
    fn new() -> Self {
        let p = std::env::temp_dir().join(format!(
            "gameboyoxide-save-test-{}-{}",
            std::process::id(),
            ID.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&p).unwrap();
        Self(p)
    }
    fn rom(&self) -> PathBuf {
        self.0.join("test.gb")
    }
    fn save(&self) -> PathBuf {
        self.0.join("test.sav")
    }
}
impl Drop for Directory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn cartridge(kind: u8) -> Cartridge {
    let mut rom = vec![0; 0x8000];
    rom[0x147] = kind;
    rom[0x149] = 3;
    Cartridge::new(rom)
}

#[test]
fn missing_save_does_not_create_an_empty_file_until_ram_changes() {
    let d = Directory::new();
    let mut c = cartridge(0x13);
    let mut save = BatterySave::open(&d.rom(), &mut c).unwrap().unwrap();
    assert!(!save.flush(&c).unwrap());
    assert!(!d.save().exists());
    c.write_rom(0, 0x0a);
    c.write_ram(0x123, 0x42);
    assert!(save.flush(&c).unwrap());
    assert!(!save.flush(&c).unwrap());
    let data = fs::read(d.save()).unwrap();
    assert_eq!(data.len(), 0x8000);
    assert_eq!(data[0x123], 0x42);
    assert_eq!(fs::read_dir(&d.0).unwrap().count(), 1); // no temporary files left
}

#[test]
fn all_banks_round_trip_and_loading_does_not_enable_ram_or_change_banking() {
    let d = Directory::new();
    let mut c = cartridge(0x13);
    let mut save = BatterySave::open(&d.rom(), &mut c).unwrap().unwrap();
    c.write_rom(0, 0x0a);
    for bank in 0..4 {
        c.write_rom(0x4000, bank);
        c.write_ram(0x1fff, bank + 1);
    }
    save.flush(&c).unwrap();
    let mut loaded = cartridge(0x13);
    BatterySave::open(&d.rom(), &mut loaded).unwrap().unwrap();
    assert!(!loaded.ram_enabled);
    assert_eq!(loaded.current_ram_bank, 0);
    loaded.write_rom(0, 0x0a);
    for bank in 0..4 {
        loaded.write_rom(0x4000, bank);
        assert_eq!(loaded.read_ram(0x1fff), bank + 1);
    }
}

#[test]
fn invalid_existing_save_is_rejected_without_modifying_ram_or_file() {
    let d = Directory::new();
    fs::write(d.save(), b"not a full battery image").unwrap();
    let original = fs::read(d.save()).unwrap();
    let mut c = cartridge(0x13);
    c.ram.fill(0x33);
    let err = BatterySave::open(&d.rom(), &mut c).err().unwrap();
    assert_eq!(err.kind(), std::io::ErrorKind::InvalidData);
    assert!(c.ram.iter().all(|&b| b == 0x33));
    assert_eq!(fs::read(d.save()).unwrap(), original);
}

#[test]
fn nonbattery_cartridges_do_not_read_or_overwrite_a_save_file() {
    let d = Directory::new();
    fs::write(d.save(), vec![0x55; 0x8000]).unwrap();
    for kind in [0x00, 0x01, 0x02, 0x11, 0x12] {
        let mut c = cartridge(kind);
        assert!(BatterySave::open(&d.rom(), &mut c).unwrap().is_none());
        assert!(c.ram.iter().all(|&b| b == 0));
    }
    assert!(fs::read(d.save()).unwrap().iter().all(|&b| b == 0x55));
}

#[test]
fn mbc1_battery_save_is_supported() {
    let d = Directory::new();
    let mut c = cartridge(0x03);
    let mut save = BatterySave::open(&d.rom(), &mut c).unwrap().unwrap();
    c.write_rom(0, 0x0a);
    c.write_ram(0, 0x77);
    save.flush(&c).unwrap();
    assert_eq!(fs::read(d.save()).unwrap()[0], 0x77);
}

#[test]
fn failed_write_can_be_retried_and_does_not_advance_the_snapshot() {
    let d = Directory::new();
    let mut c = cartridge(0x13);
    let mut save = BatterySave::open(&d.rom(), &mut c).unwrap().unwrap();
    c.ram[0] = 0x11;
    save.flush(&c).unwrap();
    c.ram[0] = 0x22;
    let moved = d.0.with_extension("moved");
    fs::rename(&d.0, &moved).unwrap();
    let result = save.flush(&c);
    fs::rename(&moved, &d.0).unwrap();
    assert!(result.is_err());
    assert_eq!(fs::read(d.save()).unwrap()[0], 0x11);
    assert!(save.flush(&c).unwrap());
    assert_eq!(fs::read(d.save()).unwrap()[0], 0x22);
}
