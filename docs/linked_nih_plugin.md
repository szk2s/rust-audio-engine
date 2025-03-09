# 実装前に特に見ておいた方が良い nih_plug のソース。

[audio_setup.rs](../nih_plug_src/audio_setup.rs)

```rust
//! Types and definitions surrounding a plugin's audio IO setup.

use std::num::NonZeroU32;

use crate::prelude::Buffer;

/// A description of a plugin's audio IO configuration. The [`Plugin`][crate::prelude::Plugin]
/// defines a list of supported audio IO configs, with the first one acting as the default layout.
/// Depending on the plugin API, the host may pick a different configuration from the list and use
/// that instead. The final chosen configuration is passed as an argument to the
/// [`Plugin::initialize()`][crate::prelude::Plugin::initialize] function so the plugin can allocate
/// its data structures based on the number of audio channels it needs to process.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AudioIOLayout {
    /// The number of main input channels for the plugin, if it has a main input port. This can be
    /// set to `None` if the plugin does not have one.
    pub main_input_channels: Option<NonZeroU32>,
    /// The number of main output channels for the plugin, if it has a main output port. This can be
    /// set to `None` if the plugin does not have one.
    pub main_output_channels: Option<NonZeroU32>,
    /// The plugin's additional sidechain inputs, if it has any. Use the [`new_nonzero_u32()`]
    /// function to construct these values until const `Option::unwrap()` gets stabilized
    /// (<https://github.com/rust-lang/rust/issues/67441>).
    pub aux_input_ports: &'static [NonZeroU32],
    /// The plugin's additional outputs, if it has any. Use the [`new_nonzero_u32()`] function to
    /// construct these values until const `Option::unwrap()` gets stabilized
    /// (<https://github.com/rust-lang/rust/issues/67441>).
    pub aux_output_ports: &'static [NonZeroU32],

    /// Optional names for the audio ports. Defining these can be useful for plugins with multiple
    /// output and input ports.
    pub names: PortNames,
}

/// Construct a `NonZeroU32` value at compile time. Equivalent to `NonZeroU32::new(n).unwrap()`.
pub const fn new_nonzero_u32(n: u32) -> NonZeroU32 {
    match NonZeroU32::new(n) {
        Some(n) => n,
        None => panic!("'new_nonzero_u32()' called with a zero value"),
    }
}

/// Contains auxiliary (sidechain) input and output buffers for a process call.
pub struct AuxiliaryBuffers<'a> {
    /// Buffers for all auxiliary (sidechain) inputs defined for this plugin. The data in these
    /// buffers can safely be overwritten. Auxiliary inputs can be defined using the
    /// [`AudioIOLayout::aux_input_ports`] field.
    pub inputs: &'a mut [Buffer<'a>],
    /// Buffers for all auxiliary outputs defined for this plugin. Auxiliary outputs can be defined using the
    /// [`AudioIOLayout::aux_output_ports`] field.
    pub outputs: &'a mut [Buffer<'a>],
}

/// Contains names for the ports defined in an `AudioIOLayout`. Setting these is optional, but it
/// makes working with multi-output plugins much more convenient.
///
/// All of these names should start with a capital letter to be consistent with automatically
/// generated names.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PortNames {
    /// The name for the audio IO layout as a whole. Useful when a plugin has multiple distinct
    /// layouts. Will be generated if not set.
    pub layout: Option<&'static str>,

    /// The name for the main input port. Will be generated if not set.
    pub main_input: Option<&'static str>,
    /// The name for the main output port. Will be generated if not set.
    pub main_output: Option<&'static str>,
    /// Names for auxiliary (sidechain) input ports. Will be generated if not set or if this slice
    /// does not contain enough names.
    pub aux_inputs: &'static [&'static str],
    /// Names for auxiliary output ports. Will be generated if not set or if this slice does not
    /// contain enough names.
    pub aux_outputs: &'static [&'static str],
}

/// Configuration for (the host's) audio buffers.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BufferConfig {
    /// The current sample rate.
    pub sample_rate: f32,
    /// The minimum buffer size the host will use. This may not be set.
    pub min_buffer_size: Option<u32>,
    /// The maximum buffer size the host will use. The plugin should be able to accept variable
    /// sized buffers up to this size, or between the minimum and the maximum buffer size if both
    /// are set.
    pub max_buffer_size: u32,
    /// The current processing mode. The host will reinitialize the plugin any time this changes.
    pub process_mode: ProcessMode,
}

/// The plugin's current processing mode. Exposed through [`BufferConfig::process_mode`]. The host
/// will reinitialize the plugin whenever this changes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessMode {
    /// The plugin is processing audio in real time at a fixed rate.
    Realtime,
    /// The plugin is processing audio at a real time-like pace, but at irregular intervals. The
    /// host may do this to process audio ahead of time to loosen realtime constraints and to reduce
    /// the chance of xruns happening. This is only used by VST3.
    Buffered,
    /// The plugin is rendering audio offline, potentially faster than realtime ('freewheeling').
    /// The host will continuously call the process function back to back until all audio has been
    /// processed.
    Offline,
}

impl AudioIOLayout {
    /// [`AudioIOLayout::default()`], but as a const function. Used when initializing
    /// `Plugin::AUDIO_IO_LAYOUTS`. (<https://github.com/rust-lang/rust/issues/67792>)
    pub const fn const_default() -> Self {
        Self {
            main_input_channels: None,
            main_output_channels: None,
            aux_input_ports: &[],
            aux_output_ports: &[],
            names: PortNames::const_default(),
        }
    }

    /// A descriptive name for the layout. This is taken from `PortNames::layout` if set. Otherwise
    /// it is generated based on the layout.
    pub fn name(&self) -> String {
        if let Some(name) = self.names.layout {
            return name.to_owned();
        }

        // If the name is not set then we'll try to come up with something descriptive
        match (
            self.main_input_channels
                .map(NonZeroU32::get)
                .unwrap_or_default(),
            self.main_output_channels
                .map(NonZeroU32::get)
                .unwrap_or_default(),
            self.aux_input_ports.len(),
            self.aux_output_ports.len(),
        ) {
            (0, 0, 0, 0) => String::from("Empty"),
            (_, 1, 0, _) | (1, 0, _, _) => String::from("Mono"),
            (_, 2, 0, _) | (2, 0, _, _) => String::from("Stereo"),
            (_, 1, _, _) => String::from("Mono with sidechain"),
            (_, 2, _, _) => String::from("Stereo with sidechain"),
            // These probably, hopefully won't occur
            (i, o, 0, 0) => format!("{i} inputs, {o} outputs"),
            (i, o, _, 0) => format!("{i} inputs, {o} outputs, with sidechain"),
            // And these don't make much sense, suggestions for something better are welcome
            (i, o, 0, aux_o) => format!("{i} inputs, {o}*{} outputs", aux_o + 1),
            (i, o, aux_i, aux_o) => format!("{i}*{} inputs, {o}*{} outputs", aux_i + 1, aux_o + 1),
        }
    }

    /// The name for the main input port. Either generated or taken from the `names` field.
    pub fn main_input_name(&self) -> String {
        self.names.main_input.unwrap_or("Input").to_owned()
    }

    /// The name for the main output port. Either generated or taken from the `names` field.
    pub fn main_output_name(&self) -> String {
        self.names.main_input.unwrap_or("Output").to_owned()
    }

    /// The name for the auxiliary input port with the given index. Either generated or taken from
    /// the `names` field.
    pub fn aux_input_name(&self, idx: usize) -> Option<String> {
        if idx >= self.aux_input_ports.len() {
            None
        } else {
            match self.names.aux_inputs.get(idx) {
                Some(name) => Some(String::from(*name)),
                None if self.aux_input_ports.len() == 1 => Some(String::from("Sidechain Input")),
                None => Some(format!("Sidechain Input {}", idx + 1)),
            }
        }
    }

    /// The name for the auxiliary output port with the given index. Either generated or taken from
    /// the `names` field.
    pub fn aux_output_name(&self, idx: usize) -> Option<String> {
        if idx >= self.aux_output_ports.len() {
            None
        } else {
            match self.names.aux_outputs.get(idx) {
                Some(name) => Some(String::from(*name)),
                None if self.aux_output_ports.len() == 1 => Some(String::from("Auxiliary Output")),
                None => Some(format!("Auxiliary Output {}", idx + 1)),
            }
        }
    }
}

impl PortNames {
    /// [`PortNames::default()`], but as a const function. Used when initializing
    /// `Plugin::AUDIO_IO_LAYOUTS`. (<https://github.com/rust-lang/rust/issues/67792>)
    pub const fn const_default() -> Self {
        Self {
            layout: None,
            main_input: None,
            main_output: None,
            aux_inputs: &[],
            aux_outputs: &[],
        }
    }
}

```

[buffer.rs](../nih_plug_src/buffer.rs)

```rust
//! Adapters and utilities for working with audio buffers.

use std::marker::PhantomData;

mod blocks;
mod samples;

pub use blocks::{Block, BlockChannelsIter, BlocksIter};
pub use samples::{ChannelSamples, ChannelSamplesIter, SamplesIter};

/// The audio buffers used during processing. This contains the output audio output buffers with the
/// inputs already copied to the outputs. You can either use the iterator adapters to conveniently
/// and efficiently iterate over the samples, or you can do your own thing using the raw audio
/// buffers.
///
/// TODO: This lifetime makes zero sense because you're going to need unsafe lifetime casts to use
///       this either way. Maybe just get rid of it in favor for raw pointers.
#[derive(Default)]
pub struct Buffer<'a> {
    /// The number of samples contained within `output_slices`. This needs to be stored separately
    /// to be able to handle 0 channel IO for MIDI-only plugins.
    num_samples: usize,

    /// Contains slices for the plugin's outputs. You can't directly create a nested slice from a
    /// pointer to pointers, so this needs to be preallocated in the setup call and kept around
    /// between process calls. And because storing a reference to this means a) that you need a lot
    /// of lifetime annotations everywhere and b) that at some point you need unsound lifetime casts
    /// because this `Buffers` either cannot have the same lifetime as the separately stored output
    /// buffers, and it also cannot be stored in a field next to it because that would mean
    /// containing mutable references to data stored in a mutex.
    output_slices: Vec<&'a mut [f32]>,
}

impl<'a> Buffer<'a> {
    /// Returns the number of samples per channel in this buffer.
    #[inline]
    pub fn samples(&self) -> usize {
        self.num_samples
    }

    /// Returns the number of channels in this buffer.
    #[inline]
    pub fn channels(&self) -> usize {
        self.output_slices.len()
    }

    /// Returns true if this buffer does not contain any samples.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.num_samples == 0
    }

    /// Obtain the raw audio buffers.
    #[inline]
    pub fn as_slice(&mut self) -> &mut [&'a mut [f32]] {
        &mut self.output_slices
    }

    /// The same as [`as_slice()`][Self::as_slice()], but for a non-mutable reference. This is
    /// usually not needed.
    #[inline]
    pub fn as_slice_immutable(&self) -> &[&'a mut [f32]] {
        &self.output_slices
    }

    /// Iterate over the samples, returning a channel iterator for each sample.
    #[inline]
    pub fn iter_samples<'slice>(&'slice mut self) -> SamplesIter<'slice, 'a> {
        SamplesIter {
            buffers: self.output_slices.as_mut_slice(),
            current_sample: 0,
            samples_end: self.samples(),
            _marker: PhantomData,
        }
    }

    /// Iterate over the buffer in blocks with the specified maximum size. The ideal maximum block
    /// size depends on the plugin in question, but 64 or 128 samples works for most plugins. Since
    /// the buffer's total size may not be cleanly divisible by the maximum size, the returned
    /// buffers may have any size in `[1, max_block_size]`. This is useful when using algorithms
    /// that work on entire blocks of audio, like those that would otherwise need to perform
    /// expensive per-sample branching or that can use per-sample SIMD as opposed to per-channel
    /// SIMD.
    ///
    /// The parameter smoothers can also produce smoothed values for an entire block using
    /// [`Smoother::next_block()`][crate::prelude::Smoother::next_block()].
    ///
    /// You can use this to obtain block-slices from a buffer so you can pass them to a library:
    ///
    /// ```ignore
    /// for block in buffer.iter_blocks(128) {
    ///     let mut block_channels = block.into_iter();
    ///     let stereo_slice = &[
    ///         block_channels.next().unwrap(),
    ///         block_channels.next().unwrap(),
    ///     ];
    ///
    ///     // Do something cool with `stereo_slice`
    /// }
    /// ````
    #[inline]
    pub fn iter_blocks<'slice>(&'slice mut self, max_block_size: usize) -> BlocksIter<'slice, 'a> {
        BlocksIter {
            buffers: self.output_slices.as_mut_slice(),
            max_block_size,
            current_block_start: 0,
            _marker: PhantomData,
        }
    }

    /// Set the slices in the raw output slice vector. This vector needs to be resized to match the
    /// number of output channels during the plugin's initialization. Then during audio processing,
    /// these slices should be updated to point to the plugin's audio buffers. The `num_samples`
    /// argument should match the length of the inner slices.
    ///
    /// # Safety
    ///
    /// The stored slices must point to live data when this object is passed to the plugins' process
    /// function. The rest of this object also assumes all channel lengths are equal. Panics will
    /// likely occur if this is not the case.
    pub unsafe fn set_slices(
        &mut self,
        num_samples: usize,
        update: impl FnOnce(&mut Vec<&'a mut [f32]>),
    ) {
        self.num_samples = num_samples;
        update(&mut self.output_slices);

        #[cfg(debug_assertions)]
        for slice in &self.output_slices {
            nih_debug_assert_eq!(slice.len(), num_samples);
        }
    }
}

#[cfg(any(miri, test))]
mod miri {
    use super::*;

