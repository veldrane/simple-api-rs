// Removed conflicting import: use tokio::net::unix::pipe::Sender;
use tokio::sync::mpsc;
use chrono::prelude::*;



pub struct Logger {
    tx: mpsc::Sender<LogMessage>,
    _logger: tokio::task::JoinHandle<()>,
}

enum LogType {
    Info,
    Warning,
    Error,
}

struct LogMessage(LogType,String);

impl Logger {

    pub fn build() -> Self {
        

        let (tx, rx): (mpsc::Sender<LogMessage>, mpsc::Receiver<LogMessage>) = mpsc::channel(100);
        let logger =   tokio::spawn(log_receiver(rx));
        let logger = Logger {
            tx,
            _logger: logger,
        };

        let _ = logger.tx.try_send(LogMessage(LogType::Info, "Starting logger".into()));
        logger
    }

    pub async fn info(&self, message: String) {
        let _ = self.tx.send(LogMessage(LogType::Info, message)).await;
    }

    pub async fn warn(&self, message: String) {
        let _ = self.tx.send(LogMessage(LogType::Warning, message)).await;
    }

    pub async fn error(&self, message: String) {
        let _ = self.tx.send(LogMessage(LogType::Error, message)).await;
    }
}




async fn log_receiver(mut rx: mpsc::Receiver<LogMessage>) {
    while let Some(log_message) = rx.recv().await {

        let timestamp =  Utc::now().to_string();
        match log_message.0 {
            LogType::Info => println!("{} {} {}",timestamp,"---| Info:", log_message.1),
            LogType::Warning => println!("{}  ---| Warning: {}",timestamp, log_message.1),
            LogType::Error => println!("{} {} {}",timestamp,"---| Error:", log_message.1),
        }
    }
}