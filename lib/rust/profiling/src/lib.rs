//!The performance-logger library allows to create performance reports via the
//! [Web Performance API](https://developer.mozilla.org/en-US/docs/Web/API/Performance).
//!
//! The API provided allows the marking of intervals between a start and end, which the gets
//! measured and marked in the Chrome DevTools Performance Monitor. Intervals can be assigned a
//! log level, for example, `TASK`, SECTION` or `DEBUG` to allow filtering based on the current
//! needs.
//!
//! Example usage
//! --------------
//! ```
//! let some_task = || "DoWork";
//!
//! // Manually start and end the measurement.
//! let gui_init = profiling::start_task!("GUI initialization");
//! some_task();
//! gui_init.end();
//! // Or use the `measure` method.
//! profiling::task_measure!("GUI initialization", some_task);
//! ```
//!
//! Note that this API and encoding formats for messages are synced with the JS equivalent in
//! `app/ide-desktop/lib/profiling/src/profiling.ts`.
//!
//! Note on adding new profiling levelsThis is done via the
//! `../../../../../app/ide-desktop/lib/profiling/src/profilers.json` config file. Just add a new
//! profiler there and add a new feature to this crate that can toggle the new profiling level to be
//! on/off.
#![warn(missing_debug_implementations)]
#![warn(missing_docs)]
#![warn(trivial_casts)]
#![warn(trivial_numeric_casts)]
#![warn(unsafe_code)]
#![warn(unused_import_braces)]

use crate::js::*;
use enso_prelude::*;
use wasm_bindgen::prelude::*;


use enso_prelude::fmt::Formatter;
use enso_web::performance;
use inflector::Inflector;
use ordered_float::OrderedFloat;
use serde::Deserialize;
use serde::Serialize;
use serde_plain::from_str;
use std::sync::Mutex;
use wasm_bindgen::JsValue;
use web_sys::PerformanceEntry;



// ================
// === Metadata ===
// ================

/// Source code location given as file path and line number.
#[derive(Serialize, Deserialize, Clone, Debug)]
#[allow(missing_docs)]
pub struct SourceLocation {
    pub file: String,
    pub line: u32,
}

/// Measurement metadata. This struct holds information about a measurement and provides
/// functionality for conversion form/to JS for use in the performance API.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Metadata {
    /// Source code location of the profiling interval.
    pub source:          SourceLocation,
    /// Profiling level of the measurement.
    pub profiling_level: ProfilingLevel,
    /// Label of the measurement..
    pub label:           String,
}

impl From<Metadata> for JsValue {
    fn from(metadata: Metadata) -> JsValue {
        JsValue::from_serde(&metadata).expect("Failed to serialise Metadata struct to JSON.")
    }
}

impl TryFrom<JsValue> for Metadata {
    type Error = serde_json::Error;

    fn try_from(value: JsValue) -> Result<Self, Self::Error> {
        value.into_serde()
    }
}



// =================================
// === Custom JS Performance API ===
// =================================

mod js {
    use super::*;
    use js_sys::JsString;
    use wasm_bindgen::JsValue;

    #[wasm_bindgen(inline_js = "
export function mark_with_metadata(markName, markOptions) {
   performance.mark(markName, markOptions)
}

export function measure_with_start_mark_and_end_mark_and_metadata(
    measureName,
    startMark,
    endMark,
    measureOptions
) {
    const options = {}
    options.start = startMark
    options.end = endMark
    options.detail = measureOptions
    performance.measure(measureName, options)
}

")]
    extern "C" {
        #[allow(unsafe_code)]
        pub fn mark_with_metadata(mark_name: JsString, mark_options: JsValue);
        #[allow(unsafe_code)]
        pub fn measure_with_start_mark_and_end_mark_and_metadata(
            measure_name: JsString,
            start_mark: JsString,
            end_mark: JsString,
            measure_options: JsValue,
        );
    }
}



// ===============
// === Loggers ===
// ===============


/// Define a new boolean variable whose value is determined by a feature flag in the crate.
/// The name of the variable is `ENABLED` and it will be true if a feature flag
/// `enable-<log_level_name>-profiling` is set, otherwise it will be false.
macro_rules! define_profiling_toggle {
    ($log_level_name:ident) => {
        paste::item! {
            #[doc = "Defines whether the log level `" $log_level_name "` should be used."]
            #[cfg(feature = "enable-" $log_level_name "-profiling")]
            pub const ENABLED: bool = true;
            #[doc = "Defines whether the log level `" $log_level_name "` should be used."]
            #[cfg(not(feature = "enable-" $log_level_name  "-profiling"))]
            pub const ENABLED: bool = false;
        }
    };
}

