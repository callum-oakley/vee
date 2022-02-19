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
            .open(".vee.log")
            .unwrap()
    );
}

macro_rules! log {
    ($($t:tt)*) => {{
        use ::std::io::Write;
        writeln!($crate::log::LOG.lock().unwrap(), $($t)*).unwrap();
    }}
}

pub(crate) use log;
