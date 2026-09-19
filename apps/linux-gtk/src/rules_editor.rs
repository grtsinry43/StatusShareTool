use std::cell::{Cell, RefCell};
use std::rc::Rc;

use gtk::prelude::*;
use gtk::{
    Box as GtkBox, Button, CheckButton, ComboBoxText, Entry, Frame, Image, Label, ListBox,
    ListBoxRow, Orientation, ScrolledWindow, Spinner,
};
use statusshare_core::{GameMeta, MatchField, MatchKind, ReportPolicy, WindowMatchRule};

use crate::game_lookup::GameLookupPreview;

#[derive(Clone)]
pub struct RulesEditor {
    root: GtkBox,
    list_box: ListBox,
    detail_panel: GtkBox,
    id_entry: Entry,
    enabled_check: CheckButton,
    field_combo: ComboBoxText,
    kind_combo: ComboBoxText,
    pattern_entry: Entry,
    case_sensitive_check: CheckButton,
    policy_combo: ComboBoxText,
    display_name_entry: Entry,
    extend_entry: Entry,
    category_combo: ComboBoxText,
    /// 游戏扩展字段的容器，仅 category = game 时可见。
    game_fields_box: GtkBox,
    game_name_entry: Entry,
    game_cover_entry: Entry,
    game_slogan_entry: Entry,
    game_desc_entry: Entry,
    game_accent_entry: Entry,
    game_url_entry: Entry,
    /// Steam 查询预览区
    lookup_status_label: Label,
    lookup_spinner: Spinner,
    lookup_result_box: GtkBox,
    lookup_cover: Image,
    lookup_name_label: Label,
    lookup_desc_label: Label,
    lookup_apply_button: Button,
    lookup_button: Button,
    /// 查询到的结果暂存，供「填入卡片字段」使用。
    lookup_result: Rc<RefCell<Option<GameLookupPreview>>>,
    on_game_lookup: Rc<RefCell<Option<Rc<dyn Fn(String)>>>>,
    rules: Rc<RefCell<Vec<WindowMatchRule>>>,
    selected_index: Rc<Cell<Option<usize>>>,
    syncing: Rc<Cell<bool>>,
    /// 与列表行一一对应的 (标题, 副标题) 标签，用于原地更新而不重建列表。
    row_labels: Rc<RefCell<Vec<(Label, Label)>>>,
    /// 配置发生实际编辑（文字/开关/增删）时触发，供外层做自动保存。
    on_change: Rc<RefCell<Option<Rc<dyn Fn()>>>>,
}

