#[doc(hidden)]
pub use log;

#[macro_export]
macro_rules! error {
    ($($arg:tt)+) => {
        ($crate::logger::log::error!(target: "mpclipboard", $($arg)+))
    };
}

#[macro_export]
macro_rules! warn {
    ($($arg:tt)+) => {
        ($crate::logger::log::warn!(target: "mpclipboard", $($arg)+))
    };
}

#[macro_export]
macro_rules! info {
    ($($arg:tt)+) => {
        ($crate::logger::log::info!(target: "mpclipboard", $($arg)+))
    };
}

#[macro_export]
macro_rules! debug {
    ($($arg:tt)+) => {
        ($crate::logger::log::debug!(target: "mpclipboard", $($arg)+))
    };
}

#[macro_export]
macro_rules! trace {
    ($($arg:tt)+) => {
        ($crate::logger::log::trace!(target: "mpclipboard", $($arg)+))
    };
}
