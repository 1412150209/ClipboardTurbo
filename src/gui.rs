use std::ops::Add;
use std::process::exit;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use clipboard_win::formats;
use eframe::CreationContext;
use eframe::epaint::text::TextWrapMode;
use egui::{Align, Align2, Context, FontData, FontId, Id, Key, Layout, ScrollArea, Sense, vec2, ViewportCommand, Visuals};
use egui::PointerButton;
use egui::WindowLevel::{AlwaysOnTop, Normal};

use crate::{CLIPBOARD_ENABLED, CLIPBOARD_QUEUE, MAX_TEXT};

pub struct MyApp {
    on_top: Arc<AtomicBool>,
}

impl MyApp {
    pub(crate) fn new(context: &CreationContext) -> Self {
        let mut fontset = egui::FontDefinitions::default();
        fontset.font_data.insert("simsun".to_owned(), FontData::from_static(include_bytes!("../assets/simsun.ttf")));
        // Put my font first (highest priority) for proportional text:
        fontset
            .families
            .entry(egui::FontFamily::Proportional)
            .or_default()
            .insert(0, "simsun".to_owned());

        // Put my font as last fallback for monospace:
        fontset
            .families
            .entry(egui::FontFamily::Monospace)
            .or_default()
            .push("simsun".to_owned());
        context.egui_ctx.set_fonts(fontset);
        Self {
            on_top: Arc::new(AtomicBool::from(false)),
        }
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        // 如果窗口被请求关闭，执行关闭操作
        if ctx.input(|i| i.viewport().close_requested()) {
            exit(0);
        }
        // 界面内快捷键
        if ctx.input(|i| i.modifiers.ctrl && i.key_released(Key::Backtick)) {
            CLIPBOARD_ENABLED.store(!CLIPBOARD_ENABLED.load(Ordering::Relaxed), Ordering::Relaxed);
        }
        // 鼠标挪出
        // if !ctx.input(|i| i.pointer.has_pointer()) {
        //     ctx.send_viewport_cmd(ViewportCommand::Visible(false));
        // }
        // 处理消息
        // while let Ok(message) = self.rx.try_recv() {
        //     match message {
        //         Signal::Visible(s) => {
        //             let Some(pos) = ctx.input(|i| i.pointer.latest_pos());
        //
        //             ctx.send_viewport_cmd(ViewportCommand::Visible(s));
        //         }
        //         Signal::Pos(p) => {
        //             ctx.send_viewport_cmd(ViewportCommand::OuterPosition(p - vec2(250.0, 400.0)));
        //         }
        //     }
        // }
        // 主窗口
        custom_window_frame(ctx, "队列粘贴板", |ui| {
            ui.horizontal(|ui| {
                ui.label("队列粘贴板启用:");
                if ui.checkbox(&mut CLIPBOARD_ENABLED.load(Ordering::Relaxed), "").clicked() {
                    CLIPBOARD_ENABLED.store(!CLIPBOARD_ENABLED.load(Ordering::Relaxed), Ordering::Relaxed);
                    ctx.request_repaint();
                }
                ui.label("|");
                ui.label("清空队列：");
                if ui.button("🔃").clicked() {
                    CLIPBOARD_QUEUE.lock().unwrap().clear();
                    ctx.request_repaint();
                }
            });
            ui.horizontal(|ui| {
                ui.label("置顶窗口：");
                if ui.checkbox(&mut self.on_top.load(Ordering::Relaxed), "").clicked() {
                    // 置顶窗口
                    self.on_top.store(!self.on_top.load(Ordering::Relaxed), Ordering::Relaxed);
                    if self.on_top.load(Ordering::Relaxed) {
                        ctx.send_viewport_cmd(ViewportCommand::WindowLevel(AlwaysOnTop));
                    } else {
                        ctx.send_viewport_cmd(ViewportCommand::WindowLevel(Normal));
                    }
                    ctx.request_repaint();
                }
                ui.label("|");
                egui::widgets::global_dark_light_mode_buttons(ui);
            });
            ui.separator();
            // 粘贴板内容
            ScrollArea::vertical().hscroll(true).show(ui, |ui| {
                show_table(ctx, ui);
            });
        });
        ctx.request_repaint();
    }

    fn clear_color(&self, _visuals: &Visuals) -> [f32; 4] {
        egui::Rgba::TRANSPARENT.to_array()
    }
}

