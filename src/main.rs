#![cfg_attr(windows, windows_subsystem = "windows")]

// Parts of the model are only used by the Windows UI.
#[cfg_attr(not(windows), allow(dead_code))]
mod model;

#[cfg(windows)]
mod app;

fn main() {
    #[cfg(windows)]
    app::run();

    #[cfg(not(windows))]
    {
        // The UI only exists on Windows; on other systems print what it would show.
        let cfg = model::Config::default();
        let now = chrono::Utc::now();
        let pc = chrono::Local::now().date_naive();
        for r in model::rows(&cfg, now, pc) {
            println!("{:<4} {} {}", r.label, r.time, r.marker);
        }
    }
}
