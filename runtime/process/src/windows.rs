//! A program's tree on Windows (decision 0105): a job object that ends
//! everything in it when it closes. The program starts suspended and
//! joins the job before it runs, so nothing it starts can leave. The job
//! counts processor time and peak committed memory for the whole tree.
#![allow(unsafe_code)]

use super::Exit;
use std::os::windows::io::AsRawHandle;
use std::os::windows::process::CommandExt;
use std::process::{Child, Command};
use windows_sys::Win32::Foundation::{CloseHandle, HANDLE, INVALID_HANDLE_VALUE};
use windows_sys::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, TH32CS_SNAPTHREAD, THREADENTRY32, Thread32First, Thread32Next,
};
use windows_sys::Win32::System::JobObjects::{
    AssignProcessToJobObject, CreateJobObjectW, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
    JOBOBJECT_BASIC_ACCOUNTING_INFORMATION, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
    JobObjectBasicAccountingInformation, JobObjectExtendedLimitInformation,
    QueryInformationJobObject, SetInformationJobObject, TerminateJobObject,
};
use windows_sys::Win32::System::ProcessStatus::{GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS};
use windows_sys::Win32::System::Threading::{
    CREATE_SUSPENDED, GetCurrentProcess, OpenThread, ResumeThread, THREAD_SUSPEND_RESUME,
};

/// The job holding a program's tree, by its handle's number.
pub struct Tree {
    job: isize,
}

impl Tree {
    fn handle(&self) -> HANDLE {
        self.job as HANDLE
    }

    /// Ends every program in the job.
    pub fn kill(&self) {
        // SAFETY: the job handle is open until `drop`, and ending a job's
        // programs touches no memory of ours.
        unsafe {
            TerminateJobObject(self.handle(), 1);
        }
    }

    /// One piece of the job's information, of type `T`.
    fn query<T>(&self, class: i32) -> Option<T> {
        // SAFETY: `T` is a plain C struct of the size given, which the
        // system fills; a zeroed one is valid before it does.
        unsafe {
            let mut info: T = std::mem::zeroed();
            let size = std::mem::size_of::<T>() as u32;
            let ptr = std::ptr::from_mut(&mut info).cast();
            let ok =
                QueryInformationJobObject(self.handle(), class, ptr, size, std::ptr::null_mut());
            (ok != 0).then_some(info)
        }
    }
}

impl Drop for Tree {
    fn drop(&mut self) {
        // SAFETY: the handle is ours and closed once; closing it ends
        // whatever is left in the job (KILL_ON_JOB_CLOSE).
        unsafe {
            CloseHandle(self.handle());
        }
    }
}

/// A new job that ends its programs when its last handle closes.
fn new_job() -> std::io::Result<Tree> {
    // SAFETY: no name and no security attributes; the handle is checked.
    let job = unsafe { CreateJobObjectW(std::ptr::null(), std::ptr::null()) };
    if job.is_null() {
        return Err(std::io::Error::last_os_error());
    }
    let tree = Tree { job: job as isize };
    // SAFETY: a zeroed limit structure is valid; only the flag is set,
    // and its size is given.
    let ok = unsafe {
        let mut limits: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = std::mem::zeroed();
        limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
        SetInformationJobObject(
            job,
            JobObjectExtendedLimitInformation,
            std::ptr::from_ref(&limits).cast(),
            std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
        )
    };
    if ok == 0 {
        return Err(std::io::Error::last_os_error());
    }
    Ok(tree)
}

/// Starts `command` suspended, puts it in a new job, then lets it run.
pub fn spawn(command: &mut Command) -> std::io::Result<(Child, Tree)> {
    let tree = new_job()?;
    let mut child = command.creation_flags(CREATE_SUSPENDED).spawn()?;
    // SAFETY: both handles are open: the job's is ours, the child's is
    // held by `child` until it is dropped.
    let joined =
        unsafe { AssignProcessToJobObject(tree.handle(), child.as_raw_handle() as HANDLE) };
    if joined == 0 {
        let e = std::io::Error::last_os_error();
        let _ = child.kill();
        return Err(e);
    }
    resume(child.id());
    Ok((child, tree))
}

/// Resumes every thread of process `pid` (it has one: it never ran).
fn resume(pid: u32) {
    // SAFETY: the snapshot and each thread handle are checked and closed;
    // the entry's size is set before the system fills it.
    unsafe {
        let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPTHREAD, 0);
        if snapshot == INVALID_HANDLE_VALUE {
            return;
        }
        let mut entry: THREADENTRY32 = std::mem::zeroed();
        entry.dwSize = std::mem::size_of::<THREADENTRY32>() as u32;
        let mut more = Thread32First(snapshot, &mut entry) != 0;
        while more {
            if entry.th32OwnerProcessID == pid {
                let thread = OpenThread(THREAD_SUSPEND_RESUME, 0, entry.th32ThreadID);
                if !thread.is_null() {
                    ResumeThread(thread);
                    CloseHandle(thread);
                }
            }
            more = Thread32Next(snapshot, &mut entry) != 0;
        }
        CloseHandle(snapshot);
    }
}

/// Waits for the program's end; the job gives what its tree cost.
pub fn wait(mut child: Child, tree: &Tree) -> Exit {
    let code = child.wait().ok().and_then(|s| s.code()).unwrap_or(-1);
    let basic: Option<JOBOBJECT_BASIC_ACCOUNTING_INFORMATION> =
        tree.query(JobObjectBasicAccountingInformation);
    let limits: Option<JOBOBJECT_EXTENDED_LIMIT_INFORMATION> =
        tree.query(JobObjectExtendedLimitInformation);
    Exit {
        code: i64::from(code),
        // In units of 100 nanoseconds.
        cpu_ms: basic.map_or(0, |b| (b.TotalUserTime + b.TotalKernelTime) / 10_000),
        peak_bytes: limits.map_or(0, |l| l.PeakJobMemoryUsed as i64),
    }
}

/// This program's largest working set so far.
pub fn peak_self() -> i64 {
    // SAFETY: the counters are a plain struct whose size is given; the
    // current process's pseudo-handle needs no closing.
    unsafe {
        let mut counters: PROCESS_MEMORY_COUNTERS = std::mem::zeroed();
        let size = std::mem::size_of::<PROCESS_MEMORY_COUNTERS>() as u32;
        if GetProcessMemoryInfo(GetCurrentProcess(), &mut counters, size) == 0 {
            return 0;
        }
        counters.PeakWorkingSetSize as i64
    }
}