pub mod local_macros {
    /// Create a Metadata struct that has the source location prepopulated via the `file!` and
    /// `line!` macros.
    #[macro_export]
    macro_rules! make_metadata {
        ($log_level:expr, $interval_name:expr) => {
            $crate::Metadata {
                source:          $crate::SourceLocation {
                    file: file!().to_string(),
                    line: line!(),
                },
                profiling_level: $log_level.to_string(),
                label:           $interval_name.to_string(),
            }
        };
    }

    #[macro_export]
    macro_rules! start_interval {
        ($log_level:expr, $interval_name:expr) => {
            $crate::start_interval($crate::make_metadata!($log_level, $interval_name));
        };
    }

    #[macro_export]
    macro_rules! end_interval {
        ($log_level:expr, $interval_name:expr) => {
            $crate::end_interval($crate::make_metadata!($log_level, $interval_name));
        };
    }

    #[macro_export]
    macro_rules! measure_interval {
        ($log_level:expr, $interval_name:expr, $($body:expr)*) => {
             $crate::start_interval!($log_level, $interval_name);
             let out = $($body)*;
             $crate::end_interval!($log_level, $interval_name);
             out
        };
    }
}



/// Define a new profiling module that exposes `start`, `end` and `measure` methods. The profiling
/// can be activated and de-activated via a crate feature flag named
/// `enable-<profiling_module_name>-profiling`, which will turn the profiling methods into no-ops.
macro_rules! define_logger {
    ($log_level:expr, $log_level_name_upper:ident, $log_level_name:ident, $start:ident, $end:ident, $measure:ident) => {
        /// Profiler module that exposes methods to measure named intervals.
        pub mod $log_level_name {

            define_profiling_toggle!($log_level_name);

            /// Start measuring a named time interval. Return an `IntervalHandle` that can be used
            /// to end the profiling.
            #[macro_export]
            macro_rules! $start {
                ($interval_name:expr) => {
                    $crate::start_interval!($log_level, $interval_name)
                };
            }

            /// Manually end measuring a named time interval.
            #[macro_export]
            macro_rules! $end {
                ($interval_name:expr) => {
                    $crate::end_interval!($log_level, $interval_name)
                };
            }

            // /// Profile the execution of the given closure.
            // #[macro_export]
            // macro_rules! $measure {
            //     ($interval_name:expr, || $($body:expr)*) => {
            //         $crate::measure_interval!($log_level, $interval_name, $($body:expr)*)
            //     };
            // }
        }
    };
}

// =================
// === Profilers ===
// =================

#[allow(missing_docs)]
type ProfilingLevel = String;


macros::with_all_profiling_levels!(define_logger);


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
    pub log_level:  ProfilingLevel,
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
        let name = measure.name();
        let metadata: Metadata = js_sys::Reflect::get(&measure, &"detail".to_string().into())
            .expect("Could not get details field of PerformanceEntry")
            .try_into()
            .or(Err(InvalidFormatting))?;

        let log_level_name = metadata.profiling_level;
        let log_level_name = log_level_name.to_class_case();

        let log_level: ProfilingLevel = from_str(&log_level_name).or(Err(InvalidLogLevel))?;
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
    PerformanceAPIExecutionFailure {
        /// Underlying error returned by the JS API.
        error: JsValue,
    },
    /// Profiling for the given profiling level was disabled.
    ProfilingDisabled,
}



// ==================================
// === Internal Profiling Methods ===
// ==================================

/// Emit a warning if the given result is an error.
fn warn_on_error(result: Result<Measurement, MeasurementError>) {
    if let Err(e) = result {
        WARNING!(format!("Failed to do profiling for an interval due to error: {:?}", e));
    }
}

/// Return a string that encodes the given log level and name for a mark that indicates the start of
/// an interval.
/// Example output: "DoThing! (FilePath:22) [START]".
fn start_interval_label(metadata: &Metadata) -> String {
    format!("{} [START]", metadata.label)
}

/// Return a string that encodes the given log level and name for a mark that indicates the end of
/// an interval.
/// Example output: "DoThing! (FilePath:22) [END]".
fn end_interval_label(metadata: &Metadata) -> String {
    format!("{} [END]", metadata.label)
}

