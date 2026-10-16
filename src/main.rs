#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use eframe::egui::{self, Color32, RichText, Stroke};
use std::time::{Duration, Instant};

const BG: Color32 = Color32::from_rgb(16, 18, 24);
const BOX: Color32 = Color32::from_rgb(30, 34, 44);
const BORDER: Color32 = Color32::from_rgb(23, 26, 34);
const ACCENT: Color32 = Color32::from_rgb(141, 128, 255);
const UNITS: [(&str, u64); 5] = [
    ("Days", 86_400_000),
    ("Hours", 3_600_000),
    ("Min", 60_000),
    ("Sec", 1000),
    ("Ms", 1),
];

#[repr(C)]
#[derive(Default)]
struct Point {
    x: i32,
    y: i32,
}

#[link(name = "user32")]
unsafe extern "system" {
    fn GetAsyncKeyState(key: i32) -> i16;
    fn GetCursorPos(point: *mut Point) -> i32;
    fn SetCursorPos(x: i32, y: i32) -> i32;
    fn GetForegroundWindow() -> *mut std::ffi::c_void;
    fn GetWindowThreadProcessId(window: *mut std::ffi::c_void, pid: *mut u32) -> u32;
    fn mouse_event(flags: u32, dx: u32, dy: u32, data: u32, extra: usize);
}

fn held(key: i32) -> bool {
    unsafe { GetAsyncKeyState(key) < 0 }
}
fn own_window_active() -> bool {
    let mut pid = 0;
    unsafe {
        GetWindowThreadProcessId(GetForegroundWindow(), &mut pid);
    }
    pid == std::process::id()
}

fn parse_time(fields: &[String; 5]) -> Result<Duration, String> {
    let mut total = 0u64;
    for (value, (_, scale)) in fields.iter().zip(UNITS) {
        let value = value.trim();
        let n = if value.is_empty() {
            0
        } else {
            if !value.bytes().all(|b| b.is_ascii_digit()) {
                return Err("Use whole, non-negative numbers in time fields.".into());
            }
            value
                .parse::<u64>()
                .map_err(|_| "Time value is too large.")?
        };
        total = total
            .checked_add(n.checked_mul(scale).ok_or("Time value is too large.")?)
            .ok_or("Time value is too large.")?;
    }
    Ok(Duration::from_millis(total))
}

struct Run {
    period: Duration,
    next: Instant,
    deadline: Option<Instant>,
    button: usize,
    target: Option<(i32, i32)>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct Hotkey {
    key: i32,
    ctrl: bool,
    alt: bool,
    shift: bool,
}

impl Hotkey {
    fn active(self) -> bool {
        held(self.key)
            && held(0x11) == self.ctrl
            && held(0x12) == self.alt
            && held(0x10) == self.shift
    }
    fn label(self) -> String {
        let mut text = String::new();
        if self.ctrl {
            text.push_str("Ctrl + ");
        }
        if self.alt {
            text.push_str("Alt + ");
        }
        if self.shift {
            text.push_str("Shift + ");
        }
        text.push_str(&key_label(self.key));
        text
    }
}

fn key_label(key: i32) -> String {
    match key {
        1 => "Left-click".into(),
        0x70..=0x7B => format!("F{}", key - 0x6F),
        _ => char::from_u32(key as u32).unwrap_or('?').to_string(),
    }
}

fn default_hotkeys() -> [Hotkey; 3] {
    [
        Hotkey {
            key: 0x75,
            ctrl: false,
            alt: false,
            shift: false,
        },
        Hotkey {
            key: 0x77,
            ctrl: false,
            alt: false,
            shift: false,
        },
        Hotkey {
            key: 1,
            ctrl: true,
            alt: false,
            shift: false,
        },
    ]
}

fn settings_path() -> std::path::PathBuf {
    std::env::var_os("LOCALAPPDATA")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
        .join("ClickerLoop")
        .join("hotkeys.txt")
}

fn read_hotkey_settings() -> std::io::Result<String> {
    let path = settings_path();
    match std::fs::read_to_string(&path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            // Preserve settings from installations predating the app rename.
            let legacy = path
                .parent()
                .unwrap()
                .parent()
                .unwrap()
                .join("Pulse")
                .join("hotkeys.txt");
            std::fs::read_to_string(legacy)
        }
        result => result,
    }
}

fn valid_hotkeys(keys: &[Hotkey; 3]) -> bool {
    keys[0] != keys[1]
        && keys[0] != keys[2]
        && keys[1] != keys[2]
        && (keys[2].key != 1 || keys[2].ctrl || keys[2].alt || keys[2].shift)
}

