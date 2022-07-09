use futures::future::{select_all, RemoteHandle};
use futures::{Future, FutureExt};
use log::error;
use regex::Regex;
use simple_error::{simple_error, SimpleResult};
use std::net::{Ipv4Addr, Ipv6Addr};

pub fn safe_string_from_slice(bytes: &[u8]) -> String {
    let r = Regex::new(r"\\x[a-f0-9]{2}").unwrap();
    r.replace_all(&bytes.escape_ascii().to_string(), "·").to_string()
}

pub trait LogErrExt<T> {
    fn log(self);
    fn log_with(self, s: &str);
    fn ok_or_log(self) -> Option<T>;
    fn ok_or_log_with(self, s: &str) -> Option<T>;
}

impl<T, E> LogErrExt<T> for Result<T, E>
where E: std::error::Error
{
    fn log(self) {
        self.log_with("");
    }

    fn log_with(self, s: &str) {
        self.err().iter().for_each(|e| error!("{} {:?}", s, e))
    }

    fn ok_or_log(self) -> Option<T> {
        self.ok_or_log_with("")
    }

    fn ok_or_log_with(self, s: &str) -> Option<T> {
        self.map_or_else(
            |e| {
                error!("{s} {e:?}");
                None
            },
            |t| Some(t),
        )
    }
}

pub trait Ipv6AddrExt {
    fn ms_u64(self) -> u64;
}
impl Ipv6AddrExt for Ipv6Addr {
    fn ms_u64(self) -> u64 {
        u64::from_be_bytes(unsafe { self.octets().as_chunks_unchecked_mut()[0] })
    }
}

pub trait Ipv4AddrExt {
    fn to_u32(self) -> u32;
}
impl Ipv4AddrExt for Ipv4Addr {
    fn to_u32(self) -> u32 {
        u32::from_be_bytes(self.octets())
    }
}

pub struct UnboundedConcurrentTaskSet<T> {
    handles: Vec<RemoteHandle<T>>,
}

impl<T> UnboundedConcurrentTaskSet<T>
where T: Send + 'static
{
    pub fn new() -> Self {
        UnboundedConcurrentTaskSet { handles: Vec::new() }
    }

    pub async fn get_next_result(&mut self) -> Option<T> {
        if self.handles.is_empty() {
            return None;
        }
        let (out, _i, remaining) = select_all(self.handles.drain(..)).await;
        self.handles = remaining;
        Some(out)
    }

    pub fn add_task<F>(&mut self, task: F)
    where F: Future<Output = T> + Send + 'static {
        let (job, handle) = task.remote_handle();
        self.handles.push(handle);
        tokio::spawn(job);
    }
}

pub trait MyFutureExt<T> {
    fn into_remote(self) -> RemoteHandle<T>;
}
impl<T, F> MyFutureExt<T> for F
where
    T: Send + 'static,
    F: Future<Output = T> + Send + 'static,
{
    fn into_remote(self) -> RemoteHandle<T> {
        let (job, handle) = self.remote_handle();
        tokio::spawn(job);
        handle
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

pub trait MySliceExt<T, const N: usize> {
    fn to_sized(&self) -> SimpleResult<[T; N]>;
    fn to_sized_or_fill(&self, fill: T) -> [T; N];
    fn to_sized_or_else_fill<F: Fn() -> T>(&self, fill: F) -> [T; N];
}
impl<T, const N: usize> MySliceExt<T, N> for [T]
where T: Copy + Default
{
    fn to_sized(&self) -> SimpleResult<[T; N]> {
        let mut result = [T::default(); N];
        if self.len() == N {
            result.copy_from_slice(self);
            Ok(result)
        } else {
            Err(simple_error!("invalid slice length"))
        }
    }

    fn to_sized_or_fill(&self, fill: T) -> [T; N] {
        let mut result = [fill; N];
        if self.len() == N {
            result.copy_from_slice(self);
        }
        result
    }

    fn to_sized_or_else_fill<F: Fn() -> T>(&self, fill: F) -> [T; N] {
        let mut result = [T::default(); N];
        if self.len() == N {
            result.copy_from_slice(self);
        } else {
            result.fill_with(fill);
        }
        result
    }
}
