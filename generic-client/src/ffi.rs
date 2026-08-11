use crate::{Connectivity, MPClipboard, Output};
use anyhow::{Context, Result};
use mpclipboard_shared::{error, info};
use std::{ffi::c_char, os::fd::AsRawFd};

macro_rules! try_or_null {
    ($v:expr) => {
        match $v {
            Ok(v) => v,
            Err(err) => {
                error!("{err:?}");
                return core::ptr::null_mut();
            }
        }
    };
}

fn cstring_to_str(s: *const c_char) -> Result<&'static str> {
    let s = unsafe { std::ffi::CStr::from_ptr(s) };
    s.to_str().context("non-utf8 string")
}
fn string_to_c(s: String) -> (*mut c_char, usize) {
    let (ptr, len, _capacity) = s.into_raw_parts();
    (ptr.cast(), len)
}

/// Initializes `MPClipboard`, must be called once at startup
#[unsafe(no_mangle)]
pub extern "C" fn mpclipboard_init() -> bool {
    if let Err(err) = MPClipboard::init() {
        error!("{err:?}");
        false
    } else {
        true
    }
}

// /// Reads the config based on the given instruction
// /// (which is either "read from XDG dir" or "read from local ./config.toml").
// /// In case of an error logs it and returns NULL.
// #[unsafe(no_mangle)]
// pub extern "C" fn mpclipboard_config_read(option: ConfigReadOption) -> *mut Config {
//     let config = try_or_null!(Config::read(option));
//     Box::leak(Box::new(config))
// }

// /// Constructs the config in-place based on given parameters that match fields 1-to-1.
// /// In case of an error logs it and returns NULL.
// #[unsafe(no_mangle)]
// pub extern "C" fn mpclipboard_config_new(
//     uri: *const c_char,
//     heartbeat_uri: *const c_char,
//     token: *const c_char,
//     name: *const c_char,
// ) -> *mut Config {
//     let uri = cstring_to_str(uri);
//     let heartbeat_uri = cstring_to_str(heartbeat_uri);
//     let token = cstring_to_str(token);
//     let name = cstring_to_str(name);
//     let config = try_or_null!(Config::new(&uri, &heartbeat_uri, token, name));
//     Box::leak(Box::new(config))
// }

/// Constructs a new `MPClipboard` using given options.
#[unsafe(no_mangle)]
pub extern "C" fn mpclipboard_new_inline(
    main_url: *const c_char,
    heartbeat_url: *const c_char,
    token: *const c_char,
    id: *const c_char,
) -> *mut MPClipboard {
    let main_url = try_or_null!(cstring_to_str(main_url));
    let heartbeat_url = try_or_null!(cstring_to_str(heartbeat_url));
    let token = try_or_null!(cstring_to_str(token));
    let id = try_or_null!(cstring_to_str(id));

    let mpclipboard = try_or_null!(MPClipboard::new_inline(main_url, heartbeat_url, token, id));
    Box::leak(Box::new(mpclipboard))
}

/// Constructs a new `MPClipboard` based on the local `config.toml` in the working directory.
#[unsafe(no_mangle)]
pub extern "C" fn mpclipboard_new_with_local_config() -> *mut MPClipboard {
    let mpclipboard = try_or_null!(MPClipboard::new_with_local_config());
    Box::leak(Box::new(mpclipboard))
}

/// Constructs a new `MPClipboard` based on the `$XDG_CONFIG_HOME/mpclipboard/config.toml`.
#[unsafe(no_mangle)]
pub extern "C" fn mpclipboard_new_with_xdg_config() -> *mut MPClipboard {
    let mpclipboard = try_or_null!(MPClipboard::new_with_xdg_config());
    Box::leak(Box::new(mpclipboard))
}

/// Returns the file descriptor for a given `MPClipboard` instance.
#[unsafe(no_mangle)]
pub extern "C" fn mpclipboard_get_fd(mpclipboard: *mut MPClipboard) -> i32 {
    let mpclipboard = unsafe { &*mpclipboard };
    mpclipboard.as_raw_fd()
}

/// Result of reading
#[repr(C)]
pub enum COutput {
    /// An event indicating that connectivity changed, guaranteed to be different from a previous one
    ConnectivityChanged {
        /// New connecivity
        connectivity: Connectivity,
    },
    /// New text clip
    NewText {
        /// New text
        ptr: *mut c_char,
        /// and its length
        len: usize,
    },
    /// Ignore
    Ignore,
    /// Error
    Error,
}
impl From<Output> for COutput {
    fn from(output: Output) -> Self {
        match output {
            Output::ConnectivityChanged { connectivity } => {
                Self::ConnectivityChanged { connectivity }
            }
            Output::NewText { text } => {
                let (ptr, len) = string_to_c(text);
                Self::NewText { ptr, len }
            }
        }
    }
}

/// Reads from a given `MPClipboard` instance.
#[unsafe(no_mangle)]
pub extern "C" fn mpclipboard_read(mpclipboard: *mut MPClipboard) -> COutput {
    let mpclipboard = unsafe { &mut *mpclipboard };
    match mpclipboard.read() {
        Ok(Some(output)) => output.into(),
        Ok(None) => COutput::Ignore,
        Err(err) => {
            error!("{err:?}");
            COutput::Error
        }
    }
}

#[repr(C)]
/// Result of pushing text to `MPClipboard`.
pub enum PushResult {
    /// The text is new, it has been sent.
    Sent,
    /// The text is stale, it's been dropped.
    DroppedAsStale,
    /// Internal error, `MPClipboard` is now in malformed state
    Error,
}

/// Pushes text from pointer + length
/// returns false if given text isn't new
#[unsafe(no_mangle)]
pub extern "C" fn mpclipboard_push_text(
    mpclipboard: *mut MPClipboard,
    ptr: *const c_char,
    len: usize,
) -> PushResult {
    let mpclipboard = unsafe { &mut *mpclipboard };
    let bytes = unsafe { core::slice::from_raw_parts(ptr.cast::<u8>(), len) };
    let text = unsafe { std::str::from_utf8_unchecked(bytes) };

    match mpclipboard.push_text(text) {
        Ok(true) => PushResult::Sent,
        Ok(false) => PushResult::DroppedAsStale,
        Err(err) => {
            error!("{err:?}");
            PushResult::Error
        }
    }
}

/// Drops an instance of `MPClipboard`, frees memory, closes files
#[unsafe(no_mangle)]
pub extern "C" fn mpclipboard_drop(mpclipboard: *mut MPClipboard) {
    unsafe { core::ptr::drop_in_place(mpclipboard) };
}

/// Prints one "info" and one "error" message, useful for testing
#[unsafe(no_mangle)]
pub extern "C" fn mpclipboard_logger_test() {
    info!("info example");
    error!("error example");
}

/// Configures rustls on JVM
#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
pub extern "C" fn mpclipboard_setup_rustls_on_jvm(
    env: *mut jni::sys::JNIEnv,
    context: jni::sys::jobject,
) {
    let mut env = unsafe { jni::EnvUnowned::from_raw(env) };
    let outcome = env.with_env(|env| {
        let context = unsafe { jni::objects::JObject::from_raw(env, context) };
        rustls_platform_verifier::android::init_with_env(env, context)
    });

    match outcome.into_outcome() {
        jni::Outcome::Ok(()) => {}
        jni::Outcome::Err(err) => {
            error!("Failed to instantiate rustls_platform_verifier: {err:?}");
        }
        jni::Outcome::Panic(_) => {
            error!("mpclipboard_setup_rustls_on_jvm panicked");
        }
    }
}