fn load_hotkeys() -> [Hotkey; 3] {
    let mut keys = default_hotkeys();
    if let Ok(text) = read_hotkey_settings() {
        let values: Vec<i32> = text
            .split_whitespace()
            .filter_map(|s| s.parse().ok())
            .collect();
        if values.len() == 12 {
            for (binding, data) in keys.iter_mut().zip(values.chunks_exact(4)) {
                if !(data[0] == 1
                    || (0x70..=0x7B).contains(&data[0])
                    || (65..=90).contains(&data[0])
                    || (48..=57).contains(&data[0]))
                {
                    return default_hotkeys();
                }
                *binding = Hotkey {
                    key: data[0],
                    ctrl: data[1] != 0,
                    alt: data[2] != 0,
                    shift: data[3] != 0,
                };
            }
            if valid_hotkeys(&keys) && keys[0].key != 1 && keys[1].key != 1 {
                return keys;
            }
        }
    }
    default_hotkeys()
}

struct ClickerLoop {
    interval: [String; 5],
    duration: [String; 5],
    timed: bool,
    fixed: bool,
    button: usize,
    point: Option<(i32, i32)>,
    run: Option<Run>,
    status: String,
    count: u64,
    hotkeys: [Hotkey; 3],
    draft_hotkeys: [Hotkey; 3],
    previous_hotkeys: [bool; 3],
    settings_open: bool,
    settings_error: String,
}

impl Default for ClickerLoop {
    fn default() -> Self {
        Self {
            interval: ["", "", "", "", "100"].map(String::from),
            duration: ["", "", "5", "", ""].map(String::from),
            timed: false,
            fixed: false,
            button: 0,
            point: None,
            run: None,
            status: "Ready when you are".into(),
            count: 0,
            hotkeys: load_hotkeys(),
            draft_hotkeys: load_hotkeys(),
            previous_hotkeys: [false; 3],
            settings_open: false,
            settings_error: String::new(),
        }
    }
}

impl ClickerLoop {
    fn hotkey_settings(&mut self, ctx: &egui::Context) {
        if !self.settings_open {
            return;
        }
        let mut open = true;
        egui::Window::new("Hotkey settings").open(&mut open).collapsible(false).resizable(false)
            .default_width(360.0).show(ctx, |ui| {
                ui.label("Choose a key and optional modifiers for each action.");
                ui.small("Shortcuts are global. Changes apply after saving.");
                for (index, title) in ["Start / stop", "Emergency stop", "Capture position"].iter().enumerate() {
                    ui.add_space(12.0);
                    caption(ui, title);
                    let binding = &mut self.draft_hotkeys[index];
                    ui.push_id(index, |ui| {
                        ui.horizontal(|ui| {
                            ui.checkbox(&mut binding.ctrl, "Ctrl");
                            ui.checkbox(&mut binding.alt, "Alt");
                            ui.checkbox(&mut binding.shift, "Shift");
                        });
                        egui::ComboBox::from_id_salt("key").selected_text(key_label(binding.key)).show_ui(ui, |ui| {
                            if index == 2 { ui.selectable_value(&mut binding.key, 1, "Left-click"); }
                            for key in (0x70..=0x7B).chain(65..=90).chain(48..=57) {
                                ui.selectable_value(&mut binding.key, key, key_label(key));
                            }
                        });
                    });
                }
                ui.add_space(12.0);
                ui.small("Left-click capture requires a modifier. Capture clicks also reach the target app.");
                if !self.settings_error.is_empty() { ui.colored_label(Color32::from_rgb(255, 135, 150), &self.settings_error); }
                ui.add_space(12.0);
                ui.horizontal(|ui| {
                    if ui.button("Reset defaults").clicked() { self.draft_hotkeys = default_hotkeys(); self.settings_error.clear(); }
                    if ui.button("Save").clicked() {
                        if !valid_hotkeys(&self.draft_hotkeys) {
                            self.settings_error = "Use distinct shortcuts; left-click capture needs a modifier.".into();
                        } else {
                            let path = settings_path();
                            let text = self.draft_hotkeys.iter().map(|key| format!("{} {} {} {}", key.key, key.ctrl as u8, key.alt as u8, key.shift as u8)).collect::<Vec<_>>().join("\n");
                            let result = std::fs::create_dir_all(path.parent().unwrap()).and_then(|_| std::fs::write(path, text));
                            match result {
                                Ok(()) => { self.hotkeys = self.draft_hotkeys; self.previous_hotkeys = self.hotkeys.map(Hotkey::active); self.settings_open = false; self.status = "Hotkeys saved".into(); }
                                Err(error) => self.settings_error = format!("Could not save: {error}"),
                            }
                        }
                    }
                    if ui.button("Cancel").clicked() { self.settings_open = false; }
                });
            });
        if !open {
            self.settings_open = false;
        }
    }

