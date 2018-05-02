
use super::*;
use gtk;
use gtk::prelude::*;
use popsicle;

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

/// Move to device selection screen
pub fn device_selection(
    state: &State,
    back: &gtk::Button,
    error: &gtk::Label,
    list: &gtk::ListBox,
    next: &gtk::Button,
    stack: &gtk::Stack,
) {
    let device_list = &state.devices;

    back.set_label("Back");
    back.get_style_context().map(|c| {
        c.add_class("back-button");
    });
    next.set_label("Flash");
    next.get_style_context().map(|c| {
        c.remove_class("suggested-action");
        c.add_class("destructive-action");
    });
    stack.set_visible_child_name("devices");

    // Remove all but the first row
    list.get_children()
        .into_iter()
        .for_each(|widget| widget.destroy());

    let mut devices = vec![];
    if let Err(why) = popsicle::get_disk_args(&mut devices) {
        eprintln!("popsicle: unable to get devices: {}", why);
    }

    let mut device_list = try_or_error!(
        device_list.lock(),
        state.view,
        back,
        next,
        stack,
        error,
        "device list mutex lock failure",
        ()
    );
    device_list.clear();
    for device in &devices {
        // Attempt to get the canonical path of the device.
        // Display the error view if this fails.
        let name = try_or_error!(
            Path::new(&device).canonicalize(),
            state.view,
            back,
            next,
            stack,
            error,
            format!("unable to get canonical path of '{}'", device),
            ()
        );

        let button = if let Some(block) = BlockDevice::new(&name) {
            CheckButton::new_with_label(&[
                &block.label(),
                " (",
                &name.to_string_lossy(),
                ")",
            ].concat())
        } else {
            CheckButton::new_with_label(&name.to_string_lossy())
        };

        list.insert(&button, -1);
        device_list.push((device.clone(), button));
    }

    list.show_all();
}
