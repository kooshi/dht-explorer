//TODO
// pub enum Color {
//     None,
//     Ironbow,
// }

// impl Color {
//     pub fn colorize(&self, string: String) -> String {}

//     pub fn color_code(&self, lvl: Level) -> &str {
//         match self {
//             Color::None => "",
//             Color::Ironbow => match lvl {
//                 Level::Error => "230",
//                 Level::Warn => "221",
//                 Level::Info => "166",
//                 Level::Debug => "124",
//                 Level::Trace => "53",
//             },
//         }
//     }
// }
use super::*;

pub fn init_logging() {
    let test_str = if cfg!(test) { "Test-" } else { "" };
    let fmt = Box::new(|color: bool| {
        // //Ironbow
        // let colors = |l: log::Level| match l {
        //     log::Level::Error => "230",
        //     log::Level::Warn => "221",
        //     log::Level::Info => "166",
        //     log::Level::Debug => "124",
        //     log::Level::Trace => "53",
        // };

        //Flame
        let colors = |l: log::Level| match l {
            log::Level::Error => "9",
            log::Level::Warn => "220",
            log::Level::Info => "228",
            log::Level::Debug => "230",
            log::Level::Trace => "248",
        };
        move |out: fern::FormatCallback, message: &std::fmt::Arguments, record: &log::Record| {
            let (ansi_pfx, ansi_sfx) = if color {
                (format!("\x1b[38;5;{}m", (colors)(record.level())), "\x1B[0m".to_owned())
            } else {
                ("".to_owned(), "".to_owned())
            };
            out.finish(format_args!(
                "{}{}[{}][{}][{}] {}{}",
                ansi_pfx,
                test_str,
                chrono::Local::now().format("%Y-%m-%d %T:%3f"),
                record.target(),
                record.level(),
                message,
                ansi_sfx
            ))
        }
    });

    crate::init_fail!(
        Dispatch::new()
            .chain(
                Dispatch::new()
                    .format((fmt)(!param!().log_no_color))
                    .level(param!().log_std_level.unwrap_or(param!().log_level))
                    .chain(std::io::stdout())
            )
            .chain(
                Dispatch::new()
                    .format((fmt)(false))
                    .level(param!().log_file_level.unwrap_or(param!().log_level))
                    .chain(init_fail!(fern::log_file(
                        chrono::Local::now()
                            .format(&(param!().log_dir.to_string() + test_str + &param!().log_file))
                            .to_string()
                    )))
            )
            .apply()
    );
}

#[cfg(test)]
mod tests {
    #[test]
    fn hello() {
        log::trace!("qwertyuiopasdfghjklzxcvbnm");
        log::debug!("qwertyuiopasdfghjklzxcvbnm");
        log::info!("qwertyuiopasdfghjklzxcvbnm");
        log::warn!("qwertyuiopasdfghjklzxcvbnm");
        log::error!("qwertyuiopasdfghjklzxcvbnm");
    }
}
