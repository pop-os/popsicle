use dbus_udisks2::DiskDevice;
use gdk;
use gtk::{self, prelude::*, SelectionData};

// Implements drag and drop support for a GTK widget.
pub fn drag_and_drop<W, F>(widget: &W, action: F)
where
    W: WidgetExt + WidgetExtManual,
    F: 'static + Fn(&SelectionData),
{
    // Configure the view as a possible drop destination.
    widget.drag_dest_set(gtk::DestDefaults::empty(), &[], gdk::DragAction::empty());

    // Then actually handle drags that are inside the view.
    widget.connect_drag_motion(|_view, ctx, _x, _y, time| {
        ctx.drag_status(gdk::DragAction::COPY, time);
        true
    });

    // Get the dropped data, if possible, when the active drag is valid.
    widget.connect_drag_drop(|view, ctx, _x, _y, time| {
        ctx.list_targets().last().map_or(false, |ref target| {
            view.drag_get_data(ctx, target, time);
            true
        })
    });

    // Then handle the dropped data, setting the image if the dropped data is valid.
    widget.connect_drag_data_received(move |_view, _ctx, _x, _y, data, _info, _time| action(data));
}

pub fn device_label(device: &DiskDevice) -> String {
    if device.drive.vendor.is_empty() {
        format!("{} ({})", device.drive.model, device.parent.preferred_device.display())
    } else {
        format!(
            "{} {} ({})",
            device.drive.vendor,
            device.drive.model,
            device.parent.preferred_device.display()
        )
    }
}
