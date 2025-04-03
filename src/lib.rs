use std::collections::HashMap;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Instant;
use std::{fmt, io};

use parking_lot::RwLock;
use tracing_core::{field::{Field, Visit}, span, Event, Subscriber};
use tracing_subscriber::{fmt::MakeWriter, registry::LookupSpan, Layer};

/// A tracing layer that outputs traces in Fuchsia Trace Format (FTF).
///
/// This layer handles span creation, events, and closing of spans,
/// and properly interns strings and thread references for efficient trace output.
#[derive(Debug)]
pub struct FtfLayer<W: for<'a> MakeWriter<'a>> {
    writer: Arc<RwLock<W>>,
    start: Instant,
    /// Cache for interned strings
    string_cache: Arc<RwLock<StringCache>>,
    /// Cache for interned thread references
    thread_cache: Arc<RwLock<ThreadCache>>,
}

#[derive(Debug)]
struct StringCache {
    by_value: HashMap<String, u16>,
    next_id: u16,
}

#[derive(Debug)]
struct ThreadCache {
    by_id: HashMap<(u64, u64), u8>,
    next_id: u8,
}

impl StringCache {
    fn new() -> Self {
        Self {
            by_value: HashMap::new(),
            next_id: 1, 
        }
    }

    fn get_or_create(&mut self, value: &str, writer: &mut impl io::Write) -> Result<ftfrs::StringRef, ftfrs::FtfError> {
        if let Some(&id) = self.by_value.get(value) {
            return Ok(ftfrs::StringRef::Ref(id));
        }

        let id = self.next_id;
        self.next_id = self.next_id.wrapping_add(1);
        if self.next_id == 0 {
            self.next_id = 1; 
        }
        self.by_value.insert(value.to_string(), id);

        let record = ftfrs::Record::create_string(id, value.to_string());
        record.write(writer)?;

        Ok(ftfrs::StringRef::Ref(id))
    }
}

impl ThreadCache {
    fn new() -> Self {
        Self {
            by_id: HashMap::new(),
            next_id: 1, 
        }
    }

    fn get_or_create(
        &mut self,
        process_id: u64,
        thread_id: u64,
        writer: &mut impl io::Write,
    ) -> Result<ftfrs::ThreadRef, ftfrs::FtfError> {
        let key = (process_id, thread_id);
        if let Some(&id) = self.by_id.get(&key) {
            return Ok(ftfrs::ThreadRef::Ref(id));
        }

        let id = self.next_id;
        self.next_id = self.next_id.wrapping_add(1);
        if self.next_id == 0 {
            self.next_id = 1; 
        }
        self.by_id.insert(key, id);

        let record = ftfrs::Record::create_thread(id, process_id, thread_id);
        record.write(writer)?;

        Ok(ftfrs::ThreadRef::Ref(id))
    }
}

#[derive(Debug, Clone)]
pub struct FtfLayerConfig {
    /// Provider information ID
    pub provider_id: u32,
    /// Provider name
    pub provider_name: String,
    /// Optional process ID to use instead of auto-detection
    pub process_id: Option<u64>,
}

impl Default for FtfLayerConfig {
    fn default() -> Self {
        Self {
            provider_id: 1,
            provider_name: "trace".to_string(),
            process_id: None,
        }
    }
}

struct ArgumentVisitor<'a> {
    arguments: Vec<ftfrs::Argument>,
    string_cache: &'a mut StringCache,
    writer: &'a mut dyn io::Write,
}

impl<'a> ArgumentVisitor<'a> {
    fn new(string_cache: &'a mut StringCache, writer: &'a mut dyn io::Write) -> Self {
        Self {
            arguments: Vec::new(),
            string_cache,
            writer,
        }
    }

    fn get_string_ref(&mut self, value: &str) -> ftfrs::StringRef {
        let mut buffer = Vec::new();
        match self.string_cache.get_or_create(value, &mut buffer) {
            Ok(string_ref) => {
                if !buffer.is_empty() {
                    if let Err(e) = self.writer.write_all(&buffer) {
                        eprintln!("Error writing string record: {}", e);
                    }
                }
                string_ref
            }
            Err(_) => {
                ftfrs::StringRef::Inline(value.to_string())
            }
        }
    }
}