    fn stop(&mut self, message: &str) {
        self.run = None;
        self.status = message.into();
    }

    fn start(&mut self, now: Instant) -> Result<(), String> {
        let period = parse_time(&self.interval)?;
        if period.is_zero() {
            return Err("Click interval must be at least 1 ms.".into());
        }
        // Check arithmetic before running, including extremely large user-entered durations.
        now.checked_add(period)
            .ok_or("Click interval is too large.")?;
        let next = now + Duration::from_secs(3);
        let deadline = if self.timed {
            let duration = parse_time(&self.duration)?;
            if duration.is_zero() {
                return Err("Session duration must be greater than zero.".into());
            }
            Some(
                next.checked_add(duration)
                    .ok_or("Session duration is too large.")?,
            )
        } else {
            None
        };
        if self.fixed && self.point.is_none() {
            return Err(format!(
                "Capture a position with {} first.",
                self.hotkeys[2].label()
            ));
        }
        self.run = Some(Run {
            period,
            next,
            deadline,
            button: self.button,
            target: if self.fixed { self.point } else { None },
        });
        self.count = 0;
        self.status = "Starting in 3 seconds — switch to your target".into();
        Ok(())
    }

    fn toggle(&mut self) {
        if self.run.is_some() {
            self.stop("Stopped");
        } else if let Err(error) = self.start(Instant::now()) {
            self.status = error;
        }
    }

    fn tick(&mut self) {
        let active = self.hotkeys.map(Hotkey::active);
        if self.settings_open {
            self.previous_hotkeys = active;
            return;
        }
        if active[1] {
            self.stop("Emergency stop");
        } else if active[0] && !self.previous_hotkeys[0] {
            self.toggle();
        }
        if !active[1] && active[2] && !self.previous_hotkeys[2] {
            self.stop("Position captured — ready");
            let mut point = Point::default();
            if unsafe { GetCursorPos(&mut point) } != 0 {
                self.point = Some((point.x, point.y));
                self.fixed = true;
            } else {
                self.status = "Cannot read pointer position.".into();
            }
        }
        self.previous_hotkeys = active;
        self.advance(Instant::now());
    }

    fn advance(&mut self, now: Instant) {
        let shortcut_held = self.hotkeys.into_iter().any(Hotkey::active);
        let Some(run) = &mut self.run else {
            return;
        };
        if run.deadline.is_some_and(|deadline| now >= deadline) {
            self.stop("Session complete — time limit reached");
            return;
        }
        if now < run.next {
            return;
        }
        if shortcut_held
            || own_window_active()
            || [1, 2, 4, 0x10, 0x11, 0x12, 0x5B, 0x5C, 0x75]
                .into_iter()
                .any(held)
        {
            self.status = "Paused — switch apps and release held controls".into();
        } else {
            if let Some((x, y)) = run.target {
                if unsafe { SetCursorPos(x, y) } == 0 {
                    self.stop("Cannot move pointer to the selected position.");
                    return;
                }
            }
            let (down, up) = [(2, 4), (8, 16), (32, 64)][run.button];
            unsafe {
                mouse_event(down, 0, 0, 0, 0);
                mouse_event(up, 0, 0, 0, 0);
            }
            self.count = self.count.saturating_add(1);
            self.status = format!("Running · {} stops immediately", self.hotkeys[1].label());
        }
        // Always schedule from the present: never replay missed clicks in a burst.
        if let Some(next) = Instant::now().checked_add(run.period) {
            run.next = next;
        } else {
            self.stop("Stopped — interval exceeds the system clock range");
        }
    }
}

fn caption(ui: &mut egui::Ui, text: &str) {
    ui.label(
        RichText::new(text)
            .size(12.0)
            .color(Color32::from_rgb(146, 155, 176)),
    );
    ui.add_space(4.0);
}

fn time_fields(ui: &mut egui::Ui, id: &str, fields: &mut [String; 5]) {
    ui.push_id(id, |ui| {
        let width = (ui.available_width() - 32.0) / 5.0;
        ui.horizontal(|ui| {
            for (field, (label, _)) in fields.iter_mut().zip(UNITS) {
                ui.vertical(|ui| {
                    ui.set_width(width);
                    caption(ui, label);
                    ui.add_sized(
                        [width, 32.0],
                        egui::TextEdit::singleline(field)
                            .horizontal_align(egui::Align::Center)
                            .vertical_align(egui::Align::Center)
                            .font(egui::FontId::proportional(15.0)),
                    );
                });
            }
        });
    });
    ui.add_space(12.0);
}

