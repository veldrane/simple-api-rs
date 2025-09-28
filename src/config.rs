use std::default;

pub struct Config {
    pub port: u16,
    pub prefix: String,
    pub addr: String,
    pub log_output: String,
    pub fault_inject: FaultInjectConfig
}

#[derive(Clone)]
pub struct FaultInjectConfig {
    pub error_rate: f32,                // 0.0..=1.0
    pub min_delay: u64,                 // minimální přidané zpoždění v mikrosekundách
    pub max_delay: u64,                 // maximální přidané zpoždění v mikrosekundách
    pub timeout: Option<u64>,           // per-request timeout v sekundách
    pub status_on_error: u16,           // status pro "fuzzy" chybu
}

impl default::Default for FaultInjectConfig {
    fn default() -> Self {
        FaultInjectConfig {
            error_rate: 0.1,
            min_delay: 50,
            max_delay: 100,
            timeout: Some(2),
            status_on_error: 500,
        }
    }
}

impl Config {
    pub fn new() -> Self {
        Config::default()
    }

    pub fn with_port(mut self, port: u16) -> Self {
        self.port = port;
        self
    }

    pub fn with_addr(mut self, addr: &str) -> Self {
        self.addr = addr.into();
        self
    }
    pub fn with_log_output(mut self, output: &str) -> Self {
        self.log_output = output.into();
        self
    }

    pub fn with_prefix(mut self, prefix: &str) -> Self {
        self.prefix = prefix.into();
        self
    }
}

impl Default for Config {
    fn default() -> Self {
        Config {
            port: 8080,
            prefix: String::from("/api/v1"),
            addr: String::from("0.0.0.0"),
            log_output: String::from("console"),
            fault_inject: FaultInjectConfig::default(),
        }
    }
}

pub trait ConfigLoader {
    fn load(&self) -> Config;
}

pub struct FileConfigLoader {
    pub path: String,
}

impl ConfigLoader for FileConfigLoader {
    fn load(&self) -> Config {
        // For simplicity, we return a default config.
        // In a real implementation, you would read from the file at self.path.
        Config::default()
    }
}