use windows::Win32::{
    Foundation::{VARIANT_FALSE, VARIANT_TRUE},
    System::TaskScheduler::{
        IExecAction2, ITaskFolder, ITaskService, TASK_ACTION_EXEC, TASK_CREATE_OR_UPDATE,
        TASK_LOGON_INTERACTIVE_TOKEN, TASK_RUNLEVEL_HIGHEST, TASK_TRIGGER_LOGON, TaskScheduler,
    },
    UI::Shell::FOLDERID_LocalAppData,
};
use windows_core::{BSTR, Interface};
use winreg::{RegKey, enums::HKEY_CURRENT_USER};

use crate::{
    enviroment::was_installed_using_msix,
    error::Result,
    windows_api::{WindowsApi, com::Com},
};

pub struct TaskSchedulerHelper {}

static GROUP_FOLDER: &str = "\\KestrelUI";
static OLD_APP_TASK_NAME: &str = "KestrelUI-legacy";
static SERVICE_TASK_NAME: &str = "KestrelUI Service";

// fallback for when the logon task can't be registered (no admin rights). The value name
// matches the one the uninstaller removes
static RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
static RUN_VALUE: &str = "KestrelUI";
static PREF_KEY: &str = r"Software\KestrelUI";
static PREF_VALUE: &str = "Autostart";

fn service_exe() -> Result<std::path::PathBuf> {
    if was_installed_using_msix() {
        Ok(WindowsApi::known_folder(FOLDERID_LocalAppData)?
            .join("Microsoft\\WindowsApps\\kestrelui-service.exe"))
    } else {
        Ok(std::env::current_exe()?)
    }
}

/// what the user picked in settings, on by default
pub fn autostart_wanted() -> bool {
    RegKey::predef(HKEY_CURRENT_USER)
        .open_subkey(PREF_KEY)
        .and_then(|key| key.get_value::<u32, _>(PREF_VALUE))
        .map(|v| v != 0)
        .unwrap_or(true)
}

fn save_autostart_wanted(enabled: bool) -> Result<()> {
    let (key, _) = RegKey::predef(HKEY_CURRENT_USER).create_subkey(PREF_KEY)?;
    key.set_value(PREF_VALUE, &(enabled as u32))?;
    Ok(())
}

impl TaskSchedulerHelper {
    unsafe fn get_task_service() -> Result<ITaskService> {
        unsafe {
            let task_service: ITaskService = Com::create_instance(&TaskScheduler)?;
            task_service.Connect(
                &Default::default(),
                &Default::default(),
                &Default::default(),
                &Default::default(),
            )?;
            Ok(task_service)
        }
    }

    unsafe fn register_task(folder: &ITaskFolder, task_name: &str, task_xml: &BSTR) -> Result<()> {
        unsafe {
            folder.RegisterTask(
                &task_name.into(),
                task_xml,
                TASK_CREATE_OR_UPDATE.0,
                &Default::default(),
                &Default::default(),
                TASK_LOGON_INTERACTIVE_TOKEN,
                &Default::default(),
            )?;
            log::debug!("Registered task {}", task_name);
            Ok(())
        }
    }

