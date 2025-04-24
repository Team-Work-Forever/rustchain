use flexi_logger::{FileSpec, Logger, WriteMode};

pub fn init_logger(debug: &str, dir_path: &str, prefix: &str) {
    Logger::try_with_str(debug)
        .unwrap()
        .log_to_file(FileSpec::default().directory(dir_path).basename(prefix))
        .write_mode(WriteMode::BufferAndFlush)
        .start()
        .unwrap();
}
