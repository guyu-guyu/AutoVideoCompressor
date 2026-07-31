//! Native Windows Task Scheduler backend implemented with the Task Scheduler
//! COM API. Each configured directory maps to one daily task inside the
//! `\AutoVideoCompressor\` task folder.

use crate::config::directory_config::DirectoryConfig;
use crate::config::global_config::DirEntry;
use chrono::Local;
use std::path::PathBuf;
use windows::core::{Interface, BSTR};
use windows::Win32::Foundation::VARIANT_BOOL;
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CoUninitialize, CLSCTX_INPROC_SERVER,
    COINIT_APARTMENTTHREADED,
};
use windows::Win32::System::TaskScheduler::{
    IDailyTrigger, IExecAction, ITaskFolder, ITaskService, TaskScheduler, TASK_ACTION_EXEC,
    TASK_CREATE_OR_UPDATE, TASK_ENUM_HIDDEN, TASK_LOGON_INTERACTIVE_TOKEN, TASK_TRIGGER_DAILY,
};
use windows::Win32::System::Variant::VARIANT;

const TASK_FOLDER_NAME: &str = "AutoVideoCompressor";
const TASK_FOLDER_PATH: &str = r"\AutoVideoCompressor";
const LEGACY_TASK_PREFIX: &str = "AutoVideoCompressor-";
const RPC_E_CHANGED_MODE: i32 = 0x80010106u32 as i32;
const HRESULT_FILE_NOT_FOUND: i32 = 0x80070002u32 as i32;

pub struct WindowsTaskScheduler {
    app_exe: PathBuf,
}

struct ComGuard {
    should_uninitialize: bool,
}

impl ComGuard {
    fn initialize() -> Result<Self, String> {
        let result = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) };
        if result.is_ok() {
            return Ok(Self {
                should_uninitialize: true,
            });
        }
        if result.0 == RPC_E_CHANGED_MODE {
            // Tauri may already have initialized this thread with another COM
            // apartment model. COM is still available; it must not be uninitialized here.
            return Ok(Self {
                should_uninitialize: false,
            });
        }
        Err(format!("无法初始化 Windows COM: {result:?}"))
    }
}

impl Drop for ComGuard {
    fn drop(&mut self) {
        if self.should_uninitialize {
            unsafe { CoUninitialize() };
        }
    }
}

impl WindowsTaskScheduler {
    pub fn new() -> Self {
        let app_exe =
            std::env::current_exe().unwrap_or_else(|_| PathBuf::from("autovideocompressor.exe"));
        Self { app_exe }
    }

    /// Converts a configured directory to the task name stored inside the
    /// dedicated application task folder.
    pub fn task_name(dir: &str) -> String {
        dir.replace('\\', "/")
            .replace(":/", "-")
            .replace('/', "-")
            .replace(':', "-")
    }

    pub fn task_path(dir: &str) -> String {
        format!(r"{}\{}", TASK_FOLDER_PATH, Self::task_name(dir))
    }

    fn legacy_task_name(dir: &str) -> String {
        format!("{LEGACY_TASK_PREFIX}{}", Self::task_name(dir))
    }

    fn with_service<T>(
        operation: impl FnOnce(&ITaskService) -> windows::core::Result<T>,
    ) -> Result<T, String> {
        let _com = ComGuard::initialize()?;
        let result = (|| -> windows::core::Result<T> {
            unsafe {
                let service: ITaskService =
                    CoCreateInstance(&TaskScheduler, None, CLSCTX_INPROC_SERVER)?;
                let empty = VARIANT::default();
                service.Connect(&empty, &empty, &empty, &empty)?;
                operation(&service)
            }
        })();
        result.map_err(|error| format!("Windows 任务计划程序操作失败: {error}"))
    }

