use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use glib::{self, ControlFlow};
use gtk::prelude::*;
use gtk::{
    Application, ApplicationWindow, Box as GtkBox, Button, CheckButton, Entry, Grid, Label,
    Orientation, ScrolledWindow, SpinButton, TextBuffer, TextView,
};
use statusshare_core::{CoreConfig, MediaInfo, StatusShareClient, StatusUpdate};

#[derive(Clone)]
enum UiMessage {
    Info(String),
}

fn main() {
    let app = Application::builder()
        .application_id("dev.grtsinry43.statusshare.linux")
        .build();

    app.connect_activate(build_ui);
    app.run();
}

fn build_ui(app: &Application) {
    let window = ApplicationWindow::builder()
        .application(app)
        .title("StatusShare GTK")
        .default_width(920)
        .default_height(760)
        .build();

    let root = GtkBox::new(Orientation::Vertical, 12);
    root.set_margin_top(24);
    root.set_margin_bottom(24);
    root.set_margin_start(24);
    root.set_margin_end(24);

    let grid = Grid::builder().column_spacing(12).row_spacing(12).build();

    let base_url_entry = Entry::builder()
        .hexpand(true)
        .text("http://127.0.0.1:3000")
        .build();
    let token_entry = Entry::builder().hexpand(true).build();
    token_entry.set_placeholder_text(Some("gt_xxx"));
    token_entry.set_visibility(false);
    let interval_spin = SpinButton::with_range(5.0, 600.0, 5.0);
    interval_spin.set_value(60.0);
    let process_entry = Entry::builder().hexpand(true).build();
    let extend_entry = Entry::builder().hexpand(true).build();
    let media_title_entry = Entry::builder().hexpand(true).build();
    let media_artist_entry = Entry::builder().hexpand(true).build();
    let media_thumbnail_entry = Entry::builder().hexpand(true).build();
    let ok_switch = CheckButton::with_label("在线 / OK = 1");
    ok_switch.set_active(true);

    attach_row(&grid, 0, "Base URL", &base_url_entry);
    attach_row(&grid, 1, "Token", &token_entry);
    attach_row(&grid, 2, "Heartbeat(s)", &interval_spin);
    attach_row(&grid, 3, "Process", &process_entry);
    attach_row(&grid, 4, "Extend", &extend_entry);
    attach_row(&grid, 5, "Media Title", &media_title_entry);
    attach_row(&grid, 6, "Media Artist", &media_artist_entry);
    attach_row(&grid, 7, "Media Thumbnail", &media_thumbnail_entry);
    grid.attach(&ok_switch, 1, 8, 1, 1);

    let button_row = GtkBox::new(Orientation::Horizontal, 12);
    let fetch_button = Button::with_label("Fetch");
    let send_button = Button::with_label("Push Once");
    let start_button = Button::with_label("Start Heartbeat");
    let stop_button = Button::with_label("Stop Heartbeat");
    button_row.append(&fetch_button);
    button_row.append(&send_button);
    button_row.append(&start_button);
    button_row.append(&stop_button);

    let output_view = TextView::builder()
        .editable(false)
        .monospace(true)
        .vexpand(true)
        .hexpand(true)
        .build();
    let output_buffer = output_view.buffer();

    let scroll = ScrolledWindow::builder()
        .hexpand(true)
        .vexpand(true)
        .child(&output_view)
        .build();

    root.append(&grid);
    root.append(&button_row);
    root.append(&scroll);
    window.set_child(Some(&root));

    let (tx, rx) = std::sync::mpsc::channel::<UiMessage>();
    let client = Rc::new(RefCell::new(StatusShareClient::new(CoreConfig::default())));

    {
        let output_buffer = output_buffer.clone();
        glib::timeout_add_local(Duration::from_millis(200), move || {
            loop {
                match rx.try_recv() {
                    Ok(UiMessage::Info(text)) => output_buffer.set_text(&text),
                    Err(std::sync::mpsc::TryRecvError::Empty) => break,
                    Err(std::sync::mpsc::TryRecvError::Disconnected) => return ControlFlow::Break,
                }
            }
            ControlFlow::Continue
        });
    }

    {
        let client = Rc::clone(&client);
        let tx = tx.clone();
        let base_url_entry = base_url_entry.clone();
        let token_entry = token_entry.clone();
        let interval_spin = interval_spin.clone();
        fetch_button.connect_clicked(move |_| {
            let client = client.borrow().clone();
            apply_config(&client, &base_url_entry, &token_entry, &interval_spin);
            let tx = tx.clone();

            thread::spawn(move || {
                let result = client.fetch_status();
                let text = serde_pretty(&result);
                let _ = tx.send(UiMessage::Info(text));
            });
        });
    }

    {
        let client = Rc::clone(&client);
        let tx = tx.clone();
        let base_url_entry = base_url_entry.clone();
        let token_entry = token_entry.clone();
        let interval_spin = interval_spin.clone();
        let process_entry = process_entry.clone();
        let extend_entry = extend_entry.clone();
        let media_title_entry = media_title_entry.clone();
        let media_artist_entry = media_artist_entry.clone();
        let media_thumbnail_entry = media_thumbnail_entry.clone();
        let ok_switch = ok_switch.clone();

        send_button.connect_clicked(move |_| {
            let client = client.borrow().clone();
            apply_config(&client, &base_url_entry, &token_entry, &interval_spin);
            let update = collect_update(
                &process_entry,
                &extend_entry,
                &media_title_entry,
                &media_artist_entry,
                &media_thumbnail_entry,
                &ok_switch,
            );
            let tx = tx.clone();

            thread::spawn(move || {
                let result = client.push_status(update);
                let _ = tx.send(UiMessage::Info(serde_pretty(&result)));
            });
        });
    }

    {
        let client = Rc::clone(&client);
        let tx = tx.clone();
        let base_url_entry = base_url_entry.clone();
        let token_entry = token_entry.clone();
        let interval_spin = interval_spin.clone();
        let process_entry = process_entry.clone();
        let extend_entry = extend_entry.clone();
        let media_title_entry = media_title_entry.clone();
        let media_artist_entry = media_artist_entry.clone();
        let media_thumbnail_entry = media_thumbnail_entry.clone();
        let ok_switch = ok_switch.clone();

        start_button.connect_clicked(move |_| {
            let client = client.borrow().clone();
            apply_config(&client, &base_url_entry, &token_entry, &interval_spin);
            let update = collect_update(
                &process_entry,
                &extend_entry,
                &media_title_entry,
                &media_artist_entry,
                &media_thumbnail_entry,
                &ok_switch,
            );

            client.start_heartbeat(update);
            let _ = tx.send(UiMessage::Info("heartbeat started".to_string()));
        });
    }

    {
        let client = Rc::clone(&client);
        let tx = tx.clone();
        stop_button.connect_clicked(move |_| {
            let client = client.borrow().clone();
            client.stop_heartbeat();
            let result = client.last_heartbeat_result();
            let text = format!("heartbeat stopped\n\n{}", serde_pretty(&result));
            let _ = tx.send(UiMessage::Info(text));
        });
    }

    {
        let client = Rc::clone(&client);
        let tx = tx.clone();
        glib::timeout_add_seconds_local(2, move || {
            let client = client.borrow().clone();
            if client.heartbeat_running() {
                let result = client.last_heartbeat_result();
                let text = format!("heartbeat running\n\n{}", serde_pretty(&result));
                let _ = tx.send(UiMessage::Info(text));
            }
            ControlFlow::Continue
        });
    }

    output_buffer.set_text("Ready. Configure the server base URL and token, then fetch or push.");
    window.present();
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

fn apply_config(
    client: &Arc<StatusShareClient>,
    base_url_entry: &Entry,
    token_entry: &Entry,
    interval_spin: &SpinButton,
) {
    client.update_config(CoreConfig {
        base_url: base_url_entry.text().to_string(),
        token: token_entry.text().to_string(),
        heartbeat_interval_secs: interval_spin.value() as u64,
        user_agent: "StatusShare GTK/0.1.0".to_string(),
    });
}

fn collect_update(
    process_entry: &Entry,
    extend_entry: &Entry,
    media_title_entry: &Entry,
    media_artist_entry: &Entry,
    media_thumbnail_entry: &Entry,
    ok_switch: &CheckButton,
) -> StatusUpdate {
    StatusUpdate {
        ok: Some(if ok_switch.is_active() { 1 } else { 0 }),
        process: string_opt(process_entry),
        extend: string_opt(extend_entry),
        media: media_opt(media_title_entry, media_artist_entry, media_thumbnail_entry),
        timestamp: None,
    }
}

fn string_opt(entry: &Entry) -> Option<String> {
    let value = entry.text().trim().to_string();
    if value.is_empty() { None } else { Some(value) }
}

fn media_opt(title: &Entry, artist: &Entry, thumbnail: &Entry) -> Option<MediaInfo> {
    let title = title.text().trim().to_string();
    let artist = artist.text().trim().to_string();
    let thumbnail = thumbnail.text().trim().to_string();

    if title.is_empty() && artist.is_empty() && thumbnail.is_empty() {
        None
    } else {
        Some(MediaInfo {
            title,
            artist,
            thumbnail,
        })
    }
}

fn serde_pretty<T>(value: &T) -> String
where
    T: serde::Serialize,
{
    serde_json::to_string_pretty(value).unwrap_or_else(|err| err.to_string())
}