impl RulesEditor {
    pub fn new() -> Self {
        let root = GtkBox::new(Orientation::Horizontal, 16);
        root.set_hexpand(true);
        root.set_vexpand(true);

        let rules = Rc::new(RefCell::new(Vec::<WindowMatchRule>::new()));
        let selected_index = Rc::new(Cell::new(None));
        let syncing = Rc::new(Cell::new(false));

        let list_column = GtkBox::new(Orientation::Vertical, 10);
        list_column.set_size_request(240, -1);

        let list_title = Label::new(Some("规则列表"));
        list_title.add_css_class("preview-title");
        list_title.set_xalign(0.0);
        let list_description = Label::new(Some(
            "一条规则代表一种窗口匹配方式。命中 Allow 就上报，命中 Deny 就跳过。",
        ));
        list_description.add_css_class("section-subtitle");
        list_description.set_wrap(true);
        list_description.set_max_width_chars(28);
        list_description.set_xalign(0.0);

        let add_button = Button::with_label("Add Rule");
        let delete_button = Button::with_label("Delete Rule");
        let list_actions = GtkBox::new(Orientation::Horizontal, 8);
        add_button.set_hexpand(true);
        delete_button.set_hexpand(true);
        list_actions.append(&add_button);
        list_actions.append(&delete_button);

        let list_box = ListBox::new();
        list_box.set_selection_mode(gtk::SelectionMode::Single);
        let list_scroll = ScrolledWindow::builder()
            .hexpand(true)
            .vexpand(true)
            .min_content_height(300)
            .child(&list_box)
            .build();

        list_column.append(&list_title);
        list_column.append(&list_description);
        list_column.append(&list_actions);
        list_column.append(&list_scroll);

        let detail_scroll = ScrolledWindow::builder()
            .hexpand(true)
            .vexpand(true)
            .min_content_height(360)
            .build();
        let detail_panel = GtkBox::new(Orientation::Vertical, 14);
        detail_scroll.set_child(Some(&detail_panel));

        let id_entry = Entry::builder().hexpand(true).build();
        let enabled_check = CheckButton::with_label("启用这条规则");

        let field_combo = ComboBoxText::new();
        for (id, label) in [
            ("WindowTitle", "Window Title"),
            ("AppName", "App Name"),
            ("ProcessName", "Process Name"),
            ("ExecutablePath", "Executable Path"),
            ("BundleId", "Bundle ID"),
        ] {
            field_combo.append(Some(id), label);
        }

        let kind_combo = ComboBoxText::new();
        for (id, label) in [
            ("Contains", "Contains"),
            ("Exact", "Exact"),
            ("Prefix", "Prefix"),
            ("Suffix", "Suffix"),
        ] {
            kind_combo.append(Some(id), label);
        }

        let pattern_entry = Entry::builder().hexpand(true).build();
        let case_sensitive_check = CheckButton::with_label("区分大小写");

        let policy_combo = ComboBoxText::new();
        for (id, label) in [("Allow", "Allow"), ("Deny", "Deny")] {
            policy_combo.append(Some(id), label);
        }

        let display_name_entry = Entry::builder().hexpand(true).build();
        let extend_entry = Entry::builder().hexpand(true).build();

        let category_combo = ComboBoxText::new();
        for (id, label) in [
            ("", "普通应用"),
            ("game", "游戏 Game"),
            ("music", "音乐 Music"),
        ] {
            category_combo.append(Some(id), label);
        }

        let game_fields_box = GtkBox::new(Orientation::Vertical, 14);
        let game_name_entry = Entry::builder().hexpand(true).build();
        let game_cover_entry = Entry::builder().hexpand(true).build();
        let game_slogan_entry = Entry::builder().hexpand(true).build();
        let game_desc_entry = Entry::builder().hexpand(true).build();
        let game_accent_entry = Entry::builder().hexpand(true).build();
        let game_url_entry = Entry::builder().hexpand(true).build();

        detail_panel.append(&field_block(
            "Rule ID",
            "给规则一个稳定标识，方便后续修改和排查命中结果。",
            &id_entry,
        ));
        detail_panel.append(&field_block(
            "Enabled",
            "关闭后会保留规则，但不会参与匹配。",
            &enabled_check,
        ));
        detail_panel.append(&field_block(
            "Match Field",
            "选择要匹配的窗口字段，比如应用名、进程名或窗口标题。",
            &field_combo,
        ));
        detail_panel.append(&field_block(
            "Match Type",
            "Contains 适合模糊匹配，Exact 适合精确匹配，Prefix/Suffix 适合固定前后缀。",
            &kind_combo,
        ));
        detail_panel.append(&field_block(
            "Pattern",
            "要匹配的文本内容，比如 kitty、WezTerm 或某个窗口标题关键字。",
            &pattern_entry,
        ));
        detail_panel.append(&field_block(
            "Case Sensitive",
            "通常 Linux 应用名和进程名大小写不稳定，默认建议关闭。",
            &case_sensitive_check,
        ));
        detail_panel.append(&field_block(
            "Report Policy",
            "Allow 表示命中后允许上报，Deny 表示命中后跳过本次上报。",
            &policy_combo,
        ));
        detail_panel.append(&field_block(
            "Display Name",
            "命中后上报给后端的 process 外显名称，比如把 kitty 显示成 Kitty。",
            &display_name_entry,
        ));
        detail_panel.append(&field_block(
            "Extend",
            "命中后上报给后端的 extend 文案，可以写一句说明当前在做什么。",
            &extend_entry,
        ));
        detail_panel.append(&field_block(
            "Category",
            "选「游戏」后，前端会把这条活动渲染成游戏卡片，并展开下面的扩展字段。",
            &category_combo,
        ));

        game_fields_box.append(&field_block(
            "Game Name",
            "游戏标准名：卡片标题 + Steam 搜索关键词。显示名可以玩梗，这里必须正经，比如 Stardew Valley。",
            &game_name_entry,
        ));
        game_fields_box.append(&field_block(
            "Game Cover",
            "游戏卡片封面横幅图 URL，可用 Steam CDN 图或自己的图床。",
            &game_cover_entry,
        ));
        game_fields_box.append(&field_block(
            "Game Slogan",
            "游戏卡片上展示的一句 slogan，比如「代码写不动了，就回鹈鹕镇种地」。",
            &game_slogan_entry,
        ));
        game_fields_box.append(&field_block(
            "Game Desc",
            "游戏卡片上展示的一句话描述。",
            &game_desc_entry,
        ));
        game_fields_box.append(&field_block(
            "Game Accent",
            "游戏卡片点缀色，hex 格式，比如 #5da84f；留空用默认色。",
            &game_accent_entry,
        ));
        game_fields_box.append(&field_block(
            "Game URL",
            "点击游戏卡片跳转的链接，比如 Steam 商店页；留空则不可点击。",
            &game_url_entry,
        ));

        // Steam 查询预览区
        let lookup_frame = Frame::new(None);
        lookup_frame.add_css_class("preview-card");
        let lookup_box = GtkBox::new(Orientation::Vertical, 8);
        lookup_box.set_margin_top(12);
        lookup_box.set_margin_bottom(12);
        lookup_box.set_margin_start(12);
        lookup_box.set_margin_end(12);

        let lookup_title = Label::new(Some("Steam 预览"));
        lookup_title.add_css_class("preview-row-title");
        lookup_title.set_xalign(0.0);
        let lookup_hint = Label::new(Some(
            "用 Game Name（留空则用 Display Name）请求博客服务端的 game-lookup 接口，效果与前台兜底卡片一致。",
        ));
        lookup_hint.add_css_class("section-subtitle");
        lookup_hint.set_wrap(true);
        lookup_hint.set_xalign(0.0);

        let lookup_action_row = GtkBox::new(Orientation::Horizontal, 8);
        let lookup_button = Button::with_label("查询 Steam");
        let lookup_spinner = Spinner::new();
        let lookup_status_label = Label::new(Some(""));
        lookup_status_label.add_css_class("section-subtitle");
        lookup_status_label.set_xalign(0.0);
        lookup_status_label.set_hexpand(true);
        lookup_action_row.append(&lookup_button);
        lookup_action_row.append(&lookup_spinner);
        lookup_action_row.append(&lookup_status_label);

        let lookup_result_box = GtkBox::new(Orientation::Horizontal, 12);
        lookup_result_box.set_visible(false);
        let lookup_cover = Image::new();
        lookup_cover.set_size_request(138, 64);
        let lookup_text_box = GtkBox::new(Orientation::Vertical, 4);
        lookup_text_box.set_valign(gtk::Align::Center);
        lookup_text_box.set_hexpand(true);
        let lookup_name_label = Label::new(None);
        lookup_name_label.add_css_class("preview-title");
        lookup_name_label.set_xalign(0.0);
        let lookup_desc_label = Label::new(None);
        lookup_desc_label.add_css_class("section-subtitle");
        lookup_desc_label.set_wrap(true);
        lookup_desc_label.set_xalign(0.0);
        let lookup_apply_button = Button::with_label("填入卡片字段");
        lookup_apply_button.set_halign(gtk::Align::Start);
        lookup_text_box.append(&lookup_name_label);
        lookup_text_box.append(&lookup_desc_label);
        lookup_text_box.append(&lookup_apply_button);
        lookup_result_box.append(&lookup_cover);
        lookup_result_box.append(&lookup_text_box);

        lookup_box.append(&lookup_title);
        lookup_box.append(&lookup_hint);
        lookup_box.append(&lookup_action_row);
        lookup_box.append(&lookup_result_box);
        lookup_frame.set_child(Some(&lookup_box));
        game_fields_box.append(&lookup_frame);

        game_fields_box.set_visible(false);
        detail_panel.append(&game_fields_box);

        root.append(&list_column);
        root.append(&detail_scroll);

        let editor = Self {
            root,
            list_box,
            detail_panel,
            id_entry,
            enabled_check,
            field_combo,
            kind_combo,
            pattern_entry,
            case_sensitive_check,
            policy_combo,
            display_name_entry,
            extend_entry,
            category_combo,
            game_fields_box,
            game_name_entry,
            game_cover_entry,
            game_slogan_entry,
            game_desc_entry,
            game_accent_entry,
            game_url_entry,
            lookup_status_label,
            lookup_spinner,
            lookup_result_box,
            lookup_cover,
            lookup_name_label,
            lookup_desc_label,
            lookup_apply_button,
            lookup_button,
            lookup_result: Rc::new(RefCell::new(None)),
            on_game_lookup: Rc::new(RefCell::new(None)),
            rules,
            selected_index,
            syncing,
            row_labels: Rc::new(RefCell::new(Vec::new())),
            on_change: Rc::new(RefCell::new(None)),
        };

        editor.connect_signals(add_button, delete_button);
        editor.set_form_sensitive(false);
        editor
    }

