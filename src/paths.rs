use std::env;
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimePaths {
    pub dev_root: PathBuf,
    pub sys_hidraw_root: PathBuf,
}

impl Default for RuntimePaths {
    fn default() -> Self {
        Self {
            dev_root: PathBuf::from("/dev"),
            sys_hidraw_root: PathBuf::from("/sys/class/hidraw"),
        }
    }
}

impl RuntimePaths {
    pub fn from_env() -> Self {
        let mut paths = Self::default();

        if let Some(value) = env::var_os("USB_STATE_SENDER_DEV_ROOT") {
            paths.dev_root = PathBuf::from(value);
        }
        if let Some(value) = env::var_os("USB_STATE_SENDER_SYS_HIDRAW_ROOT") {
            paths.sys_hidraw_root = PathBuf::from(value);
        }

        paths
    }
}
