use gtk::WidgetExt;

pub fn set_margins<W: WidgetExt>(widget: &W, value: i32) {
    widget.set_margin_top(value);
    widget.set_margin_bottom(value);
    widget.set_margin_start(value);
    widget.set_margin_end(value);
}
