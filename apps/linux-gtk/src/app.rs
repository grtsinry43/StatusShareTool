use std::cell::RefCell;
use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::Path;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;
use std::thread;

use glib::{self, ControlFlow};
use gtk::prelude::*;
use gtk::{
    Application, ApplicationWindow, Box as GtkBox, Button, CheckButton, CssProvider, Entry, Frame,
    Grid, Image, Label, Orientation, ScrolledWindow, SpinButton, Stack, StackSwitcher, TextView,
    gdk, style_context_add_provider_for_display,
};
use reqwest::blocking::Client;
use statusshare_core::{
    CoreConfig, MatchEngineConfig, MediaInfo, PersistedConfig, PersistedConfigResult,
    StatusShareClient, default_config_file_path, default_persisted_config, load_persisted_config,
    save_persisted_config,
};

use crate::monitor::{MonitorControl, MonitorTick, start_monitoring};
use crate::rules_editor::RulesEditor;

#[derive(Clone)]
enum UiMessage {
    Output(String),
    FetchOutput(String),
    MonitorUpdate(Result<MonitorTick, String>),
}

pub fn run() {
    let app = Application::builder()
        .application_id("dev.grtsinry43.statusshare.linux")
        .build();

    app.connect_activate(build_ui);
    app.run();
}

