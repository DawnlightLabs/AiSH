mod logging;
mod runtime_bootstrap;
mod setup;
mod updater;

mod legacy_main {
    include!(concat!(env!("OUT_DIR"), "/main_setup_generated.rs"));
}

fn main() {
    runtime_bootstrap::configure();
    legacy_main::run();
}