    pub fn widget(&self) -> GtkBox {
        self.root.clone()
    }

    pub fn rules(&self) -> Vec<WindowMatchRule> {
        self.rules.borrow().clone()
    }

    pub fn set_rules(&self, rules: &[WindowMatchRule]) {
        *self.rules.borrow_mut() = rules.to_vec();
        let selected = if rules.is_empty() { None } else { Some(0) };
        self.refresh_list(selected);
    }

    /// 注册编辑回调：规则内容被实际修改（含增删）时触发。
    /// 加载配置（set_rules）不会触发。
    pub fn set_on_change(&self, callback: Rc<dyn Fn()>) {
        *self.on_change.borrow_mut() = Some(callback);
    }

    fn emit_change(&self) {
        if let Some(callback) = self.on_change.borrow().as_ref() {
            callback();
        }
    }

    /// 注册 Steam 查询回调（由 app 注入，负责带 base_url 发起异步请求）。
    pub fn set_game_lookup_handler(&self, handler: Rc<dyn Fn(String)>) {
        *self.on_game_lookup.borrow_mut() = Some(handler);
    }

    fn update_game_fields_visibility(&self, category: &str) {
        self.game_fields_box.set_visible(category == "game");
    }

    fn trigger_game_lookup(&self, keyword: String) {
        if keyword.trim().is_empty() {
            self.show_game_lookup_error("请先填写 Game Name 或 Display Name".to_string());
            return;
        }
        let Some(handler) = self.on_game_lookup.borrow().as_ref().cloned() else {
            self.show_game_lookup_error("查询功能未初始化".to_string());
            return;
        };
        self.show_game_lookup_loading();
        handler(keyword);
    }

