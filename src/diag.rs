//! The Engine's one owner of the standard error stream (t-128).
//!
//! Two kinds of bytes leave the Engine through stderr, and both are written
//! only here:
//!
//! * *Mandatory diagnostics* - the entry-point error renders and the
//!   `warning:` lines the operator is owed - rendered with today's exact
//!   bytes whether tracing is on or off.
//! * *Trace records* - one LF-terminated `rtm-trace: <fields>` line per
//!   traced decision, written only while the latch enabled tracing. A
//!   record is attempted once at its decision point; a failed or closed
//!   standard error changes no decision, and the write error is discarded.
//!
//! The toggle is `RATMAC_TRACE`. The binary latches it once, after the
//! current-directory read and before any command dispatch; every library
//! entry (`cli::run_from` and the path-taking modules) never reads it, so
//! an in-process caller emits nothing. The latched flag is transient
//! in-process state: no file, no environment mutation, no persisted Engine
//! state, no event log.
//!
//! The disabled event is free: [`event`] checks one atomic, never calls its
//! closure, never formats, and never allocates, so allocating field
//! expressions behind the closure cost nothing while tracing is off.

use std::ffi::OsString;
use std::fmt;
use std::io::{self, Write};
use std::sync::atomic::{AtomicBool, Ordering};

/// The toggle's name. The binary reads it exactly once; children the Engine
/// spawns never inherit it.
pub const TRACE_VARIABLE: &str = "RATMAC_TRACE";

/// The eleven bytes every record begins with: `rtm-trace:` plus one space.
const PREFIX: &[u8] = b"rtm-trace: ";

/// The latched trace flag, set once by the binary's latch and read by every
/// event. `Acquire`/`Release` pair so a value latched before any worker
/// thread exists is visible to every thread that reports later.
static ENABLED: AtomicBool = AtomicBool::new(false);

/// Read the toggle once and latch its value: `1` enables tracing; unset,
/// empty, and exactly `0` disable it; any other value - including non-UTF-8
/// bytes - is returned as the invalid value the binary refuses.
///
/// This is the one read of the environment variable the process performs.
/// The binary calls it exactly once, after the current-directory read and
/// before dispatch; a mid-invocation environment change is invisible, and
/// nothing later re-reads the toggle.
pub fn latch_from_environment() -> Result<(), OsString> {
    let value = read_toggle();
    let enabled = match value.as_deref().map(|value| value.as_encoded_bytes()) {
        None | Some(b"") | Some(b"0") => false,
        Some(b"1") => true,
        Some(_) => return Err(value.expect("the invalid value is present")),
    };
    ENABLED.store(enabled, Ordering::Release);
    Ok(())
}

/// The toggle's raw value, read through the platform's own environment
/// lookup so the absent-variable read allocates nothing: the standard
/// `var_os` copies even the lookup name, and the contract's absent budget
/// is zero. A present value may allocate its one environment copy.
#[cfg(windows)]
fn read_toggle() -> Option<OsString> {
    use std::os::windows::ffi::OsStringExt;

    #[link(name = "kernel32")]
    extern "system" {
        fn GetEnvironmentVariableW(name: *const u16, buffer: *mut u16, size: u32) -> u32;
    }
    // "RATMAC_TRACE\0": the toggle's name, NUL-terminated, on the stack.
    const NAME: [u16; 13] = [
        b'R' as u16,
        b'A' as u16,
        b'T' as u16,
        b'M' as u16,
        b'A' as u16,
        b'C' as u16,
        b'_' as u16,
        b'T' as u16,
        b'R' as u16,
        b'A' as u16,
        b'C' as u16,
        b'E' as u16,
        0,
    ];
    let mut buffer = [0u16; 16];
    // SAFETY: `NAME` is NUL-terminated and `buffer` is a live u16 array of
    // the size passed; the call reads the caller's environment only.
    let copied = unsafe { GetEnvironmentVariableW(NAME.as_ptr(), buffer.as_mut_ptr(), 16) };
    if copied == 0 {
        // Absent or empty: the toggle table latches both disabled.
        return None;
    }
    if copied < 16 {
        return Some(OsString::from_wide(&buffer[..copied as usize]));
    }
    // A present value longer than the stack buffer: copy it through the
    // standard reader; it cannot be a valid setting, so its cost rides the
    // refusal, never the disabled budgets.
    std::env::var_os(TRACE_VARIABLE)
}

