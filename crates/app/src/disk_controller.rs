#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
    }

    impl cxx_qt::Threading for DiskController {}

    extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[qproperty(QString, disks_json, cxx_name = "disksJson")]
        #[qproperty(QString, options_json, cxx_name = "optionsJson")]
        #[qproperty(QString, plan_json, cxx_name = "planJson")]
        #[qproperty(QString, plan_error, cxx_name = "planError")]
        #[qproperty(QString, result_message, cxx_name = "resultMessage")]
        #[qproperty(QString, step_text, cxx_name = "stepText")]
        #[qproperty(QString, phase)]
        #[qproperty(f64, progress)]
        #[qproperty(bool, busy)]
        #[qproperty(QString, error)]
        #[qproperty(QString, start_device, cxx_name = "startDevice")]
        #[qproperty(QString, start_error, cxx_name = "startError")]
        type DiskController = super::DiskControllerRust;

        /// Lists disks off the UI thread; results arrive through the properties.
        #[qinvokable]
        fn refresh(self: Pin<&mut Self>);

        /// Detects which `mkfs` tools are installed and fills `optionsJson`.
        #[qinvokable]
        #[cxx_name = "loadOptions"]
        fn load_options(self: Pin<&mut Self>);

        /// French validation message for a volume label; empty when valid.
        #[qinvokable]
        #[cxx_name = "labelMessage"]
        fn label_message(self: &DiskController, fs_id: &QString, label: &QString) -> QString;

        /// Builds and validates a plan; `phase` becomes `ready` or `planError` is set.
        #[qinvokable]
        #[cxx_name = "preparePlan"]
        fn prepare_plan(
            self: Pin<&mut Self>,
            device: &QString,
            fs_id: &QString,
            table_id: &QString,
            label: &QString,
            erase_id: &QString,
        );

        /// Runs the prepared plan if `typed` is exactly the disk name.
        #[qinvokable]
        #[cxx_name = "confirmAndRun"]
        fn confirm_and_run(self: Pin<&mut Self>, typed: &QString);

        /// True only in builds made with the `smoke` feature (headless end-to-end test).
        #[qinvokable]
        #[cxx_name = "smokeEnabled"]
        fn smoke_enabled(self: &DiskController) -> bool;

        /// Text of a catalog message in the user's language (the key itself if unknown).
        #[qinvokable]
        fn tr(self: &DiskController, key: &QString) -> QString;

        #[qinvokable]
        fn tr1(self: &DiskController, key: &QString, a: &QString) -> QString;

        #[qinvokable]
        fn tr2(self: &DiskController, key: &QString, a: &QString, b: &QString) -> QString;

        #[qinvokable]
        fn tr3(
            self: &DiskController,
            key: &QString,
            a: &QString,
            b: &QString,
            c: &QString,
        ) -> QString;

        /// Forgets the plan and result (used when going back to the disk list).
        #[qinvokable]
        fn reset(self: Pin<&mut Self>);
    }
}

use core::pin::Pin;
use cxx_qt::{CxxQtType, Threading};
use cxx_qt_lib::QString;
use kformat_core::backend::{DiskBackend, Progress};
use kformat_core::cli::parse_device_arg;
use kformat_core::fs::{available_filesystems, binary_in_path};
use kformat_core::guard::run_guarded;
use kformat_core::i18n::Lang;
use kformat_core::messages::{tr, Msg};
use kformat_core::plan::FormatPlan;
use kformat_core::session::{prepare_plan, PlanInput};
use kformat_core::udisks::UdisksBackend;
use kformat_core::view::{
    backend_error_message, device_arg_error_message, disks_to_json, label_message, options_json,
    plan_error_message, plan_json, session_error_message, step_text,
};

const PHASE_IDLE: &str = "idle";
const PHASE_PLANNING: &str = "planning";
const PHASE_READY: &str = "ready";
const PHASE_RUNNING: &str = "running";
const PHASE_DONE: &str = "done";
const PHASE_FAILED: &str = "failed";

pub struct DiskControllerRust {
    disks_json: QString,
    options_json: QString,
    plan_json: QString,
    plan_error: QString,
    result_message: QString,
    step_text: QString,
    phase: QString,
    progress: f64,
    busy: bool,
    error: QString,
    /// Device given with `--device` at launch (empty when none).
    start_device: QString,
    /// Why the `--device` value was rejected (empty when fine).
    start_error: QString,
    /// Interface language, detected once at startup.
    lang: Lang,
    /// The validated plan waiting for the user's confirmation.
    plan: Option<FormatPlan>,
    /// Bumped whenever a plan request is superseded, so stale results are dropped.
    request_id: u64,
}

