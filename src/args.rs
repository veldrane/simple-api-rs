use clap::Parser;


/// A simple api for testing purposes with various infrastructure features
#[derive(Parser, Debug)]
#[clap(version, about, long_about = None)]
pub struct Args {
    /// config file path 
    #[clap(short, long, )]
    pub config: Option<String>,
}


impl Args {

    pub fn new() -> Self {
        Args::parse()
    }

    pub fn get_config_path(&self) -> Option<&str> {

        match &self.config {
            Some(c) => Some(c.as_str()),
            None => None,
        }
    }

}