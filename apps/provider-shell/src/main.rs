mod cloud_providers;
mod known_folders;
mod logging;
mod model_registry;
mod runtime_bootstrap;
mod setup;
mod theme;
mod updater;

mod legacy_main {
    include!(concat!(env!("OUT_DIR"), "/main_setup_generated.rs"));
}

fn main() {
    runtime_bootstrap::configure();
    legacy_main::run();
}
