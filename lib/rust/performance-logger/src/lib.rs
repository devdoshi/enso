//!The performance-logger library allows to create performance reports via the
//! [Web Performance API](https://developer.mozilla.org/en-US/docs/Web/API/Performance).
//!
//! The API provided allows the marking of intervals between a start and end, which the gets
//! measured and marked in the Chrome DevTools Performance Monitor. Intervals can be assigned a
//! log level, for example, `TASK`, SECTION` or `DEBUG` to allow filtering based on the current
//! needs.
#![warn(missing_debug_implementations)]
#![warn(missing_docs)]
#![warn(trivial_casts)]
#![warn(trivial_numeric_casts)]
#![warn(unsafe_code)]
#![warn(unused_import_braces)]

use enso_prelude::*;

use enso_prelude::fmt::Formatter;
use enso_web::performance;
use ordered_float::OrderedFloat;
use serde::Deserialize;
use serde::Serialize;
use web_sys::PerformanceEntry;



// ========================================
// === Compile time filtering constants ===
// ========================================

macro_rules! define_profiling_toggle {
    ($log_level_name_upper:ident, $log_level_name:ident) => {
        paste::paste! {
            #[doc = "Defines whether the log level `" $log_level_name "` should be used."]
            #[cfg(feature = "enable-" $log_level_name "-profiling")]
            pub const [< ENABLE_ $log_level_name_upper _PROFILING >]: bool = true;
            #[doc = "Defines whether the log level `" $log_level_name "` should be used."]
            #[cfg(not(feature = "enable-" $log_level_name  "-profiling"))]
            pub const [< ENABLE_ $log_level_name_upper _PROFILING >]: bool = false;
        }
    };
}

define_profiling_toggle!(SECTION, section);
define_profiling_toggle!(TASK, task);
define_profiling_toggle!(DETAIL, detail);
define_profiling_toggle!(DEBUG, debug);



// ====================
// === Measurements ===
// ====================


/// A single interval measurement.
#[derive(Debug, Clone)]
#[allow(missing_docs)]
pub struct Measurement {
    pub name:       String,
    pub start_time: f64,
    pub duration:   f64,
    pub log_level:  LogLevel,
}

impl Measurement {
    /// Return the timestamp of the interval's end.
    pub fn end_time(&self) -> f64 {
        self.start_time + self.duration
    }
}

impl Display for Measurement {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(&format!(
            "[{:.2}, {:.2}] ({:.2}) {}",
            self.start_time,
            self.end_time(),
            self.duration,
            self.name
        ))
    }
}

/// Error that can occur when converting between a `PerformanceEntry` and a `Measurement`.
#[derive(Debug, Clone)]
pub enum MeasurementConversionError {
    /// The name of the `PerformanceEntry` was not in a format that could be decoded to a
    /// `Measurement`.
    InvalidFormatting,
    /// The log level encoded in the `PerformanceEntry`s name is not a valid log level.
    InvalidLogLevel,
}

impl TryFrom<PerformanceEntry> for Measurement {
    type Error = MeasurementConversionError;

    fn try_from(measure: PerformanceEntry) -> Result<Self, Self::Error> {
        use MeasurementConversionError::*;

        let start_time = measure.start_time();
        let duration = measure.duration();
        let name_js = measure.name();
        let name_parts: Vec<_> = name_js.split(MESSAGE_DELIMITER).collect();
        if name_parts.len() < 2 {
            return Err(InvalidFormatting); //FIXME
        }
        let name = name_parts[1].to_string();
        let log_level_name = name_parts[0].to_string();

        let log_level: LogLevel =
            serde_plain::from_str(&log_level_name).or(Err(InvalidLogLevel))?;
        Ok(Measurement { start_time, duration, name, log_level })
    }
}

/// Error that can occur when taking a measurement.
#[derive(Clone, Debug)]
pub enum MeasurementError {
    /// Parsing the measurement information from the performance API failed, for example, due to an
    /// invalid log level.
    InvalidMeasurementConversion,
    /// No measurement was created in the performance API backend.
    NoMeasurementFound,
    /// A function call to the Performance API failed to execute.
    PerformanceAPIExecutionFailure,
    /// Profiling for the given profiling level was disabled.
    ProfilingDisabled,
}



// =================
// === Log Level ===
// =================