    #[test]
    fn repeated_access() {
        let mut real_buffers = vec![vec![0.0; 512]; 2];
        let mut buffer = Buffer::default();
        unsafe {
            buffer.set_slices(512, |output_slices| {
                let (first_channel, other_channels) = real_buffers.split_at_mut(1);
                *output_slices = vec![&mut first_channel[0], &mut other_channels[0]];
            })
        };

        for samples in buffer.iter_samples() {
            for sample in samples {
                *sample += 0.001;
            }
        }

        for mut samples in buffer.iter_samples() {
            for _ in 0..2 {
                for sample in samples.iter_mut() {
                    *sample += 0.001;
                }
            }
        }

        assert_eq!(real_buffers[0][0], 0.003);
    }

    #[test]
    fn repeated_slices() {
        let mut real_buffers = vec![vec![0.0; 512]; 2];
        let mut buffer = Buffer::default();
        unsafe {
            buffer.set_slices(512, |output_slices| {
                let (first_channel, other_channels) = real_buffers.split_at_mut(1);
                *output_slices = vec![&mut first_channel[0], &mut other_channels[0]];
            })
        };

        // These iterators should not alias
        let mut blocks = buffer.iter_blocks(16);
        let (_block1_offset, block1) = blocks.next().unwrap();
        let (_block2_offset, block2) = blocks.next().unwrap();
        for channel in block1 {
            for sample in channel.iter_mut() {
                *sample += 0.001;
            }
        }
        for channel in block2 {
            for sample in channel.iter_mut() {
                *sample += 0.001;
            }
        }

        for i in 0..32 {
            assert_eq!(real_buffers[0][i], 0.001);
        }
        for i in 32..48 {
            assert_eq!(real_buffers[0][i], 0.0);
        }
    }
}

```

[context.rs](../nih_plug_src/context.rs)

```rust
//! Different contexts the plugin can use to make callbacks to the host in different...contexts.

use std::fmt::Display;

pub mod gui;
pub mod init;
pub mod process;

// Contexts for more plugin-API specific features
pub mod remote_controls;

/// The currently active plugin API. This may be useful to display in an about screen in the
/// plugin's GUI for debugging purposes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PluginApi {
    Clap,
    Standalone,
    Vst3,
}

impl Display for PluginApi {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PluginApi::Clap => write!(f, "CLAP"),
            PluginApi::Standalone => write!(f, "standalone"),
            PluginApi::Vst3 => write!(f, "VST3"),
        }
    }
}

```

[debug.rs](../nih_plug_src/debug.rs)

```rust
//! Macros for logging and debug assertions. [`nih_dbg!()`], [`nih_trace!()`], and the
//! `nih_debug_assert_*!()` macros are compiled out during release builds, so they can be used for
//! asserting adiditonal invariants in debug builds. Check the [`nih_log!()`] macro for more
//! information on NIH-plug's logger. None of the logging functions are realtime-safe, and you
//! should avoid using them during release builds in any of the functions that may be called from an
//! audio thread.

// NOTE: Exporting macros in Rust is a bit weird. `#[macro_export]` causes them to be exported to
//       the crate root, but that makes it difficult to include just the macros without using
//       `#[macro_use] extern crate nih_plug;`. That's why the macros are also re-exported from this
//       module.

/// Write something to the logger. This defaults to STDERR unless the user is running Windows and a
/// debugger has been attached, in which case `OutputDebugString()` will be used instead.
///
/// The logger's behavior can be controlled by setting the `NIH_LOG` environment variable to:
///
/// - `stderr`, in which case the log output always gets written to STDERR.
/// - `windbg` (only on Windows), in which case the output always gets logged using
///   `OutputDebugString()`.
/// - A file path, in which case the output gets appended to the end of that file which will be
///   created if necessary.
#[macro_export]
macro_rules! nih_log {
    ($($args:tt)*) => (
        $crate::log::info!($($args)*)
    );
}
#[doc(inline)]
pub use nih_log;

/// Similar to `nih_log!()`, but less subtle. Used for printing warnings.
#[macro_export]
macro_rules! nih_warn {
    ($($args:tt)*) => (
        $crate::log::warn!($($args)*)
    );
}
#[doc(inline)]
pub use nih_warn;

/// Similar to `nih_log!()`, but more scream-y. Used for printing fatal errors.
#[macro_export]
macro_rules! nih_error {
    ($($args:tt)*) => (
        $crate::log::error!($($args)*)
    );
}
#[doc(inline)]
pub use nih_error;

/// The same as `nih_log!()`, but with source and thread information. Like the
/// `nih_debug_assert*!()` macros, this is only shown when compiling in debug mode.
#[macro_export]
macro_rules! nih_trace {
    ($($args:tt)*) => (
        $crate::util::permit_alloc(|| $crate::log::trace!($($args)*))
    );
}
#[doc(inline)]
pub use nih_trace;

/// Analogues to the `dbg!()` macro, but respecting the `NIH_LOG` environment variable and with all
/// of the same logging features as the other `nih_*!()` macros. Like the `nih_debug_assert*!()`
/// macros, this is only shown when compiling in debug mode, but the macro will still return the
/// value in non-debug modes.
#[macro_export]
macro_rules! nih_dbg {
    () => {
        $crate::util::permit_alloc(|| $crate::log::debug!(""));
    };
    ($val:expr $(,)?) => {
        // Match here acts as a let-binding: https://stackoverflow.com/questions/48732263/why-is-rusts-assert-eq-implemented-using-a-match/48732525#48732525
        match $val {
            tmp => {
                $crate::util::permit_alloc(|| $crate::log::debug!("{} = {:#?}", stringify!($val), &tmp));
                tmp
            }
        }
    };
    ($($val:expr),+ $(,)?) => { ($($crate::nih_dbg!($val)),+,) };
}
#[doc(inline)]
pub use nih_dbg;

/// A `debug_assert!()` analogue that prints the error with line number information instead of
/// panicking. During tests this is upgraded to a regular panicking `debug_assert!()`.
///
/// TODO: Detect if we're running under a debugger, and trigger a break if we are
#[macro_export]
macro_rules! nih_debug_assert {
    ($cond:expr $(,)?) => (
        #[allow(clippy::neg_cmp_op_on_partial_ord)]
        if cfg!(test) {
           debug_assert!($cond);
        } else if cfg!(debug_assertions) && !$cond {
            $crate::util::permit_alloc(|| $crate::log::warn!(concat!("Debug assertion failed: ", stringify!($cond))));
        }
    );
    ($cond:expr, $format:expr $(, $($args:tt)*)?) => (
        #[allow(clippy::neg_cmp_op_on_partial_ord)]
        if cfg!(test) {
           debug_assert!($cond, $format, $($($args)*)?);
        } else if cfg!(debug_assertions) && !$cond {
            $crate::util::permit_alloc(|| $crate::log::warn!(concat!("Debug assertion failed: ", stringify!($cond), ", ", $format), $($($args)*)?));
        }
    );
}
#[doc(inline)]
pub use nih_debug_assert;

/// An unconditional debug assertion failure, for if the condition has already been checked
/// elsewhere. See [`nih_debug_assert!()`] for more information.
#[macro_export]
macro_rules! nih_debug_assert_failure {
    () => (
        if cfg!(test) {
           debug_assert!(false, "Debug assertion failed");
        } else if cfg!(debug_assertions) {
            $crate::util::permit_alloc(|| $crate::log::warn!("Debug assertion failed"));
        }
    );
    ($format:expr $(, $($args:tt)*)?) => (
        if cfg!(test) {
           debug_assert!(false, concat!("Debug assertion failed: ", $format), $($($args)*)?);
        } else if cfg!(debug_assertions) {
            $crate::util::permit_alloc(|| $crate::log::warn!(concat!("Debug assertion failed: ", $format), $($($args)*)?));
        }
    );
}
#[doc(inline)]
pub use nih_debug_assert_failure;

/// A `debug_assert_eq!()` analogue that prints the error with line number information instead of
/// panicking. See [`nih_debug_assert!()`] for more information.
#[macro_export]
macro_rules! nih_debug_assert_eq {
    ($left:expr, $right:expr $(,)?) => (
        #[allow(clippy::neg_cmp_op_on_partial_ord)]
        if cfg!(test) {
           debug_assert_eq!($left, $right);
        } else if cfg!(debug_assertions) && $left != $right {
            $crate::util::permit_alloc(|| $crate::log::warn!(concat!("Debug assertion failed: ", stringify!($left), " != ", stringify!($right))));
        }
    );
    ($left:expr, $right:expr, $format:expr $(, $($args:tt)*)?) => (
        #[allow(clippy::neg_cmp_op_on_partial_ord)]
        if cfg!(test) {
           debug_assert_eq!($left, $right, $format, $($($args)*)?);
        } else if cfg!(debug_assertions) && $left != $right {
            $crate::util::permit_alloc(|| $crate::log::warn!(concat!("Debug assertion failed: ", stringify!($left), " != ", stringify!($right), ", ", $format), $($($args)*)?));
        }
    );
}
#[doc(inline)]
pub use nih_debug_assert_eq;

/// A `debug_assert_ne!()` analogue that prints the error with line number information instead of
/// panicking. See [`nih_debug_assert!()`] for more information.
#[macro_export]
macro_rules! nih_debug_assert_ne {
    ($left:expr, $right:expr $(,)?) => (
        #[allow(clippy::neg_cmp_op_on_partial_ord)]
        if cfg!(test) {
           debug_assert_ne!($left, $right);
        } else if cfg!(debug_assertions) && $left == $right {
            $crate::util::permit_alloc(|| $crate::log::warn!(concat!("Debug assertion failed: ", stringify!($left), " == ", stringify!($right))));
        }
    );
    ($left:expr, $right:expr, $format:expr $(, $($args:tt)*)?) => (
        #[allow(clippy::neg_cmp_op_on_partial_ord)]
        if cfg!(test) {
           debug_assert_ne!($left, $right, $format, $($($args)*)?);
        } else if cfg!(debug_assertions) && $left == $right  {
            $crate::util::permit_alloc(|| $crate::log::warn!(concat!("Debug assertion failed: ", stringify!($left), " == ", stringify!($right), ", ", $format), $($($args)*)?));
        }
    );
}
#[doc(inline)]
pub use nih_debug_assert_ne;

```

[event_loop.rs](../nih_plug_src/event_loop.rs)

```rust
//! An internal event loop for spooling tasks to the/a GUI thread.

use std::sync::Weak;

mod background_thread;

#[cfg(all(target_family = "unix", not(target_os = "macos")))]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "windows")]
mod windows;

pub(crate) use self::background_thread::BackgroundThread;

#[cfg_attr(not(feature = "vst3"), allow(unused_imports))]
#[cfg(all(target_family = "unix", not(target_os = "macos")))]
pub(crate) use self::linux::LinuxEventLoop as OsEventLoop;
#[cfg_attr(not(feature = "vst3"), allow(unused_imports))]
#[cfg(target_os = "macos")]
pub(crate) use self::macos::MacOSEventLoop as OsEventLoop;
#[cfg_attr(not(feature = "vst3"), allow(unused_imports))]
#[cfg(target_os = "windows")]
pub(crate) use self::windows::WindowsEventLoop as OsEventLoop;

// This needs to be pretty high to make sure parameter change events don't get dropped when there's
// lots of automation/modulation going on
pub(crate) const TASK_QUEUE_CAPACITY: usize = 4096;

/// A trait describing the functionality of a platform-specific event loop that can execute tasks of
/// type `T` in executor `E` on the operating system's main thread (if applicable). Posting a task
/// to the internal task queue should be realtime-safe. This event loop should be created during the
/// wrapper's initial initialization on the main thread.
///
/// Additionally, this trait also allows posting tasks to a background thread that's completely
/// detached from the GUI. This makes it possible for a plugin to execute long running jobs without
/// blocking GUI rendering.
///
/// This is never used generically, but having this as a trait will cause any missing functions on
/// an implementation to show up as compiler errors even when using a different platform. And since
/// the tasks and executor will be sent to a thread, they need to have static lifetimes.
pub(crate) trait EventLoop<T, E>
where
    T: Send + 'static,
    E: MainThreadExecutor<T> + 'static,
{
    /// Create and start a new event loop. The thread this is called on will be designated as the
    /// main thread, so this should be called when constructing the wrapper.
    fn new_and_spawn(executor: Weak<E>) -> Self;

    /// Either post the function to the task queue so it can be delegated to the main thread, or
    /// execute the task directly if this is the main thread. This function needs to be callable at
    /// any time without blocking.
    ///
    /// If the task queue is full, then this will return false.
    #[must_use]
    fn schedule_gui(&self, task: T) -> bool;

    /// Post a task to the background task queue so it can be run in a dedicated background thread
    /// without blocking the plugin's GUI. This function needs to be callable at any time without
    /// blocking.
    ///
    /// If the task queue is full, then this will return false.
    #[must_use]
    fn schedule_background(&self, task: T) -> bool;

    /// Whether the calling thread is the event loop's main thread. This is usually the thread the
    /// event loop instance was initialized on.
    fn is_main_thread(&self) -> bool;
}

/// Something that can execute tasks of type `T`.
pub(crate) trait MainThreadExecutor<T>: Send + Sync {
    /// Execute a task on the current thread. This is either called from the GUI thread or from
    /// another background thread, depending on how the task was scheduled in the [`EventContext`].
    fn execute(&self, task: T, is_gui_thread: bool);
}

```

[midi.rs](../nih_plug_src/midi.rs)

```rust
//! Constants and definitions surrounding MIDI support.

use midi_consts::channel_event as midi;

use self::sysex::SysExMessage;
use crate::prelude::Plugin;

pub mod sysex;

pub use midi_consts::channel_event::control_change;

