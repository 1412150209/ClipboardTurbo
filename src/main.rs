#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use clipboard_win::{formats, get_clipboard, get_clipboard_string, set_clipboard};
use clipboard_win::formats::RawData;
use clipboard_win::types::c_uint;
use eframe::{icon_data, NativeOptions, run_native};
use egui::ViewportBuilder;
use lazy_static::lazy_static;
use rdev::{EventType, Key, listen};

use crate::clipboard::Data;
use crate::gui::MyApp;

mod clipboard;
mod gui;

const SUPPORTED_FORMATS: [c_uint; 3] = [formats::CF_UNICODETEXT, formats::CF_TEXT, formats::CF_DIB]; // 支持的粘贴板类型
const MAX_TEXT: usize = 150; // 最大显示字数


lazy_static! {
    static ref CLIPBOARD_QUEUE: Arc<Mutex<VecDeque<Data>>> = Arc::new(Mutex::new(VecDeque::new()));
    static ref PREVIOUS_MD5: Arc<Mutex<[u8; 16]>> = Arc::new(Mutex::new([0; 16]));
    static ref CLIPBOARD_ENABLED: Arc<AtomicBool> = Arc::new(AtomicBool::new(false));
}

#[tokio::main]
async fn main() -> eframe::Result {
    let clipboard_queue_clone = Arc::clone(&CLIPBOARD_QUEUE);
    let previous_content_clone = Arc::clone(&PREVIOUS_MD5);
    let clipboard_enabled_clone = Arc::clone(&CLIPBOARD_ENABLED);

    // 启动一个线程监听剪切板变化
    tokio::spawn(async move {
        loop {
            if clipboard_enabled_clone.load(Ordering::Relaxed) {
                if let Some(format) = clipboard_win::raw::which_format_avail(SUPPORTED_FORMATS.as_ref()) {
                    let f = c_uint::from(format);
                    let mut current_content = Data::new(f);
                    if get_clipboard(RawData(c_uint::from(format))).map(|content: Vec<u8>| current_content.set_raw(content)).is_ok() {
                        match c_uint::from(format) {
                            // Unicode
                            formats::CF_UNICODETEXT => {
                                current_content.set_data(get_clipboard_string().unwrap_or(String::from("")));
                            }
                            // Ansi
                            formats::CF_TEXT => {
                                current_content.set_data(get_clipboard_string().unwrap_or(String::from("")));
                            }
                            // 图像
                            formats::CF_DIB => {}
                            _ => {
                                continue;
                            }
                        }
                        let mut previous_content = previous_content_clone.lock().unwrap();
                        if current_content.get_md5() != *previous_content {
                            *previous_content = current_content.get_md5().clone();
                            clipboard_queue_clone.lock().unwrap().push_back(current_content);
                        }
                    }
                }
            }
            tokio::time::sleep(Duration::from_millis(500)).await;
        }
    });

    // 启动一个线程监听全局键盘事件
    tokio::spawn(async move {
        if let Err(error) = listen(move |event| {
            static mut CTRL_PRESSED: bool = false;
            match event.event_type {
                EventType::KeyPress(Key::ControlLeft) | EventType::KeyPress(Key::ControlRight) => {
                    unsafe { CTRL_PRESSED = true };
                }
                EventType::KeyRelease(Key::ControlLeft) | EventType::KeyRelease(Key::ControlRight) => {
                    unsafe { CTRL_PRESSED = false };
                }
                EventType::KeyPress(Key::KeyV) => {
                    if CLIPBOARD_ENABLED.load(Ordering::Relaxed) {
                        unsafe {
                            if CTRL_PRESSED {
                                match CLIPBOARD_QUEUE.lock().unwrap().pop_front() {
                                    None => {
                                        // 队列耗尽
                                        CLIPBOARD_ENABLED.store(false, Ordering::Relaxed);
                                    }
                                    Some(content) => {
                                        let previous_content = Arc::clone(&PREVIOUS_MD5);
                                        if set_clipboard(RawData(content.get_type()), &content.get_content()).is_ok() {
                                            // 更新 previous_content 为下一个内容
                                            let mut previous_content_guard = previous_content.lock().unwrap();
                                            *previous_content_guard = content.get_md5().clone();
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                EventType::KeyPress(Key::BackQuote) => {
                    unsafe {
                        if CTRL_PRESSED {
                            // 切换开启状态
                            CLIPBOARD_ENABLED.store(!CLIPBOARD_ENABLED.load(Ordering::Relaxed), Ordering::Relaxed);
                            // tx.send(Signal::Visible(true)).unwrap();
                        }
                    }
                }
                _ => {}
            }
        }) {
            eprintln!("Error: {:?}", error);
        }
    });

    // 启动用户界面
    let native_options = NativeOptions {
        viewport: ViewportBuilder::default()
            .with_decorations(false)
            .with_max_inner_size([250.0, 400.0])
            .with_min_inner_size([250.0, 400.0])
            .with_resizable(false)
            .with_icon(
                icon_data::from_png_bytes(&include_bytes!("../assets/icon.png")[..])
                    .expect("Failed to load icon")
            )
            .with_transparent(true),
        ..Default::default()
    };

    run_native(
        "队列粘贴板",
        native_options,
        Box::new(|cc| Ok(Box::new(MyApp::new(cc)))),
    )
}