fn build_ui(app: &Application) {
    install_css();

    let window = ApplicationWindow::builder()
        .application(app)
        .title("StatusShare GTK")
        .default_width(1040)
        .default_height(760)
        .build();

    let root = GtkBox::new(Orientation::Vertical, 16);
    root.set_margin_top(20);
    root.set_margin_bottom(20);
    root.set_margin_start(20);
    root.set_margin_end(20);

    let scroll = ScrolledWindow::builder()
        .hexpand(true)
        .vexpand(true)
        .build();
    scroll.set_policy(gtk::PolicyType::Automatic, gtk::PolicyType::Automatic);
    let content = GtkBox::new(Orientation::Vertical, 18);
    content.add_css_class("app-shell");

    let config_path_entry = Entry::builder()
        .hexpand(true)
        .text(default_config_file_path())
        .build();
    let base_url_entry = Entry::builder()
        .hexpand(true)
        .text("http://127.0.0.1:3000")
        .build();
    let token_entry = Entry::builder().hexpand(true).build();
    token_entry.set_placeholder_text(Some("gt_xxx"));
    token_entry.set_visibility(false);
    let interval_spin = SpinButton::with_range(5.0, 600.0, 5.0);
    interval_spin.set_value(10.0);
    let monitoring_status_entry = Entry::builder().hexpand(true).editable(false).build();
    monitoring_status_entry.set_text("Stopped");
    let summary_monitor_value = preview_value("Stopped");
    let summary_backend_value = preview_value("No data yet");
    let summary_reason_value = preview_value("Waiting");
    let summary_interval_value = preview_value("10s");

    let server_grid = Grid::builder().column_spacing(12).row_spacing(12).build();
    attach_row(&server_grid, 0, "Config Path", &config_path_entry);
    attach_row(&server_grid, 1, "Base URL", &base_url_entry);
    attach_row(&server_grid, 2, "Token", &token_entry);
    attach_row(&server_grid, 3, "Report Interval(s)", &interval_spin);
    attach_row(&server_grid, 4, "Monitor Status", &monitoring_status_entry);

    let server_buttons = GtkBox::new(Orientation::Horizontal, 8);
    let server_buttons_row1 = GtkBox::new(Orientation::Horizontal, 8);
    let load_button = Button::with_label("Load Config");
    let save_button = Button::with_label("Save Config");
    let fetch_button = Button::with_label("Fetch Current Server Status");
    let start_button = Button::builder()
        .icon_name("media-playback-start-symbolic")
        .tooltip_text("Start Monitor")
        .build();
    let stop_button = Button::builder()
        .icon_name("media-playback-stop-symbolic")
        .tooltip_text("Stop Monitor")
        .build();
    for button in [&load_button, &save_button, &fetch_button] {
        button.set_hexpand(true);
        server_buttons_row1.append(button);
    }
    server_buttons.append(&server_buttons_row1);

    let matching_grid = Grid::builder().column_spacing(12).row_spacing(12).build();
    let default_report_switch = CheckButton::with_label("未命中规则时默认上报");
    default_report_switch.set_active(true);
    let default_display_entry = Entry::builder().hexpand(true).build();
    let default_extend_entry = Entry::builder().hexpand(true).build();
    matching_grid.attach(&default_report_switch, 1, 0, 1, 1);
    attach_row(&matching_grid, 1, "Default Name", &default_display_entry);
    attach_row(&matching_grid, 2, "Default Extend", &default_extend_entry);

    let rules_editor = RulesEditor::new();

    let backend_value = preview_value("No data yet");
    let window_title_value = preview_value("Waiting for monitor");
    let app_name_value = preview_value("-");
    let process_name_value = preview_value("-");
    let executable_path_value = preview_value("-");
    let bundle_id_value = preview_value("-");
    let resolved_name_value = preview_value("-");
    let resolved_extend_value = preview_value("-");
    resolved_extend_value.set_wrap(true);
    let matched_rule_value = preview_value("-");
    let report_reason_value = preview_value("Waiting");
    let media_title_value = preview_value("-");
    let media_artist_value = preview_value("-");
    let media_thumbnail_value = preview_value("-");
    media_thumbnail_value.set_wrap(true);
    let home_media_title_value = preview_value("-");
    home_media_title_value.add_css_class("media-title");
    let home_media_artist_value = preview_value("-");
    home_media_artist_value.add_css_class("media-artist");
    let home_media_cover = Image::from_icon_name("audio-x-generic-symbolic");
    home_media_cover.set_pixel_size(56);

    let final_preview_card = build_home_card(
        &start_button,
        &stop_button,
        &summary_monitor_value,
        &summary_backend_value,
        &summary_reason_value,
        &summary_interval_value,
        &resolved_name_value,
        &resolved_extend_value,
        &home_media_title_value,
        &home_media_artist_value,
        &home_media_cover,
    );

    let output_view = TextView::builder()
        .editable(false)
        .monospace(true)
        .vexpand(true)
        .hexpand(true)
        .build();
    let output_buffer = output_view.buffer();
    let output_scroll = ScrolledWindow::builder()
        .hexpand(true)
        .vexpand(true)
        .min_content_height(260)
        .child(&output_view)
        .build();

    let header = GtkBox::new(Orientation::Vertical, 6);
    let title_label = Label::new(Some("StatusShare for Linux"));
    title_label.add_css_class("hero-title");
    title_label.set_xalign(0.0);
    let subtitle_label = Label::new(Some(
        "自动读取当前活动窗口与媒体信息，按规则决定是否上报；状态变更立即推送，稳定状态按配置间隔心跳。",
    ));
    subtitle_label.add_css_class("hero-subtitle");
    subtitle_label.set_wrap(true);
    subtitle_label.set_xalign(0.0);
    header.append(&title_label);
    header.append(&subtitle_label);

    let home_page = build_tab_page(&[final_preview_card]);

    let (connection_frame, connection_box) = section_card(
        "Connection",
        "站点地址、token、配置文件路径和监控控制都在这里。",
    );
    connection_box.append(&server_grid);
    connection_box.append(&server_buttons);
    let config_page = build_tab_page(&[connection_frame]);

    let (rules_frame, rules_box) = section_card(
        "上报规则",
        "通过表单管理窗口匹配规则。先决定默认行为，再按窗口类型补规则即可。",
    );
    rules_box.append(&matching_grid);
    rules_box.append(&rules_editor.widget());
    let rules_page = build_tab_page(&[rules_frame]);

    let (window_debug_frame, window_debug_box) = preview_card(
        "Window Metadata",
        "这是 Linux 侧当前抓到的窗口元信息，用来排查为什么命中或没命中某条规则。",
    );
    window_debug_box.append(&preview_row("Backend", &backend_value));
    window_debug_box.append(&preview_row("Window Title", &window_title_value));
    window_debug_box.append(&preview_row("App Name", &app_name_value));
    window_debug_box.append(&preview_row("Process Name", &process_name_value));
    window_debug_box.append(&preview_row("Executable Path", &executable_path_value));
    window_debug_box.append(&preview_row("Bundle ID", &bundle_id_value));

    let (resolve_debug_frame, resolve_debug_box) = preview_card(
        "Resolve Result",
        "这里显示本轮匹配结果和调度原因，便于确认当前到底会不会发请求。",
    );
    resolve_debug_box.append(&preview_row("Matched Rule", &matched_rule_value));
    resolve_debug_box.append(&preview_row("Resolved Name", &resolved_name_value));
    resolve_debug_box.append(&preview_row("Resolved Extend", &resolved_extend_value));
    resolve_debug_box.append(&preview_row("Push Reason", &report_reason_value));
    resolve_debug_box.append(&preview_row("Media Title", &media_title_value));
    resolve_debug_box.append(&preview_row("Media Artist", &media_artist_value));
    resolve_debug_box.append(&preview_row("Media Thumbnail", &media_thumbnail_value));

    let debug_page = build_tab_page(&[window_debug_frame, resolve_debug_frame]);

    let (output_frame, output_box) =
        section_card("日志", "最近一次监控结果、调度决策和接口响应会显示在这里。");
    output_box.append(&output_scroll);
    let logs_page = build_tab_page(&[output_frame]);

    let stack = Stack::new();
    stack.set_hexpand(true);
    stack.set_vexpand(true);
    stack.set_transition_type(gtk::StackTransitionType::SlideLeftRight);
    stack.add_titled(&home_page, Some("home"), "首页");
    stack.add_titled(&config_page, Some("config"), "配置");
    stack.add_titled(&rules_page, Some("rules"), "上报规则");
    stack.add_titled(&debug_page, Some("debug"), "当前调试");
    stack.add_titled(&logs_page, Some("logs"), "日志");

    let switcher = StackSwitcher::new();
    switcher.set_stack(Some(&stack));
    switcher.set_halign(gtk::Align::Start);
    switcher.add_css_class("tab-switcher");

    content.append(&header);
    content.append(&switcher);
    content.append(&stack);

    scroll.set_child(Some(&content));
    root.append(&scroll);
    window.set_child(Some(&root));

    let client = StatusShareClient::new(CoreConfig::default());
    let monitor_control: Rc<RefCell<Option<MonitorControl>>> = Rc::new(RefCell::new(None));
    let (tx, rx) = std::sync::mpsc::channel::<UiMessage>();

    {
        let output_buffer = output_buffer.clone();
        let monitoring_status_entry = monitoring_status_entry.clone();
        let summary_monitor_value = summary_monitor_value.clone();
        let summary_backend_value = summary_backend_value.clone();
        let summary_reason_value = summary_reason_value.clone();
        let backend_value = backend_value.clone();
        let window_title_value = window_title_value.clone();
        let app_name_value = app_name_value.clone();
        let process_name_value = process_name_value.clone();
        let executable_path_value = executable_path_value.clone();
        let bundle_id_value = bundle_id_value.clone();
        let resolved_name_value = resolved_name_value.clone();
        let resolved_extend_value = resolved_extend_value.clone();
        let report_reason_value = report_reason_value.clone();
        let media_title_value = media_title_value.clone();
        let media_artist_value = media_artist_value.clone();
        let media_thumbnail_value = media_thumbnail_value.clone();
        let home_media_title_value = home_media_title_value.clone();
        let home_media_artist_value = home_media_artist_value.clone();
        let home_media_cover = home_media_cover.clone();

        glib::timeout_add_local(std::time::Duration::from_millis(200), move || {
            loop {
                match rx.try_recv() {
                    Ok(UiMessage::Output(text)) | Ok(UiMessage::FetchOutput(text)) => {
                        output_buffer.set_text(&text)
                    }
                    Ok(UiMessage::MonitorUpdate(result)) => match result {
                        Ok(tick) => {
                            monitoring_status_entry.set_text("Running");
                            summary_monitor_value.set_text("Running");
                            summary_backend_value.set_text(&tick.backend);
                            summary_reason_value.set_text(&format!("{:?}", tick.decision.reason));
                            backend_value.set_text(&tick.backend);
                            window_title_value.set_text(&tick.window.window_title);
                            app_name_value.set_text(&tick.window.app_name);
                            process_name_value.set_text(&tick.window.process_name);
                            executable_path_value.set_text(&tick.window.executable_path);
                            bundle_id_value.set_text(&tick.window.bundle_id);
                            matched_rule_value.set_text(
                                if tick.resolve.matched_rule_id.is_empty() {
                                    "-"
                                } else {
                                    &tick.resolve.matched_rule_id
                                },
                            );
                            resolved_name_value.set_text(&tick.resolve.process);
                            resolved_extend_value.set_text(&tick.resolve.extend);
                            report_reason_value.set_text(&format!("{:?}", tick.decision.reason));
                            apply_media_to_labels(
                                &media_title_value,
                                &media_artist_value,
                                &media_thumbnail_value,
                                tick.media.as_ref(),
                            );
                            apply_media_to_home_card(
                                &home_media_title_value,
                                &home_media_artist_value,
                                &home_media_cover,
                                tick.media.as_ref(),
                            );

                            let text = if let Some(api) = tick.api_result.as_ref() {
                                format!(
                                    "monitor tick\n\nresolve\n{}\n\nschedule\n{}\n\napi\n{}",
                                    serde_pretty(&tick.resolve),
                                    serde_pretty(&tick.decision),
                                    serde_pretty(api)
                                )
                            } else {
                                format!(
                                    "monitor tick\n\nresolve\n{}\n\nschedule\n{}",
                                    serde_pretty(&tick.resolve),
                                    serde_pretty(&tick.decision)
                                )
                            };
                            output_buffer.set_text(&text);
                        }
                        Err(err) => {
                            monitoring_status_entry.set_text("Running with errors");
                            summary_monitor_value.set_text("Running with errors");
                            summary_reason_value.set_text("Error");
                            output_buffer.set_text(&format!("monitor error\n\n{err}"));
                        }
                    },
                    Err(std::sync::mpsc::TryRecvError::Empty) => break,
                    Err(std::sync::mpsc::TryRecvError::Disconnected) => return ControlFlow::Break,
                }
            }
            ControlFlow::Continue
        });
    }

    {
        let config_path_entry = config_path_entry.clone();
        let base_url_entry = base_url_entry.clone();
        let token_entry = token_entry.clone();
        let interval_spin = interval_spin.clone();
        let default_report_switch = default_report_switch.clone();
        let default_display_entry = default_display_entry.clone();
        let default_extend_entry = default_extend_entry.clone();
        let rules_editor = rules_editor.clone();
        let output_buffer = output_buffer.clone();

        load_button.connect_clicked(move |_| {
            let result = load_persisted_config(config_path_entry.text().to_string());
            if result.success {
                if let Some(config) = result.config.as_ref() {
                    apply_persisted_config_to_widgets(
                        &base_url_entry,
                        &token_entry,
                        &interval_spin,
                        &default_report_switch,
                        &default_display_entry,
                        &default_extend_entry,
                        &rules_editor,
                        config,
                    );
                }
            }
            output_buffer.set_text(&serde_pretty(&result));
        });
    }

    {
        let config_path_entry = config_path_entry.clone();
        let base_url_entry = base_url_entry.clone();
        let token_entry = token_entry.clone();
        let interval_spin = interval_spin.clone();
        let default_report_switch = default_report_switch.clone();
        let default_display_entry = default_display_entry.clone();
        let default_extend_entry = default_extend_entry.clone();
        let rules_editor = rules_editor.clone();
        let output_buffer = output_buffer.clone();

        save_button.connect_clicked(move |_| {
            let result = match collect_persisted_config(
                &base_url_entry,
                &token_entry,
                &interval_spin,
                &default_report_switch,
                &default_display_entry,
                &default_extend_entry,
                &rules_editor,
            ) {
                Ok(config) => save_persisted_config(config_path_entry.text().to_string(), config),
                Err(err) => PersistedConfigResult {
                    success: false,
                    path: config_path_entry.text().to_string(),
                    error_message: err,
                    config: None,
                },
            };
            output_buffer.set_text(&serde_pretty(&result));
        });
    }

    {
        let client = Arc::clone(&client);
        let tx = tx.clone();
        let base_url_entry = base_url_entry.clone();
        let token_entry = token_entry.clone();
        let interval_spin = interval_spin.clone();
        fetch_button.connect_clicked(move |_| {
            let config = collect_core_config(&base_url_entry, &token_entry, &interval_spin);
            client.update_config(config);
            let client = Arc::clone(&client);
            let tx = tx.clone();
            thread::spawn(move || {
                let result = client.fetch_status();
                let _ = tx.send(UiMessage::FetchOutput(serde_pretty(&result)));
            });
        });
    }

    {
        let tx = tx.clone();
        let monitor_control = Rc::clone(&monitor_control);
        let monitoring_status_entry = monitoring_status_entry.clone();
        let summary_monitor_value = summary_monitor_value.clone();
        let base_url_entry = base_url_entry.clone();
        let token_entry = token_entry.clone();
        let interval_spin = interval_spin.clone();
        let default_report_switch = default_report_switch.clone();
        let default_display_entry = default_display_entry.clone();
        let default_extend_entry = default_extend_entry.clone();
        let rules_editor = rules_editor.clone();

        start_button.connect_clicked(move |_| {
            if let Some(control) = monitor_control.borrow_mut().take() {
                control.stop();
            }

            let config = match collect_persisted_config(
                &base_url_entry,
                &token_entry,
                &interval_spin,
                &default_report_switch,
                &default_display_entry,
                &default_extend_entry,
                &rules_editor,
            ) {
                Ok(config) => config,
                Err(err) => {
                    let _ = tx.send(UiMessage::Output(err));
                    return;
                }
            };

            monitoring_status_entry.set_text("Starting");
            summary_monitor_value.set_text("Starting");
            let control = start_monitoring(config, {
                let tx = tx.clone();
                let (internal_tx, internal_rx) =
                    std::sync::mpsc::channel::<Result<MonitorTick, String>>();
                thread::spawn(move || {
                    while let Ok(message) = internal_rx.recv() {
                        let _ = tx.send(UiMessage::MonitorUpdate(message));
                    }
                });
                internal_tx
            });
            *monitor_control.borrow_mut() = Some(control);
        });
    }

    {
        let monitor_control = Rc::clone(&monitor_control);
        let monitoring_status_entry = monitoring_status_entry.clone();
        let summary_monitor_value = summary_monitor_value.clone();
        let summary_reason_value = summary_reason_value.clone();
        let output_buffer = output_buffer.clone();
        stop_button.connect_clicked(move |_| {
            if let Some(control) = monitor_control.borrow_mut().take() {
                control.stop();
            }
            monitoring_status_entry.set_text("Stopped");
            summary_monitor_value.set_text("Stopped");
            summary_reason_value.set_text("Stopped");
            output_buffer.set_text("monitor stopped");
        });
    }

    {
        let summary_interval_value = summary_interval_value.clone();
        interval_spin.connect_value_changed(move |spin| {
            summary_interval_value.set_text(&format!("{}s", spin.value_as_int()));
        });
    }

    if let Some(loaded) = load_or_default_config() {
        apply_persisted_config_to_widgets(
            &base_url_entry,
            &token_entry,
            &interval_spin,
            &default_report_switch,
            &default_display_entry,
            &default_extend_entry,
            &rules_editor,
            &loaded,
        );
    }

    output_buffer.set_text(
        "准备就绪。\n\n建议顺序：\n1. 确认 Base URL 和 gt_ token\n2. 先设置默认名称、默认文案，再补充窗口规则\n3. 点击 Start Monitor 开始自动监控",
    );
    window.present();
}

