use std::sync::Mutex;

pub static ENV_MUTEX: Mutex<()> = Mutex::new(());
