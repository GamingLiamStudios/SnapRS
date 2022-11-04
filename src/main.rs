mod config;

use std::io::Write;

use log::{debug, error, info, trace, warn};

fn setup_logger(log_level: log::LevelFilter) -> Result<(), fern::InitError> {
    if log_level == log::LevelFilter::Error || log_level == log::LevelFilter::Off {
        println!("\x1B[{}mWARNING: Important messages will be hidden. Please consider setting log_level to \"info\" or \"warn\" in the config file.\x1B[0m", 
            fern::colors::Color::Yellow.to_fg_str(),

        );
    }

    if log_level == log::LevelFilter::Off {
        return Ok(());
    }

    // Colors for the different log levels
    let colors = fern::colors::ColoredLevelConfig::new()
        .error(fern::colors::Color::Red)
        .warn(fern::colors::Color::Yellow)
        .info(fern::colors::Color::White)
        .debug(fern::colors::Color::Blue)
        .trace(fern::colors::Color::Magenta);

    // Shared logger configuration
    let fmt_str = |message: &std::fmt::Arguments, record: &log::Record| -> String {
        // Lifetimes throw a fit if `format_args!`
        format!(
            "[{}][{}] {}",
            chrono::Local::now().format("%Y-%m-%d %H:%M:%S"), //record.target(),
            record.level(),
            message
        )
    };

    // Log to file (without colors)
    let default = fern::Dispatch::new()
        // Log to file (without colors)
        .format(move |out, message, record| {
            out.finish(format_args!("{}", fmt_str(message, record)))
        })
        .chain(fern::log_file(format!(
            "logs/{}.log",
            chrono::Local::now().format("%Y-%m-%d %H:%M:%S")
        ))?);

    // Log to stdout (with colors)
    let color = fern::Dispatch::new()
        .format(move |out, message, record| {
            out.finish(format_args!(
                "{}{}\x1B[0m",
                format_args!("\x1B[{}m", colors.get_color(&record.level()).to_fg_str()),
                fmt_str(message, record)
            ))
        })
        .chain(std::io::stdout());

    // Dispatch to both loggers
    fern::Dispatch::new()
        .chain(default)
        .chain(color)
        .level(log_level)
        .apply()?;
    Ok(())
}

fn main() {
    // Create 'logs' directory if it doesn't exist
    std::fs::create_dir_all("logs").unwrap();

    // Load config from file and merge with default config
    let config = config::Config::load("config.toml"); // merge done inside

    setup_logger(config.general.log_level.into()).unwrap(); // Really hate how I have to use .into()

    info!("Hello, world!");
    debug!("This should not be printed");
    warn!("This is a warning");
    error!("This is an error");
    trace!("This should not be printed");

    // Cleanup

    // Save config to file
    let mut file = std::fs::File::create("config.toml").unwrap();
    file.write_all(toml::to_string(&config).unwrap().as_bytes())
        .unwrap();
}