fn section_card(title: &str, description: &str) -> (Frame, GtkBox) {
    let frame = Frame::new(None);
    frame.add_css_class("section-card");

    let wrapper = GtkBox::new(Orientation::Vertical, 12);
    wrapper.set_margin_top(16);
    wrapper.set_margin_bottom(16);
    wrapper.set_margin_start(16);
    wrapper.set_margin_end(16);

    let title_label = Label::new(Some(title));
    title_label.add_css_class("section-title");
    title_label.set_xalign(0.0);
    let desc_label = Label::new(Some(description));
    desc_label.add_css_class("section-subtitle");
    desc_label.set_wrap(true);
    desc_label.set_xalign(0.0);

    wrapper.append(&title_label);
    wrapper.append(&desc_label);
    frame.set_child(Some(&wrapper));
    (frame, wrapper)
}

fn build_tab_page(sections: &[Frame]) -> ScrolledWindow {
    let wrapper = GtkBox::new(Orientation::Vertical, 16);
    wrapper.set_margin_top(8);
    wrapper.set_margin_bottom(8);
    wrapper.set_margin_start(4);
    wrapper.set_margin_end(4);
    for section in sections {
        wrapper.append(section);
    }

    let scroll = ScrolledWindow::builder()
        .hexpand(true)
        .vexpand(true)
        .child(&wrapper)
        .build();
    scroll.set_policy(gtk::PolicyType::Automatic, gtk::PolicyType::Automatic);
    scroll
}

