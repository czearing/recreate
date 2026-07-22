#[cfg(windows)]
mod platform {
    use std::{mem::size_of, ptr::null_mut};
    use windows_sys::Win32::{
        Foundation::{CloseHandle, HANDLE},
        System::{
            JobObjects::{
                AssignProcessToJobObject, CreateJobObjectW, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
                JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JobObjectExtendedLimitInformation,
                SetInformationJobObject,
            },
            Threading::{OpenProcess, PROCESS_SET_QUOTA, PROCESS_TERMINATE},
        },
    };

    pub struct ProcessJob(HANDLE);

    impl ProcessJob {
        pub fn attach(process_id: u32) -> anyhow::Result<Self> {
            unsafe {
                let job = CreateJobObjectW(null_mut(), null_mut());
                anyhow::ensure!(!job.is_null(), "create browser job object");
                let mut information: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = std::mem::zeroed();
                information.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
                let configured = SetInformationJobObject(
                    job,
                    JobObjectExtendedLimitInformation,
                    &mut information as *mut _ as *mut _,
                    size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
                );
                if configured == 0 {
                    CloseHandle(job);
                    anyhow::bail!("configure browser job object");
                }
                let process = OpenProcess(PROCESS_SET_QUOTA | PROCESS_TERMINATE, 0, process_id);
                if process.is_null() || AssignProcessToJobObject(job, process) == 0 {
                    if !process.is_null() {
                        CloseHandle(process);
                    }
                    CloseHandle(job);
                    anyhow::bail!("assign browser process to job object");
                }
                CloseHandle(process);
                Ok(Self(job))
            }
        }
    }

    impl Drop for ProcessJob {
        fn drop(&mut self) {
            unsafe {
                CloseHandle(self.0);
            }
        }
    }
}

#[cfg(not(windows))]
mod platform {
    pub struct ProcessJob;

    impl ProcessJob {
        pub fn attach(_: u32) -> anyhow::Result<Self> {
            Ok(Self)
        }
    }
}

pub use platform::ProcessJob;
