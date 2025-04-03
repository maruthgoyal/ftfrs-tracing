use std::fs::File;

use ftfrs_tracing::FtfLayer;
use tracing::{event, instrument, trace_span, Level};
use tracing_subscriber::{self, layer::SubscriberExt};
fn main() {
    let layer = FtfLayer::new(File::create("./test.ftf").unwrap());
    let subscriber = tracing_subscriber::Registry::default().with(layer);
    tracing::subscriber::set_global_default(subscriber).unwrap();

    {
        // This span has ftf=true and uses the default "trace" category
        let sp = trace_span!("default_category", ftf = true, id = 123, name = "test span");
        let _guard = sp.enter();
        
        // This event will use the parent span's default category
        event!(Level::INFO, message = "Inside default category span", value = 42.5);
    }
    
    {
        // This span has ftf=true and a custom "rendering" category
        let sp = trace_span!("custom_category", ftf = true, category = "rendering", id = 456, name = "render span");
        let _guard = sp.enter();
        
        // This event will inherit the parent span's "rendering" category
        event!(Level::INFO, message = "Using parent's rendering category");
        
        // This event overrides with its own "io" category
        event!(Level::INFO, ftf = true, category = "io", message = "Using explicit IO category");
    }
    
    // This event is outside any span and uses its own category
    event!(Level::INFO, ftf = true, category = "standalone", message = "Standalone event");
    
    // This span is not recorded (no ftf)
    {
        let sp = trace_span!("ignored", id = 789, name = "ignored span");
        let _guard = sp.enter();
        
        // But this event will be recorded with its own category
        event!(Level::INFO, ftf = true, category = "networking", message = "Explicit category with ftf");
    }
    
    // This function has ftf=true in its instrument attribute
    my_thing(42, "test");
    
    // This function call doesn't have ftf=true and won't be recorded
    my_other_thing(84, "second test");
}

#[instrument(fields(ftf = true, category = "database", extra = "data", count = 100))]
fn my_thing(id: u32, name: &str) -> u8 {
    // This event will inherit the parent span's database category
    event!(Level::DEBUG, operation = "collecting", items = 999);
    
    let mut v: Vec<u16> = (1..1000).collect();
    my_other(v.len());
    v.sort();
    
    // This event will use a custom category
    event!(Level::INFO, category = "metrics", sorted = true, size = v.len());
    
    id as u8
}

#[instrument]  // No ftf=true, so not recorded
fn my_other_thing(id: u32, name: &str) -> u8 {
    // This event will NOT be recorded
    event!(Level::DEBUG, operation = "untraced", items = id);
    
    id as u8
}

#[instrument(fields(ftf = true, category = "compute", type = "helper"))]
fn my_other(size: usize) -> u8 {
    let mut v: Vec<u16> = (1..1000).collect();
    
    // This event will inherit the compute category
    event!(
        Level::TRACE, 
        action = "sorting",
        before_first = v[0],
        before_last = v[v.len() - 1]
    );
    
    v.sort();
    
    // This event overrides with a custom category
    event!(
        Level::DEBUG,
        category = "results",
        after_first = v[0],
        after_last = v[v.len() - 1]
    );
    
    1
}