fn build_home_card(
    start_button: &Button,
    stop_button: &Button,
    monitor_value: &Label,
    backend_value: &Label,
    push_reason_value: &Label,
    interval_value: &Label,
    resolved_name_value: &Label,
    resolved_extend_value: &Label,
    media_title_value: &Label,
    media_artist_value: &Label,
    media_cover: &Image,
) -> Frame {
    let frame = Frame::new(None);
    frame.add_css_class("home-card");
    let content = GtkBox::new(Orientation::Vertical, 14);
    content.set_margin_top(18);
    content.set_margin_bottom(18);
    content.set_margin_start(18);
    content.set_margin_end(18);

    let hero_value = Label::new(Some("-"));
    hero_value.add_css_class("hero-preview-title");
    hero_value.set_xalign(0.0);
    hero_value.set_wrap(true);

    let hero_extend = Label::new(Some("-"));
    hero_extend.add_css_class("hero-preview-extend");
    hero_extend.set_xalign(0.0);
    hero_extend.set_wrap(true);

    let header_row = GtkBox::new(Orientation::Horizontal, 12);
    let heading_box = GtkBox::new(Orientation::Horizontal, 6);
    let prefix_label = Label::new(Some("正在使用"));
    prefix_label.add_css_class("home-prefix");
    prefix_label.set_xalign(0.0);
    heading_box.append(&prefix_label);
    heading_box.append(&hero_value);
    header_row.append(&heading_box);

    let action_row = GtkBox::new(Orientation::Horizontal, 8);
    action_row.set_halign(gtk::Align::End);
    action_row.set_hexpand(true);
    start_button.add_css_class("icon-action");
    stop_button.add_css_class("icon-action");
    action_row.append(start_button);
    action_row.append(stop_button);
    header_row.append(&action_row);

    resolved_name_value
        .bind_property("label", &hero_value, "label")
        .build();
    resolved_extend_value
        .bind_property("label", &hero_extend, "label")
        .build();

    let summary_row = GtkBox::new(Orientation::Horizontal, 10);
    summary_row.set_homogeneous(true);
    summary_row.append(&metric_chip("Monitor", monitor_value));
    summary_row.append(&metric_chip("Backend", backend_value));
    summary_row.append(&metric_chip("Reason", push_reason_value));
    summary_row.append(&metric_chip("Interval", interval_value));

    let media_frame = Frame::new(None);
    media_frame.add_css_class("media-card");
    let media_box = GtkBox::new(Orientation::Horizontal, 12);
    media_box.set_margin_top(12);
    media_box.set_margin_bottom(12);
    media_box.set_margin_start(12);
    media_box.set_margin_end(12);
    let media_cover_frame = Frame::new(None);
    media_cover_frame.add_css_class("media-cover-frame");
    media_cover_frame.set_child(Some(media_cover));
    let media_text_box = GtkBox::new(Orientation::Vertical, 4);
    media_text_box.set_valign(gtk::Align::Center);
    media_text_box.append(media_title_value);
    media_text_box.append(media_artist_value);
    media_box.append(&media_cover_frame);
    media_box.append(&media_text_box);
    media_frame.set_child(Some(&media_box));

    content.append(&header_row);
    content.append(&hero_extend);
    content.append(&summary_row);
    content.append(&media_frame);
    frame.set_child(Some(&content));

    frame
}

