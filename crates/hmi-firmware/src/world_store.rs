//! Recoverable SD snapshots. I/O runs on a worker, never on the simulation loop.
use hmi_core::living::{
    Automaton, Rule, GRID_HEIGHT, GRID_WIDTH, LEGACY_GRID_HEIGHT, LEGACY_GRID_WIDTH,
};
use std::{
    fs::{self, File},
    io::{self, Read, Write},
    path::{Path, PathBuf},
};

pub struct WorldStore {
    root: PathBuf,
}

impl WorldStore {
    pub fn new(root: impl AsRef<Path>) -> Self {
        Self {
            root: root.as_ref().into(),
        }
    }
    fn path(&self, rule: Rule, suffix: &str) -> PathBuf {
        // The on-board FAT volume can reject long extensions (EINVAL). Keep
        // every migration artifact compatible with 8.3 filenames.
        match suffix {
            "legacy" => self.root.join(format!("old-{:02}.bin", rule as u8)),
            "legacy.tmp" => self.root.join(format!("old-{:02}.tmp", rule as u8)),
            _ => self.root.join(format!("world-{:02}.{suffix}", rule as u8)),
        }
    }
    fn read_valid(&self, rule: Rule, suffix: &str) -> Option<Automaton> {
        let file = File::open(self.path(rule, suffix)).ok()?;
        let mut bytes = Vec::new();
        file.take((GRID_WIDTH * GRID_HEIGHT + 27) as u64)
            .read_to_end(&mut bytes)
            .ok()?;
        let a = Automaton::restore(&bytes)?;
        if a.rule != rule
            || !matches!(
                a.dimensions(),
                (GRID_WIDTH, GRID_HEIGHT) | (LEGACY_GRID_WIDTH, LEGACY_GRID_HEIGHT)
            )
        {
            return None;
        }
        Some(a)
    }
    pub fn load(&self, rule: Rule) -> Option<Automaton> {
        self.read_valid(rule, "bin")
            .or_else(|| self.read_valid(rule, "bak"))
            .or_else(|| self.read_valid(rule, "legacy"))
    }
    fn preserve_legacy(&self, rule: Rule) -> io::Result<()> {
        if self.read_valid(rule, "legacy").is_some() {
            return Ok(());
        }
        for suffix in ["bin", "bak"] {
            if self
                .read_valid(rule, suffix)
                .is_some_and(|world| world.dimensions() == (LEGACY_GRID_WIDTH, LEGACY_GRID_HEIGHT))
            {
                let mut old = File::open(self.path(rule, suffix))?;
                let temporary = self.path(rule, "legacy.tmp");
                let mut archived = File::create(&temporary)?;
                io::copy(&mut old, &mut archived)?;
                archived.sync_all()?;
                drop(archived);
                let legacy = self.path(rule, "legacy");
                if legacy.exists() {
                    fs::remove_file(&legacy)?;
                }
                fs::rename(temporary, legacy)?;
                break;
            }
        }
        Ok(())
    }
    pub fn save(&self, rule: Rule, bytes: &[u8]) -> io::Result<()> {
        let world = Automaton::restore(bytes)
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "invalid snapshot"))?;
        if world.rule != rule
            || !matches!(
                world.dimensions(),
                (GRID_WIDTH, GRID_HEIGHT) | (LEGACY_GRID_WIDTH, LEGACY_GRID_HEIGHT)
            )
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "wrong automaton",
            ));
        }
        fs::create_dir_all(&self.root)?;
        let pending = self.path(rule, "tmp");
        let primary = self.path(rule, "bin");
        let backup = self.path(rule, "bak");
        let mut file = File::create(&pending)?;
        file.write_all(bytes)?;
        file.sync_all()?;
        drop(file);
        self.preserve_legacy(rule)?;
        // A corrupt primary must never replace a known-good backup.
        if self.read_valid(rule, "bin").is_some() {
            match fs::remove_file(&backup) {
                Ok(()) => {}
                Err(e) if e.kind() == io::ErrorKind::NotFound => {}
                Err(e) => return Err(e),
            }
            fs::rename(&primary, &backup)?;
        } else if primary.exists() {
            fs::remove_file(&primary)?;
        }
        fs::rename(&pending, &primary)?;
        // Not every FAT VFS supports syncing a directory. File data is already synced.
        if let Ok(directory) = File::open(&self.root) {
            let _ = directory.sync_all();
        }
        Ok(())
    }
}
