use log::error;
use simple_error::SimpleResult;

pub fn safe_string_from_slice(bytes: &[u8]) -> String {
    bytes.iter().map(|c| format!("{:?}", *c as char).replace("'", "")).collect::<String>()
}

pub trait LogErrExt {
    fn log(self) -> ();
    fn log_with(self, s: &str) -> ();
}

impl<T> LogErrExt for SimpleResult<T> {
    fn log(self) -> () {
        self.log_with("");
    }

    fn log_with(self, s: &str) -> () {
        self.err().iter().for_each(|e| error!("{} {:?}", s, e))
    }
}
// #[macro_export]
// macro_rules! err_trc {
//     () => {
//         .err().iter().for_each(|e| error!("{}: {} | {:?}", file!(), line!(), e))
//     };
// }
// #[macro_export]
// macro_rules! log {
//     () => {
//         map_err(|e| {
//             error!("{}: {} | {:?}", file!(), line!(), e);
//             e
//         })
//     };
// }