fn metric_chip(title: &str, value: &Label) -> Frame {
    let frame = Frame::new(None);
    frame.add_css_class("metric-chip");

    let wrapper = GtkBox::new(Orientation::Vertical, 4);
    wrapper.set_margin_top(10);
    wrapper.set_margin_bottom(10);
    wrapper.set_margin_start(10);
    wrapper.set_margin_end(10);

    let title_label = Label::new(Some(title));
    title_label.add_css_class("preview-row-title");
    title_label.set_xalign(0.0);

    wrapper.append(&title_label);
    wrapper.append(value);
    frame.set_child(Some(&wrapper));
    frame
}

fn preview_card(title: &str, description: &str) -> (Frame, GtkBox) {
    let frame = Frame::new(None);
    frame.add_css_class("preview-card");

    let wrapper = GtkBox::new(Orientation::Vertical, 10);
    let title_label = Label::new(Some(title));
    title_label.add_css_class("preview-title");
    title_label.set_xalign(0.0);
    let desc_label = Label::new(Some(description));
    desc_label.add_css_class("section-subtitle");
    desc_label.set_wrap(true);
    desc_label.set_xalign(0.0);
    wrapper.set_margin_top(14);
    wrapper.set_margin_bottom(14);
    wrapper.set_margin_start(14);
    wrapper.set_margin_end(14);
    wrapper.append(&title_label);
    wrapper.append(&desc_label);
    frame.set_child(Some(&wrapper));
    (frame, wrapper)
}