fn custom_window_frame(ctx: &Context, title: &str, add_contents: impl FnOnce(&mut egui::Ui)) {
    use egui::*;

    let panel_frame = Frame {
        fill: ctx.style().visuals.window_fill(),
        rounding: 10.0.into(),
        stroke: ctx.style().visuals.widgets.noninteractive.fg_stroke,
        outer_margin: 0.5.into(), // so the stroke is within the bounds
        ..Default::default()
    };

    CentralPanel::default().frame(panel_frame).show(ctx, |ui| {
        let app_rect = ui.max_rect();

        let title_bar_height = 32.0;
        let title_bar_rect = {
            let mut rect = app_rect;
            rect.max.y = rect.min.y + title_bar_height;
            rect
        };
        title_bar_ui(ui, title_bar_rect, title);

        // Add the contents:
        let content_rect = {
            let mut rect = app_rect;
            rect.min.y = title_bar_rect.max.y;
            rect
        }
            .shrink(4.0);
        let mut content_ui = ui.child_ui(content_rect, *ui.layout(), None);
        add_contents(&mut content_ui);
    });
}

fn title_bar_ui(ui: &mut egui::Ui, title_bar_rect: eframe::epaint::Rect, title: &str) {
    let painter = ui.painter();

    let title_bar_response = ui.interact(
        title_bar_rect,
        Id::new("title_bar"),
        Sense::click_and_drag(),
    );

    // Paint the title:
    painter.text(
        title_bar_rect.center(),
        Align2::CENTER_CENTER,
        title,
        FontId::proportional(15.0),
        ui.style().visuals.text_color(),
    );

    // Paint the line under the title:
    painter.line_segment(
        [
            title_bar_rect.left_bottom() + vec2(1.0, 0.0),
            title_bar_rect.right_bottom() + vec2(-1.0, 0.0),
        ],
        ui.visuals().widgets.noninteractive.bg_stroke,
    );

    if title_bar_response.drag_started_by(PointerButton::Primary) {
        ui.ctx().send_viewport_cmd(ViewportCommand::StartDrag);
    }

    ui.allocate_ui_at_rect(title_bar_rect, |ui| {
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.spacing_mut().item_spacing.x = 1.0;
            ui.visuals_mut().button_frame = false;
            ui.add_space(8.0);
            close_minimize(ui);
        });
    });
}

/// Show some close/maximize/minimize buttons for the native window.
fn close_minimize(ui: &mut egui::Ui) {
    use egui::{Button, RichText};

    let button_height = 15.0;

    let close_response = ui
        .add(Button::new(RichText::new("❌").size(button_height)))
        .on_hover_text("关闭程序");
    if close_response.clicked() {
        ui.ctx().send_viewport_cmd(ViewportCommand::Close);
    }

    let minimized_response = ui
        .add(Button::new(RichText::new("➖").size(button_height)))
        .on_hover_text("最小化程序");
    if minimized_response.clicked() {
        ui.ctx().send_viewport_cmd(ViewportCommand::Minimized(true));
    }
}

fn show_table(ctx: &Context, ui: &mut egui::Ui) {
    use egui_extras::{Column, TableBuilder};
    let mut clipboard_queue = CLIPBOARD_QUEUE.lock().unwrap();
    let mut indices_to_remove: Vec<usize> = Vec::new();
    let text_height = egui::TextStyle::Body
        .resolve(ui.style())
        .size
        .max(ui.spacing().interact_size.y);
    let table = TableBuilder::new(ui)
        .cell_layout(Layout::left_to_right(Align::Center))
        .striped(true)
        .column(Column::initial(20.0))
        .column(Column::exact(40.0))
        .column(Column::remainder())
        .stick_to_bottom(true)
        .min_scrolled_height(0.0);
    table
        .header(20.0, |mut header| {
            header.col(|ui| {
                ui.strong("序号");
            });
            header.col(|ui| {
                ui.strong("操作");
            });
            header.col(|ui| {
                ui.strong("内容");
            });
        })
        .body(|mut body| {
            for (index, item) in clipboard_queue.iter().enumerate() {
                body.row(text_height, |mut row| {
                    row.col(|ui| {
                        ui.label((index + 1).to_string());
                    });
                    row.col(|ui| {
                        if ui.button("删除").clicked() {
                            indices_to_remove.push(index);
                        }
                    });
                    match item.get_type() {
                        formats::CF_UNICODETEXT | formats::CF_TEXT => {
                            row.col(|ui| {
                                let str = item.get_data();
                                let display: String = str.chars().take(MAX_TEXT).collect();
                                if str.len() > MAX_TEXT {
                                    ui.add(
                                        egui::Label::new(display.add("...")).wrap_mode(TextWrapMode::Extend)
                                    );
                                } else {
                                    ui.add(
                                        egui::Label::new(display).wrap_mode(TextWrapMode::Extend)
                                    );
                                }
                            });
                        }
                        formats::CF_DIB => {
                            let texture = egui::TextureHandle::from(
                                ctx.load_texture(format!("Image_{}", index), item.get_image(), egui::TextureOptions::default())
                            );
                            row.col(|ui| {
                                ui.image(&texture);
                            });
                        }
                        _ => {}
                    }
                });
            }
        });
    for i in indices_to_remove {
        clipboard_queue.remove(i);
    }
}