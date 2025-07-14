
pub struct Config {
    pub port: u16,
    pub prefix: String,
    pub addr: String,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            port: 8080,
            prefix: String::from("/api/v1"),
            addr: String::from("0.0.0.0"),
        }
    }
}