/// A plugin-specific note event type.
///
/// The reason why this is defined like this instead of parameterizing `NoteEvent` with `P` is
/// because deriving trait bounds requires all of the plugin's generic parameters to implement those
/// traits. And we can't require `P` to implement things like `Clone`.
///
/// <https://github.com/rust-lang/rust/issues/26925>
pub type PluginNoteEvent<P> = NoteEvent<<P as Plugin>::SysExMessage>;

/// Determines which note events a plugin can send and receive.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum MidiConfig {
    /// The plugin will not have a note input or output port and will thus not receive any not
    /// events.
    None,
    /// The plugin receives note on/off/choke events, pressure, and potentially a couple
    /// standardized expression types depending on the plugin standard and host. If the plugin sets
    /// up configuration for polyphonic modulation (see [`ClapPlugin`][crate::prelude::ClapPlugin])
    /// and assigns polyphonic modulation IDs to some of its parameters, then it will also receive
    /// polyphonic modulation events. This level is also needed to be able to send SysEx events.
    Basic,
    /// The plugin receives full MIDI CCs as well as pitch bend information. For VST3 plugins this
    /// involves adding 130*16 parameters to bind to the the 128 MIDI CCs, pitch bend, and channel
    /// pressure.
    MidiCCs,
}

// FIXME: Like the voice ID, channel and note number can also be omitted in CLAP. And instead of an
//        Option, maybe this should use a dedicated type to more clearly indicate that missing
//        values should be treated as wildcards.

/// Event for (incoming) notes. The set of supported note events depends on the value of
/// [`Plugin::MIDI_INPUT`][crate::prelude::Plugin::MIDI_INPUT]. Also check out the
/// [`util`][crate::util] module for convenient conversion functions.
///
/// `S` is a MIDI SysEx message type that needs to implement [`SysExMessage`] to allow converting
/// this `NoteEvent` to and from raw MIDI data. `()` is provided as a default implementing for
/// plugins that don't use SysEx.
///
/// All of the timings are sample offsets within the current buffer. Out of bound timings are
/// clamped to the current buffer's length. All sample, channel and note numbers are zero-indexed.
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub enum NoteEvent<S> {
    /// A note on event, available on [`MidiConfig::Basic`] and up.
    NoteOn {
        timing: u32,
        /// A unique identifier for this note, if available. Using this to refer to a note is
        /// required when allowing overlapping voices for CLAP plugins.
        voice_id: Option<i32>,
        /// The note's channel, in `0..16`.
        channel: u8,
        /// The note's MIDI key number, in `0..128`.
        note: u8,
        /// The note's velocity, in `[0, 1]`. Some plugin APIs may allow higher precision than the
        /// 128 levels available in MIDI.
        velocity: f32,
    },
    /// A note off event, available on [`MidiConfig::Basic`] and up. Bitwig Studio does not provide
    /// a voice ID for this event.
    NoteOff {
        timing: u32,
        /// A unique identifier for this note, if available. Using this to refer to a note is
        /// required when allowing overlapping voices for CLAP plugins.
        voice_id: Option<i32>,
        /// The note's channel, in `0..16`.
        channel: u8,
        /// The note's MIDI key number, in `0..128`.
        note: u8,
        /// The note's velocity, in `[0, 1]`. Some plugin APIs may allow higher precision than the
        /// 128 levels available in MIDI.
        velocity: f32,
    },
    /// A note choke event, available on [`MidiConfig::Basic`] and up. When the host sends this to
    /// the plugin, it indicates that a voice or all sound associated with a note should immediately
    /// stop playing.
    Choke {
        timing: u32,
        /// A unique identifier for this note, if available. Using this to refer to a note is
        /// required when allowing overlapping voices for CLAP plugins.
        voice_id: Option<i32>,
        /// The note's channel, in `0..16`.
        channel: u8,
        /// The note's MIDI key number, in `0..128`.
        note: u8,
    },

    /// Sent by the plugin to the host to indicate that a voice has ended. This **needs** to be sent
    /// when a voice terminates when using polyphonic modulation. Otherwise you can ignore this
    /// event.
    VoiceTerminated {
        timing: u32,
        /// The voice's unique identifier. Setting this allows a single voice to be terminated if
        /// the plugin allows multiple overlapping voices for a single key.
        voice_id: Option<i32>,
        /// The note's channel, in `0..16`.
        channel: u8,
        /// The note's MIDI key number, in `0..128`.
        note: u8,
    },
    /// A polyphonic modulation event, available on [`MidiConfig::Basic`] and up. This will only be
    /// sent for parameters that were decorated with the `.with_poly_modulation_id()` modifier, and
    /// only by supported hosts. This event contains a _normalized offset value_ for the parameter's
    /// current, **unmodulated** value. That is, an offset for the current value before monophonic
    /// modulation is applied, as polyphonic modulation overrides monophonic modulation. There are
    /// multiple ways to incorporate this polyphonic modulation into a synthesizer, but a simple way
    /// to incorporate this would work as follows:
    ///
    /// - By default, a voice uses the parameter's global value, which may or may not include
    ///   monophonic modulation. This is `parameter.value` for unsmoothed parameters, and smoothed
    ///   parameters should use block smoothing so the smoothed values can be reused by multiple
    ///   voices.
    /// - If a `PolyModulation` event is emitted for the voice, that voice should use the the
    ///   _normalized offset_ contained within the event to compute the voice's modulated value and
    ///   use that in place of the global value.
    ///   - This value can be obtained by calling `param.preview_plain(param.normalized_value() +
    ///     event.normalized_offset)`. These functions automatically clamp the values as necessary.
    ///   - If the parameter uses smoothing, then the parameter's smoother can be copied to the
    ///     voice. [`Smoother::set_target()`][crate::prelude::Smoother::set_target()] can then be
    ///     used to have the smoother use the modulated value.
    ///   - One caveat with smoothing is that copying the smoother like this only works correctly if it last
    ///     produced a value during the sample before the `PolyModulation` event. Otherwise there
    ///     may still be an audible jump in parameter values. A solution for this would be to first
    ///     call the [`Smoother::reset()`][crate::prelude::Smoother::reset()] with the current
    ///     sample's global value before calling `set_target()`.
    ///   - Finally, if the polyphonic modulation happens on the same sample as the `NoteOn` event,
    ///     then the smoothing should not start at the current global value. In this case, `reset()`
    ///     should be called with the voice's modulated value.
    /// - If a `MonoAutomation` event is emitted for a parameter, then the values or target values
    ///   (if the parameter uses smoothing) for all voices must be updated. The normalized value
    ///   from the `MonoAutomation` and the voice's normalized modulation offset must be added and
    ///   converted back to a plain value. This value can be used directly for unsmoothed
    ///   parameters, or passed to `set_target()` for smoothed parameters. The global value will
    ///   have already been updated, so this event only serves as a notification to update
    ///   polyphonic modulation.
    /// - When a voice ends, either because the amplitude envelope has hit zero or because the voice
    ///   was stolen, the plugin must send a `VoiceTerminated` to the host to let it know that it
    ///   can reuse the resources it used to modulate the value.
    PolyModulation {
        timing: u32,
        /// The identifier of the voice this polyphonic modulation event should affect. This voice
        /// should use the values from this and subsequent polyphonic modulation events instead of
        /// the global value.
        voice_id: i32,
        /// The ID that was set for the modulated parameter using the `.with_poly_modulation_id()`
        /// method.
        poly_modulation_id: u32,
        /// The normalized offset value. See the event's docstring for more information.
        normalized_offset: f32,
    },
    /// A notification to inform the plugin that a polyphonically modulated parameter has received a
    /// new automation value. This is used in conjunction with the `PolyModulation` event. See that
    /// event's documentation for more details. The parameter's global value has already been
    /// updated when this event is emitted.
    MonoAutomation {
        timing: u32,
        /// The ID that was set for the modulated parameter using the `.with_poly_modulation_id()`
        /// method.
        poly_modulation_id: u32,
        /// The parameter's new normalized value. This needs to be added to a voice's normalized
        /// offset to get that voice's modulated normalized value. See the `PolyModulation` event's
        /// docstring for more information.
        normalized_value: f32,
    },

    /// A polyphonic note pressure/aftertouch event, available on [`MidiConfig::Basic`] and up. Not
    /// all hosts may support polyphonic aftertouch.
    ///
    /// # Note
    ///
    /// When implementing MPE support you should use MIDI channel pressure instead as polyphonic key
    /// pressure + MPE is undefined as per the MPE specification. Or as a more generic catch all,
    /// you may manually combine the polyphonic key pressure and MPE channel pressure.
    PolyPressure {
        timing: u32,
        /// A unique identifier for this note, if available. Using this to refer to a note is
        /// required when allowing overlapping voices for CLAP plugins.
        voice_id: Option<i32>,
        /// The note's channel, in `0..16`.
        channel: u8,
        /// The note's MIDI key number, in `0..128`.
        note: u8,
        /// The note's pressure, in `[0, 1]`.
        pressure: f32,
    },
    /// A volume expression event, available on [`MidiConfig::Basic`] and up. Not all hosts may
    /// support these expressions.
    PolyVolume {
        timing: u32,
        /// A unique identifier for this note, if available. Using this to refer to a note is
        /// required when allowing overlapping voices for CLAP plugins.
        voice_id: Option<i32>,
        /// The note's channel, in `0..16`.
        channel: u8,
        /// The note's MIDI key number, in `0..128`.
        note: u8,
        /// The note's voltage gain ratio, where 1.0 is unity gain.
        gain: f32,
    },
    /// A panning expression event, available on [`MidiConfig::Basic`] and up. Not all hosts may
    /// support these expressions.
    PolyPan {
        timing: u32,
        /// A unique identifier for this note, if available. Using this to refer to a note is
        /// required when allowing overlapping voices for CLAP plugins.
        voice_id: Option<i32>,
        /// The note's channel, in `0..16`.
        channel: u8,
        /// The note's MIDI key number, in `0..128`.
        note: u8,
        /// The note's panning from, in `[-1, 1]`, with -1 being panned hard left, and 1
        /// being panned hard right.
        pan: f32,
    },
    /// A tuning expression event, available on [`MidiConfig::Basic`] and up. Not all hosts may support
    /// these expressions.
    PolyTuning {
        timing: u32,
        /// A unique identifier for this note, if available. Using this to refer to a note is
        /// required when allowing overlapping voices for CLAP plugins.
        voice_id: Option<i32>,
        /// The note's channel, in `0..16`.
        channel: u8,
        /// The note's MIDI key number, in `0..128`.
        note: u8,
        /// The note's tuning in semitones, in `[-128, 128]`.
        tuning: f32,
    },
    /// A vibrato expression event, available on [`MidiConfig::Basic`] and up. Not all hosts may support
    /// these expressions.
    PolyVibrato {
        timing: u32,
        /// A unique identifier for this note, if available. Using this to refer to a note is
        /// required when allowing overlapping voices for CLAP plugins.
        voice_id: Option<i32>,
        /// The note's channel, in `0..16`.
        channel: u8,
        /// The note's MIDI key number, in `0..128`.
        note: u8,
        /// The note's vibrato amount, in `[0, 1]`.
        vibrato: f32,
    },
    /// A expression expression (yes, expression expression) event, available on
    /// [`MidiConfig::Basic`] and up. Not all hosts may support these expressions.
    PolyExpression {
        timing: u32,
        /// A unique identifier for this note, if available. Using this to refer to a note is
        /// required when allowing overlapping voices for CLAP plugins.
        voice_id: Option<i32>,
        /// The note's channel, in `0..16`.
        channel: u8,
        /// The note's MIDI key number, in `0..128`.
        note: u8,
        /// The note's expression amount, in `[0, 1]`.
        expression: f32,
    },
    /// A brightness expression event, available on [`MidiConfig::Basic`] and up. Not all hosts may support
    /// these expressions.
    PolyBrightness {
        timing: u32,
        /// A unique identifier for this note, if available. Using this to refer to a note is
        /// required when allowing overlapping voices for CLAP plugins.
        voice_id: Option<i32>,
        /// The note's channel, in `0..16`.
        channel: u8,
        /// The note's MIDI key number, in `0..128`.
        note: u8,
        /// The note's brightness amount, in `[0, 1]`.
        brightness: f32,
    },
    /// A MIDI channel pressure event, available on [`MidiConfig::MidiCCs`] and up.
    MidiChannelPressure {
        timing: u32,
        /// The affected channel, in `0..16`.
        channel: u8,
        /// The pressure, normalized to `[0, 1]` to match the poly pressure event.
        pressure: f32,
    },
    /// A MIDI pitch bend, available on [`MidiConfig::MidiCCs`] and up.
    MidiPitchBend {
        timing: u32,
        /// The affected channel, in `0..16`.
        channel: u8,
        /// The pressure, normalized to `[0, 1]`. `0.5` means no pitch bend.
        value: f32,
    },
    /// A MIDI control change event, available on [`MidiConfig::MidiCCs`] and up.
    ///
    /// # Note
    ///
    /// The wrapper does not perform any special handling for two message 14-bit CCs (where the CC
    /// number is in `0..32`, and the next CC is that number plus 32) or for four message RPN
    /// messages. For now you will need to handle these CCs yourself.
    MidiCC {
        timing: u32,
        /// The affected channel, in `0..16`.
        channel: u8,
        /// The control change number. See [`control_change`] for a list of CC numbers.
        cc: u8,
        /// The CC's value, normalized to `[0, 1]`. Multiply by 127 to get the original raw value.
        value: f32,
    },
    /// A MIDI program change event, available on [`MidiConfig::MidiCCs`] and up. VST3 plugins
    /// cannot receive these events.
    MidiProgramChange {
        timing: u32,
        /// The affected channel, in `0..16`.
        channel: u8,
        /// The program number, in `0..128`.
        program: u8,
    },
    /// A MIDI SysEx message supported by the plugin's `SysExMessage` type, available on
    /// [`MidiConfig::Basic`] and up. If the conversion from the raw byte array fails (e.g. the
    /// plugin doesn't support this kind of message), then this will be logged during debug builds
    /// of the plugin, and no event is emitted.
    MidiSysEx { timing: u32, message: S },
}