impl Default for DiskControllerRust {
    fn default() -> Self {
        let lang = Lang::from_env();
        let args: Vec<String> = std::env::args().collect();
        let (start_device, start_error) = match parse_device_arg(&args) {
            Ok(device) => (device.unwrap_or_default(), String::new()),
            Err(value) => (String::new(), device_arg_error_message(lang, &value)),
        };
        Self {
            disks_json: QString::from("[]"),
            options_json: QString::from("{}"),
            plan_json: QString::from("{}"),
            plan_error: QString::default(),
            result_message: QString::default(),
            step_text: QString::default(),
            phase: QString::from(PHASE_IDLE),
            progress: 0.0,
            busy: false,
            error: QString::default(),
            start_device: QString::from(start_device.as_str()),
            start_error: QString::from(start_error.as_str()),
            lang,
            plan: None,
            request_id: 0,
        }
    }
}

fn qs(text: &str) -> QString {
    QString::from(text)
}

/// Headless end-to-end test hook: only exists in builds made with `--features smoke`.
#[cfg(feature = "smoke")]
fn smoke_mode() -> bool {
    std::env::args().any(|arg| arg == "--smoke")
}

#[cfg(not(feature = "smoke"))]
fn smoke_mode() -> bool {
    false
}

/// In smoke mode only throw-away loop devices may be targeted.
fn device_allowed(device: &str) -> bool {
    !smoke_mode() || device.starts_with("/dev/loop")
}

/// Lists the disks and renders them as JSON, or a user-facing error message.
fn load_disks_json(lang: Lang) -> Result<String, String> {
    let backend = UdisksBackend::connect(false).map_err(|e| backend_error_message(lang, &e))?;
    let disks = backend
        .list_disks()
        .map_err(|e| backend_error_message(lang, &e))?;
    Ok(disks_to_json(lang, &disks))
}

/// Builds the plan for the user's choices (and whether sleep can be blocked while it
/// runs), or a user-facing error message.
fn plan_for(lang: Lang, input: &PlanInput) -> Result<(FormatPlan, bool), String> {
    if !device_allowed(input.device) {
        return Err(tr(lang, Msg::SmokeDeviceRefused, &[]));
    }
    let backend =
        UdisksBackend::connect(smoke_mode()).map_err(|e| session_error_message(lang, &e.into()))?;
    let available = available_filesystems(&binary_in_path);
    let plan =
        prepare_plan(&backend, &available, input).map_err(|e| session_error_message(lang, &e))?;
    Ok((plan, backend.sleep_protection_available()))
}

impl qobject::DiskController {
    pub fn refresh(mut self: Pin<&mut Self>) {
        if *self.busy() {
            return;
        }
        self.as_mut().set_busy(true);
        self.as_mut().set_error(QString::default());

        let lang = self.rust().lang;
        let qt_thread = self.qt_thread();
        std::thread::spawn(move || {
            let result = run_guarded(lang, || load_disks_json(lang));
            // `Err` only means the controller was destroyed meanwhile: nothing to update.
            let _ = qt_thread.queue(move |mut controller| {
                match result {
                    Ok(json) => controller.as_mut().set_disks_json(qs(&json)),
                    Err(message) => controller.as_mut().set_error(qs(&message)),
                }
                controller.as_mut().set_busy(false);
            });
        });
    }

    pub fn load_options(self: Pin<&mut Self>) {
        let json = options_json(self.rust().lang, &available_filesystems(&binary_in_path));
        self.set_options_json(qs(&json));
    }

    pub fn label_message(&self, fs_id: &QString, label: &QString) -> QString {
        qs(&label_message(
            self.rust().lang,
            &fs_id.to_string(),
            &label.to_string(),
        ))
    }

    fn translate(&self, key: &QString, args: &[&QString]) -> QString {
        let key = key.to_string();
        let args: Vec<String> = args.iter().map(|a| a.to_string()).collect();
        let args: Vec<&str> = args.iter().map(String::as_str).collect();
        match Msg::from_key(&key) {
            Some(msg) => qs(&tr(self.rust().lang, msg, &args)),
            None => qs(&key),
        }
    }

    pub fn tr(&self, key: &QString) -> QString {
        self.translate(key, &[])
    }

    pub fn tr1(&self, key: &QString, a: &QString) -> QString {
        self.translate(key, &[a])
    }

    pub fn tr2(&self, key: &QString, a: &QString, b: &QString) -> QString {
        self.translate(key, &[a, b])
    }

    pub fn tr3(&self, key: &QString, a: &QString, b: &QString, c: &QString) -> QString {
        self.translate(key, &[a, b, c])
    }

    pub fn smoke_enabled(&self) -> bool {
        cfg!(feature = "smoke")
    }

