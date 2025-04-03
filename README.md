# ftfrs-tracing

A Rust [tracing](https://crates.io/crates/tracing) Layer implementation for the Fuchsia Trace Format (FTF).

This library provides a bridge between Rust's tracing ecosystem and Fuchsia's trace format, allowing you to emit trace data from Rust applications that can be visualized and analyzed using Fuchsia's trace tooling.

> ⚠️ **WARNING** ⚠️  
> This is prototype, in-development software. The API may change significantly between versions and some features are not yet fully implemented. Use in production environments is not recommended at this time.

## Features

- **Efficient String and Thread Interning**: Automatically interns string and thread references using StringRef::Ref and ThreadRef::Ref for improved performance.
- **Attribute Support**: Captures span and event attributes as FTF Arguments for rich, detailed trace data.
- **Selective Tracing**: Spans and events can be selectively included in the trace via the `ftf=true` attribute.
- **Custom Categories**: Support for custom trace categories via the `category="name"` attribute.
- **Proper Thread ID Handling**: Consistent thread ID handling for accurate trace visualization.
- **Robust Error Handling**: Graceful handling of errors during trace recording.

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
ftfrs-tracing = "0.1.0"
tracing = "0.1.41"
tracing-subscriber = "0.3.19"
```

## Usage

### Basic Setup

```rust
use std::fs::File;
use ftfrs_tracing::FtfLayer;
use tracing_subscriber::{self, layer::SubscriberExt};

fn main() {
    // Create an FtfLayer that writes to a file
    let layer = FtfLayer::new(File::create("./trace.ftf").unwrap());
    
    // Add the layer to the tracing subscriber
    let subscriber = tracing_subscriber::Registry::default().with(layer);
    
    // Set the subscriber as the global default
    tracing::subscriber::set_global_default(subscriber).unwrap();
    
    // Your application code here...
}
```

### Selective Tracing with `ftf=true`

Only spans and events with the `ftf=true` attribute will be included in the trace:

```rust
use tracing::{span, event, Level};

// This span will be included in the trace
let span = span!(Level::INFO, "my_span", ftf = true);
let _guard = span.enter();

// This event will be included (inside a span with ftf=true)
event!(Level::INFO, message = "This event will be traced");

// This event explicitly opts into tracing
event!(Level::INFO, ftf = true, message = "Explicit tracing event");
```

### Using Custom Categories

Categorize your spans and events for better organization:

```rust
// Span with custom category
let span = span!(Level::INFO, "render_frame", ftf = true, category = "rendering");
let _guard = span.enter();

// Event inherits parent span's category
event!(Level::DEBUG, message = "Drawing UI components");

// Event with explicit category
event!(Level::INFO, ftf = true, category = "metrics", 
       message = "Frame time", duration_ms = 16.7);
```

### Instrumenting Functions

Use the `#[instrument]` attribute macro for convenient function tracing:

```rust
use tracing::instrument;

#[instrument(fields(ftf = true, category = "database"))]
fn query_database(query: &str) -> Result<Vec<Record>, Error> {
    // Function body...
    event!(Level::DEBUG, query = query, rows = result.len());
    Ok(result)
}
```

## Configuration

You can customize the layer with `FtfLayerConfig`:

```rust
use ftfrs_tracing::{FtfLayer, FtfLayerConfig};

let config = FtfLayerConfig {
    provider_id: 42,
    provider_name: "my_app".to_string(),
    process_id: None, // Auto-detect
};

let layer = FtfLayer::with_config(output, config);
```

## Attribute Types

The following attribute types are supported and will be converted to appropriate FTF Arguments:

- Strings → `Argument::Str`
- Integers (i64/u64) → `Argument::Int64`/`Argument::UInt64`
- Floats (f64) → `Argument::Float`
- Booleans → `Argument::Boolean`
- Other types → Converted to string representation

## Special Attributes

- `ftf = true` - Marks a span or event for inclusion in the trace
- `category = "name"` - Sets the category for a span or event

## License

MIT License