    pub fn show_game_lookup_loading(&self) {
        *self.lookup_result.borrow_mut() = None;
        self.lookup_result_box.set_visible(false);
        self.lookup_status_label.set_text("查询中…");
        self.lookup_spinner.start();
    }

    pub fn show_game_lookup_result(&self, preview: GameLookupPreview) {
        self.lookup_spinner.stop();
        if !preview.found {
            *self.lookup_result.borrow_mut() = None;
            self.lookup_result_box.set_visible(false);
            self.lookup_status_label
                .set_text("没有查到匹配的游戏，可以换个更正式的名字试试");
            return;
        }

        if let Some(path) = preview.header_image_path.as_ref() {
            self.lookup_cover.set_from_file(Some(path));
        } else {
            self.lookup_cover.set_icon_name(Some("image-x-generic-symbolic"));
        }
        self.lookup_name_label.set_text(&preview.name);
        self.lookup_desc_label.set_text(&preview.short_description);
        self.lookup_status_label.set_text("查询成功");
        self.lookup_result_box.set_visible(true);
        *self.lookup_result.borrow_mut() = Some(preview);
    }

    pub fn show_game_lookup_error(&self, message: String) {
        self.lookup_spinner.stop();
        *self.lookup_result.borrow_mut() = None;
        self.lookup_result_box.set_visible(false);
        self.lookup_status_label.set_text(&message);
    }

