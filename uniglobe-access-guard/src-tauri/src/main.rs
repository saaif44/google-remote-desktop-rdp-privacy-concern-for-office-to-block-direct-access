use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::{path::Path, process::Command, sync::Mutex, thread, time::Duration as StdDuration};
use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    AppHandle, Manager, WindowEvent,
};

const DEFAULT_COUNTDOWN_SECONDS: u32 = 60;
const MAX_APPROVAL_MINUTES: u32 = 480;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Mode {
    AlwaysAllow,
    AskToAllow,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum AccessState {
    Enabled,
    Blocked,
    Countdown,
    Temporary,
    Error,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ServiceStatus {
    running: bool,
    autostart_enabled: bool,
}

impl ServiceStatus {
    fn blocked() -> Self {
        Self {
            running: false,
            autostart_enabled: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct StatusDto {
    mode: Mode,
    state: AccessState,
    service_running: bool,
    autostart_enabled: bool,
    approved_until: Option<String>,
    countdown_remaining_seconds: Option<u32>,
    approved_remaining_seconds: Option<u32>,
    incoming_request_active: bool,
    incoming_request_at: Option<String>,
    message: Option<String>,
    mock_mode: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AuditEvent {
    ts: String,
    event: String,
    mode: Mode,
    state: AccessState,
    message: String,
    duration_minutes: Option<u32>,
    countdown_seconds: Option<u32>,
}

#[derive(Debug, Clone)]
struct IncomingRequest {
    requested_at: DateTime<Utc>,
}

trait RemoteDesktopController: Send {
    fn name(&self) -> &'static str;
    fn is_mock(&self) -> bool {
        false
    }
    fn status(&self) -> Result<ServiceStatus, String>;
    fn enable_autostart(&self) -> Result<(), String>;
    fn disable_autostart(&self) -> Result<(), String>;
    fn start(&self) -> Result<(), String>;
    fn stop(&self) -> Result<(), String>;
}

#[derive(Debug)]
struct MockRemoteDesktopController {
    service: Mutex<ServiceStatus>,
}

impl MockRemoteDesktopController {
    fn new() -> Self {
        Self {
            service: Mutex::new(ServiceStatus::blocked()),
        }
    }

    fn update<F>(&self, f: F) -> Result<(), String>
    where
        F: FnOnce(&mut ServiceStatus),
    {
        let mut service = self
            .service
            .lock()
            .map_err(|_| "mock service state lock is poisoned".to_string())?;
        f(&mut service);
        Ok(())
    }
}

impl RemoteDesktopController for MockRemoteDesktopController {
    fn name(&self) -> &'static str {
        "mock"
    }

    fn is_mock(&self) -> bool {
        true
    }

    fn status(&self) -> Result<ServiceStatus, String> {
        self.service
            .lock()
            .map(|service| *service)
            .map_err(|_| "mock service state lock is poisoned".to_string())
    }

    fn enable_autostart(&self) -> Result<(), String> {
        self.update(|service| service.autostart_enabled = true)
    }

    fn disable_autostart(&self) -> Result<(), String> {
        self.update(|service| service.autostart_enabled = false)
    }

    fn start(&self) -> Result<(), String> {
        self.update(|service| service.running = true)
    }

    fn stop(&self) -> Result<(), String> {
        self.update(|service| service.running = false)
    }
}

fn run_cmd(program: &str, args: &[&str]) -> Result<String, String> {
    let output = Command::new(program)
        .args(args)
        .output()
        .map_err(|error| format!("failed to run {program}: {error}"))?;

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();

    if output.status.success() {
        Ok(stdout)
    } else {
        let detail = [stdout, stderr]
            .into_iter()
            .filter(|part| !part.is_empty())
            .collect::<Vec<_>>()
            .join(" ");
        Err(format!("{program} {} failed: {detail}", args.join(" ")))
    }
}

fn run_cmd_allowing(program: &str, args: &[&str], allowed_error: &str) -> Result<String, String> {
    match run_cmd(program, args) {
        Ok(output) => Ok(output),
        Err(error) if error.to_lowercase().contains(allowed_error) => Ok(error),
        Err(error) => Err(error),
    }
}

#[cfg(target_os = "windows")]
#[allow(dead_code)]
struct WindowsCrdController;

#[cfg(target_os = "windows")]
impl RemoteDesktopController for WindowsCrdController {
    fn name(&self) -> &'static str {
        "windows_chromoting_service"
    }

    fn status(&self) -> Result<ServiceStatus, String> {
        let query = run_cmd("sc.exe", &["query", "chromoting"])?;
        let config = run_cmd("sc.exe", &["qc", "chromoting"])?;
        Ok(ServiceStatus {
            running: query.to_uppercase().contains("RUNNING"),
            autostart_enabled: config.to_uppercase().contains("AUTO_START"),
        })
    }

    fn enable_autostart(&self) -> Result<(), String> {
        run_cmd("sc.exe", &["config", "chromoting", "start=", "auto"])?;
        Ok(())
    }

    fn disable_autostart(&self) -> Result<(), String> {
        run_cmd("sc.exe", &["config", "chromoting", "start=", "disabled"])?;
        Ok(())
    }

    fn start(&self) -> Result<(), String> {
        let config = run_cmd("sc.exe", &["qc", "chromoting"])?;
        if config.to_uppercase().contains("DISABLED") {
            run_cmd("sc.exe", &["config", "chromoting", "start=", "demand"])?;
        }
        if !self.status()?.running {
            run_cmd_allowing("sc.exe", &["start", "chromoting"], "already been started")?;
        }
        Ok(())
    }

    fn stop(&self) -> Result<(), String> {
        if self.status()?.running {
            run_cmd_allowing("sc.exe", &["stop", "chromoting"], "has not been started")?;
        }
        Ok(())
    }
}

#[cfg(target_os = "macos")]
#[allow(dead_code)]
struct MacCrdController;

#[cfg(target_os = "macos")]
impl RemoteDesktopController for MacCrdController {
    fn name(&self) -> &'static str {
        "macos_chromoting_launchagent"
    }

    fn status(&self) -> Result<ServiceStatus, String> {
        verify_macos_plist()?;
        let uid = macos_uid()?;
        let target = macos_launch_target(&uid);
        let domain = macos_launch_domain(&uid);
        let running = run_cmd("launchctl", &["print", &target])
            .map(|output| output.contains("state = running") || output.contains("pid ="))
            .unwrap_or(false);
        let disabled = run_cmd("launchctl", &["print-disabled", &domain])
            .map(|output| {
                output.lines().any(|line| {
                    line.contains("org.chromium.chromoting") && line.contains("=> disabled")
                })
            })
            .unwrap_or(false);
        Ok(ServiceStatus {
            running,
            autostart_enabled: !disabled,
        })
    }

    fn enable_autostart(&self) -> Result<(), String> {
        verify_macos_plist()?;
        let uid = macos_uid()?;
        let target = macos_launch_target(&uid);
        let domain = macos_launch_domain(&uid);
        let _ = run_cmd("launchctl", &["enable", &target]);
        run_cmd_allowing(
            "launchctl",
            &["bootstrap", &domain, MACOS_CHROMOTING_PLIST],
            "already bootstrapped",
        )?;
        Ok(())
    }

    fn disable_autostart(&self) -> Result<(), String> {
        verify_macos_plist()?;
        let uid = macos_uid()?;
        let target = macos_launch_target(&uid);
        let domain = macos_launch_domain(&uid);
        let _ = run_cmd("launchctl", &["bootout", &domain, MACOS_CHROMOTING_PLIST]);
        let _ = run_cmd("launchctl", &["disable", &target]);
        Ok(())
    }

    fn start(&self) -> Result<(), String> {
        verify_macos_plist()?;
        let uid = macos_uid()?;
        let target = macos_launch_target(&uid);
        self.enable_autostart()?;
        run_cmd("launchctl", &["kickstart", "-k", &target])?;
        Ok(())
    }

    fn stop(&self) -> Result<(), String> {
        verify_macos_plist()?;
        let uid = macos_uid()?;
        let domain = macos_launch_domain(&uid);
        let _ = run_cmd("launchctl", &["bootout", &domain, MACOS_CHROMOTING_PLIST]);
        Ok(())
    }
}

#[cfg(target_os = "macos")]
const MACOS_CHROMOTING_PLIST: &str = "/Library/LaunchAgents/org.chromium.chromoting.plist";

#[cfg(target_os = "macos")]
fn verify_macos_plist() -> Result<(), String> {
    if Path::new(MACOS_CHROMOTING_PLIST).exists() {
        Ok(())
    } else {
        Err(format!(
            "Chrome Remote Desktop LaunchAgent plist was not found at {MACOS_CHROMOTING_PLIST}"
        ))
    }
}

#[cfg(target_os = "macos")]
fn macos_uid() -> Result<String, String> {
    run_cmd("id", &["-u"]).map(|uid| uid.trim().to_string())
}

#[cfg(target_os = "macos")]
fn macos_launch_domain(uid: &str) -> String {
    format!("gui/{uid}")
}

#[cfg(target_os = "macos")]
fn macos_launch_target(uid: &str) -> String {
    format!("{}/org.chromium.chromoting", macos_launch_domain(uid))
}

#[cfg(target_os = "windows")]
fn default_controller() -> Box<dyn RemoteDesktopController> {
    Box::new(WindowsCrdController)
}

#[cfg(target_os = "macos")]
fn default_controller() -> Box<dyn RemoteDesktopController> {
    Box::new(MacCrdController)
}

#[cfg(not(any(target_os = "windows", target_os = "macos")))]
fn default_controller() -> Box<dyn RemoteDesktopController> {
    Box::new(MockRemoteDesktopController::new())
}

#[derive(Debug, Clone)]
struct CountdownApproval {
    started_at: DateTime<Utc>,
    duration_minutes: Option<u32>,
    countdown_seconds: u32,
}

impl CountdownApproval {
    fn ends_at(&self) -> DateTime<Utc> {
        self.started_at + Duration::seconds(self.countdown_seconds as i64)
    }

    fn remaining_seconds(&self, now: DateTime<Utc>) -> u32 {
        remaining_seconds(self.ends_at(), now)
    }
}

struct GuardMachine {
    mode: Mode,
    state: AccessState,
    controller: Box<dyn RemoteDesktopController>,
    countdown: Option<CountdownApproval>,
    approved_until: Option<DateTime<Utc>>,
    incoming_request: Option<IncomingRequest>,
    message: Option<String>,
    audit_log: Vec<AuditEvent>,
}

impl GuardMachine {
    fn new(now: DateTime<Utc>) -> Self {
        if std::env::var("UNIGLOBE_CONTROLLER")
            .map(|value| value.eq_ignore_ascii_case("mock"))
            .unwrap_or(false)
        {
            return Self::new_mock(now);
        }

        Self::new_with_controller(now, default_controller())
    }

    fn new_mock(now: DateTime<Utc>) -> Self {
        Self::new_with_controller(now, Box::new(MockRemoteDesktopController::new()))
    }

    fn new_with_controller(
        now: DateTime<Utc>,
        controller: Box<dyn RemoteDesktopController>,
    ) -> Self {
        let is_mock = controller.is_mock();
        let controller_name = controller.name();
        let mut state = AccessState::Blocked;
        let mut message = if is_mock {
            "Mock service state is active. No Chrome Remote Desktop commands will run.".to_string()
        } else {
            format!(
                "System controller active: {controller_name}. Ask to Allow is enforced by default."
            )
        };

        if !is_mock {
            match controller
                .stop()
                .and_then(|_| controller.disable_autostart())
            {
                Ok(()) => {
                    message =
                        "Ask to Allow is active. Chrome Remote Desktop was stopped and disabled."
                            .to_string();
                }
                Err(error) => {
                    state = AccessState::Error;
                    message = format!("Unable to enforce Ask to Allow on startup: {error}");
                }
            }
        }

        let mut machine = Self {
            mode: Mode::AskToAllow,
            state,
            controller,
            countdown: None,
            approved_until: None,
            incoming_request: None,
            message: Some(message),
            audit_log: Vec::new(),
        };
        machine.audit(
            now,
            "app_started",
            if is_mock {
                "Started with mock service state"
            } else {
                "Started with system service controller"
            },
            None,
            None,
        );
        machine
    }

    fn reconcile(&mut self, now: DateTime<Utc>) -> Result<(), String> {
        if let Some(countdown) = self.countdown.clone() {
            if now >= countdown.ends_at() {
                self.controller.enable_autostart()?;
                self.controller.start()?;
                self.approved_until = countdown
                    .duration_minutes
                    .map(|minutes| now + Duration::minutes(minutes as i64));
                self.state = AccessState::Temporary;
                self.countdown = None;

                let message = match countdown.duration_minutes {
                    Some(minutes) => {
                        format!("{} is allowed for {minutes} minutes.", self.access_label())
                    }
                    None => format!("{} is allowed until revoked.", self.access_label()),
                };
                self.message = Some(message.clone());
                self.audit(
                    now,
                    "access_started",
                    &message,
                    countdown.duration_minutes,
                    Some(countdown.countdown_seconds),
                );
            }
        }

        if self.state == AccessState::Temporary {
            if let Some(approved_until) = self.approved_until {
                if now >= approved_until {
                    self.controller.stop()?;
                    if self.mode == Mode::AskToAllow {
                        self.controller.disable_autostart()?;
                    }
                    self.approved_until = None;
                    self.state = AccessState::Blocked;
                    let message =
                        format!("{} expired and returned to blocked.", self.access_label());
                    self.message = Some(message.clone());
                    self.audit(now, "access_expired", &message, None, None);
                }
            }
        }

        Ok(())
    }

    fn status(&self, now: DateTime<Utc>) -> Result<StatusDto, String> {
        let (service, state, message) = match self.controller.status() {
            Ok(service) => (service, self.state, self.message.clone()),
            Err(error) => (
                ServiceStatus::blocked(),
                AccessState::Error,
                Some(format!("{} status error: {error}", self.controller.name())),
            ),
        };
        Ok(StatusDto {
            mode: self.mode,
            state,
            service_running: service.running,
            autostart_enabled: service.autostart_enabled,
            approved_until: self.approved_until.map(|ts| ts.to_rfc3339()),
            countdown_remaining_seconds: self
                .countdown
                .as_ref()
                .map(|countdown| countdown.remaining_seconds(now)),
            approved_remaining_seconds: self
                .approved_until
                .map(|approved_until| remaining_seconds(approved_until, now)),
            incoming_request_active: self.incoming_request.is_some(),
            incoming_request_at: self
                .incoming_request
                .as_ref()
                .map(|request| request.requested_at.to_rfc3339()),
            message,
            mock_mode: self.controller.is_mock(),
        })
    }

    fn set_mode(&mut self, mode: Mode, now: DateTime<Utc>) -> Result<(), String> {
        self.mode = mode;
        self.countdown = None;
        self.approved_until = None;
        self.incoming_request = None;

        match mode {
            Mode::AlwaysAllow => {
                self.controller.enable_autostart()?;
                self.controller.start()?;
                self.state = AccessState::Enabled;
                self.message = Some(format!(
                    "Always Allow mode: {} is running and set to autostart.",
                    self.access_label()
                ));
            }
            Mode::AskToAllow => {
                self.controller.stop()?;
                self.controller.disable_autostart()?;
                self.state = AccessState::Blocked;
                self.message = Some(format!(
                    "Ask to Allow mode: {} is stopped and disabled.",
                    self.access_label()
                ));
            }
        }

        let audit_message = self
            .message
            .clone()
            .unwrap_or_else(|| "Mode changed in mock service state".to_string());
        self.audit(now, "mode_changed", &audit_message, None, None);
        Ok(())
    }

    fn approve_once(
        &mut self,
        minutes: u32,
        countdown_seconds: u32,
        now: DateTime<Utc>,
    ) -> Result<(), String> {
        if minutes == 0 || minutes > MAX_APPROVAL_MINUTES {
            return Err(format!(
                "approval duration must be between 1 and {MAX_APPROVAL_MINUTES} minutes"
            ));
        }

        self.incoming_request = None;
        self.start_countdown(Some(minutes), countdown_seconds, now)?;
        let message = format!("Approved {} for {minutes} minutes", self.access_label());
        self.audit(
            now,
            "access_approved",
            &message,
            Some(minutes),
            Some(countdown_seconds),
        );
        Ok(())
    }

    fn approve_until_revoked(
        &mut self,
        countdown_seconds: u32,
        now: DateTime<Utc>,
    ) -> Result<(), String> {
        self.incoming_request = None;
        self.start_countdown(None, countdown_seconds, now)?;
        let message = format!("Approved {} until revoked", self.access_label());
        self.audit(
            now,
            "access_approved_until_revoked",
            &message,
            None,
            Some(countdown_seconds),
        );
        Ok(())
    }

    fn terminate_now(&mut self, now: DateTime<Utc>) -> Result<(), String> {
        self.countdown = None;
        self.approved_until = None;
        self.incoming_request = None;
        self.controller.stop()?;
        if self.mode == Mode::AskToAllow {
            self.controller.disable_autostart()?;
        }
        self.state = AccessState::Blocked;
        let message = format!("{} terminated.", self.access_label());
        self.message = Some(message.clone());
        self.audit(now, "access_terminated", &message, None, None);
        Ok(())
    }

    fn incoming_request(&mut self, now: DateTime<Utc>) {
        self.incoming_request = Some(IncomingRequest { requested_at: now });
        self.message =
            Some("Incoming remote access request. App window was brought forward.".into());
        self.audit(
            now,
            "incoming_request",
            "Incoming remote access request",
            None,
            None,
        );
    }

    fn start_countdown(
        &mut self,
        duration_minutes: Option<u32>,
        countdown_seconds: u32,
        now: DateTime<Utc>,
    ) -> Result<(), String> {
        self.controller.stop()?;
        self.state = AccessState::Countdown;
        self.countdown = Some(CountdownApproval {
            started_at: now,
            duration_minutes,
            countdown_seconds,
        });
        self.approved_until = None;
        self.message = Some(format!(
            "Countdown active. {} will start when it reaches zero.",
            self.access_label()
        ));
        Ok(())
    }

    fn access_label(&self) -> &'static str {
        if self.controller.is_mock() {
            "mock access"
        } else {
            "Chrome Remote Desktop access"
        }
    }

    fn audit(
        &mut self,
        now: DateTime<Utc>,
        event: &str,
        message: &str,
        duration_minutes: Option<u32>,
        countdown_seconds: Option<u32>,
    ) {
        self.audit_log.push(AuditEvent {
            ts: now.to_rfc3339(),
            event: event.to_string(),
            mode: self.mode,
            state: self.state,
            message: message.to_string(),
            duration_minutes,
            countdown_seconds,
        });
    }
}

struct AppState {
    machine: Mutex<GuardMachine>,
}

impl AppState {
    fn new() -> Self {
        Self {
            machine: Mutex::new(GuardMachine::new(Utc::now())),
        }
    }
}

fn remaining_seconds(target: DateTime<Utc>, now: DateTime<Utc>) -> u32 {
    let seconds = (target - now).num_seconds();
    if seconds <= 0 {
        0
    } else {
        seconds as u32
    }
}

fn with_machine_state<T, F>(state: &AppState, f: F) -> Result<T, String>
where
    F: FnOnce(&mut GuardMachine, DateTime<Utc>) -> Result<T, String>,
{
    let now = Utc::now();
    let mut machine = state
        .machine
        .lock()
        .map_err(|_| "application state lock is poisoned".to_string())?;
    machine.reconcile(now)?;
    f(&mut machine, now)
}

fn with_machine<T, F>(state: tauri::State<'_, AppState>, f: F) -> Result<T, String>
where
    F: FnOnce(&mut GuardMachine, DateTime<Utc>) -> Result<T, String>,
{
    with_machine_state(state.inner(), f)
}

fn show_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

fn schedule_reconcile(app: &AppHandle, seconds: u64) {
    let app = app.clone();
    thread::spawn(move || {
        thread::sleep(StdDuration::from_secs(seconds));
        let state = app.state::<AppState>();
        let _ = with_machine_state(state.inner(), |_machine, _now| Ok(()));
    });
}

#[tauri::command]
fn get_status(state: tauri::State<'_, AppState>) -> Result<StatusDto, String> {
    with_machine(state, |machine, now| machine.status(now))
}

#[tauri::command]
fn set_mode(mode: Mode, state: tauri::State<'_, AppState>) -> Result<StatusDto, String> {
    with_machine(state, |machine, now| {
        machine.set_mode(mode, now)?;
        machine.status(now)
    })
}

#[tauri::command]
fn approve_once(
    minutes: u32,
    app: AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<StatusDto, String> {
    let status = with_machine(state, |machine, now| {
        machine.approve_once(minutes, DEFAULT_COUNTDOWN_SECONDS, now)?;
        machine.status(now)
    })?;
    schedule_reconcile(&app, DEFAULT_COUNTDOWN_SECONDS as u64);
    schedule_reconcile(
        &app,
        DEFAULT_COUNTDOWN_SECONDS as u64 + (minutes as u64 * 60) + 1,
    );
    Ok(status)
}

#[tauri::command]
fn approve_until_revoked(
    app: AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<StatusDto, String> {
    let status = with_machine(state, |machine, now| {
        machine.approve_until_revoked(DEFAULT_COUNTDOWN_SECONDS, now)?;
        machine.status(now)
    })?;
    schedule_reconcile(&app, DEFAULT_COUNTDOWN_SECONDS as u64);
    Ok(status)
}

#[tauri::command]
fn terminate_now(state: tauri::State<'_, AppState>) -> Result<StatusDto, String> {
    with_machine(state, |machine, now| {
        machine.terminate_now(now)?;
        machine.status(now)
    })
}

#[tauri::command]
fn get_audit_log(state: tauri::State<'_, AppState>) -> Result<Vec<AuditEvent>, String> {
    with_machine(state, |machine, _now| Ok(machine.audit_log.clone()))
}

#[tauri::command]
fn mock_incoming_request(
    app: AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<StatusDto, String> {
    let status = with_machine(state, |machine, now| {
        machine.incoming_request(now);
        machine.status(now)
    })?;
    show_main_window(&app);
    Ok(status)
}

fn main() {
    tauri::Builder::default()
        .manage(AppState::new())
        .setup(|app| {
            #[cfg(any(target_os = "macos", windows, target_os = "linux"))]
            {
                use tauri_plugin_autostart::{MacosLauncher, ManagerExt as _};

                app.handle().plugin(tauri_plugin_autostart::init(
                    MacosLauncher::LaunchAgent,
                    None,
                ))?;

                if std::env::var_os("UNIGLOBE_DISABLE_AUTOSTART").is_none() {
                    if let Err(error) = app.autolaunch().enable() {
                        eprintln!("failed to enable UniGlobe Access Guard autostart: {error}");
                    }
                }
            }

            let show = MenuItem::with_id(app, "show", "Show", true, None::<&str>)?;
            let incoming =
                MenuItem::with_id(app, "incoming", "Test Incoming Request", true, None::<&str>)?;
            let allow_15 =
                MenuItem::with_id(app, "allow_15", "Allow 15 minutes", true, None::<&str>)?;
            let terminate = MenuItem::with_id(app, "terminate", "Terminate", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show, &incoming, &allow_15, &terminate, &quit])?;

            TrayIconBuilder::with_id("access-guard")
                .tooltip("UniGlobe Access Guard")
                .menu(&menu)
                .show_menu_on_left_click(true)
                .on_menu_event(|app, event| match event.id().as_ref() {
                    "show" => {
                        show_main_window(app);
                    }
                    "incoming" => {
                        let state = app.state::<AppState>();
                        let _ = with_machine_state(state.inner(), |machine, now| {
                            machine.incoming_request(now);
                            Ok(())
                        });
                        show_main_window(app);
                    }
                    "allow_15" => {
                        let state = app.state::<AppState>();
                        let _ = with_machine_state(state.inner(), |machine, now| {
                            machine.approve_once(15, DEFAULT_COUNTDOWN_SECONDS, now)?;
                            Ok(())
                        });
                        schedule_reconcile(app, DEFAULT_COUNTDOWN_SECONDS as u64);
                        schedule_reconcile(app, DEFAULT_COUNTDOWN_SECONDS as u64 + (15 * 60) + 1);
                    }
                    "terminate" => {
                        let state = app.state::<AppState>();
                        let _ = with_machine_state(state.inner(), |machine, now| {
                            machine.terminate_now(now)?;
                            Ok(())
                        });
                    }
                    "quit" => app.exit(0),
                    _ => {}
                })
                .build(app)?;

            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .invoke_handler(tauri::generate_handler![
            get_status,
            set_mode,
            approve_once,
            approve_until_revoked,
            terminate_now,
            get_audit_log,
            mock_incoming_request
        ])
        .run(tauri::generate_context!())
        .expect("error while running UniGlobe Access Guard");
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(seconds: i64) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339("2026-04-30T12:00:00Z")
            .unwrap()
            .with_timezone(&Utc)
            + Duration::seconds(seconds)
    }

    #[test]
    fn ask_to_allow_starts_blocked() {
        let machine = GuardMachine::new_mock(at(0));
        let status = machine.status(at(0)).unwrap();

        assert_eq!(status.mode, Mode::AskToAllow);
        assert_eq!(status.state, AccessState::Blocked);
        assert!(!status.service_running);
        assert!(!status.autostart_enabled);
    }

    #[test]
    fn always_allow_marks_mock_service_running() {
        let mut machine = GuardMachine::new_mock(at(0));
        machine.set_mode(Mode::AlwaysAllow, at(1)).unwrap();
        let status = machine.status(at(1)).unwrap();

        assert_eq!(status.state, AccessState::Enabled);
        assert!(status.service_running);
        assert!(status.autostart_enabled);
    }

    #[test]
    fn approve_once_waits_for_countdown_before_starting_mock_service() {
        let mut machine = GuardMachine::new_mock(at(0));
        machine.approve_once(30, 10, at(1)).unwrap();

        let before = machine.status(at(5)).unwrap();
        assert_eq!(before.state, AccessState::Countdown);
        assert!(!before.service_running);
        assert_eq!(before.countdown_remaining_seconds, Some(6));

        machine.reconcile(at(11)).unwrap();
        let after = machine.status(at(11)).unwrap();
        assert_eq!(after.state, AccessState::Temporary);
        assert!(after.service_running);
        assert_eq!(after.approved_remaining_seconds, Some(1800));
    }

    #[test]
    fn temporary_approval_expires_back_to_blocked_in_ask_mode() {
        let mut machine = GuardMachine::new_mock(at(0));
        machine.approve_once(1, 1, at(1)).unwrap();

        machine.reconcile(at(2)).unwrap();
        assert_eq!(machine.status(at(2)).unwrap().state, AccessState::Temporary);

        machine.reconcile(at(63)).unwrap();
        let status = machine.status(at(63)).unwrap();
        assert_eq!(status.state, AccessState::Blocked);
        assert!(!status.service_running);
        assert!(!status.autostart_enabled);
    }

    #[test]
    fn terminate_cancels_countdown_without_starting_service() {
        let mut machine = GuardMachine::new_mock(at(0));
        machine.approve_until_revoked(10, at(1)).unwrap();
        machine.terminate_now(at(2)).unwrap();

        let status = machine.status(at(12)).unwrap();
        assert_eq!(status.state, AccessState::Blocked);
        assert!(!status.service_running);
        assert_eq!(status.countdown_remaining_seconds, None);
    }

    #[test]
    fn incoming_request_is_visible_in_status() {
        let mut machine = GuardMachine::new_mock(at(0));
        machine.incoming_request(at(4));
        let status = machine.status(at(4)).unwrap();

        assert!(status.incoming_request_active);
        assert_eq!(status.incoming_request_at, Some(at(4).to_rfc3339()));
        assert_eq!(status.state, AccessState::Blocked);
    }

    #[test]
    fn approving_request_clears_incoming_request() {
        let mut machine = GuardMachine::new_mock(at(0));
        machine.incoming_request(at(4));
        machine.approve_once(15, 10, at(5)).unwrap();
        let status = machine.status(at(5)).unwrap();

        assert!(!status.incoming_request_active);
        assert_eq!(status.state, AccessState::Countdown);
    }
}