    /// this task handles the startup of the service and the app on login
    pub fn create_service_task() -> Result<()> {
        log::debug!("Creating service task...");
        let service_path = service_exe()?;
        Com::run_with_context(|| unsafe {
            let task_service = Self::get_task_service()?;
            // the task is re-registered on every elevated start (the exe may have moved), keep
            // the triggers of the existing one so a disabled autostart stays disabled
            let mut old_task = None;
            if let Ok(seelen_folder) = task_service.GetFolder(&GROUP_FOLDER.into()) {
                let _ = seelen_folder.DeleteTask(&OLD_APP_TASK_NAME.into(), 0);
                old_task = seelen_folder.GetTask(&SERVICE_TASK_NAME.into()).ok();
            };
            let root_folder = task_service.GetFolder(&"\\".into())?;

            let task = task_service.NewTask(0)?;
            task.Principal()?.SetRunLevel(TASK_RUNLEVEL_HIGHEST)?;

            let settings = task.Settings()?;
            settings.SetPriority(2)?;
            settings.SetHidden(VARIANT_TRUE)?;
            settings.SetAllowDemandStart(VARIANT_TRUE)?;
            settings.SetDisallowStartIfOnBatteries(VARIANT_FALSE)?;
            settings.SetStopIfGoingOnBatteries(VARIANT_FALSE)?;

            let triggers = task.Triggers()?;
            if let Some(old) = old_task {
                let old_triggers = old.Definition()?.Triggers()?;
                task.SetTriggers(&old_triggers)?;
            } else if autostart_wanted() {
                triggers.Create(TASK_TRIGGER_LOGON)?;
            }

            let actions = task.Actions()?;
            let exec_action: IExecAction2 = actions.Create(TASK_ACTION_EXEC)?.cast()?;
            exec_action.SetPath(&service_path.to_string_lossy().to_string().into())?;
            exec_action.SetArguments(&"--startup".into())?;

            let mut task_xml = BSTR::new();
            task.XmlText(&mut task_xml)?;
            Self::register_task(
                &root_folder,
                &format!("{GROUP_FOLDER}\\{SERVICE_TASK_NAME}"),
                &task_xml,
            )?;
            Ok(())
        })
    }

    pub fn set_run_on_logon(enabled: bool) -> Result<()> {
        Com::run_with_context(|| unsafe {
            let task_service = Self::get_task_service()?;
            let seelen_folder = task_service.GetFolder(&GROUP_FOLDER.into())?;
            let task = seelen_folder.GetTask(&SERVICE_TASK_NAME.into())?;
            let task = task.Definition()?;
            let triggers = task.Triggers()?;
            triggers.Clear()?;
            if enabled {
                triggers.Create(TASK_TRIGGER_LOGON)?;
            }
            let mut task_xml = BSTR::new();
            task.XmlText(&mut task_xml)?;
            Self::register_task(&seelen_folder, SERVICE_TASK_NAME, &task_xml)?;
            Ok(())
        })
    }

    pub fn task_exists() -> bool {
        Com::run_with_context(|| unsafe {
            let task_service = Self::get_task_service()?;
            task_service
                .GetFolder(&GROUP_FOLDER.into())?
                .GetTask(&SERVICE_TASK_NAME.into())?;
            Ok(())
        })
        .is_ok()
    }

    pub fn set_run_key(enabled: bool) -> Result<()> {
        let (key, _) = RegKey::predef(HKEY_CURRENT_USER).create_subkey(RUN_KEY)?;
        if enabled {
            let exe = service_exe()?;
            key.set_value(RUN_VALUE, &format!("\"{}\" --startup", exe.display()))?;
        } else if key.get_raw_value(RUN_VALUE).is_ok() {
            key.delete_value(RUN_VALUE)?;
        }
        Ok(())
    }

    /// the logon task when it can be changed, the Run key otherwise. Never both, so the
    /// service isn't started twice on logon
    pub fn set_autostart(enabled: bool) -> Result<()> {
        save_autostart_wanted(enabled)?;
        match Self::set_run_on_logon(enabled) {
            Ok(()) => Self::set_run_key(false),
            Err(err) => {
                log::debug!("logon task not usable ({err:?}), using the Run key");
                Self::set_run_key(enabled)
            }
        }
    }

    /// called on every start: keeps the Run key in place (and pointing at this exe) when
    /// there is no logon task to do the job
    pub fn sync_run_key() -> Result<()> {
        let use_key = !Self::task_exists() && autostart_wanted();
        Self::set_run_key(use_key)
    }

    pub fn remove_service_task() -> Result<()> {
        Com::run_with_context(|| unsafe {
            let task_service: ITaskService = Com::create_instance(&TaskScheduler)?;
            task_service.Connect(
                &Default::default(),
                &Default::default(),
                &Default::default(),
                &Default::default(),
            )?;
            if let Ok(seelen_folder) = task_service.GetFolder(&GROUP_FOLDER.into()) {
                let _ = seelen_folder.DeleteTask(&SERVICE_TASK_NAME.into(), 0);
            }
            Ok(())
        })
    }
}
