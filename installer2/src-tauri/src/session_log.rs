use std::{
    fs,
    io::{self, Write},
    path::PathBuf,
};

const MAX_ENTRIES: usize = 2_000;
const MAX_BYTES: u64 = 2 * 1024 * 1024;

#[derive(Default)]
pub struct SessionLog {
    pub entries: Vec<String>,
    path: Option<PathBuf>,
}

impl SessionLog {
    pub fn open(path: PathBuf) -> io::Result<Self> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let content = match fs::read_to_string(&path) {
            Ok(content) => content,
            Err(error) if error.kind() == io::ErrorKind::NotFound => String::new(),
            Err(error) => return Err(error),
        };
        let mut entries: Vec<_> = content
            .lines()
            .rev()
            .take(MAX_ENTRIES)
            .map(str::to_owned)
            .collect();
        entries.reverse();
        Ok(Self {
            entries,
            path: Some(path),
        })
    }

    pub fn push(&mut self, entry: String) {
        self.entries.push(entry.clone());
        if self.entries.len() > MAX_ENTRIES {
            self.entries.remove(0);
        }
        let Some(path) = &self.path else {
            return;
        };
        let result: io::Result<()> = (|| {
            if fs::metadata(path).is_ok_and(|metadata| metadata.len() > MAX_BYTES) {
                fs::write(path, self.entries.join("\n") + "\n")?;
            } else {
                let mut file = fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(path)?;
                writeln!(file, "{entry}")?;
                file.sync_data()?;
            }
            Ok(())
        })();
        if let Err(error) = result {
            eprintln!("Could not persist diagnostic log: {error}");
        }
    }

    pub fn clear(&mut self) -> io::Result<()> {
        if let Some(path) = &self.path {
            fs::write(path, b"")?;
        }
        self.entries.clear();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn previous_session_is_loaded_and_clear_is_persistent() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("logs/installer.log");
        let mut first = SessionLog::open(path.clone()).unwrap();
        first.push("previous session".into());
        let mut second = SessionLog::open(path.clone()).unwrap();
        assert_eq!(second.entries, vec!["previous session"]);
        second.clear().unwrap();
        assert!(SessionLog::open(path).unwrap().entries.is_empty());
    }
}