/// The result of converting a `NoteEvent<S>` to MIDI. This is a bit weirder than it would have to
/// be because it's not possible to use associated constants in type definitions.
#[derive(Debug, Clone)]
pub enum MidiResult<S: SysExMessage> {
    /// A basic three byte MIDI event.
    Basic([u8; 3]),
    /// A SysEx event. The message was written to the `S::Buffer` and may include padding at the
    /// end. The `usize` value indicates the message's actual length, including headers and end of
    /// SysEx byte.
    SysEx(S::Buffer, usize),
}

impl<S> NoteEvent<S> {
    /// Returns the sample within the current buffer this event belongs to.
    pub fn timing(&self) -> u32 {
        match self {
            NoteEvent::NoteOn { timing, .. } => *timing,
            NoteEvent::NoteOff { timing, .. } => *timing,
            NoteEvent::Choke { timing, .. } => *timing,
            NoteEvent::VoiceTerminated { timing, .. } => *timing,
            NoteEvent::PolyModulation { timing, .. } => *timing,
            NoteEvent::MonoAutomation { timing, .. } => *timing,
            NoteEvent::PolyPressure { timing, .. } => *timing,
            NoteEvent::PolyVolume { timing, .. } => *timing,
            NoteEvent::PolyPan { timing, .. } => *timing,
            NoteEvent::PolyTuning { timing, .. } => *timing,
            NoteEvent::PolyVibrato { timing, .. } => *timing,
            NoteEvent::PolyExpression { timing, .. } => *timing,
            NoteEvent::PolyBrightness { timing, .. } => *timing,
            NoteEvent::MidiChannelPressure { timing, .. } => *timing,
            NoteEvent::MidiPitchBend { timing, .. } => *timing,
            NoteEvent::MidiCC { timing, .. } => *timing,
            NoteEvent::MidiProgramChange { timing, .. } => *timing,
            NoteEvent::MidiSysEx { timing, .. } => *timing,
        }
    }

    /// Returns the event's voice ID, if it has any.
    pub fn voice_id(&self) -> Option<i32> {
        match self {
            NoteEvent::NoteOn { voice_id, .. } => *voice_id,
            NoteEvent::NoteOff { voice_id, .. } => *voice_id,
            NoteEvent::Choke { voice_id, .. } => *voice_id,
            NoteEvent::VoiceTerminated { voice_id, .. } => *voice_id,
            NoteEvent::PolyModulation { voice_id, .. } => Some(*voice_id),
            NoteEvent::MonoAutomation { .. } => None,
            NoteEvent::PolyPressure { voice_id, .. } => *voice_id,
            NoteEvent::PolyVolume { voice_id, .. } => *voice_id,
            NoteEvent::PolyPan { voice_id, .. } => *voice_id,
            NoteEvent::PolyTuning { voice_id, .. } => *voice_id,
            NoteEvent::PolyVibrato { voice_id, .. } => *voice_id,
            NoteEvent::PolyExpression { voice_id, .. } => *voice_id,
            NoteEvent::PolyBrightness { voice_id, .. } => *voice_id,
            NoteEvent::MidiChannelPressure { .. } => None,
            NoteEvent::MidiPitchBend { .. } => None,
            NoteEvent::MidiCC { .. } => None,
            NoteEvent::MidiProgramChange { .. } => None,
            NoteEvent::MidiSysEx { .. } => None,
        }
    }

    /// Returns the event's channel, if it has any.
    pub fn channel(&self) -> Option<u8> {
        match self {
            NoteEvent::NoteOn { channel, .. } => Some(*channel),
            NoteEvent::NoteOff { channel, .. } => Some(*channel),
            NoteEvent::Choke { channel, .. } => Some(*channel),
            NoteEvent::VoiceTerminated { channel, .. } => Some(*channel),
            NoteEvent::PolyModulation { .. } => None,
            NoteEvent::MonoAutomation { .. } => None,
            NoteEvent::PolyPressure { channel, .. } => Some(*channel),
            NoteEvent::PolyVolume { channel, .. } => Some(*channel),
            NoteEvent::PolyPan { channel, .. } => Some(*channel),
            NoteEvent::PolyTuning { channel, .. } => Some(*channel),
            NoteEvent::PolyVibrato { channel, .. } => Some(*channel),
            NoteEvent::PolyExpression { channel, .. } => Some(*channel),
            NoteEvent::PolyBrightness { channel, .. } => Some(*channel),
            NoteEvent::MidiChannelPressure { channel, .. } => Some(*channel),
            NoteEvent::MidiPitchBend { channel, .. } => Some(*channel),
            NoteEvent::MidiCC { channel, .. } => Some(*channel),
            NoteEvent::MidiProgramChange { channel, .. } => Some(*channel),
            NoteEvent::MidiSysEx { .. } => None,
        }
    }
}

impl<S: SysExMessage> NoteEvent<S> {
    /// Parse MIDI into a [`NoteEvent`]. Supports both basic three bytes messages as well as SysEx.
    /// Will return `Err(event_type)` if the parsing failed.
    pub fn from_midi(timing: u32, midi_data: &[u8]) -> Result<Self, u8> {
        let status_byte = midi_data.first().copied().unwrap_or_default();
        let event_type = status_byte & midi::EVENT_TYPE_MASK;
        let channel = status_byte & midi::MIDI_CHANNEL_MASK;

        if midi_data.len() >= 3 {
            // TODO: Maybe add special handling for 14-bit CCs and RPN messages at some
            //       point, right now the plugin has to figure it out for itself
            match event_type {
                // You thought this was a note on? Think again! This is a cleverly disguised note off
                // event straight from the 80s when Baud rate was still a limiting factor!
                midi::NOTE_ON if midi_data[2] == 0 => {
                    return Ok(NoteEvent::NoteOff {
                        timing,
                        voice_id: None,
                        channel,
                        note: midi_data[1],
                        // Few things use release velocity. Just having this be zero here is fine, right?
                        velocity: 0.0,
                    });
                }
                midi::NOTE_ON => {
                    return Ok(NoteEvent::NoteOn {
                        timing,
                        voice_id: None,
                        channel,
                        note: midi_data[1],
                        velocity: midi_data[2] as f32 / 127.0,
                    });
                }
                midi::NOTE_OFF => {
                    return Ok(NoteEvent::NoteOff {
                        timing,
                        voice_id: None,
                        channel,
                        note: midi_data[1],
                        velocity: midi_data[2] as f32 / 127.0,
                    });
                }
                midi::POLYPHONIC_KEY_PRESSURE => {
                    return Ok(NoteEvent::PolyPressure {
                        timing,
                        voice_id: None,
                        channel,
                        note: midi_data[1],
                        pressure: midi_data[2] as f32 / 127.0,
                    });
                }
                midi::PITCH_BEND_CHANGE => {
                    return Ok(NoteEvent::MidiPitchBend {
                        timing,
                        channel,
                        value: (midi_data[1] as u16 + ((midi_data[2] as u16) << 7)) as f32
                            / ((1 << 14) - 1) as f32,
                    });
                }
                midi::CONTROL_CHANGE => {
                    return Ok(NoteEvent::MidiCC {
                        timing,
                        channel,
                        cc: midi_data[1],
                        value: midi_data[2] as f32 / 127.0,
                    });
                }
                _ => (),
            }
        }
        if midi_data.len() >= 2 {
            match event_type {
                midi::CHANNEL_KEY_PRESSURE => {
                    return Ok(NoteEvent::MidiChannelPressure {
                        timing,
                        channel,
                        pressure: midi_data[1] as f32 / 127.0,
                    });
                }
                midi::PROGRAM_CHANGE => {
                    return Ok(NoteEvent::MidiProgramChange {
                        timing,
                        channel,
                        program: midi_data[1],
                    });
                }
                _ => (),
            }
        }

        // Every other message is parsed as SysEx, even if they don't have the `0xf0` status byte.
        // This allows the `SysExMessage` trait to have a bit more flexibility if needed. Regular
        // note event parsing however still has higher priority.
        match S::from_buffer(midi_data) {
            Some(message) => Ok(NoteEvent::MidiSysEx { timing, message }),
            None => {
                if event_type == 0xf0 {
                    if midi_data.len() <= 32 {
                        nih_trace!("Unhandled MIDI system message: {midi_data:02x?}");
                    } else {
                        nih_trace!("Unhandled MIDI system message of {} bytes", midi_data.len());
                    }
                } else {
                    nih_trace!("Unhandled MIDI status byte {status_byte:#x}");
                }

                Err(event_type)
            }
        }
    }

    /// Create a MIDI message from this note event. Returns `None` if this even does not have a
    /// direct MIDI equivalent. `PolyPressure` will be converted to polyphonic key pressure, but the
    /// other polyphonic note expression types will not be converted to MIDI CC messages.
    pub fn as_midi(self) -> Option<MidiResult<S>> {
        match self {
            NoteEvent::NoteOn {
                timing: _,
                voice_id: _,
                channel,
                note,
                velocity,
            } => Some(MidiResult::Basic([
                midi::NOTE_ON | channel,
                note,
                // MIDI treats note ons with zero velocity as note offs, because reasons
                (velocity * 127.0).round().clamp(1.0, 127.0) as u8,
            ])),
            NoteEvent::NoteOff {
                timing: _,
                voice_id: _,
                channel,
                note,
                velocity,
            } => Some(MidiResult::Basic([
                midi::NOTE_OFF | channel,
                note,
                (velocity * 127.0).round().clamp(0.0, 127.0) as u8,
            ])),
            NoteEvent::PolyPressure {
                timing: _,
                voice_id: _,
                channel,
                note,
                pressure,
            } => Some(MidiResult::Basic([
                midi::POLYPHONIC_KEY_PRESSURE | channel,
                note,
                (pressure * 127.0).round().clamp(0.0, 127.0) as u8,
            ])),
            NoteEvent::MidiChannelPressure {
                timing: _,
                channel,
                pressure,
            } => Some(MidiResult::Basic([
                midi::CHANNEL_KEY_PRESSURE | channel,
                (pressure * 127.0).round().clamp(0.0, 127.0) as u8,
                0,
            ])),
            NoteEvent::MidiPitchBend {
                timing: _,
                channel,
                value,
            } => {
                const PITCH_BEND_RANGE: f32 = ((1 << 14) - 1) as f32;
                let midi_value = (value * PITCH_BEND_RANGE)
                    .round()
                    .clamp(0.0, PITCH_BEND_RANGE) as u16;

                Some(MidiResult::Basic([
                    midi::PITCH_BEND_CHANGE | channel,
                    (midi_value & ((1 << 7) - 1)) as u8,
                    (midi_value >> 7) as u8,
                ]))
            }
            NoteEvent::MidiCC {
                timing: _,
                channel,
                cc,
                value,
            } => Some(MidiResult::Basic([
                midi::CONTROL_CHANGE | channel,
                cc,
                (value * 127.0).round().clamp(0.0, 127.0) as u8,
            ])),
            NoteEvent::MidiProgramChange {
                timing: _,
                channel,
                program,
            } => Some(MidiResult::Basic([
                midi::PROGRAM_CHANGE | channel,
                program,
                0,
            ])),
            // `message` is serialized and written to `sysex_buffer`, and the result contains the
            // message's actual length
            NoteEvent::MidiSysEx { timing: _, message } => {
                let (padded_sysex_buffer, length) = message.to_buffer();
                Some(MidiResult::SysEx(padded_sysex_buffer, length))
            }
            NoteEvent::Choke { .. }
            | NoteEvent::VoiceTerminated { .. }
            | NoteEvent::PolyModulation { .. }
            | NoteEvent::MonoAutomation { .. }
            | NoteEvent::PolyVolume { .. }
            | NoteEvent::PolyPan { .. }
            | NoteEvent::PolyTuning { .. }
            | NoteEvent::PolyVibrato { .. }
            | NoteEvent::PolyExpression { .. }
            | NoteEvent::PolyBrightness { .. } => None,
        }
    }

    /// Subtract a sample offset from this event's timing, needed to compensate for the block
    /// splitting in the VST3 wrapper implementation because all events have to be read upfront.
    #[cfg_attr(not(feature = "vst3"), allow(dead_code))]
    pub(crate) fn subtract_timing(&mut self, samples: u32) {
        match self {
            NoteEvent::NoteOn { timing, .. } => *timing -= samples,
            NoteEvent::NoteOff { timing, .. } => *timing -= samples,
            NoteEvent::Choke { timing, .. } => *timing -= samples,
            NoteEvent::VoiceTerminated { timing, .. } => *timing -= samples,
            NoteEvent::PolyModulation { timing, .. } => *timing -= samples,
            NoteEvent::MonoAutomation { timing, .. } => *timing -= samples,
            NoteEvent::PolyPressure { timing, .. } => *timing -= samples,
            NoteEvent::PolyVolume { timing, .. } => *timing -= samples,
            NoteEvent::PolyPan { timing, .. } => *timing -= samples,
            NoteEvent::PolyTuning { timing, .. } => *timing -= samples,
            NoteEvent::PolyVibrato { timing, .. } => *timing -= samples,
            NoteEvent::PolyExpression { timing, .. } => *timing -= samples,
            NoteEvent::PolyBrightness { timing, .. } => *timing -= samples,
            NoteEvent::MidiChannelPressure { timing, .. } => *timing -= samples,
            NoteEvent::MidiPitchBend { timing, .. } => *timing -= samples,
            NoteEvent::MidiCC { timing, .. } => *timing -= samples,
            NoteEvent::MidiProgramChange { timing, .. } => *timing -= samples,
            NoteEvent::MidiSysEx { timing, .. } => *timing -= samples,
        }
    }
}

