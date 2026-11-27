fn main() {
    // Turns Rust panics into readable console errors instead of a bare
    // "unreachable" in the browser devtools.
    console_error_panic_hook::set_once();
    leptos::mount::mount_to_body(wizard::App);
}