impl eframe::App for ClickerLoop {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.tick();
        ctx.request_repaint_after(Duration::from_millis(if self.run.is_some() {
            1
        } else {
            16
        }));
        egui::TopBottomPanel::bottom("status_footer")
            .frame(
                egui::Frame::new()
                    .fill(BG)
                    .inner_margin(egui::Margin::symmetric(22, 12)),
            )
            .show(ctx, |ui| {
                ui.label(&self.status);
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    ui.small(format!("{} clicks", self.count));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(
                            RichText::new(format!("{}  STOP", self.hotkeys[1].label()))
                                .size(12.0)
                                .color(ACCENT),
                        );
                    });
                });
            });
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(BG).inner_margin(22.0))
            .show(ctx, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new("CLICKER LOOP")
                                .size(20.0)
                                .strong()
                                .color(ACCENT),
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui
                                .add_enabled(self.run.is_none(), egui::Button::new("Hotkeys"))
                                .clicked()
                            {
                                self.draft_hotkeys = self.hotkeys;
                                self.settings_error.clear();
                                self.settings_open = true;
                            }
                        });
                    });
                    ui.add_space(12.0);
                    ui.add_enabled_ui(self.run.is_none(), |ui| {
                        caption(ui, "MOUSE BUTTON");
                        egui::ComboBox::from_id_salt("button")
                            .width(ui.available_width() - 8.0)
                            .selected_text(["Left", "Right", "Middle"][self.button])
                            .show_ui(ui, |ui| {
                                for (index, label) in ["Left", "Right", "Middle"].iter().enumerate()
                                {
                                    ui.selectable_value(&mut self.button, index, *label);
                                }
                            });
                        ui.add_space(14.0);
                        caption(ui, "CLICK INTERVAL · BLANK = 0");
                        time_fields(ui, "interval", &mut self.interval);
                        caption(ui, "SESSION");
                        egui::ComboBox::from_id_salt("session")
                            .width(ui.available_width() - 8.0)
                            .selected_text(if self.timed {
                                "Run for a duration"
                            } else {
                                "Run until stopped"
                            })
                            .show_ui(ui, |ui| {
                                ui.selectable_value(&mut self.timed, false, "Run until stopped");
                                ui.selectable_value(&mut self.timed, true, "Run for a duration");
                            });
                        ui.add_space(12.0);
                        ui.add_enabled_ui(self.timed, |ui| {
                            caption(ui, "RUN DURATION · BLANK = 0");
                            time_fields(ui, "duration", &mut self.duration);
                        });
                        caption(ui, "CLICK LOCATION");
                        egui::ComboBox::from_id_salt("location")
                            .width(ui.available_width() - 8.0)
                            .selected_text(if self.fixed {
                                "Fixed position"
                            } else {
                                "Current pointer"
                            })
                            .show_ui(ui, |ui| {
                                ui.selectable_value(&mut self.fixed, false, "Current pointer");
                                ui.selectable_value(&mut self.fixed, true, "Fixed position");
                            });
                    });
                    ui.add_space(12.0);
                    egui::Frame::new()
                        .fill(BOX)
                        .stroke(Stroke::new(1.0_f32, BORDER))
                        .corner_radius(8)
                        .inner_margin(12.0)
                        .show(ui, |ui| {
                            ui.set_width(ui.available_width());
                            ui.strong(
                                self.point.map_or("No position selected".into(), |(x, y)| {
                                    format!("X  {x}     Y  {y}")
                                }),
                            );
                            ui.add_space(4.0);
                            ui.small(format!("{} to capture a pixel", self.hotkeys[2].label()));
                        });
                    ui.add_space(14.0);
                    caption(ui, "SAFE PACING");
                    ui.small("Single clicks · no catch-up bursts");
                    ui.small("Pauses over this app or while controls are held.");
                    ui.add_space(14.0);
                    let running = self.run.is_some();
                    if ui
                        .add_sized(
                            [ui.available_width(), 42.0],
                            egui::Button::new(
                                RichText::new(if running {
                                    format!("Stop clicking   /   {}", self.hotkeys[1].label())
                                } else {
                                    format!("Start clicking   /   {}", self.hotkeys[0].label())
                                })
                                .strong()
                                .color(BG),
                            )
                            .fill(if running {
                                Color32::from_rgb(255, 135, 150)
                            } else {
                                ACCENT
                            })
                            .stroke(Stroke::NONE),
                        )
                        .clicked()
                    {
                        self.toggle();
                    }
                });
            });
        self.hotkey_settings(ctx);
    }
}