#[cfg(test)]
mod tests {
    pub use super::*;

    pub const TIMING: u32 = 5;

    /// Converts an event to and from MIDI. Panics if any part of the conversion fails.
    fn roundtrip_basic_event(event: NoteEvent<()>) -> NoteEvent<()> {
        let midi_data = match event.as_midi().unwrap() {
            MidiResult::Basic(midi_data) => midi_data,
            MidiResult::SysEx(_, _) => panic!("Unexpected SysEx result"),
        };

        NoteEvent::from_midi(TIMING, &midi_data).unwrap()
    }

    #[test]
    fn test_note_on_midi_conversion() {
        let event = NoteEvent::<()>::NoteOn {
            timing: TIMING,
            voice_id: None,
            channel: 1,
            note: 2,
            // The value will be rounded in the conversion to MIDI, hence this overly specific value
            velocity: 0.6929134,
        };

        assert_eq!(roundtrip_basic_event(event), event);
    }

    #[test]
    fn test_note_off_midi_conversion() {
        let event = NoteEvent::<()>::NoteOff {
            timing: TIMING,
            voice_id: None,
            channel: 1,
            note: 2,
            velocity: 0.6929134,
        };

        assert_eq!(roundtrip_basic_event(event), event);
    }

    #[test]
    fn test_poly_pressure_midi_conversion() {
        let event = NoteEvent::<()>::PolyPressure {
            timing: TIMING,
            voice_id: None,
            channel: 1,
            note: 2,
            pressure: 0.6929134,
        };

        assert_eq!(roundtrip_basic_event(event), event);
    }

    #[test]
    fn test_channel_pressure_midi_conversion() {
        let event = NoteEvent::<()>::MidiChannelPressure {
            timing: TIMING,
            channel: 1,
            pressure: 0.6929134,
        };

        assert_eq!(roundtrip_basic_event(event), event);
    }

    #[test]
    fn test_pitch_bend_midi_conversion() {
        let event = NoteEvent::<()>::MidiPitchBend {
            timing: TIMING,
            channel: 1,
            value: 0.6929134,
        };

        assert_eq!(roundtrip_basic_event(event), event);
    }

    #[test]
    fn test_cc_midi_conversion() {
        let event = NoteEvent::<()>::MidiCC {
            timing: TIMING,
            channel: 1,
            cc: 2,
            value: 0.6929134,
        };

        assert_eq!(roundtrip_basic_event(event), event);
    }

    #[test]
    fn test_program_change_midi_conversion() {
        let event = NoteEvent::<()>::MidiProgramChange {
            timing: TIMING,
            channel: 1,
            program: 42,
        };

        assert_eq!(roundtrip_basic_event(event), event);
    }

    mod sysex {
        use super::*;

        #[derive(Clone, Debug, PartialEq)]
        enum MessageType {
            Foo(f32),
        }

        impl SysExMessage for MessageType {
            type Buffer = [u8; 4];

            fn from_buffer(buffer: &[u8]) -> Option<Self> {
                match buffer {
                    [0xf0, 0x69, n, 0xf7] => Some(MessageType::Foo(*n as f32 / 127.0)),
                    _ => None,
                }
            }

            fn to_buffer(self) -> (Self::Buffer, usize) {
                match self {
                    MessageType::Foo(x) => ([0xf0, 0x69, (x * 127.0).round() as u8, 0xf7], 4),
                }
            }
        }

        #[test]
        fn test_parse_from_buffer() {
            let midi_data = [0xf0, 0x69, 127, 0xf7];
            let parsed = NoteEvent::from_midi(TIMING, &midi_data).unwrap();

            assert_eq!(
                parsed,
                NoteEvent::MidiSysEx {
                    timing: TIMING,
                    message: MessageType::Foo(1.0)
                }
            );
        }

        #[test]
        fn test_convert_to_buffer() {
            let message = MessageType::Foo(1.0);
            let event = NoteEvent::MidiSysEx {
                timing: TIMING,
                message,
            };

            match event.as_midi() {
                Some(MidiResult::SysEx(padded_sysex_buffer, length)) => {
                    assert_eq!(padded_sysex_buffer[..length], [0xf0, 0x69, 127, 0xf7])
                }
                result => panic!("Unexpected result: {result:?}"),
            }
        }

        #[test]
        fn test_invalid_parse() {
            let midi_data = [0xf0, 0x0, 127, 0xf7];
            let parsed = NoteEvent::<MessageType>::from_midi(TIMING, &midi_data);

            assert!(parsed.is_err());
        }
    }
}

```

[params.rs](../nih_plug_src/params.rs)

```rust
//! NIH-plug can handle floating point, integer, boolean, and enum parameters. Parameters are
//! managed by creating a struct deriving the [`Params`][Params] trait containing fields
//! for those parameter types, and then returning a reference to that object from your
//! [`Plugin::params()`][crate::prelude::Plugin::params()] method. See the `Params` trait for more
//! information.

use std::collections::BTreeMap;
use std::fmt::{Debug, Display};
use std::sync::Arc;

use self::internals::ParamPtr;

// The proc-macro for deriving `Params`
pub use nih_plug_derive::Params;

// Parameter types
mod boolean;
pub mod enums;
mod float;
mod integer;

pub mod internals;
pub mod persist;
pub mod range;
pub mod smoothing;

pub use boolean::BoolParam;
pub use enums::EnumParam;
pub use float::FloatParam;
pub use integer::IntParam;

bitflags::bitflags! {
    /// Flags for controlling a parameter's behavior.
    #[repr(transparent)]
    #[derive(Default)]
    pub struct ParamFlags: u32 {
        /// When applied to a [`BoolParam`], this will cause the parameter to be linked to the
        /// host's bypass control. Only a single parameter can be marked as a bypass parameter. If
        /// you don't have a bypass parameter, then NIH-plug will add one for you. You will need to
        /// implement this yourself if your plugin introduces latency.
        const BYPASS = 1 << 0;
        /// The parameter cannot be changed from an automation lane. The parameter can however still
        /// be manually changed by the user from either the plugin's own GUI or from the host's
        /// generic UI.
        const NON_AUTOMATABLE = 1 << 1;
        /// Hides the parameter in the host's generic UI for this plugin. This also implies
        /// `NON_AUTOMATABLE`. Setting this does not prevent you from changing the parameter in the
        /// plugin's editor GUI.
        const HIDDEN = 1 << 2;
        /// Don't show this parameter when generating a generic UI for the plugin using one of
        /// NIH-plug's generic UI widgets.
        const HIDE_IN_GENERIC_UI = 1 << 3;
    }
}

// See https://rust-lang.github.io/api-guidelines/future-proofing.html for more information
mod sealed {
    /// Dummy trait to prevent [`Param`] from being implemented outside of NIH-plug. This is not
    /// possible because of the way `ParamPtr` works, so it's best to just make it flat out impossible.
    pub trait Sealed {}
}
pub(crate) use sealed::Sealed;

/// Describes a single parameter of any type. Most parameter implementations also have a field
/// called `value` that and a field called `smoothed`. The former stores the latest unsmoothed
/// value, and the latter can be used to access the smoother. These two fields should be used in DSP
/// code to either get the parameter's current (smoothed) value. In UI code the getters from this
/// trait should be used instead.
///
/// # Sealed
///
/// This trait cannot be implemented outside of NIH-plug itself. If you want to create new
/// abstractions around parameters, consider wrapping them in a struct instead. Then use the
/// `#[nested(id_prefix = "foo")]` syntax from the [`Params`] trait to reuse that wrapper in
/// multiple places.
pub trait Param: Display + Debug + sealed::Sealed {
    /// The plain parameter type.
    type Plain: PartialEq;

    /// Get the human readable name for this parameter.
    fn name(&self) -> &str;

    /// Get the unit label for this parameter, if any.
    fn unit(&self) -> &'static str;

    /// Get this parameter's polyphonic modulation ID. If this is set for a parameter in a CLAP
    /// plugin, then polyphonic modulation will be enabled for that parameter. Polyphonic modulation
    /// is communicated to the plugin through
    /// [`NoteEvent::PolyModulation`][crate::prelude::NoteEvent::PolyModulation] and
    /// [`NoteEvent::MonoAutomation`][crate::prelude::NoteEvent::MonoAutomation] events. See the
    /// documentation on those events for more information.
    ///
    /// # Important
    ///
    /// After enabling polyphonic modulation, the plugin **must** start sending
    /// [`NoteEvent::VoiceTerminated`][crate::prelude::NoteEvent::VoiceTerminated] events to the
    /// host when a voice has fully ended. This allows the host to reuse its modulation resources.
    fn poly_modulation_id(&self) -> Option<u32>;

    /// Get the unnormalized value for this parameter.
    fn modulated_plain_value(&self) -> Self::Plain;

    /// Get the normalized `[0, 1]` value for this parameter.
    fn modulated_normalized_value(&self) -> f32;

    /// Get the unnormalized value for this parameter before any (monophonic) modulation coming from
    /// the host has been applied. If the host is not currently modulating this parameter than this
    /// will be the same as [`modulated_plain_value()`][Self::modulated_plain_value()]. This may be
    /// useful for displaying modulation differently in plugin GUIs. Right now only CLAP plugins in
    /// Bitwig Studio use modulation.
    fn unmodulated_plain_value(&self) -> Self::Plain;

    /// Get the normalized `[0, 1]` value for this parameter before any (monophonic) modulation
    /// coming from the host has been applied. If the host is not currently modulating this
    /// parameter than this will be the same as
    /// [`modulated_normalized_value()`][Self::modulated_normalized_value()]. This may be useful for
    /// displaying modulation differently in plugin GUIs. Right now only CLAP plugins in Bitwig
    /// Studio use modulation.
    fn unmodulated_normalized_value(&self) -> f32;

    /// Get the unnormalized default value for this parameter.
    fn default_plain_value(&self) -> Self::Plain;

    /// Get the normalized `[0, 1]` default value for this parameter.
    #[inline]
    fn default_normalized_value(&self) -> f32 {
        self.preview_normalized(self.default_plain_value())
    }

    /// Get the number of steps for this parameter, if it is discrete. Used for the host's generic
    /// UI.
    fn step_count(&self) -> Option<usize>;

    /// Returns the previous step from a specific value for this parameter. This can be the same as
    /// `from` if the value is at the start of its range. This is mainly used for scroll wheel
    /// interaction in plugin GUIs. When the parameter is not discrete then a step should cover one
    /// hundredth of the normalized range instead.
    ///
    /// If `finer` is true, then the step size should be decreased if the parameter is continuous.
    fn previous_step(&self, from: Self::Plain, finer: bool) -> Self::Plain;

    /// Returns the next step from a specific value for this parameter. This can be the same as
    /// `from` if the value is at the end of its range. This is mainly used for scroll wheel
    /// interaction in plugin GUIs. When the parameter is not discrete then a step should cover one
    /// hundredth of the normalized range instead.
    ///
    /// If `finer` is true, then the step size should be decreased if the parameter is continuous.
    fn next_step(&self, from: Self::Plain, finer: bool) -> Self::Plain;

    /// The same as [`previous_step()`][Self::previous_step()], but for normalized values. This is
    /// mostly useful for GUI widgets.
    fn previous_normalized_step(&self, from: f32, finer: bool) -> f32 {
        self.preview_normalized(self.previous_step(self.preview_plain(from), finer))
    }

    /// The same as [`next_step()`][Self::next_step()], but for normalized values. This is mostly
    /// useful for GUI widgets.
    fn next_normalized_step(&self, from: f32, finer: bool) -> f32 {
        self.preview_normalized(self.next_step(self.preview_plain(from), finer))
    }

    /// Get the string representation for a normalized value. Used as part of the wrappers. Most
    /// plugin formats already have support for units, in which case it shouldn't be part of this
    /// string or some DAWs may show duplicate units.
    fn normalized_value_to_string(&self, normalized: f32, include_unit: bool) -> String;

    /// Get the string representation for a normalized value. Used as part of the wrappers.
    fn string_to_normalized_value(&self, string: &str) -> Option<f32>;

    /// Get the normalized value for a plain, unnormalized value, as a float. Used as part of the
    /// wrappers.
    fn preview_normalized(&self, plain: Self::Plain) -> f32;

    /// Get the plain, unnormalized value for a normalized value, as a float. Used as part of the
    /// wrappers. This **does** snap to step sizes for continuous parameters (i.e. [`FloatParam`]).
    fn preview_plain(&self, normalized: f32) -> Self::Plain;

    /// Get the plain, unnormalized value for this parameter after polyphonic modulation has been
    /// applied. This is a convenience method for calling [`preview_plain()`][Self::preview_plain()]
    /// with `unmodulated_normalized_value() + normalized_offset`.
    #[inline]
    fn preview_modulated(&self, normalized_offset: f32) -> Self::Plain {
        self.preview_plain(self.unmodulated_normalized_value() + normalized_offset)
    }

    /// Flags to control the parameter's behavior. See [`ParamFlags`].
    fn flags(&self) -> ParamFlags;

    /// Internal implementation detail for implementing [`Params`][Params]. This should
    /// not be used directly.
    fn as_ptr(&self) -> internals::ParamPtr;
}