/// Return a string that encodes the given log level and name for a measurement.  This is done by
/// separating the information by the `MESSAGE_DELIMITER`.
/// Example output: "DoThing! (FilePath:22)".
fn measure_interval_label(metadata: &Metadata) -> String {
    format!("{} ({}:{})", metadata.label, metadata.source.file, metadata.source.line)
}

/// Start measuring an interval. Returns a `IntervalHandle` that an be used to end the created
/// interval. The interval can also be ended by calling `end_interval` with the same label and log
/// level.
pub fn start_interval(metadata: Metadata) -> IntervalHandle {
    let interval_name = start_interval_label(&metadata);
    if profiling_level_is_active(metadata.profiling_level.clone()) {
        mark_with_metadata(interval_name.into(), metadata.clone().into());
    }
    IntervalHandle::new(metadata)
}

fn get_latest_performance_entry() -> Option<PerformanceEntry> {
    let entries: js_sys::Array = performance().get_entries_by_type("measure");

    if entries.length() < 1 {
        return None;
    }
    let measure = entries.get(entries.length() - 1);
    let measure: PerformanceEntry = measure.into();
    Some(measure)
}

/// End measuring an interval. Return the measurement taken between start and end of the interval,
/// if possible.
fn end_interval(metadata: Metadata) -> Result<Measurement, MeasurementError> {
    // metadata.event_type = MeasurementEvent::End;
    let profiling_level = metadata.profiling_level.clone();
    let start_label = start_interval_label(&metadata);
    let end_label = end_interval_label(&metadata);
    let measurement_label = measure_interval_label(&metadata);
    if !profiling_level_is_active(profiling_level) {
        Err(MeasurementError::ProfilingDisabled)
    } else {
        mark_with_metadata(end_label.clone().into(), metadata.clone().into());
        measure_with_start_mark_and_end_mark_and_metadata(
            measurement_label.into(),
            start_label.into(),
            end_label.into(),
            metadata.into(),
        );

        let measure = get_latest_performance_entry().ok_or(MeasurementError::NoMeasurementFound)?;

        let measurement: Measurement =
            measure.try_into().or(Err(MeasurementError::InvalidMeasurementConversion))?;
        Ok(measurement)
    }
}

/// Measure the execution time of the given interval. The interval is executed and the return value
/// and the measurement result are returned in the `IntervalMeasurementResult`.
pub fn measure_interval<T, F: FnMut() -> T>(
    metadata: Metadata,
    mut closure: F,
) -> IntervalMeasurementResult<T> {
    start_interval(metadata.clone()).release();
    let value = closure();
    let measurement = end_interval(metadata);

    IntervalMeasurementResult { value, measurement }
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



// ======================
// === IntervalHandle ===
// ======================

/// Handle that allows ending the interval.
#[derive(Clone, Debug)]
pub struct IntervalHandle {
    metadata: Metadata,
    released: bool,
}

impl IntervalHandle {
    fn new(metadata: Metadata) -> Self {
        IntervalHandle { metadata, released: false }
    }

    /// Measure the interval.
    pub fn end(mut self) {
        self.released = true;
        warn_on_error(end_interval(self.metadata.clone()));
    }

    /// Release the handle to prevent a warning to be emitted when it is garbage collected without
    /// a call to `end`. This can be useful if one wants to call `end_interval` manually, or the
    /// equivalent call to `end_interval` is in Rust code.
    pub fn release(mut self) {
        self.released = true;
        drop(self)
    }
}

impl Drop for IntervalHandle {
    fn drop(&mut self) {
        if !self.released {
            warn_on_error(end_interval(self.metadata.clone()));
            WARNING!(format!("{} was dropped without a call to `measure`.", self.metadata.label));
        }
    }
}



// =====================
// === AttachedStats ===
// =====================
// TODO(akavel): naming: Metrics? Stats? Correlates? Health? Impact?

type AttachedStats = HashMap<String, StatsAggregate>;

thread_local! {
    static ATTACHED_STATS: RefCell<AttachedStats> = RefCell::new(AttachedStats::new());
}

pub fn push_stats(stats: &Vec<f64>) {
    // FIXME(akavel): labeled data in `stats`
}

// FIXME(akavel): do we need Clone?
#[derive(Clone, Debug)]
struct StatsAggregate {
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

/// Return a report on all recorded measurement entries.
pub fn entries() -> Report {
    Report::generate()
}



// =============
// === Tests ===
// =============


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn macro_expansion() {
        // Checks that macros work correctly and create valid code.
        let task_handle = start_task!("sample_task");
        task_handle.release();
        end_task!("sample_task");

        // TODO measure example
    }
}