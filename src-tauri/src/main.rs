#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    #[cfg(target_os = "linux")]
    unsafe {
        std::env::set_var("GTK_OVERLAY_SCROLLING", "0");
        if std::env::var("WAYLAND_DISPLAY").unwrap_or_default().is_empty() {
            std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
        }
    }
    finanzas_lib::run()
}