/// Contains the setters for parameters. These should not be exposed to plugins to avoid confusion.
pub(crate) trait ParamMut: Param {
    /// Set this parameter based on a plain, unnormalized value. This does not snap to step sizes
    /// for continuous parameters (i.e. [`FloatParam`]). If
    /// [`modulate_value()`][Self::modulate_value()] has previously been called with a non zero
    /// value then this offset is taken into account to form the effective value.
    ///
    /// Returns whether or not the value has changed. Any parameter callbacks are only run the value
    /// has actually changed.
    ///
    /// This does **not** update the smoother.
    fn set_plain_value(&self, plain: Self::Plain) -> bool;

    /// Set this parameter based on a normalized value. The normalized value will be snapped to the
    /// step size for continuous parameters (i.e. [`FloatParam`]). If
    /// [`modulate_value()`][Self::modulate_value()] has previously been called with a non zero
    /// value then this offset is taken into account to form the effective value.
    ///
    /// Returns whether or not the value has changed. Any parameter callbacks are only run the value
    /// has actually changed.
    ///
    /// This does **not** update the smoother.
    fn set_normalized_value(&self, normalized: f32) -> bool;

    /// Add a modulation offset to the value's unmodulated value. This value sticks until this
    /// function is called again with a 0.0 value. Out of bound values will be clamped to the
    /// parameter's range. The normalized value will be snapped to the step size for continuous
    /// parameters (i.e. [`FloatParam`]).
    ///
    /// Returns whether or not the value has changed. Any parameter callbacks are only run the value
    /// has actually changed.
    ///
    /// This does **not** update the smoother.
    fn modulate_value(&self, modulation_offset: f32) -> bool;

    /// Update the smoother state to point to the current value. Also used when initializing and
    /// restoring a plugin so everything is in sync. In that case the smoother should completely
    /// reset to the current value.
    fn update_smoother(&self, sample_rate: f32, reset: bool);
}

/// Describes a struct containing parameters and other persistent fields.
///
/// # Deriving `Params` and `#[id = "stable"]`
///
/// This trait can be derived on a struct containing [`FloatParam`] and other parameter fields by
/// adding `#[derive(Params)]`. When deriving this trait, any of those parameter fields should have
/// the `#[id = "stable"]` attribute, where `stable` is an up to 6 character long string (to avoid
/// collisions) that will be used to identify the parameter internally so you can safely move it
/// around and rename the field without breaking compatibility with old presets.
///
/// ## `#[persist = "key"]`
///
/// The struct can also contain other fields that should be persisted along with the rest of the
/// preset data. These fields should be [`PersistentField`][persist::PersistentField]s annotated
/// with the `#[persist = "key"]` attribute containing types that can be serialized and deserialized
/// with [Serde](https://serde.rs/).
///
/// ## `#[nested]`, `#[nested(group_name = "group name")]`
///
/// Finally, the `Params` object may include parameters from other objects. Setting a group name is
/// optional, but some hosts can use this information to display the parameters in a tree structure.
/// Parameter IDs and persisting keys still need to be **unique** when using nested parameter
/// structs.
///
/// Take a look at the example gain example plugin to see how this is used.
///
/// ## `#[nested(id_prefix = "foo", group_name = "Foo")]`
///
/// Adding this attribute to a `Params` sub-object works similarly to the regular `#[nested]`
/// attribute, but it also adds an ID to all parameters from the nested object. If a parameter in
/// the nested nested object normally has parameter ID `bar`, the parameter's ID will now be renamed
/// to `foo_bar`. The same thing happens with persistent field keys to support multiple copies of
/// the field. _This makes it possible to reuse the same parameter struct with different names and
/// parameter indices._
///
/// ## `#[nested(array, group_name = "Foo")]`
///
/// This can be applied to an array-like data structure and it works similar to a `nested` attribute
/// with an `id_name`, except that it will iterate over the array and create unique indices for all
/// nested parameters. If the nested parameters object has a parameter called `bar`, then that
/// parameter will belong to the group `Foo {array_index + 1}`, and it will have the renamed
/// parameter ID `bar_{array_index + 1}`. The same thing applies to persistent field keys.
///
/// # Safety
///
/// This implementation is safe when using from the wrapper because the plugin's returned `Params`
/// object lives in an `Arc`, and the wrapper also holds a reference to this `Arc`.
pub unsafe trait Params: 'static + Send + Sync {
    /// Create a mapping from unique parameter IDs to parameter pointers along with the name of the
    /// group/unit/module they are in, as a `(param_id, param_ptr, group)` triple. The order of the
    /// `Vec` determines the display order in the (host's) generic UI. The group name is either an
    /// empty string for top level parameters, or a slash/delimited `"group name 1/Group Name 2"` if
    /// this `Params` object contains nested child objects. All components of a group path must
    /// exist or you may encounter panics. The derive macro does this for every parameter field
    /// marked with `#[id = "stable"]`, and it also inlines all fields from nested child `Params`
    /// structs marked with `#[nested(...)]` while prefixing that group name before the parameter's
    /// original group name. Dereferencing the pointers stored in the values is only valid as long
    /// as this object is valid.
    ///
    /// # Note
    ///
    /// This uses `String` even though for the `Params` derive macro `&'static str` would have been
    /// fine to be able to support custom reusable Params implementations.
    fn param_map(&self) -> Vec<(String, ParamPtr, String)>;

    /// Serialize all fields marked with `#[persist = "stable_name"]` into a hash map containing
    /// JSON-representations of those fields so they can be written to the plugin's state and
    /// recalled later. This uses [`persist::serialize_field()`] under the hood.
    fn serialize_fields(&self) -> BTreeMap<String, String> {
        BTreeMap::new()
    }

    /// Restore all fields marked with `#[persist = "stable_name"]` from a hashmap created by
    /// [`serialize_fields()`][Self::serialize_fields()]. All of these fields should be wrapped in a
    /// [`persist::PersistentField`] with thread safe interior mutability, like an `RwLock` or a
    /// `Mutex`. This gets called when the plugin's state is being restored. This uses
    /// [`persist::deserialize_field()`] under the hood.
    #[allow(unused_variables)]
    fn deserialize_fields(&self, serialized: &BTreeMap<String, String>) {}
}

/// This may be useful when building generic UIs using nested `Params` objects.
unsafe impl<P: Params> Params for Arc<P> {
    fn param_map(&self) -> Vec<(String, ParamPtr, String)> {
        self.as_ref().param_map()
    }

    fn serialize_fields(&self) -> BTreeMap<String, String> {
        self.as_ref().serialize_fields()
    }

    fn deserialize_fields(&self, serialized: &BTreeMap<String, String>) {
        self.as_ref().deserialize_fields(serialized)
    }
}

```

[plugin.rs](../nih_plug_src/plugin.rs)

```rust
//! Traits and structs describing plugins and editors. This includes extension structs for features
//! that are specific to one or more plugin-APIs.

use std::sync::Arc;

use crate::prelude::{
    AsyncExecutor, AudioIOLayout, AuxiliaryBuffers, Buffer, BufferConfig, Editor, InitContext,
    MidiConfig, Params, PluginState, ProcessContext, SysExMessage,
};

pub mod clap;
#[cfg(feature = "vst3")]
pub mod vst3;

/// A function that can execute a plugin's [`BackgroundTask`][Plugin::BackgroundTask]s. A plugin can
/// dispatch these tasks from the `initialize()` function, the `process()` function, or the GUI, so
/// they can be deferred for later to avoid blocking realtime contexts.
pub type TaskExecutor<P> = Box<dyn Fn(<P as Plugin>::BackgroundTask) + Send>;

/// The main plugin trait covering functionality common across most plugin formats. Most formats
/// also have another trait with more specific data and functionality that needs to be implemented
/// before the plugin can be exported to that format. The wrappers will use this to expose the
/// plugin in a particular plugin format.
///
/// NIH-plug is semi-declarative, meaning that most information about a plugin is defined
/// declaratively but it also doesn't shy away from maintaining state when that is the path of least
/// resistance. As such, the definitions on this trait fall in one of the following classes:
///
/// - `Plugin` objects are stateful. During their lifetime the plugin API wrappers will call the
///   various lifecycle methods defined below, with the `initialize()`, `reset()`, and `process()`
///   functions being the most important ones.
/// - Most of the rest of the trait statically describes the plugin. You will find this done in
///   three different ways:
///   - Most of this data, including the supported audio IO layouts, is simple enough that it can be
///     defined through compile-time constants.
///   - Some of the data is queried through a method as doing everything at compile time would
///     impose a lot of restrictions on code structure and meta programming without any real
///     benefits. In those cases the trait defines a method that is queried once and only once,
///     immediately after instantiating the `Plugin` through `Plugin::default()`. Examples of these
///     methods are [`Plugin::params()`], and
///     [`ClapPlugin::remote_controls()`][clap::ClapPlugin::remote_controls()].
///   - Some of the data is defined through associated types. Rust currently sadly does not support
///     default values for associated types, but all of these types can be set to `()` if you wish
///     to ignore them. Examples of these types are [`Plugin::SysExMessage`] and
///     [`Plugin::BackgroundTask`].
/// - Finally, there are some functions that return extension structs and handlers, similar to how
///   the `params()` function returns a data structure describing the plugin's parameters. Examples
///   of these are the [`Plugin::editor()`] and [`Plugin::task_executor()`] functions, and they're
///   also called once and only once after the plugin object has been created. This allows the audio
///   thread to have exclusive access to the `Plugin` object, and it makes it easier to compose
///   these extension structs since they're more loosely coupled to a specific `Plugin`
///   implementation.
///
/// The main thing you need to do is define a `[Params]` struct containing all of your parameters.
/// See the trait's documentation for more information on how to do that, or check out the examples.
/// The plugin also needs a `Default` implementation so it can be initialized. Most of the other
/// functionality is optional and comes with default trait method implementations.
#[allow(unused_variables)]
pub trait Plugin: Default + Send + 'static {
    /// The plugin's name.
    const NAME: &'static str;
    /// The name of the plugin's vendor.
    const VENDOR: &'static str;
    /// A URL pointing to the plugin's web page.
    const URL: &'static str;
    /// The vendor's email address.
    const EMAIL: &'static str;

    /// Semver compatible version string (e.g. `0.0.1`). Hosts likely won't do anything with this,
    /// but just in case they do this should only contain decimals values and dots.
    const VERSION: &'static str;

    /// The plugin's supported audio IO layouts. The first config will be used as the default config
    /// if the host doesn't or can't select an alternative configuration. Because of that it's
    /// recommended to begin this slice with a stereo layout. For maximum compatibility with the
    /// different plugin formats this default layout should also include all of the plugin's
    /// auxiliary input and output ports, if the plugin has any. If the slice is empty, then the
    /// plugin will not have any audio IO.
    ///
    /// Both [`AudioIOLayout`] and [`PortNames`][crate::prelude::PortNames] have `.const_default()`
    /// functions for compile-time equivalents to `Default::default()`:
    ///
    /// ```
    /// # use nih_plug::prelude::*;
    /// const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[AudioIOLayout {
    ///     main_input_channels: NonZeroU32::new(2),
    ///     main_output_channels: NonZeroU32::new(2),
    ///
    ///     aux_input_ports: &[new_nonzero_u32(2)],
    ///
    ///     ..AudioIOLayout::const_default()
    /// }];
    /// ```
    ///
    /// # Note
    ///
    /// Some plugin hosts, like Ableton Live, don't support MIDI-only plugins and may refuse to load
    /// plugins with no main output or with zero main output channels.
    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout];

    /// Whether the plugin accepts note events, and what which events it wants to receive. If this
    /// is set to [`MidiConfig::None`], then the plugin won't receive any note events.
    const MIDI_INPUT: MidiConfig = MidiConfig::None;
    /// Whether the plugin can output note events. If this is set to [`MidiConfig::None`], then the
    /// plugin won't have a note output port. When this is set to another value, then in most hosts
    /// the plugin will consume all note and MIDI CC input. If you don't want that, then you will
    /// need to forward those events yourself.
    const MIDI_OUTPUT: MidiConfig = MidiConfig::None;
    /// If enabled, the audio processing cycle may be split up into multiple smaller chunks if
    /// parameter values change occur in the middle of the buffer. Depending on the host these
    /// blocks may be as small as a single sample. Bitwig Studio sends at most one parameter change
    /// every 64 samples.
    const SAMPLE_ACCURATE_AUTOMATION: bool = false;

    /// If this is set to true, then the plugin will report itself as having a hard realtime
    /// processing requirement when the host asks for it. Supported hosts will never ask the plugin
    /// to do offline processing.
    const HARD_REALTIME_ONLY: bool = false;

    /// The plugin's SysEx message type if it supports sending or receiving MIDI SysEx messages, or
    /// `()` if it does not. This type can be a struct or enum wrapping around one or more message
    /// types, and the [`SysExMessage`] trait is then used to convert between this type and basic
    /// byte buffers. The [`MIDI_INPUT`][Self::MIDI_INPUT] and [`MIDI_OUTPUT`][Self::MIDI_OUTPUT]
    /// fields need to be set to [`MidiConfig::Basic`] or above to be able to send and receive
    /// SysEx.
    type SysExMessage: SysExMessage;

    /// A type encoding the different background tasks this plugin wants to run, or `()` if it
    /// doesn't have any background tasks. This is usually set to an enum type. The task type should
    /// not contain any heap allocated data like [`Vec`]s and [`Box`]es. Tasks can be send using the
    /// methods on the various [`*Context`][crate::context] objects.
    //
    // NOTE: Sadly it's not yet possible to default this and the `async_executor()` function to
    //       `()`: https://github.com/rust-lang/rust/issues/29661
    type BackgroundTask: Send;
    /// A function that executes the plugin's tasks. When implementing this you will likely want to
    /// pattern match on the task type, and then send any resulting data back over a channel or
    /// triple buffer. See [`BackgroundTask`][Self::BackgroundTask].
    ///
    /// Queried only once immediately after the plugin instance is created. This function takes
    /// `&mut self` to make it easier to move data into the closure.
    fn task_executor(&mut self) -> TaskExecutor<Self> {
        // In the default implementation we can simply ignore the value
        Box::new(|_| ())
    }

    /// The plugin's parameters. The host will update the parameter values before calling
    /// `process()`. These string parameter IDs parameters should never change as they are used to
    /// distinguish between parameters.
    ///
    /// Queried only once immediately after the plugin instance is created.
    fn params(&self) -> Arc<dyn Params>;

    /// Returns an extension struct for interacting with the plugin's editor, if it has one. Later
    /// the host may call [`Editor::spawn()`] to create an editor instance. To read the current
    /// parameter values, you will need to clone and move the `Arc` containing your `Params` object
    /// into the editor. You can later modify the parameters through the
    /// [`GuiContext`][crate::prelude::GuiContext] and [`ParamSetter`][crate::prelude::ParamSetter]
    /// after the editor GUI has been created. NIH-plug comes with wrappers for several common GUI
    /// frameworks that may have their own ways of interacting with parameters. See the repo's
    /// readme for more information.
    ///
    /// Queried only once immediately after the plugin instance is created. This function takes
    /// `&mut self` to make it easier to move data into the `Editor` implementation.
    fn editor(&mut self, async_executor: AsyncExecutor<Self>) -> Option<Box<dyn Editor>> {
        None
    }

    /// This function is always called just before a [`PluginState`] is loaded. This lets you
    /// directly modify old plugin state to perform migrations based on the [`PluginState::version`]
    /// field. Some examples of use cases for this are renaming parameter indices, remapping
    /// parameter values, and preserving old preset compatibility when introducing new parameters
    /// with default values that would otherwise change the sound of a preset. Keep in mind that
    /// automation may still be broken in the first two use cases.
    ///
    /// # Note
    ///
    /// This is an advanced feature that the vast majority of plugins won't need to implement.
    fn filter_state(state: &mut PluginState) {}

    //
    // The following functions follow the lifetime of the plugin.
    //

    /// Initialize the plugin for the given audio IO configuration. From this point onwards the
    /// audio IO layouts and the buffer sizes are fixed until this function is called again.
    ///
    /// Before this point, the plugin should not have done any expensive initialization. Please
    /// don't be that plugin that takes twenty seconds to scan.
    ///
    /// After this function [`reset()`][Self::reset()] will always be called. If you need to clear
    /// state, such as filters or envelopes, then you should do so in that function instead.
    ///
    /// - If you need to access this information in your process function, then you can copy the
    ///   values to your plugin instance's object.
    /// - If the plugin is being restored from an old state,
    ///   then that state will have already been restored at this point.
    /// - If based on those parameters (or for any reason whatsoever) the plugin needs to introduce
    ///   latency, then you can do so here using the process context.
    /// - Depending on how the host restores plugin state, this function may be called multiple
    ///   times in rapid succession. It may thus be useful to check if the initialization work for
    ///   the current bufffer and audio IO configurations has already been performed first.
    /// - If the plugin fails to initialize for whatever reason, then this should return `false`.
    fn initialize(
        &mut self,
        audio_io_layout: &AudioIOLayout,
        buffer_config: &BufferConfig,
        context: &mut impl InitContext<Self>,
    ) -> bool {
        true
    }

    /// Clear internal state such as filters and envelopes. This is always called after
    /// [`initialize()`][Self::initialize()], and it may also be called at any other time from the
    /// audio thread. You should thus not do any allocations in this function.
    fn reset(&mut self) {}

    /// Process audio. The host's input buffers have already been copied to the output buffers if
    /// they are not processing audio in place (most hosts do however). All channels are also
    /// guaranteed to contain the same number of samples. Lastly, denormals have already been taken
    /// case of by NIH-plug, and you can optionally enable the `assert_process_allocs` feature to
    /// abort the program when any allocation occurs in the process function while running in debug
    /// mode.
    ///
    /// The framework provides convenient iterators on the [`Buffer`] object to process audio either
    /// either per-sample per-channel, or per-block per-channel per-sample. The first approach is
    /// preferred for plugins that don't require block-based processing because of their use of
    /// per-sample SIMD or excessive branching. The parameter smoothers can also work in both modes:
    /// use [`Smoother::next()`][crate::prelude::Smoother::next()] for per-sample processing, and
    /// [`Smoother::next_block()`][crate::prelude::Smoother::next_block()] for block-based
    /// processing.
    ///
    /// The `context` object contains context information as well as callbacks for working with note
    /// events. The [`AuxiliaryBuffers`] contain the plugin's sidechain input buffers and
    /// auxiliary output buffers if it has any.
    ///
    /// TODO: Provide a way to access auxiliary input channels if the IO configuration is
    ///       asymmetric
    fn process(
        &mut self,
        buffer: &mut Buffer,
        aux: &mut AuxiliaryBuffers,
        context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus;

    /// Called when the plugin is deactivated. The host will call
    /// [`initialize()`][Self::initialize()] again before the plugin resumes processing audio. These
    /// two functions will not be called when the host only temporarily stops processing audio. You
    /// can clean up or deallocate resources here. In most cases you can safely ignore this.
    ///
    /// There is no one-to-one relationship between calls to `initialize()` and `deactivate()`.
    /// `initialize()` may be called more than once before `deactivate()` is called, for instance
    /// when restoring state while the plugin is still activate.
    fn deactivate(&mut self) {}
}