/// The Unix spelling of the same allocation-free read.
#[cfg(not(windows))]
fn read_toggle() -> Option<OsString> {
    use std::ffi::{CStr, OsStr};
    use std::os::unix::ffi::OsStrExt;

    #[link(name = "c")]
    extern "C" {
        fn getenv(name: *const std::os::raw::c_char) -> *const std::os::raw::c_char;
    }
    // SAFETY: the name is a NUL-terminated static C string; `getenv`
    // returns a pointer into the process's own environment or null.
    unsafe {
        let value = getenv(b"RATMAC_TRACE\0".as_ptr().cast());
        if value.is_null() {
            return None;
        }
        Some(OsStr::from_bytes(CStr::from_ptr(value).to_bytes()).to_owned())
    }
}

/// Whether the latch enabled tracing.
fn enabled() -> bool {
    ENABLED.load(Ordering::Acquire)
}

/// One trace record under construction. Exposes field insertion only: a
/// field is one `key="value"` pair whose value renders through the one
/// escaping rule, appended in the caller's order after the record's code.
pub struct Record {
    line: Vec<u8>,
}

impl Record {
    /// Append one `key="value"` field.
    pub fn field(&mut self, key: &'static str, value: impl AsRef<[u8]>) -> &mut Self {
        self.line.push(b' ');
        self.line.extend_from_slice(key.as_bytes());
        self.line.push(b'=');
        self.line.push(b'"');
        escape_into(&mut self.line, value.as_ref());
        self.line.push(b'"');
        self
    }
}

/// Emit one trace record: `rtm-trace: code="..."` plus the fields `build`
/// inserts, one LF-terminated line, one write to standard error.
///
/// The same entry serves every internal site and the public surface. When
/// tracing is disabled it returns immediately: `build` is never called (so
/// allocating field expressions never evaluate) and the call itself
/// performs no formatting and no heap allocation. The write is attempted
/// once and its error, if any, is discarded.
pub fn event(code: &'static str, build: impl FnOnce(&mut Record)) {
    if !enabled() {
        return;
    }
    let mut record = Record {
        line: PREFIX.to_vec(),
    };
    record.line.extend_from_slice(b"code=\"");
    escape_into(&mut record.line, code.as_bytes());
    record.line.push(b'"');
    build(&mut record);
    record.line.push(b'\n');
    let _ = io::stderr().lock().write_all(&record.line);
}

/// Write one mandatory diagnostic line to standard error, byte-identical to
/// today's renders, and discard any write error. Every `warning:` line and
/// entry-point error the Engine owes the operator flows through here.
pub fn diagnostic(text: fmt::Arguments<'_>) {
    let mut line = String::new();
    let _ = fmt::write(&mut line, text);
    line.push('\n');
    let _ = io::stderr().lock().write_all(line.as_bytes());
}

/// The binary's invalid-toggle refusal: exactly one LF-terminated line
/// naming the value through the one escaping rule. Written before any work,
/// so the caller exits without running anything.
pub fn invalid_toggle(value: &OsString) {
    let mut line = b"rtm: RATMAC_TRACE=\"".to_vec();
    escape_into(&mut line, value.as_encoded_bytes());
    line.extend_from_slice(
        b"\" is not a valid trace setting; set it to 1 to enable the decision trace \
          or 0 to disable it; nothing was run\n",
    );
    let _ = io::stderr().lock().write_all(&line);
}

/// The one value escaping every quoted field value and the invalid-toggle
/// refusal use: inside double quotes a backslash renders `\\`, a double
/// quote `\"`, line feed `\n`, carriage return `\r`, tab `\t`; every other
/// byte below 0x20 and byte 0x7F renders `\u00XX` with two lowercase hex
/// digits; every other byte renders verbatim.
fn escape_into(out: &mut Vec<u8>, value: &[u8]) {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    for &byte in value {
        match byte {
            b'\\' => out.extend_from_slice(b"\\\\"),
            b'"' => out.extend_from_slice(b"\\\""),
            b'\n' => out.extend_from_slice(b"\\n"),
            b'\r' => out.extend_from_slice(b"\\r"),
            b'\t' => out.extend_from_slice(b"\\t"),
            0x00..=0x1f | 0x7f => {
                out.extend_from_slice(b"\\u00");
                out.push(HEX[usize::from(byte >> 4)]);
                out.push(HEX[usize::from(byte & 0x0f)]);
            }
            _ => out.push(byte),
        }
    }
}

/// WEB-006 (t-134): the Engine's nested-invocation children - the endpoint
/// process the guard launches and the driver the endpoint launches - run
/// with detached input and captured output. Only the owning module names
/// the standard error stream while wiring it (t-128's closed set).
#[allow(clippy::needless_pass_by_ref_mut)]
pub(crate) fn nested_child_stdio(command: &mut std::process::Command) {
    command
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());
}

#[cfg(test)]
mod tests {
    use super::{event, latch_from_environment, Record, PREFIX, TRACE_VARIABLE};
    use std::alloc::{GlobalAlloc, Layout, System};
    use std::cell::Cell;
    use std::sync::atomic::{AtomicUsize, Ordering};

