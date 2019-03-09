use crate::app::events::{self, BackgroundEvent, PrivilegedEvent, UiEvent};
use crate::block::BlockDevice;
use atomic::Atomic;
use crossbeam_channel::{unbounded, Receiver, Sender};
use libc;
use std::cell::{Cell, RefCell};
use std::env;
use std::fs::File;
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ActiveView {
    Images,
    Devices,
    Flashing,
    Summary,
    Error,
}

pub struct State {
    pub ui_event_tx: Sender<UiEvent>,
    pub ui_event_rx: Receiver<UiEvent>,
    pub back_event_tx: Sender<BackgroundEvent>,
    pub priv_event_tx: Sender<PrivilegedEvent>,

    pub active_view: Cell<ActiveView>,

    pub image: RefCell<Option<File>>,
    pub image_path: RefCell<PathBuf>,
    pub image_size: Arc<Atomic<u64>>,

    pub available_devices: RefCell<Box<[BlockDevice]>>,
    pub selected_devices: RefCell<Vec<BlockDevice>>,
}

impl State {
    pub fn new() -> Self {
        let (back_event_tx, back_event_rx) = unbounded();
        let (priv_event_tx, priv_event_rx) = unbounded();
        let (ui_event_tx, ui_event_rx) = unbounded();

        events::privileged(ui_event_tx.clone(), priv_event_rx);

        // If running in pkexec or sudo, restore home directory for open dialog,
        // and then downgrade permissions back to a regular user.
        if let Ok(pkexec_uid) = env::var("PKEXEC_UID").or_else(|_| env::var("SUDO_UID")) {
            if let Ok(uid) = pkexec_uid.parse::<u32>() {
                if let Some(passwd) = pwd::Passwd::from_uid(uid) {
                    env::set_var("HOME", passwd.dir);
                    downgrade_permissions(passwd.uid, passwd.gid);
                }
            }
        }

        events::unprivileged(ui_event_tx.clone(), back_event_rx);

        Self {
            ui_event_rx,
            ui_event_tx,
            back_event_tx,
            priv_event_tx,
            active_view: Cell::new(ActiveView::Images),
            image: RefCell::new(None),
            image_path: RefCell::new(PathBuf::new()),
            image_size: Arc::new(Atomic::new(0u64)),
            available_devices: RefCell::new(Box::new([])),
            selected_devices: RefCell::new(Vec::new()),
        }
    }
}

/// Downgrades the permissions of the current thread to the specified user and group ID.
fn downgrade_permissions(uid: u32, gid: u32) {
    unsafe {
        // By using system calls directly, we apply this on a per-thread basis.
        // The setresuid() and setresguid() functions apply to all threads.
        libc::syscall(libc::SYS_setresgid, gid, gid, gid);
        libc::syscall(libc::SYS_setresuid, uid, uid, uid);
    }
}