    /// 「填入卡片字段」：把查询结果写回 Game Name / Cover / URL 输入框，
    /// 走输入框的 changed 信号，等价于用户手动输入（会触发规则更新与自动保存）。
    fn apply_lookup_result(&self) {
        let Some(preview) = self.lookup_result.borrow().clone() else {
            return;
        };
        if !preview.name.is_empty() {
            self.game_name_entry.set_text(&preview.name);
        }
        if !preview.header_image.is_empty() {
            self.game_cover_entry.set_text(&preview.header_image);
        }
        if !preview.store_url.is_empty() {
            self.game_url_entry.set_text(&preview.store_url);
        }
    }

    fn connect_signals(&self, add_button: Button, delete_button: Button) {
        let editor = self.clone();
        add_button.connect_clicked(move |_| {
            let mut rules = editor.rules.borrow_mut();
            let next_index = rules.len() + 1;
            rules.push(WindowMatchRule {
                id: format!("rule-{next_index}"),
                enabled: true,
                field: MatchField::AppName,
                kind: MatchKind::Contains,
                pattern: String::new(),
                case_sensitive: false,
                report_policy: ReportPolicy::Allow,
                display_name: String::new(),
                extend: String::new(),
                category: String::new(),
                game: GameMeta::default(),
            });
            drop(rules);
            editor.refresh_list(Some(next_index - 1));
            editor.emit_change();
        });

        let editor = self.clone();
        delete_button.connect_clicked(move |_| {
            let Some(index) = editor.selected_index.get() else {
                return;
            };
            let mut rules = editor.rules.borrow_mut();
            if index >= rules.len() {
                return;
            }
            rules.remove(index);
            let next_selection = if rules.is_empty() {
                None
            } else if index >= rules.len() {
                Some(rules.len() - 1)
            } else {
                Some(index)
            };
            drop(rules);
            editor.refresh_list(next_selection);
            editor.emit_change();
        });

        let editor = self.clone();
        self.list_box.connect_row_selected(move |_, row| {
            editor
                .selected_index
                .set(row.map(|item| item.index() as usize));
            editor.sync_form_from_selection();
        });

        let editor = self.clone();
        self.id_entry.connect_changed(move |entry| {
            editor.update_selected_rule(|rule| rule.id = entry.text().to_string());
        });

        let editor = self.clone();
        self.enabled_check.connect_toggled(move |check| {
            editor.update_selected_rule(|rule| rule.enabled = check.is_active());
        });

        let editor = self.clone();
        self.field_combo.connect_changed(move |combo| {
            if let Some(rule_field) = combo.active_id().as_deref().and_then(parse_match_field) {
                editor.update_selected_rule(|rule| rule.field = rule_field);
            }
        });

        let editor = self.clone();
        self.kind_combo.connect_changed(move |combo| {
            if let Some(kind) = combo.active_id().as_deref().and_then(parse_match_kind) {
                editor.update_selected_rule(|rule| rule.kind = kind);
            }
        });

        let editor = self.clone();
        self.pattern_entry.connect_changed(move |entry| {
            editor.update_selected_rule(|rule| rule.pattern = entry.text().to_string());
        });

        let editor = self.clone();
        self.case_sensitive_check.connect_toggled(move |check| {
            editor.update_selected_rule(|rule| rule.case_sensitive = check.is_active());
        });

        let editor = self.clone();
        self.policy_combo.connect_changed(move |combo| {
            if let Some(policy) = combo.active_id().as_deref().and_then(parse_report_policy) {
                editor.update_selected_rule(|rule| rule.report_policy = policy);
            }
        });

        let editor = self.clone();
        self.display_name_entry.connect_changed(move |entry| {
            editor.update_selected_rule(|rule| rule.display_name = entry.text().to_string());
        });

        let editor = self.clone();
        self.extend_entry.connect_changed(move |entry| {
            editor.update_selected_rule(|rule| rule.extend = entry.text().to_string());
        });

        let editor = self.clone();
        self.category_combo.connect_changed(move |combo| {
            let category = combo.active_id().unwrap_or_default().to_string();
            editor.update_game_fields_visibility(&category);
            editor.update_selected_rule(|rule| rule.category = category.clone());
        });

        let editor = self.clone();
        self.game_name_entry.connect_changed(move |entry| {
            editor.update_selected_rule(|rule| rule.game.name = entry.text().to_string());
        });

        let editor = self.clone();
        self.game_cover_entry.connect_changed(move |entry| {
            editor.update_selected_rule(|rule| rule.game.cover = entry.text().to_string());
        });

        let editor = self.clone();
        self.game_slogan_entry.connect_changed(move |entry| {
            editor.update_selected_rule(|rule| rule.game.slogan = entry.text().to_string());
        });

        let editor = self.clone();
        self.game_desc_entry.connect_changed(move |entry| {
            editor.update_selected_rule(|rule| rule.game.desc = entry.text().to_string());
        });

        let editor = self.clone();
        self.game_accent_entry.connect_changed(move |entry| {
            editor.update_selected_rule(|rule| rule.game.accent = entry.text().to_string());
        });

        let editor = self.clone();
        self.game_url_entry.connect_changed(move |entry| {
            editor.update_selected_rule(|rule| rule.game.url = entry.text().to_string());
        });

        // Steam 查询预览
        let editor = self.clone();
        self.lookup_apply_button.connect_clicked(move |_| {
            editor.apply_lookup_result();
        });

        let editor = self.clone();
        let lookup_button = self.lookup_button.clone();
        let game_name_entry = self.game_name_entry.clone();
        let display_name_entry = self.display_name_entry.clone();
        lookup_button.connect_clicked(move |_| {
            let keyword = {
                let name = game_name_entry.text().to_string();
                if name.trim().is_empty() {
                    display_name_entry.text().to_string()
                } else {
                    name
                }
            };
            editor.trigger_game_lookup(keyword);
        });
    }

