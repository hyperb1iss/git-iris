use chrono::Local;
use once_cell::sync::Lazy;
use std::fs::OpenOptions;
use std::io::Write;
use std::sync::Mutex;

/// Static mutex-protected log file handle
static LOG_FILE: Lazy<Mutex<std::fs::File>> = Lazy::new(|| {
    Mutex::new(
        OpenOptions::new()
            .create(true)
            .append(true)
            .open("git-iris-debug.log")
            .expect("Failed to open log file"),
    )
});

/// Flag to control whether logging is enabled
static LOGGING_ENABLED: Lazy<Mutex<bool>> = Lazy::new(|| Mutex::new(false));

/// Function to enable logging
pub fn enable_logging() {
    let mut logging_enabled = LOGGING_ENABLED.lock().unwrap();
    *logging_enabled = true;
}

/// Function to disable logging
pub fn disable_logging() {
    let mut logging_enabled = LOGGING_ENABLED.lock().unwrap();
    *logging_enabled = false;
}

/// Log a message with the given level
pub fn log(level: &str, message: &str) {
    let logging_enabled = LOGGING_ENABLED.lock().unwrap();
    if *logging_enabled {
        let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S%.3f");
        let log_message = format!("{} {}: {}\n", timestamp, level, message);

        // Write to file only
        if let Ok(mut file) = LOG_FILE.lock() {
            let _ = file.write_all(log_message.as_bytes());
        }
    }
}

/// Macro for logging debug messages
#[macro_export]
macro_rules! log_debug {
    ($($arg:tt)*) => {
        $crate::logger::log("DEBUG", &format!($($arg)*))
    };
}

/// Macro for logging error messages
#[macro_export]
macro_rules! log_error {
    ($($arg:tt)*) => {
        $crate::logger::log("ERROR", &format!($($arg)*))
    };
}
