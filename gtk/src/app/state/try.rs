#[macro_export]
macro_rules! try_or_error {
    (
        $act:expr,
        $view:expr,
        $back:expr,
        $next:expr,
        $stack:expr,
        $error:expr,
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
