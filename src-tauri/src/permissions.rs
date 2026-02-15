use tauri::App;

#[allow(unused_variables)]
pub fn initialize(app: &App) {
    #[cfg(target_os = "macos")]
    macos::initialize(app);
}

#[cfg(target_os = "macos")]
mod macos {
    use anyhow::{anyhow, Context, Result};
    use block2::StackBlock;
    use objc2::runtime::{AnyClass, Bool};
    use objc2_foundation::NSString;
    use std::ffi::CStr;
    use std::time::Duration;
    use tauri::App;
    use tauri::Emitter;
    use tokio::time::sleep;

    const PRIVACY_URL: &str =
        "x-apple.systempreferences:com.apple.preference.security?Privacy_Microphone";

    const AUTH_STATUS_NOT_DETERMINED: i32 = 0;
    const AUTH_STATUS_RESTRICTED: i32 = 1;
    const AUTH_STATUS_DENIED: i32 = 2;
    const AUTH_STATUS_AUTHORIZED: i32 = 3;

    #[link(name = "AVFoundation", kind = "framework")]
    extern "C" {
        static AVMediaTypeAudio: *const NSString;
    }

    static AVCAPTURE_DEVICE_CLASS: &CStr = c"AVCaptureDevice";

    pub fn initialize(app: &App) {
        let handle = app.handle().clone();
        tauri::async_runtime::spawn(async move {
            if let Err(error) = ensure_permission(handle.clone()).await {
                tracing::warn!(?error, "mic permission check failed");
            }
        });
    }

    async fn ensure_permission(app: tauri::AppHandle) -> Result<()> {
        let mut status = unsafe { authorization_status()? };

        if status == AUTH_STATUS_NOT_DETERMINED {
            unsafe {
                request_access().context("failed to request microphone access")?;
            }

            for _ in 0..20 {
                sleep(Duration::from_millis(200)).await;
                status = unsafe { authorization_status()? };
                if status != AUTH_STATUS_NOT_DETERMINED {
                    break;
                }
            }
        }

        match status {
            AUTH_STATUS_AUTHORIZED => Ok(()),
            AUTH_STATUS_DENIED | AUTH_STATUS_RESTRICTED => {
                open_privacy_settings().context("failed to open privacy settings")?;
                let _ = app.emit(
                    "permissions:microphone",
                    &serde_json::json!({
                        "status": "denied",
                    }),
                );
                Ok(())
            }
            _ => Ok(()),
        }
    }

    unsafe fn authorization_status() -> Result<i32> {
        let cls: &'static AnyClass = AnyClass::get(AVCAPTURE_DEVICE_CLASS)
            .ok_or_else(|| anyhow!("AVCaptureDevice not available"))?;
        let status: i32 = objc2::msg_send![cls, authorizationStatusForMediaType: AVMediaTypeAudio];
        Ok(status)
    }

    unsafe fn request_access() -> Result<()> {
        let cls: &'static AnyClass = AnyClass::get(AVCAPTURE_DEVICE_CLASS)
            .ok_or_else(|| anyhow!("AVCaptureDevice not available"))?;
        let block = StackBlock::new(|granted: Bool| {
            if granted == Bool::YES {
                tracing::info!("microphone access granted by user");
            } else {
                tracing::warn!("microphone access denied by user");
            }
        })
        .copy();

        let _: () = objc2::msg_send![cls, requestAccessForMediaType: AVMediaTypeAudio, completionHandler: &*block];
        Ok(())
    }

    fn open_privacy_settings() -> Result<()> {
        let status = std::process::Command::new("open")
            .arg(PRIVACY_URL)
            .status()
            .context("failed to run open command")?;

        if status.success() {
            Ok(())
        } else {
            Err(anyhow!("open command exited with status {status}"))
        }
    }
}
