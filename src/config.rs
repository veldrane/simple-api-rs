use std::default;
use serde_yaml::Value;



#[derive(Clone, Debug)]
pub struct Config {
    pub port: u16,
    pub prefix: String,
    pub addr: String,
    pub log_output: String,
    pub fault_inject: FaultInjectConfig
}

#[derive(Clone, Debug)]
pub struct FaultInjectConfig {
    pub error_rate: f32,                // 0.0..=1.0
    pub min_delay: u64,                 // minimální přidané zpoždění v mikrosekundách
    pub max_delay: u64,                 // maximální přidané zpoždění v mikrosekundách
    pub timeout: Option<u64>,           // per-request timeout v sekundách
    pub status_on_error: u16,           // status pro "fuzzy" chybu
}

impl FaultInjectConfig {
    pub fn with_error_rate(mut self, p: f32) -> Self { self.error_rate = p; self }
    pub fn with_min_delay(mut self, min: u64) -> Self { self.min_delay = min; self }
    pub fn with_max_delay(mut self, max: u64) -> Self { self.max_delay = max; self }
    pub fn with_timeout(mut self, dur: u64) -> Self { self.timeout = Some(dur); self }
    pub fn with_status_on_error(mut self, status: u16) -> Self { self.status_on_error = status; self }
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

    pub fn with_fault_inject(mut self, fi: FaultInjectConfig) -> Self {
        self.fault_inject = fi;
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

        let r = std::fs::File::open(&self.path).unwrap();
        let file_value: Value = serde_yaml::from_reader(r).unwrap();
        let mut config = Config::default();

        if let Value::Mapping(map) = file_value {
            for (k, v) in map {
                match v {
                    Value::String(_) => {
                        match k {
                            Value::String(s) if s == "addr" => {
                                config = config.with_addr(v.as_str().unwrap());
                            },
                            Value::String(s) if s == "prefix" => {
                                config = config.with_prefix(v.as_str().unwrap());
                            },
                            Value::String(s) if s == "log" => {
                                config = config.with_log_output(v.as_str().unwrap());
                            },
                            _ => continue,
                        }
                    },
                    Value::Number(_) => {
                        match k {
                            Value::String(s) if s == "port" => {
                                config = config.with_port(v.as_u64().unwrap() as u16);
                            },
                            _ => continue,
                        }
                    },
                    Value::Mapping(_) => {
                        match k {
                            Value::String(s) if s == "fault_inject" => {
                                let fi_config = get_file_inject_config(v);
                                    config = config.with_fault_inject(fi_config);
                                },
                            _ => continue
                            }
                        }
                        // nested mapping - currently not used

                    _ => continue,
                    }
                }
            }
            config
    }
}


fn get_file_inject_config(v: Value) -> FaultInjectConfig   {


    let mut fi_config = FaultInjectConfig::default();

    match v {
        Value::Mapping(fi_map) => {
            for (fi_k, fi_v) in fi_map {
                match fi_v {
                    Value::Number(_) => {
                        match fi_k {
                            Value::String(s) if s == "error_rate" => {
                                fi_config = fi_config.with_error_rate(fi_v.as_f64().unwrap() as f32);
                            },
                            Value::String(s) if s == "min_delay" => {
                                fi_config = fi_config.with_min_delay(fi_v.as_u64().unwrap());
                            },
                            Value::String(s) if s == "max_delay" => {
                                fi_config = fi_config.with_max_delay(fi_v.as_u64().unwrap());
                            },
                            Value::String(s) if s == "timeout" => {
                                fi_config = fi_config.with_timeout(fi_v.as_u64().unwrap());
                            },
                            Value::String(s) if s == "status_on_error" => {
                                fi_config = fi_config.with_status_on_error(fi_v.as_u64().unwrap() as u16);
                            },
                            _ => continue,
                        }
                    },
                    _ => continue,
                }
            }
        },
        _ => {}
    }

    fi_config   
    
}