fn main() -> eframe::Result {
    let icon = eframe::icon_data::from_png_bytes(include_bytes!("../assets/clicker-loop-icon.png"))
        .expect("Embedded ClickerLoop icon must be a valid PNG");
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_icon(icon)
            .with_inner_size([430.0, 760.0])
            .with_min_inner_size([410.0, 560.0])
            .with_resizable(true),
        ..Default::default()
    };
    eframe::run_native(
        "Clicker Loop - Autoclicker",
        options,
        Box::new(|cc| {
            let mut fonts = egui::FontDefinitions::default();
            if let Ok(data) = std::fs::read("C:/Windows/Fonts/segoeui.ttf") {
                fonts
                    .font_data
                    .insert("Segoe UI".into(), egui::FontData::from_owned(data).into());
                fonts
                    .families
                    .entry(egui::FontFamily::Proportional)
                    .or_default()
                    .insert(0, "Segoe UI".into());
            }
            cc.egui_ctx.set_fonts(fonts);
            let mut style = (*cc.egui_ctx.style()).clone();
            style
                .text_styles
                .insert(egui::TextStyle::Body, egui::FontId::proportional(14.0));
            style
                .text_styles
                .insert(egui::TextStyle::Button, egui::FontId::proportional(14.0));
            style
                .text_styles
                .insert(egui::TextStyle::Small, egui::FontId::proportional(12.0));
            style.visuals = egui::Visuals::dark();
            style.visuals.panel_fill = BG;
            style.visuals.extreme_bg_color = BOX;
            style.visuals.override_text_color = Some(Color32::from_rgb(237, 240, 247));
            for widget in [
                &mut style.visuals.widgets.noninteractive,
                &mut style.visuals.widgets.inactive,
                &mut style.visuals.widgets.hovered,
                &mut style.visuals.widgets.active,
                &mut style.visuals.widgets.open,
            ] {
                widget.bg_fill = BOX;
                widget.weak_bg_fill = BOX;
                widget.bg_stroke = Stroke::new(1.0_f32, BORDER);
                widget.corner_radius = 6.into();
            }
            style.spacing.button_padding = egui::vec2(10.0, 8.0);
            cc.egui_ctx.set_style(style);
            Ok(Box::new(ClickerLoop::default()))
        }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn hotkey_conflicts_and_labels() {
        let mut keys = default_hotkeys();
        assert!(valid_hotkeys(&keys));
        assert_eq!(keys[2].label(), "Ctrl + Left-click");
        keys[1] = keys[0];
        assert!(!valid_hotkeys(&keys));
        keys = default_hotkeys();
        keys[2].ctrl = false;
        assert!(!valid_hotkeys(&keys));
        keys[2].shift = true;
        assert!(valid_hotkeys(&keys));
        assert_eq!(keys[2].label(), "Shift + Left-click");
    }
    #[test]
    fn units_and_validation() {
        assert_eq!(
            parse_time(&["1", "1", "1", "10", "250"].map(String::from)).unwrap(),
            Duration::from_millis(90_070_250)
        );
        assert_eq!(parse_time(&Default::default()).unwrap(), Duration::ZERO);
        for value in ["-1", "nan", "1.5", "18446744073709551616"] {
            assert!(parse_time(&[value, "", "", "", ""].map(String::from)).is_err());
        }
    }
    #[test]
    fn sessions_and_boundaries() {
        let mut app = ClickerLoop::default();
        let now = Instant::now();
        app.start(now).unwrap();
        assert!(app.run.as_ref().unwrap().deadline.is_none());
        app.stop("test");
        app.timed = true;
        app.duration = ["1", "", "", "", ""].map(String::from);
        app.start(now).unwrap();
        let deadline = now + Duration::from_secs(86403);
        assert_eq!(app.run.as_ref().unwrap().deadline, Some(deadline));
        app.advance(deadline);
        assert!(app.run.is_none());
        app.duration = Default::default();
        assert!(app.start(now).is_err());
        app.timed = false;
        app.interval = Default::default();
        assert!(app.start(now).is_err());
        app.interval[4] = "1".into();
        app.start(now).unwrap();
        app.stop("test");
        app.fixed = true;
        assert!(app.start(now).is_err());
    }
}
