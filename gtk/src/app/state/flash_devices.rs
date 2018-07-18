use super::*;
use gtk;
use gtk::prelude::*;
use std::sync::{Arc, Mutex};
use popsicle::mnt;
use std::mem;
use std::path::Path;
use std::time::{Duration, Instant};

macro_rules! try_or_error {
    (
        $act:expr,
        $view:expr,
        $back:expr,
        $next:expr,
        $stack:ident,
        $error:ident,
        $msg:expr,
        $val:expr
    ) => {
        match $act {
            Ok(value) => value,
            Err(why) => {
                $back.set_visible(false);
                $next.set_visible(true);
                $next.set_label("Close");
                $next.get_style_context().map(|c| {
                    c.remove_class("destructive-action");
                    c.remove_class("suggested-action");
                });
                $error.set_text(&format!("{}: {:?}", $msg, why));
                $view.set(2);
                $stack.set_visible_child_name("error");
                return $val;
            }
        }
    };
}

/// Begins the flashing process.
pub fn flash_devices(
    state: &State,
    back: &gtk::Button,
    error: &gtk::Label,
    next: &gtk::Button,
    stack: &gtk::Stack,
    summary_grid: &gtk::Grid,
) {
    let task_handles = &state.task_handles;
    let bars = &state.bars;
    let start = &state.start;
    let tasks = &state.tasks;
    let flash_state = state.flash_state.clone();

    let mut data = Vec::new();
    let image_data: Arc<Vec<u8>>;

    {
        let mut image_buffer_lock = try_or_error!(
            state.buffer.data.lock(),
            state.view,
            back,
            next,
            stack,
            error,
            "mutex lock failure",
            ()
        );

        let device_list = try_or_error!(
            state.devices.lock(),
            state.view,
            back,
            next,
            stack,
            error,
            "device list mutex lock failure",
            ()
        );
        let devs = device_list
            .iter()
            .filter(|x| x.1.get_active())
            .map(|x| x.0.clone())
            .collect::<Vec<_>>();

        let mounts = try_or_error!(
            mnt::get_submounts(Path::new("/")),
            state.view,
            back,
            next,
            stack,
            error,
            "unable to obtain mount points",
            ()
        );

        try_or_error!(
            state.devices_request.send((devs, mounts.clone())),
            state.view,
            back,
            next,
            stack,
            error,
            "unable to send device request",
            ()
        );

        let disks_result = try_or_error!(
            state.devices_response.recv(),
            state.view,
            back,
            next,
            stack,
            error,
            "unable to get device request response",
            ()
        );

        let disks = try_or_error!(
            disks_result,
            state.view,
            back,
            next,
            stack,
            error,
            "unable to get devices",
            ()
        );

        back.get_style_context().map(|c| {
            c.remove_class("back-button");
            c.add_class("destructive-action");
        });

        back.set_label("Cancel");
        back.set_visible(true);
        next.set_visible(false);
        stack.set_visible_child_name("flash");

        // Clear the progress bar summaries.
        let mut bars = bars.borrow_mut();
        bars.clear();
        summary_grid.get_children().iter().for_each(|c| c.destroy());

        *start.borrow_mut() = Instant::now();
        let mut tasks = try_or_error!(
            tasks.lock(),
            state.view,
            back,
            next,
            stack,
            error,
            "tasks mutex lock failure",
            ()
        );

        let mut task_handles = try_or_error!(
            task_handles.lock(),
            state.view,
            back,
            next,
            stack,
            error,
            "task handles mutex lock failure",
            ()
        );

        state.flash_state.store(1, Ordering::SeqCst);

        // Take ownership of the data, so that we may wrap it within an `Arc`
        // and redistribute it across threads.
        //
        // Note: Possible optimization could be done to avoid the wrap.
        //       Avoiding the wrap could eliminate two allocations.
        image_data = {
            let (_, ref mut image_data) = *image_buffer_lock;
            mem::swap(&mut data, image_data);
            Arc::new(data)
        };

        for (id, (disk_path, mut disk)) in disks.into_iter().enumerate() {
            let id = id as i32;
            let image_data = image_data.clone();
            let progress = Arc::new(AtomicUsize::new(0));
            let finished = Arc::new(AtomicUsize::new(0));
            let pbar = ProgressBar::new();
            pbar.set_hexpand(true);

            let label = {
                let disk_path = try_or_error!(
                    Path::new(&disk_path).canonicalize(),
                    state.view,
                    back,
                    next,
                    stack,
                    error,
                    format!("unable to get canonical path of {}", disk_path),
                    ()
                );
                if let Some(block) = BlockDevice::new(&disk_path) {
                    gtk::Label::new(
                        [&block.label(), " (", &disk_path.to_string_lossy(), ")"]
                            .concat()
                            .as_str(),
                    )
                } else {
                    gtk::Label::new(disk_path.to_string_lossy().as_ref())
                }
            };

            label.set_justify(gtk::Justification::Right);
            label.get_style_context().map(|c| c.add_class("bold"));
            let bar_label = gtk::Label::new("");
            bar_label.set_halign(gtk::Align::Center);
            let bar_container = gtk::Box::new(Orientation::Vertical, 0);
            bar_container.pack_start(&pbar, false, false, 0);
            bar_container.pack_start(&bar_label, false, false, 0);
            summary_grid.attach(&label, 0, id, 1, 1);
            summary_grid.attach(&bar_container, 1, id, 1, 1);
            bars.push((pbar, bar_label));

            // Spawn a thread that will update the progress value over time.
            //
            // The value will be stored within an intermediary atomic integer,
            // because it is unsafe to send GTK widgets across threads.
            task_handles.push({
                let _ = state.flash_request.send(FlashRequest::new(
                    disk,
                    disk_path,
                    image_data.len() as u64,
                    image_data,
                    flash_state.clone(),
                    progress.clone(),
                    finished.clone()
                ));

                state.flash_response.recv().expect("expected join handle to be returned")
            });

            tasks.push(FlashTask {
                previous: Arc::new(Mutex::new([0; 7])),
                progress,
                finished,
            });
        }
    }

    // Return ownership of the image data as soon as possible
    let buffer = state.buffer.clone();
    thread::spawn(move || loop {
        if flash_state.load(Ordering::SeqCst) == 2 {
            flash_state.store(3, Ordering::SeqCst);

            // Wait for the main GTK event loop to sleep so that we have exclusive state lock access.
            while 4 != flash_state.load(Ordering::SeqCst) {
                thread::sleep(Duration::from_millis(16))
            }

            // This will be 1 once the device flashing threads have exited.
            while 1 != Arc::strong_count(&image_data) {
                thread::sleep(Duration::from_millis(16));
            }

            let (_, ref mut data) = *buffer.data.lock().expect("failed to get lock on buffer.data");
            let mut replace_with = Arc::try_unwrap(image_data).expect("image_data is still shared");
            mem::swap(data, &mut replace_with);

            flash_state.store(5, Ordering::SeqCst);
            break
        }

        thread::sleep(Duration::from_millis(16));
    });

    summary_grid.show_all();
}