impl Visit for ArgumentVisitor<'_> {
    fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
        let name_ref = self.get_string_ref(field.name());
        let value_str = format!("{:?}", value);
        let value_ref = self.get_string_ref(&value_str);
        
        self.arguments.push(ftfrs::Argument::Str(name_ref, value_ref));
    }

    fn record_i64(&mut self, field: &Field, value: i64) {
        let name_ref = self.get_string_ref(field.name());
        
        self.arguments.push(ftfrs::Argument::Int64(name_ref, value));
    }

    fn record_u64(&mut self, field: &Field, value: u64) {
        let name_ref = self.get_string_ref(field.name());
        
        self.arguments.push(ftfrs::Argument::UInt64(name_ref, value));
    }

    fn record_bool(&mut self, field: &Field, value: bool) {
        let name_ref = self.get_string_ref(field.name());
        
        self.arguments.push(ftfrs::Argument::Boolean(name_ref, value));
    }

    fn record_str(&mut self, field: &Field, value: &str) {
        let name_ref = self.get_string_ref(field.name());
        let value_ref = self.get_string_ref(value);
        
        self.arguments.push(ftfrs::Argument::Str(name_ref, value_ref));
    }

    fn record_f64(&mut self, field: &Field, value: f64) {
        let name_ref = self.get_string_ref(field.name());
        
        self.arguments.push(ftfrs::Argument::Float(name_ref, value));
    }

    fn record_i128(&mut self, field: &Field, value: i128) {
        self.record_debug(field, &value);
    }

    fn record_u128(&mut self, field: &Field, value: u128) {
        self.record_debug(field, &value);
    }

    fn record_error(&mut self, field: &Field, error: &(dyn std::error::Error + 'static)) {
        self.record_debug(field, &format!("{}", error));
    }
}

impl<W: for<'a> MakeWriter<'a>> FtfLayer<W> {
    pub fn new(writer: W) -> Self {
        Self::with_config(writer, FtfLayerConfig::default())
    }

    pub fn with_config(writer: W, config: FtfLayerConfig) -> Self {
        let writer = Arc::new(RwLock::new(writer));
        let string_cache = Arc::new(RwLock::new(StringCache::new()));
        let thread_cache = Arc::new(RwLock::new(ThreadCache::new()));
        
        {
            let writer_guard = writer.write();
            let mut w = writer_guard.make_writer();
            
            let magic = ftfrs::Record::create_magic_number();
            if let Err(e) = magic.write(&mut w) {
                eprintln!("Error writing magic number: {}", e);
            }
            
            if let Err(e) = ftfrs::Record::create_provider_info(config.provider_id, config.provider_name)
                .write(&mut w)
            {
                eprintln!("Error writing provider info: {}", e);
            }
        }
        
        Self {
            writer,
            start: Instant::now(),
            string_cache,
            thread_cache,
        }
    }

    /// Get the current time as nanoseconds elapsed since layer creation
    fn now(&self) -> u64 {
        self.start.elapsed().as_nanos() as u64
    }

    /// Get the current process ID
    fn process_id(&self) -> u64 {
        // Return the process ID from the current environment
        std::process::id() as u64
    }

    /// Get the current thread ID
    fn thread_id(&self) -> u64 {
        thread_local! {
            static THREAD_ID: u64 = {
                use std::sync::atomic::AtomicU64;
                static NEXT_THREAD_ID: AtomicU64 = AtomicU64::new(1);
                NEXT_THREAD_ID.fetch_add(1, Ordering::SeqCst)
            }
        }
        
        THREAD_ID.with(|id| *id)
    }
    
    /// Get an interned string reference
    fn get_string_ref(
        &self, 
        value: &str
    ) -> ftfrs::StringRef {
        let mut string_cache = self.string_cache.write();
        let writer_guard = self.writer.write();
        let mut writer = writer_guard.make_writer();
        
        match string_cache.get_or_create(value, &mut writer) {
            Ok(string_ref) => string_ref,
            Err(_) => {
                ftfrs::StringRef::Inline(value.to_string())
            }
        }
    }
    
    /// Get an interned thread reference
    fn get_thread_ref(&self) -> ftfrs::ThreadRef {
        let process_id = self.process_id();
        let thread_id = self.thread_id();
        
        let mut thread_cache = self.thread_cache.write();
        let writer_guard = self.writer.write();
        let mut writer = writer_guard.make_writer();
        
        match thread_cache.get_or_create(process_id, thread_id, &mut writer) {
            Ok(thread_ref) => thread_ref,
            Err(_) => {
                ftfrs::ThreadRef::Inline { 
                    process_koid: process_id, 
                    thread_koid: thread_id 
                }
            }
        }
    }
    
    /// Write a record to the underlying writer
    fn write_record(&self, record: ftfrs::Record) {
        let writer_guard = self.writer.write();
        let mut writer = writer_guard.make_writer();
        if let Err(e) = record.write(&mut writer) {
            eprintln!("Error writing FTF record: {}", e);
        }
    }

    /// Extract arguments from span attributes
    fn record_attributes(
        &self, 
        attrs: &span::Attributes<'_>
    ) -> Vec<ftfrs::Argument> {
        let mut string_cache = self.string_cache.write();
        let writer_guard = self.writer.write();
        let mut writer = writer_guard.make_writer();
        
        let mut visitor = ArgumentVisitor::new(&mut string_cache, &mut writer);
        
        attrs.record(&mut visitor);
        
        visitor.arguments
    }

