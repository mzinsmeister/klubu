use app::App;
use leptos::mount_to_body;

fn main() {
    _ = console_error_panic_hook::set_once();
    mount_to_body(App);
}
