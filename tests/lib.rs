use git_iris::logger;

#[cfg(test)]
mod tests {
    use super::*;

    fn _setup() {
        let _ = logger::init(); // Initialize the logger
        logger::enable_logging(); // Enable logging
        logger::set_log_to_stdout(true); // Set logging to stdout
    }
}