use log::{debug, error, info, trace, warn};

fn setup_logger() -> Result<(), fern::InitError> {
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
        .level(log::LevelFilter::Trace)
        .apply()?;
    Ok(())
}

fn main() {
    // Create 'logs' directory if it doesn't exist
    std::fs::create_dir_all("logs").unwrap();

    setup_logger().unwrap();

    info!("Hello, world!");
    debug!("This should not be printed");
    warn!("This is a warning");
    error!("This is an error");
    trace!("This should not be printed");
}