    fn refresh_list(&self, selected: Option<usize>) {
        self.selected_index.set(selected);
        while let Some(child) = self.list_box.first_child() {
            self.list_box.remove(&child);
        }

        let mut row_labels = self.row_labels.borrow_mut();
        row_labels.clear();

        for rule in self.rules.borrow().iter() {
            let row = ListBoxRow::new();
            let wrapper = GtkBox::new(Orientation::Vertical, 4);
            wrapper.set_margin_top(10);
            wrapper.set_margin_bottom(10);
            wrapper.set_margin_start(10);
            wrapper.set_margin_end(10);

            let title = Label::new(Some(&rule_title(rule)));
            title.set_xalign(0.0);
            title.add_css_class("preview-title");
            title.set_wrap(true);

            let subtitle = Label::new(Some(&rule_summary(rule)));
            subtitle.set_xalign(0.0);
            subtitle.add_css_class("section-subtitle");
            subtitle.set_wrap(true);
            subtitle.set_max_width_chars(24);

            wrapper.append(&title);
            wrapper.append(&subtitle);
            row.set_child(Some(&wrapper));
            self.list_box.append(&row);
            row_labels.push((title, subtitle));
        }
        drop(row_labels);

        self.apply_selected_row();
    }

    fn apply_selected_row(&self) {
        let selected = self.selected_index.get();
        match selected.and_then(|index| self.list_box.row_at_index(index as i32)) {
            Some(row) => {
                self.list_box.select_row(Some(&row));
                self.sync_form_from_selection();
            }
            None => {
                self.list_box.unselect_all();
                self.clear_form();
                self.set_form_sensitive(false);
            }
        }
    }