    /// Extract arguments from event fields
    fn record_event_fields(&self, event: &Event<'_>) -> Vec<ftfrs::Argument> {
        let mut string_cache = self.string_cache.write();
        let writer_guard = self.writer.write();
        let mut writer = writer_guard.make_writer();
        
        let mut visitor = ArgumentVisitor::new(&mut string_cache, &mut writer);
        
        event.record(&mut visitor);
        
        visitor.arguments
    }
}

/// Filter to check if a span should be included in FTF tracing
/// and to extract additional metadata like category
struct FtfFilter {
    should_record: bool,
    category: Option<String>,
}

impl FtfFilter {
    fn new() -> Self {
        Self {
            should_record: false,
            category: None,
        }
    }
}

impl Visit for FtfFilter {
    fn record_bool(&mut self, field: &Field, value: bool) {
        if field.name() == "ftf" && value {
            self.should_record = true;
        }
    }

    fn record_str(&mut self, field: &Field, value: &str) {
        if field.name() == "category" {
            self.category = Some(value.to_string());
        }
    }

    fn record_debug(&mut self, _field: &Field, _value: &dyn fmt::Debug) {}
    fn record_i64(&mut self, _field: &Field, _value: i64) {}
    fn record_u64(&mut self, _field: &Field, _value: u64) {}
    fn record_f64(&mut self, _field: &Field, _value: f64) {}
    fn record_i128(&mut self, _field: &Field, _value: i128) {}
    fn record_u128(&mut self, _field: &Field, _value: u128) {}
    fn record_error(&mut self, _field: &Field, _error: &(dyn std::error::Error + 'static)) {}
}

impl<W, S> Layer<S> for FtfLayer<W>
where
    S: Subscriber + for<'a> LookupSpan<'a>,
    W: for<'writer> MakeWriter<'writer> + 'static,
{
    
    fn on_event(&self, event: &Event<'_>, ctx: tracing_subscriber::layer::Context<'_, S>) {
        let mut filter = FtfFilter::new();
        event.record(&mut filter);
        
        let parent_span_active = if !filter.should_record {
            if let Some(current_span) = ctx.current_span().id() {
                if let Some(span) = ctx.span(current_span) {
                    span.extensions().get::<bool>().copied().unwrap_or(false)
                } else {
                    false
                }
            } else {
                false
            }
        } else {
            false
        };
        
        if !filter.should_record && !parent_span_active {
            return;
        }

        let category = if let Some(cat) = filter.category {
            cat
        } else if let Some(current_span) = ctx.current_span().id() {
            if let Some(span) = ctx.span(current_span) {
                span.extensions().get::<String>().cloned().unwrap_or_else(|| "default".to_string())
            } else {
                "default".to_string()
            }
        } else {
            "default".to_string()
        };
        
        let category_ref = self.get_string_ref(&category);
        let name_ref = self.get_string_ref(event.metadata().name());
        let thread_ref = self.get_thread_ref();
        
        let arguments = self.record_event_fields(event);
        
        let record = ftfrs::Record::create_instant_event(
            self.now(),
            thread_ref,
            category_ref,
            name_ref,
            arguments,
        );
        
        self.write_record(record);
    }

    fn on_close(
        &self,
        id: span::Id,
        ctx: tracing_subscriber::layer::Context<'_, S>,
    ) {
        let span = match ctx.span(&id) {
            Some(span) => span,
            None => return, 
        };
        
        if !span.extensions().get::<bool>().copied().unwrap_or(false) {
            return; 
        }

        let category = span.extensions().get::<String>().cloned().unwrap_or_else(|| "default".to_string());
        let category_ref = self.get_string_ref(&category);
        
        let name_ref = self.get_string_ref(span.name());
        let thread_ref = self.get_thread_ref();

        let event = ftfrs::Record::create_duration_end_event(
            self.now(),
            thread_ref,
            category_ref,
            name_ref,
            Vec::new(),
        );
        
        self.write_record(event);
    }
    
    fn on_new_span(
        &self,
        attrs: &span::Attributes<'_>,
        id: &span::Id,
        ctx: tracing_subscriber::layer::Context<'_, S>,
    ) {
        let mut filter = FtfFilter::new();
        attrs.record(&mut filter);
        
        if let Some(span) = ctx.span(id) {
            span.extensions_mut().insert(filter.should_record);
            if let Some(category) = filter.category.as_ref() {
                span.extensions_mut().insert(category.clone());
            }
        }
        
        if !filter.should_record {
            return; 
        }

        let span = ctx.span(id).expect("span should exist");
        
        let category = filter.category.unwrap_or_else(|| "default".to_string());
        let category_ref = self.get_string_ref(&category);
        
        let name_ref = self.get_string_ref(span.name());
        let thread_ref = self.get_thread_ref();

        let arguments = self.record_attributes(attrs);

        let event = ftfrs::Record::create_duration_begin_event(
            self.now(),
            thread_ref,
            category_ref,
            name_ref,
            arguments,
        );
        
        self.write_record(event);
    }
}

impl<W: for<'a> MakeWriter<'a>> fmt::Display for FtfLayer<W> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "FtfLayer")
    }
}