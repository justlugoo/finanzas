#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    #[cfg(target_os = "linux")]
    unsafe {
        std::env::set_var("GTK_OVERLAY_SCROLLING", "0");
    }
    finanzas_lib::run()
}
