#[cfg(target_os = "macos")]
mod platform {
    use std::ffi::{c_char, c_void, CStr};
    use std::ptr;
    use std::sync::mpsc::Sender;

    use anyhow::{anyhow, Result};

    #[derive(Debug, Clone)]
    pub struct SystemAudioFrame {
        pub data: Vec<f32>,
        pub sample_rate: f64,
    }

    type TapCallback = unsafe extern "C" fn(*const f32, usize, f64, u32, *mut c_void);

    extern "C" {
        fn system_audio_tap_start(
            preferred_sample_rate: f64,
            callback: TapCallback,
            user_data: *mut c_void,
            error_out: *mut *mut c_char,
            actual_sample_rate: *mut f64,
            actual_channels: *mut u32,
        ) -> *mut c_void;

        fn system_audio_tap_stop(handle: *mut c_void);

        fn system_audio_tap_free_error(ptr: *mut c_char);
    }

    struct CallbackContext {
        tx: Sender<SystemAudioFrame>,
    }

    pub struct SystemAudioCapture {
        handle: *mut c_void,
        context: *mut CallbackContext,
        sample_rate: f64,
    }

    unsafe extern "C" fn forward_samples(
        data: *const f32,
        frames: usize,
        sample_rate: f64,
        channels: u32,
        user_data: *mut c_void,
    ) {
        if data.is_null() || user_data.is_null() || frames == 0 {
            return;
        }

        let context = &*(user_data as *const CallbackContext);
        let slice = std::slice::from_raw_parts(data, frames * channels as usize);

        let mut buffer = Vec::with_capacity(frames);
        if channels <= 1 {
            buffer.extend_from_slice(slice);
        } else {
            for frame in slice.chunks(channels as usize) {
                let sum: f32 = frame.iter().copied().sum();
                buffer.push(sum / channels as f32);
            }
        }

        let frame = SystemAudioFrame {
            data: buffer,
            sample_rate,
        };

        let _ = context.tx.send(frame);
    }

    impl SystemAudioCapture {
        pub fn start(
            preferred_sample_rate: Option<f64>,
            tx: Sender<SystemAudioFrame>,
        ) -> Result<Self> {
            let context = Box::new(CallbackContext { tx });
            let context_ptr = Box::into_raw(context);
            let mut error_ptr: *mut c_char = ptr::null_mut();
            let mut out_sample_rate: f64 = preferred_sample_rate.unwrap_or(0.0);
            let mut out_channels: u32 = 0;

            let handle = unsafe {
                system_audio_tap_start(
                    preferred_sample_rate.unwrap_or(0.0),
                    forward_samples,
                    context_ptr.cast::<c_void>(),
                    &mut error_ptr,
                    &mut out_sample_rate,
                    &mut out_channels,
                )
            };

            if handle.is_null() {
                let message = if !error_ptr.is_null() {
                    let c_str = unsafe { CStr::from_ptr(error_ptr) };
                    let text = c_str.to_string_lossy().into_owned();
                    unsafe { system_audio_tap_free_error(error_ptr) };
                    text
                } else {
                    "Failed to start system audio capture".to_string()
                };

                unsafe {
                    drop(Box::from_raw(context_ptr));
                }

                return Err(anyhow!(message));
            }

            Ok(Self {
                handle,
                context: context_ptr,
                sample_rate: out_sample_rate,
            })
        }

        pub fn sample_rate(&self) -> f64 {
            self.sample_rate
        }

        pub fn stop(&mut self) {
            if !self.handle.is_null() {
                unsafe { system_audio_tap_stop(self.handle) };
                self.handle = ptr::null_mut();
            }

            if !self.context.is_null() {
                unsafe {
                    drop(Box::from_raw(self.context));
                }
                self.context = ptr::null_mut();
            }
        }
    }

    impl Drop for SystemAudioCapture {
        fn drop(&mut self) {
            self.stop();
        }
    }

    unsafe impl Send for SystemAudioCapture {}
    unsafe impl Sync for SystemAudioCapture {}

    pub use SystemAudioFrame as Frame;
}

#[cfg(not(target_os = "macos"))]
mod platform {
    use std::sync::mpsc::Sender;

    use anyhow::{anyhow, Result};

    #[derive(Debug, Clone)]
    pub struct SystemAudioFrame {
        pub data: Vec<f32>,
        pub sample_rate: f64,
    }

    pub struct SystemAudioCapture;

    impl SystemAudioCapture {
        pub fn start(
            _preferred_sample_rate: Option<f64>,
            _tx: Sender<SystemAudioFrame>,
        ) -> Result<Self> {
            Err(anyhow!("System audio capture is only available on macOS"))
        }

        pub fn sample_rate(&self) -> f64 {
            0.0
        }

        pub fn channels(&self) -> u32 {
            0
        }

        pub fn stop(&mut self) {}
    }

    pub use SystemAudioFrame as Frame;
}

pub use platform::{Frame as SystemAudioFrame, SystemAudioCapture};
