use log::*;
fn main() {
    env_logger::init();
    info!("starting");
    relatable::get_tagged_files("s:/git/terable/testdata/").unwrap();
    info!("finished");
}