    /// The counting global allocator. Its counter is thread-checked: only
    /// the thread that opens a measurement window is counted, so the crate's
    /// parallel unit tests cannot perturb any budget.
    static ALLOCATIONS: AtomicUsize = AtomicUsize::new(0);

    thread_local! {
        /// Whether this thread's allocations count right now.
        static COUNTING: Cell<bool> = const { Cell::new(false) };
    }

    struct Counting;

    impl Counting {
        fn count() {
            if COUNTING.try_with(|cell| cell.get()).unwrap_or(false) {
                ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
            }
        }
    }

    unsafe impl GlobalAlloc for Counting {
        unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
            Self::count();
            unsafe { System.alloc(layout) }
        }
        unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
            Self::count();
            unsafe { System.alloc_zeroed(layout) }
        }
        unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
            Self::count();
            unsafe { System.realloc(pointer, layout, new_size) }
        }
        unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
            unsafe { System.dealloc(pointer, layout) }
        }
    }

    #[global_allocator]
    static COUNTING_ALLOCATOR: Counting = Counting;

    /// Run `measured` with this thread's allocations counted, returning the
    /// count's delta across the call with the call's own result.
    fn budget<R>(measured: impl FnOnce() -> R) -> (usize, R) {
        let _ = COUNTING.try_with(|cell| cell.set(true));
        ALLOCATIONS.store(0, Ordering::Relaxed);
        let result = measured();
        let counted = ALLOCATIONS.load(Ordering::Relaxed);
        let _ = COUNTING.try_with(|cell| cell.set(false));
        (counted, result)
    }

    /// The three budgets the contract names, asserted separately: a disabled
    /// `event` whose closure allocates and sets a flag allocates nothing and
    /// never sets the flag; the latch with the variable absent allocates
    /// nothing; and the explicitly-present read, measured on its own, may
    /// allocate for its environment copy - that budget never excuses the
    /// disabled-event budget.
    #[test]
    fn the_disabled_channel_spends_nothing() {
        // Budget one: the disabled event. The latch is disabled by the
        // absent variable, and the closure both allocates and sets a flag.
        std::env::remove_var(TRACE_VARIABLE);
        latch_from_environment().expect("the absent toggle latches disabled");
        let (disabled_event, called) = budget(|| {
            let mut called = false;
            event("budget", |record: &mut Record| {
                called = true;
                let allocated = vec![b'x'; 512];
                record.field("payload", format!("{allocated:?}"));
            });
            called
        });
        assert!(!called, "the disabled event never calls its closure");
        assert_eq!(
            disabled_event, 0,
            "the disabled event never formats and never allocates"
        );

        // Budget two: the latch while the variable is absent.
        std::env::remove_var(TRACE_VARIABLE);
        let (absent_latch, latched) = budget(latch_from_environment);
        assert_eq!(latched, Ok(()), "the absent toggle latches disabled");
        assert_eq!(
            absent_latch, 0,
            "the latch with the variable absent allocates nothing"
        );

        // Budget three, measured on its own: the explicitly-present read may
        // allocate for its environment copy.
        std::env::set_var(TRACE_VARIABLE, "0");
        let (present_latch, latched) = budget(latch_from_environment);
        assert_eq!(latched, Ok(()), "the value 0 latches disabled");
        assert!(
            present_latch <= 1,
            "the present read copies at most its environment value: {present_latch}"
        );
        // That present read latched a disabled value, and the disabled-event
        // budget still holds beside it.
        let (still_disabled, called) = budget(|| {
            let mut called = false;
            event("budget", |record: &mut Record| {
                called = true;
                let allocated = String::from("allocated").repeat(64);
                record.field("payload", allocated);
            });
            called
        });
        assert!(!called, "the value 0 keeps the event disabled");
        assert_eq!(
            still_disabled, 0,
            "the present read's budget never excuses the disabled-event budget"
        );

        // Leave the process unlatched and the variable absent.
        std::env::remove_var(TRACE_VARIABLE);
        let _ = latch_from_environment();
    }

    /// The one escaping rule, over every byte class it names.
    #[test]
    fn every_field_value_escapes_through_the_one_rule() {
        let value: &[u8] = b"plain \\ \" \n \r \t \x01\x1f\x7f \x80\xff";
        let mut record = Record {
            line: [PREFIX, b"code=\"escape\""].concat(),
        };
        record.field("value", value);
        assert_eq!(
            record.line,
            b"rtm-trace: code=\"escape\" value=\"plain \\\\ \\\" \\n \\r \\t \\u0001\\u001f\\u007f \x80\xff\""
                .as_slice()
        );
    }
}
