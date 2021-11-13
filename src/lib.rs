#![deny(missing_docs)]
#![deny(missing_doc_code_examples)]
//! A [loki](https://grafana.com/oss/loki/) logger for the [`log`](https://crates.io/crates/log) facade.
//! One event is written and send to loki per log call. Each event contain the time in nano second
//! it was scheduled to be sent, in most cases, when the logging occured.
//!
//! # Examples
//!
//! You simply need to specify your [loki push URL](https://grafana.com/docs/loki/latest/api/#post-lokiapiv1push) and the minimum log level to start the logger.
//!
//! ```rust
//! # extern crate log;
//! # extern crate loki_logger;
//! use log::LevelFilter;
//!
//! # #[tokio::main]
//! # async fn main() {
//! loki_logger::init(
//!     "http://loki:3100/loki/api/v1/push",
//!     log::LevelFilter::Info,
//! ).unwrap();
//!
//! log::info!("Logged into Loki !");
//! # }
//! ```
//!
//! Or specify [static labels](https://grafana.com/docs/loki/latest/best-practices/#static-labels-are-good) to use in your loki streams.
//! Those labels are overwrited by event-specific label, if any.
//!
//! ```rust
//! # extern crate log;
//! # extern crate loki_logger;
//! # use std::iter::FromIterator;
//! use std::collections::HashMap;
//! use log::LevelFilter;
//!
//! # #[tokio::main]
//! # async fn main() {
//! let initial_labels = HashMap::from_iter([
//!     ("application".to_string(), "loki_logger".to_string()),
//!     ("environment".to_string(), "development".to_string()),
//! ]);
//!
//! loki_logger::init_with_labels(
//!     "http://loki:3100/loki/api/v1/push",
//!     log::LevelFilter::Info,
//!     initial_labels,
//! ).unwrap();
//!
//! log::info!("Logged into Loki !");
//! # }
//! ```
//! # Log format
//!
//! Each and every log event sent to loki will contain at least the [`level`](log::Level) of the event as well as the [time in nanoseconds](std::time::Duration::as_nanos) of the event scheduling.
//!
//! # Notice on extra labels
//!
//! Starting from 0.4.7, the [`log`](https://crates.io/crates/log) crate started introducing the new key/value system for structured logging.
//!
//! This crate makes heavy use of such system as to create and send custom loki labels.
//!
//! If you want to use this system, you have to use the git version of the log crate and enable the [`kv_unstable`](https://docs.rs/crate/log/0.4.14/features#kv_unstable) feature:
//!
//! ```toml
//! [dependencies.log]
//! # It is recommended that you pin this version to a specific commit to avoid issues.
//! git = "https://github.com/rust-lang/log.git"
//! features = ["kv_unstable"]
//! ```
//!
//! This feature will allow you to use the [`log`](https://crates.io/crates/log) facade as such:
//!
//! ```rust
//! # extern crate log;
//! # extern crate loki_logger;
//! # use std::iter::FromIterator;
//! use std::collections::HashMap;
//! use log::LevelFilter;
//!
//! # #[tokio::main]
//! # async fn main() {
//! let initial_labels = HashMap::from_iter([
//!     ("application".to_string(), "loki_logger".to_string()),
//!     ("environment".to_string(), "development".to_string()),
//! ]);
//!
//! loki_logger::init_with_labels(
//!     "http://loki:3100/loki/api/v1/push",
//!     log::LevelFilter::Info,
//!     initial_labels,
//! ).unwrap();
//!
//! // Due to stabilization issue, this is still unstable,
//! // the log macros needs to have at least one formatting parameter for this to work.
//! log::info!(foo = "bar"; "Logged into Loki !{}", "");
//! # }
//! ```
//!
//! # Notice on asynchronous execution
//!
//! This crate ships with asynchronous execution, orchestrated with [`tokio`](https://tokio.rs/), by default.
//!
//! This means that for the logging operations to work, you need to be in the scope of a asynchronous runtime first.
//!
//! Otherwise, you can activate the `blocking` feature of this crate to use a blocking client.
//!
//! THIS IS NOT RECOMMENDED FOR PRODUCTIONS WORKLOAD.

use serde::Serialize;
use std::{
    collections::HashMap,
    time::{SystemTime, UNIX_EPOCH},
};

use log::{
    kv::{Source, Visitor},
    LevelFilter, Metadata, Record, SetLoggerError,
};

#[derive(Serialize)]
struct LokiStream {
    stream: HashMap<String, String>,
    values: Vec<[String; 2]>,
}

#[derive(Serialize)]
struct LokiRequest {
    streams: Vec<LokiStream>,
}

#[cfg(not(feature = "blocking"))]
struct LokiLogger {
    url: String,
    initial_labels: Option<HashMap<String, String>>,
    client: reqwest::Client,
}

#[cfg(feature = "blocking")]
struct LokiLogger {
    url: String,
    initial_labels: Option<HashMap<String, String>>,
    client: reqwest::blocking::Client,
}

fn init_inner<S: AsRef<str>>(
    url: S,
    max_log_level: LevelFilter,
    initial_labels: Option<HashMap<String, String>>,
) -> Result<(), SetLoggerError> {
    let logger = Box::new(LokiLogger::new(url, initial_labels));
    log::set_boxed_logger(logger).map(|()| log::set_max_level(max_log_level))
}