/// Indicates the current situation after the plugin has processed audio.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessStatus {
    /// Something went wrong while processing audio.
    Error(&'static str),
    /// The plugin has finished processing audio. When the input is silent, the host may suspend the
    /// plugin to save resources as it sees fit.
    Normal,
    /// The plugin has a (reverb) tail with a specific length in samples.
    Tail(u32),
    /// This plugin will continue to produce sound regardless of whether or not the input is silent,
    /// and should thus not be deactivated by the host. This is essentially the same as having an
    /// infinite tail.
    KeepAlive,
}

```

[util.rs](../nih_plug_src/util.rs)

```rust
//! General conversion functions and utilities.

mod stft;
pub mod window;

pub use stft::StftHelper;

pub const MINUS_INFINITY_DB: f32 = -100.0;
pub const MINUS_INFINITY_GAIN: f32 = 1e-5; // 10f32.powf(MINUS_INFINITY_DB / 20)
pub const NOTES: [&str; 12] = [
    "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
];

/// Temporarily allow allocations within `func` if NIH-plug was configured with the
/// `assert_process_allocs` feature.
#[cfg(all(debug_assertions, feature = "assert_process_allocs"))]
pub fn permit_alloc<T, F: FnOnce() -> T>(func: F) -> T {
    assert_no_alloc::permit_alloc(func)
}

/// Temporarily allow allocations within `func` if NIH-plug was configured with the
/// `assert_process_allocs` feature.
#[cfg(not(all(debug_assertions, feature = "assert_process_allocs")))]
pub fn permit_alloc<T, F: FnOnce() -> T>(func: F) -> T {
    func()
}

/// Convert decibels to a voltage gain ratio, treating anything below -100 dB as minus infinity.
#[inline]
pub fn db_to_gain(dbs: f32) -> f32 {
    if dbs > MINUS_INFINITY_DB {
        10.0f32.powf(dbs * 0.05)
    } else {
        0.0
    }
}

/// Convert a voltage gain ratio to decibels. Gain ratios that aren't positive will be treated as
/// [`MINUS_INFINITY_DB`].
#[inline]
pub fn gain_to_db(gain: f32) -> f32 {
    f32::max(gain, MINUS_INFINITY_GAIN).log10() * 20.0
}

/// An approximation of [`db_to_gain()`] using `exp()`. Does not treat values below
/// [`MINUS_INFINITY_DB`] as 0.0 gain to avoid branching. As a result this function will thus also
/// never return 0.0 for normal input values. Will run faster on most architectures, but the result
/// may be slightly different.
#[inline]
pub fn db_to_gain_fast(dbs: f32) -> f32 {
    const CONVERSION_FACTOR: f32 = std::f32::consts::LN_10 / 20.0;
    (dbs * CONVERSION_FACTOR).exp()
}

/// [`db_to_gain_fast()`], but this version does truncate values below [`MINUS_INFINITY_DB`] to 0.0.
/// Bikeshedding over a better name is welcome.
#[inline]
pub fn db_to_gain_fast_branching(dbs: f32) -> f32 {
    if dbs > MINUS_INFINITY_DB {
        db_to_gain_fast(dbs)
    } else {
        0.0
    }
}

/// An approximation of [`gain_to_db()`] using `ln()`. Will run faster on most architectures, but
/// the result may be slightly different.
#[inline]
pub fn gain_to_db_fast(gain: f32) -> f32 {
    const CONVERSION_FACTOR: f32 = std::f32::consts::LOG10_E * 20.0;
    f32::max(gain, MINUS_INFINITY_GAIN).ln() * CONVERSION_FACTOR
}

/// [`db_to_gain_fast()`], but the minimum gain value is set to [`f32::EPSILON`]instead of
/// [`MINUS_INFINITY_GAIN`]. Useful in conjunction with [`db_to_gain_fast()`].
#[inline]
pub fn gain_to_db_fast_epsilon(gain: f32) -> f32 {
    const CONVERSION_FACTOR: f32 = std::f32::consts::LOG10_E * 20.0;
    f32::max(gain, MINUS_INFINITY_GAIN).ln() * CONVERSION_FACTOR
}

/// Convert a MIDI note ID to a frequency at A4 = 440 Hz equal temperament and middle C = note 60 =
/// C4.
#[inline]
pub fn midi_note_to_freq(note: u8) -> f32 {
    f32_midi_note_to_freq(note as f32)
}

/// The same as [`midi_note_to_freq()`], but for arbitrary note numbers including those outside of
/// the MIDI range. This also supports fractional note numbers, which is useful when working with
/// cents.
#[inline]
pub fn f32_midi_note_to_freq(note: f32) -> f32 {
    2.0f32.powf((note - 69.0) / 12.0) * 440.0
}

/// The inverse of [`f32_midi_note_to_freq()`]. This returns a fractional note number. Round to a
/// whole number, subtract that from the result, and multiply the fractional part by 100 to get the
/// number of cents.
#[inline]
pub fn freq_to_midi_note(freq: f32) -> f32 {
    ((freq / 440.0).log2() * 12.0) + 69.0
}

#[cfg(test)]
mod tests {
    mod db_gain_conversion {
        use super::super::*;

        #[test]
        fn test_db_to_gain_positive() {
            assert_eq!(db_to_gain(3.0), 1.4125376);
        }

        #[test]
        fn test_db_to_gain_negative() {
            assert_eq!(db_to_gain(-3.0), 1.4125376f32.recip());
        }

        #[test]
        fn test_db_to_gain_minus_infinity() {
            assert_eq!(db_to_gain(-100.0), 0.0);
        }

        #[test]
        fn test_gain_to_db_positive() {
            assert_eq!(gain_to_db(4.0), 12.041201);
        }

        #[test]
        fn test_gain_to_db_negative() {
            assert_eq!(gain_to_db(0.25), -12.041201);
        }

        #[test]
        fn test_gain_to_db_minus_infinity_zero() {
            assert_eq!(gain_to_db(0.0), MINUS_INFINITY_DB);
        }

        #[test]
        fn test_gain_to_db_minus_infinity_negative() {
            assert_eq!(gain_to_db(-2.0), MINUS_INFINITY_DB);
        }
    }

    mod fast_db_gain_conversion {
        use super::super::*;

        #[test]
        fn test_db_to_gain_positive() {
            approx::assert_relative_eq!(
                db_to_gain(3.0),
                db_to_gain_fast_branching(3.0),
                epsilon = 1e-7
            );
        }

        #[test]
        fn test_db_to_gain_negative() {
            approx::assert_relative_eq!(
                db_to_gain(-3.0),
                db_to_gain_fast_branching(-3.0),
                epsilon = 1e-7
            );
        }

        #[test]
        fn test_db_to_gain_minus_infinity() {
            approx::assert_relative_eq!(
                db_to_gain(-100.0),
                db_to_gain_fast_branching(-100.0),
                epsilon = 1e-7
            );
        }

        #[test]
        fn test_gain_to_db_positive() {
            approx::assert_relative_eq!(gain_to_db(4.0), gain_to_db_fast(4.0), epsilon = 1e-7);
        }

        #[test]
        fn test_gain_to_db_negative() {
            approx::assert_relative_eq!(gain_to_db(0.25), gain_to_db_fast(0.25), epsilon = 1e-7);
        }

        #[test]
        fn test_gain_to_db_minus_infinity_zero() {
            approx::assert_relative_eq!(gain_to_db(0.0), gain_to_db_fast(0.0), epsilon = 1e-7);
        }

        #[test]
        fn test_gain_to_db_minus_infinity_negative() {
            approx::assert_relative_eq!(gain_to_db(-2.0), gain_to_db_fast(-2.0), epsilon = 1e-7);
        }
    }
}

```


# 実装前に見てほしい nih_plug の example

[gain](../nih_plug_examples/gain/src/lib.rs)

```rust
use nih_plug::prelude::*;
use parking_lot::Mutex;
use std::sync::Arc;

struct Gain {
    params: Arc<GainParams>,
}

/// The [`Params`] derive macro gathers all of the information needed for the wrapper to know about
/// the plugin's parameters, persistent serializable fields, and nested parameter groups. You can
/// also easily implement [`Params`] by hand if you want to, for instance, have multiple instances
/// of a parameters struct for multiple identical oscillators/filters/envelopes.
#[derive(Params)]
struct GainParams {
    /// The parameter's ID is used to identify the parameter in the wrapped plugin API. As long as
    /// these IDs remain constant, you can rename and reorder these fields as you wish. The
    /// parameters are exposed to the host in the same order they were defined. In this case, this
    /// gain parameter is stored as linear gain while the values are displayed in decibels.
    #[id = "gain"]
    pub gain: FloatParam,