    fn sync_form_from_selection(&self) {
        self.syncing.set(true);

        if let Some(index) = self.selected_index.get() {
            let rules = self.rules.borrow();
            if let Some(rule) = rules.get(index) {
                self.set_form_sensitive(true);
                self.id_entry.set_text(&rule.id);
                self.enabled_check.set_active(rule.enabled);
                self.field_combo
                    .set_active_id(Some(match_field_id(rule.field)));
                self.kind_combo
                    .set_active_id(Some(match_kind_id(rule.kind)));
                self.pattern_entry.set_text(&rule.pattern);
                self.case_sensitive_check.set_active(rule.case_sensitive);
                self.policy_combo
                    .set_active_id(Some(report_policy_id(rule.report_policy)));
                self.display_name_entry.set_text(&rule.display_name);
                self.extend_entry.set_text(&rule.extend);
                let category_id = match rule.category.as_str() {
                    "game" => Some("game"),
                    "music" => Some("music"),
                    _ => Some(""),
                };
                self.category_combo.set_active_id(category_id);
                self.update_game_fields_visibility(&rule.category);
                self.game_name_entry.set_text(&rule.game.name);
                self.game_cover_entry.set_text(&rule.game.cover);
                self.game_slogan_entry.set_text(&rule.game.slogan);
                self.game_desc_entry.set_text(&rule.game.desc);
                self.game_accent_entry.set_text(&rule.game.accent);
                self.game_url_entry.set_text(&rule.game.url);
            } else {
                self.clear_form();
                self.set_form_sensitive(false);
            }
        } else {
            self.clear_form();
            self.set_form_sensitive(false);
        }

        self.syncing.set(false);
    }

    fn update_selected_rule<F>(&self, update: F)
    where
        F: FnOnce(&mut WindowMatchRule),
    {
        if self.syncing.get() {
            return;
        }

        let Some(index) = self.selected_index.get() else {
            return;
        };

        let mut rules = self.rules.borrow_mut();
        let Some(rule) = rules.get_mut(index) else {
            return;
        };
        update(rule);
        let title = rule_title(rule);
        let summary = rule_summary(rule);
        drop(rules);

        // 只原地更新选中行的文字，不重建列表：
        // 重建会销毁正在输入的焦点 widget，导致输入一个字符就失焦。
        if let Some((title_label, subtitle_label)) =
            self.row_labels.borrow().get(index)
        {
            title_label.set_text(&title);
            subtitle_label.set_text(&summary);
        }
        self.emit_change();
    }

    fn clear_form(&self) {
        self.id_entry.set_text("");
        self.enabled_check.set_active(false);
        self.field_combo.set_active_id(None);
        self.kind_combo.set_active_id(None);
        self.pattern_entry.set_text("");
        self.case_sensitive_check.set_active(false);
        self.policy_combo.set_active_id(None);
        self.display_name_entry.set_text("");
        self.extend_entry.set_text("");
        self.category_combo.set_active_id(Some(""));
        self.game_fields_box.set_visible(false);
        self.game_name_entry.set_text("");
        self.game_cover_entry.set_text("");
        self.game_slogan_entry.set_text("");
        self.game_desc_entry.set_text("");
        self.game_accent_entry.set_text("");
        self.game_url_entry.set_text("");
    }