/// Configure the [`log`](https://crates.io/crates/log) facade to log to [loki](https://grafana.com/oss/loki/).
///
/// This function initialize the logger with no defaults [static labels](https://grafana.com/docs/loki/latest/best-practices/#static-labels-are-good).
/// To use them, you may want to use [`init_with_labels`].
///
/// # Example
///
/// Usage:
///
/// ```rust
/// # extern crate log;
/// # extern crate loki_logger;
/// use log::LevelFilter;
///
/// # #[tokio::main]
/// # async fn main() {
/// loki_logger::init(
///     "http://loki:3100/loki/api/v1/push",
///     log::LevelFilter::Info,
/// ).unwrap();
///
/// log::info!("Logged into Loki !");
/// # }
/// ```
pub fn init<S: AsRef<str>>(url: S, max_log_level: LevelFilter) -> Result<(), SetLoggerError> {
    init_inner(url, max_log_level, None)
}

/// Configure the [`log`](https://crates.io/crates/log) facade to log to [loki](https://grafana.com/oss/loki/).
///
/// This function initialize the logger with defaults [static labels](https://grafana.com/docs/loki/latest/best-practices/#static-labels-are-good).
/// To not use them, you may want to use [`init`].
///
/// # Example
///
/// Usage:
///
/// ```rust
/// # extern crate log;
/// # extern crate loki_logger;
/// # use std::iter::FromIterator;
/// use std::collections::HashMap;
/// use log::LevelFilter;
///
/// # #[tokio::main]
/// # async fn main() {
/// let initial_labels = HashMap::from_iter([
///     ("application".to_string(), "loki_logger".to_string()),
///     ("environment".to_string(), "development".to_string()),
/// ]);
///
/// loki_logger::init_with_labels(
///     "http://loki:3100/loki/api/v1/push",
///     log::LevelFilter::Info,
///     initial_labels
/// ).unwrap();
///
/// log::info!("Logged into Loki !");
/// # }
/// ```
pub fn init_with_labels<S: AsRef<str>>(
    url: S,
    max_log_level: LevelFilter,
    initial_labels: HashMap<String, String>,
) -> Result<(), SetLoggerError> {
    init_inner(url, max_log_level, Some(initial_labels))
}

struct LokiVisitor<'kvs> {
    values: HashMap<log::kv::Key<'kvs>, log::kv::Value<'kvs>>,
}

impl<'kvs> LokiVisitor<'kvs> {
    pub fn new(count: usize) -> Self {
        Self {
            values: HashMap::with_capacity(count),
        }
    }

    pub fn read_kv(
        &'kvs mut self,
        source: &'kvs dyn Source,
    ) -> Result<&HashMap<log::kv::Key<'kvs>, log::kv::Value<'kvs>>, log::kv::Error> {
        for _ in 0..source.count() {
            source.visit(self)?;
        }
        Ok(&self.values)
    }
}

impl<'kvs> Visitor<'kvs> for LokiVisitor<'kvs> {
    fn visit_pair(
        &mut self,
        key: log::kv::Key<'kvs>,
        value: log::kv::Value<'kvs>,
    ) -> Result<(), log::kv::Error> {
        self.values.insert(key, value);
        Ok(())
    }
}

impl log::Log for LokiLogger {
    fn enabled(&self, _: &Metadata) -> bool {
        true
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            let kv = record.key_values();
            let mut visitor = LokiVisitor::new(kv.count());
            let values = visitor.read_kv(kv).unwrap();

            let message = format!("{:?}", record.args());

            let mut labels = if let Some(initial_labels) = &self.initial_labels {
                let mut labels = initial_labels.clone();
                for value in values {
                    labels.insert(value.0.to_string(), value.1.to_string());
                }
                labels
            } else {
                let mut labels = HashMap::new();
                for value in values {
                    labels.insert(value.0.to_string(), value.1.to_string());
                }
                labels
            };

            labels.insert(
                "level".to_string(),
                record.level().to_string().to_ascii_lowercase(),
            );

            self.log_to_loki(message, labels)
        }
    }

    fn flush(&self) {}
}

impl LokiLogger {
    #[cfg(not(feature = "blocking"))]
    fn new<S: AsRef<str>>(url: S, initial_labels: Option<HashMap<String, String>>) -> Self {
        Self {
            url: url.as_ref().to_string(),
            initial_labels,
            client: reqwest::Client::new(),
        }
    }

    #[cfg(feature = "blocking")]
    fn new<S: AsRef<str>>(url: S, initial_labels: Option<HashMap<String, String>>) -> Self {
        Self {
            url: url.as_ref().to_string(),
            initial_labels,
            client: reqwest::blocking::Client::new(),
        }
    }

    #[cfg(not(feature = "blocking"))]
    fn log_to_loki(&self, message: String, labels: HashMap<String, String>) {
        let client = self.client.clone();
        let url = self.url.clone();

        tokio::spawn(async move {
            let start = SystemTime::now();
            let since_the_epoch = start
                .duration_since(UNIX_EPOCH)
                .expect("Time went backwards");

            let time_ns = since_the_epoch.as_nanos().to_string();

            let loki_request = LokiRequest {
                streams: vec![LokiStream {
                    stream: labels,
                    values: vec![[time_ns, message]],
                }],
            };

            if let Err(e) = client.post(url).json(&loki_request).send().await {
                eprintln!("{:?}", e);
            };
        });
    }

    #[cfg(feature = "blocking")]
    fn log_to_loki(&self, message: String, labels: HashMap<String, String>) {
        let url = self.url.clone();
        let start = SystemTime::now();
        let since_the_epoch = start
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards");

        let time_ns = since_the_epoch.as_nanos().to_string();

        let loki_request = LokiRequest {
            streams: vec![LokiStream {
                stream: labels,
                values: vec![[time_ns, message]],
            }],
        };

        if let Err(e) = self.client.post(url).json(&loki_request).send() {
            eprintln!("{:?}", e);
        };
    }
}