    unsafe fn get_or_create_task_folder(
        service: &ITaskService,
    ) -> windows::core::Result<ITaskFolder> {
        let path = BSTR::from(TASK_FOLDER_PATH);
        match service.GetFolder(&path) {
            Ok(folder) => Ok(folder),
            Err(error) if is_not_found(&error) => {
                let root = service.GetFolder(&BSTR::from(r"\"))?;
                let empty = VARIANT::default();
                match root.CreateFolder(&BSTR::from(TASK_FOLDER_NAME), &empty) {
                    Ok(folder) => Ok(folder),
                    // Another synchronization may have created it concurrently.
                    Err(_) => service.GetFolder(&path),
                }
            }
            Err(error) => Err(error),
        }
    }

    fn add_daily_task(&self, dir: &str, time: &str, wake_to_run: bool) -> Result<(), String> {
        validate_time(time)?;

        let task_name = Self::task_name(dir);
        let legacy_task_name = Self::legacy_task_name(dir);
        let app_path = self.app_exe.to_string_lossy().to_string();
        let arguments = self.scheduled_arguments(dir);
        let start_boundary = format!("{}T{time}:00", Local::now().format("%Y-%m-%d"));

        Self::with_service(|service| unsafe {
            let folder = Self::get_or_create_task_folder(service)?;
            let definition = service.NewTask(0)?;

            let registration = definition.RegistrationInfo()?;
            registration.SetAuthor(&BSTR::from("AutoVideoCompressor"))?;
            registration.SetDescription(&BSTR::from(format!("按计划压缩目录: {dir}")))?;

            let settings = definition.Settings()?;
            settings.SetWakeToRun(VARIANT_BOOL::from(wake_to_run))?;
            settings.SetStartWhenAvailable(VARIANT_BOOL::from(false))?;

            let principal = definition.Principal()?;
            principal.SetLogonType(TASK_LOGON_INTERACTIVE_TOKEN)?;

            let trigger: IDailyTrigger =
                definition.Triggers()?.Create(TASK_TRIGGER_DAILY)?.cast()?;
            trigger.SetStartBoundary(&BSTR::from(start_boundary))?;
            trigger.SetDaysInterval(1)?;

            let action: IExecAction = definition.Actions()?.Create(TASK_ACTION_EXEC)?.cast()?;
            action.SetPath(&BSTR::from(app_path))?;
            action.SetArguments(&BSTR::from(arguments))?;

            let empty = VARIANT::default();
            folder.RegisterTaskDefinition(
                &BSTR::from(task_name),
                &definition,
                TASK_CREATE_OR_UPDATE.0,
                &empty,
                &empty,
                TASK_LOGON_INTERACTIVE_TOKEN,
                &empty,
            )?;

            // Migrate the known pre-0.4 root-level task only after the new task
            // has been registered successfully.
            let root = service.GetFolder(&BSTR::from(r"\"))?;
            delete_task_if_exists(&root, &legacy_task_name)?;
            Ok(())
        })
    }

    fn scheduled_arguments(&self, dir: &str) -> String {
        format!("--scheduled --directory {}", quote_windows_arg(dir))
    }

    /// Removes both the current folder task and the legacy root-level task.
    /// Missing tasks are treated as success.
    pub fn remove_task(&self, dir: &str) -> Result<(), String> {
        let task_name = Self::task_name(dir);
        let legacy_task_name = Self::legacy_task_name(dir);
        Self::with_service(|service| unsafe {
            match service.GetFolder(&BSTR::from(TASK_FOLDER_PATH)) {
                Ok(folder) => delete_task_if_exists(&folder, &task_name)?,
                Err(error) if is_not_found(&error) => {}
                Err(error) => return Err(error),
            }

            let root = service.GetFolder(&BSTR::from(r"\"))?;
            delete_task_if_exists(&root, &legacy_task_name)?;
            cleanup_empty_task_folder(service);
            Ok(())
        })
    }

    pub fn sync_directory(
        &self,
        dir: &str,
        schedule_time: Option<&str>,
        enabled: bool,
        wake_to_run: bool,
    ) -> Result<(), String> {
        match (schedule_time, enabled) {
            (Some(time), true) if !time.is_empty() => self.add_daily_task(dir, time, wake_to_run),
            _ => self.remove_task(dir),
        }
    }

    pub fn sync_all(&self, directories: &[DirEntry], wake_to_run: bool) {
        for directory in directories {
            let config = DirectoryConfig::load(&directory.path);
            if let Err(error) = self.sync_directory(
                &directory.path,
                config.schedule_time.as_deref(),
                directory.enabled,
                wake_to_run,
            ) {
                eprintln!(
                    "[windows_task_scheduler] 同步目录失败 '{}': {error}",
                    directory.path
                );
            }
        }
    }

    pub fn remove_all(&self, directories: &[DirEntry]) {
        for directory in directories {
            if let Err(error) = self.remove_task(&directory.path) {
                eprintln!(
                    "[windows_task_scheduler] 删除目录任务失败 '{}': {error}",
                    directory.path
                );
            }
        }
    }

    pub fn run_task(&self, dir: &str) -> Result<(), String> {
        let task_name = Self::task_name(dir);
        let legacy_task_name = Self::legacy_task_name(dir);
        Self::with_service(|service| unsafe {
            let empty = VARIANT::default();
            match service.GetFolder(&BSTR::from(TASK_FOLDER_PATH)) {
                Ok(folder) => match folder.GetTask(&BSTR::from(&task_name)) {
                    Ok(task) => {
                        task.Run(&empty)?;
                        return Ok(());
                    }
                    Err(error) if is_not_found(&error) => {}
                    Err(error) => return Err(error),
                },
                Err(error) if is_not_found(&error) => {}
                Err(error) => return Err(error),
            }

            // Compatibility fallback for a task that has not yet been migrated.
            let root = service.GetFolder(&BSTR::from(r"\"))?;
            root.GetTask(&BSTR::from(legacy_task_name))?.Run(&empty)?;
            Ok(())
        })
    }
}

impl Default for WindowsTaskScheduler {
    fn default() -> Self {
        Self::new()
    }
}

unsafe fn delete_task_if_exists(
    folder: &ITaskFolder,
    task_name: &str,
) -> windows::core::Result<()> {
    match folder.GetTask(&BSTR::from(task_name)) {
        Ok(_) => folder.DeleteTask(&BSTR::from(task_name), 0),
        Err(error) if is_not_found(&error) => Ok(()),
        Err(error) => Err(error),
    }
}

unsafe fn cleanup_empty_task_folder(service: &ITaskService) {
    let Ok(folder) = service.GetFolder(&BSTR::from(TASK_FOLDER_PATH)) else {
        return;
    };
    let is_empty = folder
        .GetTasks(TASK_ENUM_HIDDEN.0)
        .and_then(|tasks| tasks.Count())
        .map(|count| count == 0)
        .unwrap_or(false);
    if is_empty {
        if let Ok(root) = service.GetFolder(&BSTR::from(r"\")) {
            let _ = root.DeleteFolder(&BSTR::from(TASK_FOLDER_NAME), 0);
        }
    }
}

fn is_not_found(error: &windows::core::Error) -> bool {
    error.code().0 == HRESULT_FILE_NOT_FOUND
}

fn validate_time(time: &str) -> Result<(), String> {
    let Some((hour, minute)) = time.split_once(':') else {
        return Err(format!("无效的计划任务时间: {time}"));
    };
    if hour.len() != 2 || minute.len() != 2 {
        return Err(format!("无效的计划任务时间: {time}"));
    }
    let hour = hour
        .parse::<u8>()
        .map_err(|_| format!("无效的计划任务时间: {time}"))?;
    let minute = minute
        .parse::<u8>()
        .map_err(|_| format!("无效的计划任务时间: {time}"))?;
    if hour > 23 || minute > 59 {
        return Err(format!("无效的计划任务时间: {time}"));
    }
    Ok(())
}

fn quote_windows_arg(arg: &str) -> String {
    if !arg.is_empty() && !arg.chars().any(|c| c.is_whitespace() || c == '"') {
        return arg.to_string();
    }

    let mut quoted = String::from("\"");
    let mut backslashes = 0;
    for ch in arg.chars() {
        match ch {
            '\\' => backslashes += 1,
            '"' => {
                quoted.push_str(&"\\".repeat(backslashes * 2 + 1));
                quoted.push('"');
                backslashes = 0;
            }
            _ => {
                quoted.push_str(&"\\".repeat(backslashes));
                backslashes = 0;
                quoted.push(ch);
            }
        }
    }
    quoted.push_str(&"\\".repeat(backslashes * 2));
    quoted.push('"');
    quoted
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn task_name_and_path_use_dedicated_folder() {
        assert_eq!(
            WindowsTaskScheduler::task_name(r"D:\Videos\Movies"),
            "D-Videos-Movies"
        );
        assert_eq!(
            WindowsTaskScheduler::task_path(r"D:\Videos\Movies"),
            r"\AutoVideoCompressor\D-Videos-Movies"
        );
    }

    #[test]
    fn legacy_task_name_preserves_migration_target() {
        assert_eq!(
            WindowsTaskScheduler::legacy_task_name(r"E:\My Videos"),
            "AutoVideoCompressor-E-My Videos"
        );
    }

    #[test]
    fn validates_daily_time() {
        assert!(validate_time("00:00").is_ok());
        assert!(validate_time("23:59").is_ok());
        assert!(validate_time("24:00").is_err());
        assert!(validate_time("9:30").is_err());
    }

    #[test]
    fn quotes_windows_action_arguments() {
        assert_eq!(quote_windows_arg(r"D:\Videos"), r"D:\Videos");
        assert_eq!(quote_windows_arg(r"D:\My Videos"), r#""D:\My Videos""#);
        assert_eq!(quote_windows_arg(""), r#""""#);
    }

    #[test]
    fn scheduled_arguments_target_exactly_one_directory() {
        let scheduler = WindowsTaskScheduler {
            app_exe: PathBuf::from(r"C:\Program Files\AutoVideoCompressor.exe"),
        };
        assert_eq!(
            scheduler.scheduled_arguments(r"D:\My Videos"),
            r#"--scheduled --directory "D:\My Videos""#
        );
    }
}