    fn set_form_sensitive(&self, sensitive: bool) {
        self.detail_panel.set_sensitive(sensitive);
    }
}

fn field_block<W>(title: &str, description: &str, widget: &W) -> Frame
where
    W: IsA<gtk::Widget>,
{
    let frame = Frame::new(None);
    frame.add_css_class("preview-card");

    let wrapper = GtkBox::new(Orientation::Vertical, 8);
    wrapper.set_margin_top(12);
    wrapper.set_margin_bottom(12);
    wrapper.set_margin_start(12);
    wrapper.set_margin_end(12);

    let title_label = Label::new(Some(title));
    title_label.add_css_class("preview-row-title");
    title_label.set_xalign(0.0);

    let description_label = Label::new(Some(description));
    description_label.add_css_class("section-subtitle");
    description_label.set_xalign(0.0);
    description_label.set_wrap(true);

    wrapper.append(&title_label);
    wrapper.append(&description_label);
    wrapper.append(widget);
    frame.set_child(Some(&wrapper));
    frame
}

fn rule_title(rule: &WindowMatchRule) -> String {
    if !rule.display_name.trim().is_empty() {
        rule.display_name.clone()
    } else if !rule.pattern.trim().is_empty() {
        rule.pattern.clone()
    } else if !rule.id.trim().is_empty() {
        rule.id.clone()
    } else {
        "Untitled Rule".to_string()
    }
}

fn rule_summary(rule: &WindowMatchRule) -> String {
    format!(
        "{} · {} · {}",
        if rule.enabled { "Enabled" } else { "Disabled" },
        match_field_label(rule.field),
        match rule.report_policy {
            ReportPolicy::Allow => "Allow",
            ReportPolicy::Deny => "Deny",
        }
    )
}

fn match_field_id(value: MatchField) -> &'static str {
    match value {
        MatchField::WindowTitle => "WindowTitle",
        MatchField::AppName => "AppName",
        MatchField::ProcessName => "ProcessName",
        MatchField::ExecutablePath => "ExecutablePath",
        MatchField::BundleId => "BundleId",
    }
}

fn match_kind_id(value: MatchKind) -> &'static str {
    match value {
        MatchKind::Contains => "Contains",
        MatchKind::Exact => "Exact",
        MatchKind::Prefix => "Prefix",
        MatchKind::Suffix => "Suffix",
    }
}

fn report_policy_id(value: ReportPolicy) -> &'static str {
    match value {
        ReportPolicy::Allow => "Allow",
        ReportPolicy::Deny => "Deny",
    }
}

fn parse_match_field(value: &str) -> Option<MatchField> {
    match value {
        "WindowTitle" => Some(MatchField::WindowTitle),
        "AppName" => Some(MatchField::AppName),
        "ProcessName" => Some(MatchField::ProcessName),
        "ExecutablePath" => Some(MatchField::ExecutablePath),
        "BundleId" => Some(MatchField::BundleId),
        _ => None,
    }
}

fn parse_match_kind(value: &str) -> Option<MatchKind> {
    match value {
        "Contains" => Some(MatchKind::Contains),
        "Exact" => Some(MatchKind::Exact),
        "Prefix" => Some(MatchKind::Prefix),
        "Suffix" => Some(MatchKind::Suffix),
        _ => None,
    }
}

fn parse_report_policy(value: &str) -> Option<ReportPolicy> {
    match value {
        "Allow" => Some(ReportPolicy::Allow),
        "Deny" => Some(ReportPolicy::Deny),
        _ => None,
    }
}

fn match_field_label(value: MatchField) -> &'static str {
    match value {
        MatchField::WindowTitle => "Window Title",
        MatchField::AppName => "App Name",
        MatchField::ProcessName => "Process Name",
        MatchField::ExecutablePath => "Executable Path",
        MatchField::BundleId => "Bundle ID",
    }
}