    /// This field isn't used in this example, but anything written to the vector would be restored
    /// together with a preset/state file saved for this plugin. This can be useful for storing
    /// things like sample data.
    #[persist = "industry_secrets"]
    pub random_data: Mutex<Vec<f32>>,

    /// You can also nest parameter structs. These will appear as a separate nested group if your
    /// DAW displays parameters in a tree structure.
    #[nested(group = "Subparameters")]
    pub sub_params: SubParams,

    /// Nested parameters also support some advanced functionality for reusing the same parameter
    /// struct multiple times.
    #[nested(array, group = "Array Parameters")]
    pub array_params: [ArrayParams; 3],
}

#[derive(Params)]
struct SubParams {
    #[id = "thing"]
    pub nested_parameter: FloatParam,
}

#[derive(Params)]
struct ArrayParams {
    /// This parameter's ID will get a `_1`, `_2`, and a `_3` suffix because of how it's used in
    /// `array_params` above.
    #[id = "noope"]
    pub nope: FloatParam,
}

impl Default for Gain {
    fn default() -> Self {
        Self {
            params: Arc::new(GainParams::default()),
        }
    }
}

impl Default for GainParams {
    fn default() -> Self {
        Self {
            // This gain is stored as linear gain. NIH-plug comes with useful conversion functions
            // to treat these kinds of parameters as if we were dealing with decibels. Storing this
            // as decibels is easier to work with, but requires a conversion for every sample.
            gain: FloatParam::new(
                "Gain",
                util::db_to_gain(0.0),
                FloatRange::Skewed {
                    min: util::db_to_gain(-30.0),
                    max: util::db_to_gain(30.0),
                    // This makes the range appear as if it was linear when displaying the values as
                    // decibels
                    factor: FloatRange::gain_skew_factor(-30.0, 30.0),
                },
            )
            // Because the gain parameter is stored as linear gain instead of storing the value as
            // decibels, we need logarithmic smoothing
            .with_smoother(SmoothingStyle::Logarithmic(50.0))
            .with_unit(" dB")
            // There are many predefined formatters we can use here. If the gain was stored as
            // decibels instead of as a linear gain value, we could have also used the
            // `.with_step_size(0.1)` function to get internal rounding.
            .with_value_to_string(formatters::v2s_f32_gain_to_db(2))
            .with_string_to_value(formatters::s2v_f32_gain_to_db()),
            // Persisted fields can be initialized like any other fields, and they'll keep their
            // values when restoring the plugin's state.
            random_data: Mutex::new(Vec::new()),
            sub_params: SubParams {
                nested_parameter: FloatParam::new(
                    "Unused Nested Parameter",
                    0.5,
                    FloatRange::Skewed {
                        min: 2.0,
                        max: 2.4,
                        factor: FloatRange::skew_factor(2.0),
                    },
                )
                .with_value_to_string(formatters::v2s_f32_rounded(2)),
            },
            array_params: [1, 2, 3].map(|index| ArrayParams {
                nope: FloatParam::new(
                    format!("Nope {index}"),
                    0.5,
                    FloatRange::Linear { min: 1.0, max: 2.0 },
                ),
            }),
        }
    }
}

impl Plugin for Gain {
    const NAME: &'static str = "Gain";
    const VENDOR: &'static str = "Moist Plugins GmbH";
    // You can use `env!("CARGO_PKG_HOMEPAGE")` to reference the homepage field from the
    // `Cargo.toml` file here
    const URL: &'static str = "https://youtu.be/dQw4w9WgXcQ";
    const EMAIL: &'static str = "info@example.com";

    const VERSION: &'static str = env!("CARGO_PKG_VERSION");

    // The first audio IO layout is used as the default. The other layouts may be selected either
    // explicitly or automatically by the host or the user depending on the plugin API/backend.
    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[
        AudioIOLayout {
            main_input_channels: NonZeroU32::new(2),
            main_output_channels: NonZeroU32::new(2),

            aux_input_ports: &[],
            aux_output_ports: &[],

            // Individual ports and the layout as a whole can be named here. By default these names
            // are generated as needed. This layout will be called 'Stereo', while the other one is
            // given the name 'Mono' based no the number of input and output channels.
            names: PortNames::const_default(),
        },
        AudioIOLayout {
            main_input_channels: NonZeroU32::new(1),
            main_output_channels: NonZeroU32::new(1),
            ..AudioIOLayout::const_default()
        },
    ];

    const MIDI_INPUT: MidiConfig = MidiConfig::None;
    // Setting this to `true` will tell the wrapper to split the buffer up into smaller blocks
    // whenever there are inter-buffer parameter changes. This way no changes to the plugin are
    // required to support sample accurate automation and the wrapper handles all of the boring
    // stuff like making sure transport and other timing information stays consistent between the
    // splits.
    const SAMPLE_ACCURATE_AUTOMATION: bool = true;

    // If the plugin can send or receive SysEx messages, it can define a type to wrap around those
    // messages here. The type implements the `SysExMessage` trait, which allows conversion to and
    // from plain byte buffers.
    type SysExMessage = ();
    // More advanced plugins can use this to run expensive background tasks. See the field's
    // documentation for more information. `()` means that the plugin does not have any background
    // tasks.
    type BackgroundTask = ();

    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }

    // This plugin doesn't need any special initialization, but if you need to do anything expensive
    // then this would be the place. State is kept around when the host reconfigures the
    // plugin. If we do need special initialization, we could implement the `initialize()` and/or
    // `reset()` methods

    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        _context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        for channel_samples in buffer.iter_samples() {
            // Smoothing is optionally built into the parameters themselves
            let gain = self.params.gain.smoothed.next();

            for sample in channel_samples {
                *sample *= gain;
            }
        }

        ProcessStatus::Normal
    }

    // This can be used for cleaning up special resources like socket connections whenever the
    // plugin is deactivated. Most plugins won't need to do anything here.
    fn deactivate(&mut self) {}
}

impl ClapPlugin for Gain {
    const CLAP_ID: &'static str = "com.moist-plugins-gmbh.gain";
    const CLAP_DESCRIPTION: Option<&'static str> = Some("A smoothed gain parameter example plugin");
    const CLAP_MANUAL_URL: Option<&'static str> = Some(Self::URL);
    const CLAP_SUPPORT_URL: Option<&'static str> = None;
    const CLAP_FEATURES: &'static [ClapFeature] = &[
        ClapFeature::AudioEffect,
        ClapFeature::Stereo,
        ClapFeature::Mono,
        ClapFeature::Utility,
    ];
}

impl Vst3Plugin for Gain {
    const VST3_CLASS_ID: [u8; 16] = *b"GainMoistestPlug";
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] =
        &[Vst3SubCategory::Fx, Vst3SubCategory::Tools];
}

nih_export_clap!(Gain);
nih_export_vst3!(Gain);

```

[sine](../nih_plug_examples/sine/src/lib.rs)

```rust
use nih_plug::prelude::*;
use std::f32::consts;
use std::sync::Arc;

/// A test tone generator that can either generate a sine wave based on the plugin's parameters or
/// based on the current MIDI input.
pub struct Sine {
    params: Arc<SineParams>,
    sample_rate: f32,

    /// The current phase of the sine wave, always kept between in `[0, 1]`.
    phase: f32,

    /// The MIDI note ID of the active note, if triggered by MIDI.
    midi_note_id: u8,
    /// The frequency if the active note, if triggered by MIDI.
    midi_note_freq: f32,
    /// A simple attack and release envelope to avoid clicks. Controlled through velocity and
    /// aftertouch.
    ///
    /// Smoothing is built into the parameters, but you can also use them manually if you need to
    /// smooth soemthing that isn't a parameter.
    midi_note_gain: Smoother<f32>,
}

#[derive(Params)]
struct SineParams {
    #[id = "gain"]
    pub gain: FloatParam,

    #[id = "freq"]
    pub frequency: FloatParam,

    #[id = "usemid"]
    pub use_midi: BoolParam,
}

impl Default for Sine {
    fn default() -> Self {
        Self {
            params: Arc::new(SineParams::default()),
            sample_rate: 1.0,

            phase: 0.0,

            midi_note_id: 0,
            midi_note_freq: 1.0,
            midi_note_gain: Smoother::new(SmoothingStyle::Linear(5.0)),
        }
    }
}

impl Default for SineParams {
    fn default() -> Self {
        Self {
            gain: FloatParam::new(
                "Gain",
                -10.0,
                FloatRange::Linear {
                    min: -30.0,
                    max: 0.0,
                },
            )
            .with_smoother(SmoothingStyle::Linear(3.0))
            .with_step_size(0.01)
            .with_unit(" dB"),
            frequency: FloatParam::new(
                "Frequency",
                420.0,
                FloatRange::Skewed {
                    min: 1.0,
                    max: 20_000.0,
                    factor: FloatRange::skew_factor(-2.0),
                },
            )
            .with_smoother(SmoothingStyle::Linear(10.0))
            // We purposely don't specify a step size here, but the parameter should still be
            // displayed as if it were rounded. This formatter also includes the unit.
            .with_value_to_string(formatters::v2s_f32_hz_then_khz(0))
            .with_string_to_value(formatters::s2v_f32_hz_then_khz()),
            use_midi: BoolParam::new("Use MIDI", false),
        }
    }
}

impl Sine {
    fn calculate_sine(&mut self, frequency: f32) -> f32 {
        let phase_delta = frequency / self.sample_rate;
        let sine = (self.phase * consts::TAU).sin();

        self.phase += phase_delta;
        if self.phase >= 1.0 {
            self.phase -= 1.0;
        }

        sine
    }
}

impl Plugin for Sine {
    const NAME: &'static str = "Sine Test Tone";
    const VENDOR: &'static str = "Moist Plugins GmbH";
    const URL: &'static str = "https://youtu.be/dQw4w9WgXcQ";
    const EMAIL: &'static str = "info@example.com";

    const VERSION: &'static str = env!("CARGO_PKG_VERSION");

    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[
        AudioIOLayout {
            // This is also the default and can be omitted here
            main_input_channels: None,
            main_output_channels: NonZeroU32::new(2),
            ..AudioIOLayout::const_default()
        },
        AudioIOLayout {
            main_input_channels: None,
            main_output_channels: NonZeroU32::new(1),
            ..AudioIOLayout::const_default()
        },
    ];

    const MIDI_INPUT: MidiConfig = MidiConfig::Basic;
    const SAMPLE_ACCURATE_AUTOMATION: bool = true;

    type SysExMessage = ();
    type BackgroundTask = ();

    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }

    fn initialize(
        &mut self,
        _audio_io_layout: &AudioIOLayout,
        buffer_config: &BufferConfig,
        _context: &mut impl InitContext<Self>,
    ) -> bool {
        self.sample_rate = buffer_config.sample_rate;

        true
    }

    fn reset(&mut self) {
        self.phase = 0.0;
        self.midi_note_id = 0;
        self.midi_note_freq = 1.0;
        self.midi_note_gain.reset(0.0);
    }

    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        let mut next_event = context.next_event();
        for (sample_id, channel_samples) in buffer.iter_samples().enumerate() {
            // Smoothing is optionally built into the parameters themselves
            let gain = self.params.gain.smoothed.next();

            // This plugin can be either triggered by MIDI or controleld by a parameter
            let sine = if self.params.use_midi.value() {
                // Act on the next MIDI event
                while let Some(event) = next_event {
                    if event.timing() > sample_id as u32 {
                        break;
                    }

                    match event {
                        NoteEvent::NoteOn { note, velocity, .. } => {
                            self.midi_note_id = note;
                            self.midi_note_freq = util::midi_note_to_freq(note);
                            self.midi_note_gain.set_target(self.sample_rate, velocity);
                        }
                        NoteEvent::NoteOff { note, .. } if note == self.midi_note_id => {
                            self.midi_note_gain.set_target(self.sample_rate, 0.0);
                        }
                        NoteEvent::PolyPressure { note, pressure, .. }
                            if note == self.midi_note_id =>
                        {
                            self.midi_note_gain.set_target(self.sample_rate, pressure);
                        }
                        _ => (),
                    }

                    next_event = context.next_event();
                }

                // This gain envelope prevents clicks with new notes and with released notes
                self.calculate_sine(self.midi_note_freq) * self.midi_note_gain.next()
            } else {
                let frequency = self.params.frequency.smoothed.next();
                self.calculate_sine(frequency)
            };

            for sample in channel_samples {
                *sample = sine * util::db_to_gain_fast(gain);
            }
        }

        ProcessStatus::KeepAlive
    }
}

impl ClapPlugin for Sine {
    const CLAP_ID: &'static str = "com.moist-plugins-gmbh.sine";
    const CLAP_DESCRIPTION: Option<&'static str> =
        Some("An optionally MIDI controlled sine test tone");
    const CLAP_MANUAL_URL: Option<&'static str> = Some(Self::URL);
    const CLAP_SUPPORT_URL: Option<&'static str> = None;
    const CLAP_FEATURES: &'static [ClapFeature] = &[
        ClapFeature::Instrument,
        ClapFeature::Synthesizer,
        ClapFeature::Stereo,
        ClapFeature::Mono,
        ClapFeature::Utility,
    ];
}

impl Vst3Plugin for Sine {
    const VST3_CLASS_ID: [u8; 16] = *b"SineMoistestPlug";
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] = &[
        Vst3SubCategory::Instrument,
        Vst3SubCategory::Synth,
        Vst3SubCategory::Tools,
    ];
}

nih_export_clap!(Sine);
nih_export_vst3!(Sine);

```