#[derive(Copy, Clone, Debug)]
#[derive(Serialize, Deserialize)]
#[allow(missing_docs)]
pub enum LogLevel {
    SECTION,
    TASK,
    DETAIL,
    DEBUG,
}



// =========================
// === Profiling Methods ===
// =========================

/// Delimiter used to to encode information in the `PerformanceEntry` name.
const MESSAGE_DELIMITER: &str = "//";

/// Check at compile time whether the given log level should perform any logging activity.
const fn log_level_is_active(log_level: LogLevel) -> bool {
    match log_level {
        LogLevel::SECTION => ENABLE_SECTION_PROFILING,
        LogLevel::TASK => ENABLE_TASK_PROFILING,
        LogLevel::DETAIL => ENABLE_DETAIL_PROFILING,
        LogLevel::DEBUG => ENABLE_DEBUG_PROFILING,
    }
}

/// Return a string that encodes the given log level and name for a mark that indicates the start of
/// an interval.
fn start_interval_label(log_level: LogLevel, interval_name: &str) -> String {
    format!(
        "{1}{0}{2}{0}start",
        MESSAGE_DELIMITER,
        serde_plain::to_string(&log_level).unwrap(),
        interval_name
    )
}

/// Return a string that encodes the given log level and name for a mark that indicates the end of
/// an interval.
fn end_interval_label(log_level: LogLevel, interval_name: &str) -> String {
    format!(
        "{1}{0}{2}{0}end",
        MESSAGE_DELIMITER,
        serde_plain::to_string(&log_level).unwrap(),
        interval_name
    )
}

/// Return a string that encodes the given log level and name for a measurement.
fn measure_interval_label(log_level: LogLevel, interval_name: &str) -> String {
    format!(
        "{1}{0}{2}{0}measure",
        MESSAGE_DELIMITER,
        serde_plain::to_string(&log_level).unwrap(),
        interval_name
    )
}

/// Start measuring an interval. Returns a `IntervalHandle` that an be used to end the created
/// interval. The interval can also be ended by calling `end_interval` with the same label and log
/// level.
pub fn start_interval(log_level: LogLevel, label: &str) -> IntervalHandle {
    let interval_name = ImString::from(label);
    if log_level_is_active(log_level) {
        performance().mark(&start_interval_label(log_level, &interval_name)).unwrap();
    }
    IntervalHandle::new(log_level, &interval_name)
}

/// End measuring an interval. Return the measurement taken between start and end of the interval,
/// if possible.
pub fn end_interval(
    log_level: LogLevel,
    interval_name: &str,
) -> Result<Measurement, MeasurementError> {
    let start_label = start_interval_label(log_level, interval_name);
    let end_label = end_interval_label(log_level, interval_name);
    let measurement_label = measure_interval_label(log_level, interval_name);
    if !log_level_is_active(log_level) {
        Err(MeasurementError::ProfilingDisabled)
    } else {
        performance().mark(&end_label).or(Err(MeasurementError::PerformanceAPIExecutionFailure))?;
        performance()
            .measure_with_start_mark_and_end_mark(&measurement_label, &start_label, &end_label)
            .or(Err(MeasurementError::PerformanceAPIExecutionFailure))?;

        let entries: js_sys::Array = performance().get_entries_by_type("measure");

        if entries.length() < 1 {
            return Err(MeasurementError::NoMeasurementFound);
        }
        let measure = entries.get(entries.length() - 1);
        let measure: PerformanceEntry = measure.into();

        let measurement: Measurement =
            measure.try_into().or(Err(MeasurementError::InvalidMeasurementConversion))?;
        Ok(measurement)
    }
}

/// Handle that allows ending the interval.
#[derive(Clone, Debug)]
pub struct IntervalHandle {
    interval_name: ImString,
    log_level:     LogLevel,
    released:      bool,
}

impl IntervalHandle {
    fn new(log_level: LogLevel, interval_name: &str) -> Self {
        IntervalHandle { interval_name: interval_name.into(), log_level, released: false }
    }

    /// Measure the interval.
    pub fn measure(mut self) -> Result<Measurement, MeasurementError> {
        self.released = true;
        end_interval(self.log_level, &self.interval_name)
    }

    /// Release the handle to manually call `end_interval` without emitting a warning.
    pub fn release(mut self) {
        self.released = true;
        drop(self)
    }
}

