// use crate::console;
use log::{self, Level, LevelFilter, Log, Metadata, Record};

struct Logger;
impl Log for Logger {
    fn enabled(&self, _metadata: &Metadata) -> bool {
        true
    }

    fn log(&self, record: &Record) {
        if !self.enabled(record.metadata()) {
            return
        }

        print!("\x1b[{}m", log_level_to_color_code(record.level()));
        println!("[{}] {}", record.level(), record.args());
        print!("\x1b[0m");
    }

    fn flush(&self) {}
}

fn log_level_to_color_code(level: Level) -> u8 {
    match level {
        Level::Error => 31, //Red
        Level::Warn => 93, //Yellow
        Level::Info => 34, //Blue
        Level::Debug => 32, //Green
        Level::Trace => 90, //Black
    }
}

pub fn init() {
   static LOGGER: Logger = Logger;
   log::set_logger(&LOGGER).unwrap();
   
   log::set_max_level(match option_env!("LOG") {
       Some("error") => LevelFilter::Error,
       Some("warn") => LevelFilter::Warn,
       Some("info") => LevelFilter::Info,
       Some("debug") => LevelFilter::Debug,
       Some("trace") => LevelFilter::Trace,
       _ => LevelFilter::Off,
   });
   
   //log::set_max_level(LevelFilter::Info);
}