fn preview_row(title: &str, value: &Label) -> GtkBox {
    let row = GtkBox::new(Orientation::Vertical, 4);
    row.add_css_class("preview-row");
    let title_label = Label::new(Some(title));
    title_label.add_css_class("preview-row-title");
    title_label.set_xalign(0.0);
    row.append(&title_label);
    row.append(value);
    row
}

fn preview_value(initial: &str) -> Label {
    let label = Label::new(Some(initial));
    label.add_css_class("preview-value");
    label.set_wrap(true);
    label.set_selectable(true);
    label.set_xalign(0.0);
    label
}

fn install_css() {
    let provider = CssProvider::new();
    provider.load_from_data(
        r#"
        .hero-title {
            font-size: 28px;
            font-weight: 700;
        }

        .hero-subtitle {
            font-size: 14px;
        }

        .section-card {
            border-radius: 18px;
        }

        .preview-card {
            border-radius: 16px;
        }

        .home-card {
            border-radius: 22px;
        }

        .metric-chip {
            border-radius: 14px;
        }

        .media-card {
            border-radius: 14px;
        }

        .media-cover-frame {
            border-radius: 12px;
            min-width: 56px;
            min-height: 56px;
        }

        .section-title {
            font-size: 18px;
            font-weight: 700;
        }

        .home-prefix {
            font-size: 18px;
            font-weight: 700;
        }

        .preview-title {
            font-size: 16px;
            font-weight: 700;
        }

        .section-subtitle {
            font-size: 13px;
        }

        .preview-row {
            margin-top: 4px;
            margin-bottom: 4px;
        }

        .preview-row-title {
            font-size: 12px;
            font-weight: 700;
            opacity: 0.75;
        }

        .preview-value {
            font-size: 15px;
        }

        .media-title {
            font-size: 16px;
            font-weight: 600;
        }

        .media-artist {
            font-size: 14px;
            opacity: 0.72;
        }

        .hero-preview-title {
            font-size: 20px;
            font-weight: 700;
        }

        .hero-preview-extend {
            font-size: 15px;
            margin-bottom: 12px;
        }

        .icon-action {
            min-width: 38px;
            min-height: 38px;
            border-radius: 999px;
        }
        "#,
    );

    if let Some(display) = gdk::Display::default() {
        style_context_add_provider_for_display(
            &display,
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }
}

fn attach_row<W>(grid: &Grid, row: i32, label: &str, widget: &W)
where
    W: IsA<gtk::Widget>,
{
    let label_widget = Label::new(Some(label));
    label_widget.set_xalign(0.0);
    grid.attach(&label_widget, 0, row, 1, 1);
    grid.attach(widget, 1, row, 1, 1);
}

fn collect_core_config(
    base_url_entry: &Entry,
    token_entry: &Entry,
    interval_spin: &SpinButton,
) -> CoreConfig {
    CoreConfig {
        base_url: base_url_entry.text().to_string(),
        token: token_entry.text().to_string(),
        heartbeat_interval_secs: interval_spin.value() as u64,
        user_agent: "StatusShare GTK/0.1.0".to_string(),
    }
}

fn collect_persisted_config(
    base_url_entry: &Entry,
    token_entry: &Entry,
    interval_spin: &SpinButton,
    default_report_switch: &CheckButton,
    default_display_entry: &Entry,
    default_extend_entry: &Entry,
    rules_editor: &RulesEditor,
) -> Result<PersistedConfig, String> {
    Ok(PersistedConfig {
        schema_version: 1,
        core: collect_core_config(base_url_entry, token_entry, interval_spin),
        matching: collect_matching_config(
            default_report_switch,
            default_display_entry,
            default_extend_entry,
            rules_editor,
        )?,
    })
}

fn collect_matching_config(
    default_report_switch: &CheckButton,
    default_display_entry: &Entry,
    default_extend_entry: &Entry,
    rules_editor: &RulesEditor,
) -> Result<MatchEngineConfig, String> {
    Ok(MatchEngineConfig {
        default_report: default_report_switch.is_active(),
        default_display_name: default_display_entry.text().to_string(),
        default_extend: default_extend_entry.text().to_string(),
        rules: rules_editor.rules(),
    })
}

fn apply_persisted_config_to_widgets(
    base_url_entry: &Entry,
    token_entry: &Entry,
    interval_spin: &SpinButton,
    default_report_switch: &CheckButton,
    default_display_entry: &Entry,
    default_extend_entry: &Entry,
    rules_editor: &RulesEditor,
    config: &PersistedConfig,
) {
    base_url_entry.set_text(&config.core.base_url);
    token_entry.set_text(&config.core.token);
    interval_spin.set_value(config.core.heartbeat_interval_secs as f64);
    default_report_switch.set_active(config.matching.default_report);
    default_display_entry.set_text(&config.matching.default_display_name);
    default_extend_entry.set_text(&config.matching.default_extend);
    rules_editor.set_rules(&config.matching.rules);
}

fn load_or_default_config() -> Option<PersistedConfig> {
    let loaded = load_persisted_config(default_config_file_path());
    if loaded.success {
        loaded.config
    } else {
        Some(default_persisted_config())
    }
}

fn apply_media_to_labels(
    title_label: &Label,
    artist_label: &Label,
    thumbnail_label: &Label,
    media: Option<&MediaInfo>,
) {
    if let Some(media) = media {
        title_label.set_text(&media.title);
        artist_label.set_text(&media.artist);
        thumbnail_label.set_text(&media.thumbnail);
    } else {
        title_label.set_text("-");
        artist_label.set_text("-");
        thumbnail_label.set_text("-");
    }
}

fn apply_media_to_home_card(
    title_label: &Label,
    artist_label: &Label,
    cover_image: &Image,
    media: Option<&MediaInfo>,
) {
    if let Some(media) = media {
        title_label.set_text(non_empty_label(&media.title, "No media title"));
        artist_label.set_text(non_empty_label(&media.artist, "Unknown artist"));
        apply_thumbnail_image(cover_image, &media.thumbnail);
    } else {
        title_label.set_text("暂无媒体");
        artist_label.set_text("当前没有可上报的媒体信息");
        cover_image.set_icon_name(Some("audio-x-generic-symbolic"));
        cover_image.set_pixel_size(56);
    }
}

fn apply_thumbnail_image(image: &Image, thumbnail: &str) {
    let trimmed = thumbnail.trim();
    if trimmed.is_empty() {
        image.set_icon_name(Some("audio-x-generic-symbolic"));
        image.set_pixel_size(56);
        return;
    }

    if trimmed.starts_with("file://") {
        if let Some(path) = file_uri_to_path(trimmed) {
            image.set_from_file(Some(path));
            image.set_pixel_size(56);
            return;
        }
    }

    if Path::new(trimmed).exists() {
        image.set_from_file(Some(trimmed));
        image.set_pixel_size(56);
        return;
    }

    if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
        if let Some(path) = cached_thumbnail_path(trimmed) {
            image.set_from_file(Some(path));
            image.set_pixel_size(56);
            return;
        }
    }

    image.set_icon_name(Some("image-x-generic-symbolic"));
    image.set_pixel_size(56);
}

fn non_empty_label<'a>(value: &'a str, fallback: &'a str) -> &'a str {
    if value.trim().is_empty() {
        fallback
    } else {
        value
    }
}

fn file_uri_to_path(uri: &str) -> Option<PathBuf> {
    let stripped = uri.strip_prefix("file://")?;
    Some(PathBuf::from(stripped))
}

fn cached_thumbnail_path(url: &str) -> Option<PathBuf> {
    let cache_dir = std::env::temp_dir().join("statussharetool-media-cache");
    if fs::create_dir_all(&cache_dir).is_err() {
        return None;
    }

    let mut hasher = DefaultHasher::new();
    url.hash(&mut hasher);
    let cache_path = cache_dir.join(format!("{:x}.img", hasher.finish()));

    if cache_path.exists() {
        return Some(cache_path);
    }

    let response = Client::new().get(url).send().ok()?;
    if !response.status().is_success() {
        return None;
    }
    let bytes = response.bytes().ok()?;
    fs::write(&cache_path, bytes).ok()?;
    Some(cache_path)
}

fn serde_pretty<T>(value: &T) -> String
where
    T: serde::Serialize,
{
    serde_json::to_string_pretty(value).unwrap_or_else(|err| err.to_string())
}
