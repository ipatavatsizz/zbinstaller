use std::{
    fs,
    io::{self, Write},
    path::PathBuf,
};

const MAX_ENTRIES: usize = 2_000;
const MAX_BYTES: u64 = 2 * 1024 * 1024;

fn trim_entries(entries: &mut Vec<String>) {
    let mut bytes = 0usize;
    let mut keep_from = entries.len();
    for (index, entry) in entries.iter().enumerate().rev() {
        let size = entry.len() + usize::from(keep_from < entries.len());
        if entries.len() - index > MAX_ENTRIES || bytes + size > MAX_BYTES as usize {
            break;
        }
        bytes += size;
        keep_from = index;
    }
    if keep_from > 0 {
        entries.drain(..keep_from);
    }
}

fn serialized(entries: &[String]) -> String {
    if entries.is_empty() {
        String::new()
    } else {
        entries.join("\n") + "\n"
    }
}

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
        trim_entries(&mut entries);
        Ok(Self {
            entries,
            path: Some(path),
        })
    }

    pub fn push(&mut self, entry: String) {
        self.entries.push(entry.clone());
        let old_len = self.entries.len();
        trim_entries(&mut self.entries);
        let trimmed = self.entries.len() < old_len;
        let Some(path) = &self.path else {
            return;
        };
        let result: io::Result<()> = (|| {
            let current_bytes = fs::metadata(path).map_or(0, |metadata| metadata.len());
            let entry_bytes = entry.len() as u64 + 1;
            if trimmed || current_bytes + entry_bytes > MAX_BYTES {
                fs::write(path, serialized(&self.entries))?;
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

    #[test]
    fn persisted_history_stays_within_byte_limit() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("installer.log");
        let mut log = SessionLog::open(path.clone()).unwrap();
        log.push("x".repeat(MAX_BYTES as usize + 1));
        assert!(log.entries.is_empty());
        assert_eq!(fs::metadata(&path).unwrap().len(), 0);

        log.push("recent entry".into());
        assert!(fs::metadata(&path).unwrap().len() <= MAX_BYTES);
        assert_eq!(
            SessionLog::open(path).unwrap().entries,
            vec!["recent entry"]
        );
    }
}