    pub fn prepare_plan(
        mut self: Pin<&mut Self>,
        device: &QString,
        fs_id: &QString,
        table_id: &QString,
        label: &QString,
        erase_id: &QString,
    ) {
        if matches!(
            self.phase().to_string().as_str(),
            PHASE_PLANNING | PHASE_RUNNING
        ) {
            return;
        }
        let request_id = self.rust().request_id + 1;
        self.as_mut().rust_mut().request_id = request_id;
        self.as_mut().set_plan_error(QString::default());
        self.as_mut().set_phase(qs(PHASE_PLANNING));

        let (device, fs_id, table_id) =
            (device.to_string(), fs_id.to_string(), table_id.to_string());
        let (label, erase_id) = (label.to_string(), erase_id.to_string());
        let lang = self.rust().lang;
        let qt_thread = self.qt_thread();
        std::thread::spawn(move || {
            let result = run_guarded(lang, || {
                plan_for(
                    lang,
                    &PlanInput {
                        device: &device,
                        fs_id: &fs_id,
                        table_id: &table_id,
                        label: &label,
                        erase_id: &erase_id,
                    },
                )
            });
            let _ = qt_thread.queue(move |mut controller| {
                // The user moved on (reset or a newer request): drop this result.
                if controller.rust().request_id != request_id {
                    return;
                }
                match result {
                    Ok((plan, sleep_protected)) => {
                        controller.as_mut().set_plan_json(qs(&plan_json(
                            lang,
                            &plan,
                            sleep_protected,
                        )));
                        controller.as_mut().rust_mut().plan = Some(plan);
                        controller.as_mut().set_phase(qs(PHASE_READY));
                    }
                    Err(message) => {
                        controller.as_mut().rust_mut().plan = None;
                        controller.as_mut().set_plan_error(qs(&message));
                        controller.as_mut().set_phase(qs(PHASE_IDLE));
                    }
                }
            });
        });
    }

    pub fn confirm_and_run(mut self: Pin<&mut Self>, typed: &QString) {
        if self.phase().to_string() != PHASE_READY {
            return;
        }
        let lang = self.rust().lang;
        let Some(plan) = self.rust().plan.clone() else {
            self.as_mut()
                .set_plan_error(qs(&tr(lang, Msg::NoPlanToRun, &[])));
            return;
        };
        let confirmed = match plan.confirm(&typed.to_string()) {
            Ok(confirmed) => confirmed,
            Err(err) => {
                self.as_mut()
                    .set_plan_error(qs(&plan_error_message(lang, &err)));
                return;
            }
        };

        let texts: Vec<String> = confirmed
            .steps()
            .iter()
            .map(|step| step_text(lang, step))
            .collect();
        self.as_mut().rust_mut().plan = None;
        self.as_mut().set_plan_error(QString::default());
        self.as_mut().set_result_message(QString::default());
        self.as_mut().set_progress(0.0);
        self.as_mut()
            .set_step_text(qs(texts.first().map_or("", String::as_str)));
        self.as_mut().set_phase(qs(PHASE_RUNNING));

        let qt_thread = self.qt_thread();
        std::thread::spawn(move || {
            let progress_thread = qt_thread.clone();
            let outcome = run_guarded(lang, || {
                let backend = UdisksBackend::connect(smoke_mode())
                    .map_err(|e| backend_error_message(lang, &e))?;
                backend
                    .execute(&confirmed, &mut |p: Progress| {
                        let next = texts.get(p.completed).cloned().unwrap_or_default();
                        // Nothing to update if the controller is gone.
                        let _ = progress_thread.queue(move |mut controller| {
                            controller.as_mut().set_progress(p.fraction());
                            if !next.is_empty() {
                                controller.as_mut().set_step_text(qs(&next));
                            }
                        });
                    })
                    .map_err(|e| backend_error_message(lang, &e))
            });
            let _ = qt_thread.queue(move |mut controller| match outcome {
                Ok(()) => {
                    controller.as_mut().set_progress(1.0);
                    controller.as_mut().set_result_message(qs(&tr(
                        lang,
                        Msg::FormatSucceeded,
                        &[],
                    )));
                    controller.as_mut().set_phase(qs(PHASE_DONE));
                }
                Err(message) => {
                    controller.as_mut().set_result_message(qs(&message));
                    controller.as_mut().set_phase(qs(PHASE_FAILED));
                }
            });
        });
    }

    pub fn reset(mut self: Pin<&mut Self>) {
        if self.phase().to_string() == PHASE_RUNNING {
            return;
        }
        // Invalidate any plan request still in flight.
        let request_id = self.rust().request_id + 1;
        self.as_mut().rust_mut().request_id = request_id;
        self.as_mut().rust_mut().plan = None;
        self.as_mut().set_plan_error(QString::default());
        self.as_mut().set_result_message(QString::default());
        self.as_mut().set_progress(0.0);
        self.as_mut().set_phase(qs(PHASE_IDLE));
    }
}
