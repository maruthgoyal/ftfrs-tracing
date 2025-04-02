use std::fs::File;

use tracing::{instrument, trace_span};
use tracing_subscriber::{self, layer::SubscriberExt};
fn main() {
    let layer = ftfrs_subscriber::FtfLayer::new(File::create("./test.ftf").unwrap());
    let subscriber = tracing_subscriber::Registry::default().with(layer);
    tracing::subscriber::set_global_default(subscriber).unwrap();

    {
    let sp = trace_span!("foo");
        let _guard = sp.enter();
    }
    my_thing();
    my_thing();
}

#[instrument]
fn my_thing() -> u8 {
    let mut v: Vec<u16> = (1..1000).collect();
    my_other();
    v.sort();
    1
}

#[instrument]
fn my_other() -> u8 {
    let mut v: Vec<u16> = (1..1000).collect();
    v.sort();
    1
}