impl Drop for IntervalHandle {
    fn drop(&mut self) {
        if !self.released {
            WARNING!(format!("{} was dropped without a call to `measure`.", self.interval_name));
        }
    }
}

/// Result of profiling a closure via `measure_interval`. Contains the measurement result and the
/// closure return value.
#[derive(Clone, Debug)]
pub struct IntervalMeasurementResult<T> {
    /// Return value of the measured closure.
    pub value:       T,
    /// Measurement result.
    pub measurement: Result<Measurement, MeasurementError>,
}

/// Measure the execution time of the given interval. The interval is executed and the return value
/// and the measurement result are returned in the `IntervalMeasurementResult`.
pub fn measure_interval<T, F: FnMut() -> T>(
    log_level: LogLevel,
    interval_name: &str,
    mut closure: F,
) -> IntervalMeasurementResult<T> {
    start_interval(log_level, interval_name);
    let value = closure();
    let measurement = end_interval(log_level, interval_name);

    IntervalMeasurementResult { value, measurement }
}



// ===============
// === Reports ===
// ===============

/// Report of all measurements taken during the current session.
#[derive(Clone, Debug)]
pub struct Report {
    measures: Vec<Measurement>,
}

impl Report {
    /// Create a new report from the measurements registered in the Performance API.
    pub fn generate() -> Self {
        let js_measures = performance().get_entries_by_type("measure");

        let mut measures = Vec::default();

        js_measures.for_each(&mut |entry, _, _| {
            let entry: PerformanceEntry = entry.into();
            let measurement: Result<Measurement, _> = entry.try_into();
            if let Ok(measurement) = measurement {
                measures.push(measurement)
            }
        });
        Report { measures }
    }
}

impl Display for Report {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let sorted_measurements =
            self.measures.iter().sorted_by_key(|measure| OrderedFloat(measure.start_time));
        for to_print in sorted_measurements {
            f.write_str(&format!("{}\n", &to_print))?;
        }
        Ok(())
    }
}



// ===============================
// === Convenience Definitions ===
// ===============================

macro_rules! define_start_interval_helpers {
    ($log_level:expr, $log_level_name:ident) => {
        paste::paste! {
            #[doc = "Start measuring an interval with log level `" $log_level_name "`. See `start_interval` for more information."]
            pub fn  [< start_interval_ $log_level_name >](interval_name: &str) -> IntervalHandle {
                start_interval($log_level, interval_name)
            }
        }
    };
}

define_start_interval_helpers!(LogLevel::SECTION, section);
define_start_interval_helpers!(LogLevel::TASK, task);
define_start_interval_helpers!(LogLevel::DETAIL, detail);
define_start_interval_helpers!(LogLevel::DEBUG, debug);


macro_rules! define_end_interval_helpers {
    ($log_level:expr, $log_level_name:ident) => {
        paste::paste! {
            #[doc = "End measuring an interval with log level `" $log_level_name "`. See `end_interval` for more information."]
            pub fn  [< end_interval_ $log_level_name >](interval_name: &str) {
                if let Err(error) = end_interval($log_level, interval_name) {
                    WARNING!(format!("Performance logging encountered an error when ending an interval: {:?}", error));
                };
            }
        }
    };
}

define_end_interval_helpers!(LogLevel::SECTION, section);
define_end_interval_helpers!(LogLevel::TASK, task);
define_end_interval_helpers!(LogLevel::DETAIL, detail);
define_end_interval_helpers!(LogLevel::DEBUG, debug);


macro_rules! define_measure_interval_helpers {
    ($log_level:expr, $log_level_name:ident) => {
        paste::paste! {
            #[doc = "Measure the execution time of the given interval with log level `" $log_level_name  "`. See `measure_interval` for more information."]
            pub fn [< measure_interval_ $log_level_name >]<T, F: FnMut() -> T>(interval_name: &str, closure: F) -> IntervalMeasurementResult<T>  {
                measure_interval($log_level, interval_name, closure)
            }
        }
    };
}


define_measure_interval_helpers!(LogLevel::SECTION, section);
define_measure_interval_helpers!(LogLevel::TASK, task);
define_measure_interval_helpers!(LogLevel::DETAIL, detail);
define_measure_interval_helpers!(LogLevel::DEBUG, debug);
