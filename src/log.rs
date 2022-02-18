use {
    lazy_static::lazy_static,
    std::{
        fs::{File, OpenOptions},
        sync::Mutex,
    },
};

lazy_static! {
    pub static ref LOG: Mutex<File> = Mutex::new(
        OpenOptions::new()
            .write(true)
            .truncate(true)
            .create(true)
            .open("vee.log")
            .unwrap()
    );
}

#[allow(unused_macros)]
macro_rules! log {
    ($($arg:tt)*) => {{
        use std::io::Write;
        writeln!(crate::log::LOG.lock().unwrap(), $($arg)*).unwrap();
    }};
}

#[allow(unused_imports)]
pub(crate) use log;
