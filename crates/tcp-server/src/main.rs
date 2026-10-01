fn main() {
    esp_idf_svc::sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();

    if let Err(e) = run() {
        eprint!("{}", e);
        std::process::exit(1);
    }
}

fn run() -> anyhow::Result<()> {
    todo!();
}
