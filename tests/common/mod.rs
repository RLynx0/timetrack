use std::{
    fs,
    path::PathBuf,
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

pub struct Fixture {
    pub path: PathBuf,
}

impl Fixture {
    pub fn new(prefix: &str) -> Self {
        let id = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("{prefix}-{}-{id}", std::process::id()));
        fs::create_dir(&path).unwrap();
        Self { path }
    }

    pub fn command(&self) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_timetrack"));
        command
            .env("TIMETRACK_HOME", &self.path)
            .env("TIMETRACK_DATA_HOME", &self.path)
            .env("TZ", "UTC")
            .env("NO_COLOR", "1")
            .env_remove("FORCE_COLOR")
            .env_remove("MAX_TABLE_WIDTH");
        command
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.path).unwrap();
    }
}
