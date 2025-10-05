use crossbeam_channel::{Receiver, Sender};
use tracing::info;
use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};
#[cfg(target_os = "windows")]
use winit::platform::windows::EventLoopBuilderExtWindows;

pub enum UiCommand {
    Show,
}

pub enum UiEvent {
    HelloClicked,
}

pub struct UiHandles {
    pub command_sender: Sender<UiCommand>,
    pub event_receiver: Receiver<UiEvent>,
}

pub fn spawn_ui_thread() -> UiHandles {
    let (command_sender, command_receiver) = crossbeam_channel::unbounded::<UiCommand>();
    let (event_sender, event_receiver) = crossbeam_channel::unbounded::<UiEvent>();

    std::thread::spawn(move || {
        let mut native_options = eframe::NativeOptions::default();
        native_options.event_loop_builder = Some(Box::new(|builder| {
            #[cfg(target_os = "windows")]
            {
                builder.with_any_thread(true);
            }
        }));
        eframe::run_native(
            "Serpentines",
            native_options,
            Box::new(move |creation_context| {
                let (_pending_command_sender, pending_command_receiver) = crossbeam_channel::unbounded::<UiCommand>();
                #[cfg(target_os = "windows")]
                let hwnd_from_context_value: Option<isize> = {
                    if let Ok(window_handle) = creation_context.window_handle() {
                        match window_handle.as_raw() {
                            RawWindowHandle::Win32(win) => {
                                let val = win.hwnd.get();
                                Some(val)
                            }
                            _ => None,
                        }
                    } else {
                        None
                    }
                };
                let egui_context = creation_context.egui_ctx.clone();
                std::thread::spawn(move || {
                    // Block on incoming commands from platform threads and wake egui per message
                    while let Ok(incoming) = command_receiver.recv() {
                        match incoming {
                            UiCommand::Show => {
                                info!("UI forwarder: Show -> OS Show + Visible(true) + Focus");
                                #[cfg(target_os = "windows")]
                                if let Some(hwnd_val) = hwnd_from_context_value {
                                    use windows::Win32::UI::WindowsAndMessaging::{ShowWindow, SetForegroundWindow, SW_SHOW};
                                    use windows::Win32::Foundation::HWND;
                                    let hwnd = HWND(hwnd_val as *mut core::ffi::c_void);
                                    unsafe {
                                        let _ = ShowWindow(hwnd, SW_SHOW);
                                        let _ = SetForegroundWindow(hwnd);
                                    }
                                }
                                egui_context.send_viewport_cmd(egui::ViewportCommand::Visible(true));
                                egui_context.send_viewport_cmd(egui::ViewportCommand::Focus);
                                egui_context.request_repaint();
                            }
                        }
                    }
                });

                Ok(Box::new(SerpentinesApp {
                    pending_command_receiver,
                    event_sender,
                }))
            }),
        ).expect("eframe failed to start");
    });

    UiHandles { command_sender, event_receiver }
}

pub struct SerpentinesApp {
    pending_command_receiver: Receiver<UiCommand>,
    event_sender: Sender<UiEvent>,
}

impl eframe::App for SerpentinesApp {
    fn update(&mut self, context: &egui::Context, _frame: &mut eframe::Frame) {
        // Intercept OS close requests via egui/eframe: hide (Visible(false)) and cancel the close.
        let close_requested = context.input(|i| i.viewport().close_requested());
        if close_requested {
            info!("UI: CloseRequested -> hide and cancel");
            context.send_viewport_cmd(egui::ViewportCommand::Visible(false));
            context.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            context.request_repaint();
        }
        while let Ok(_command) = self.pending_command_receiver.try_recv() {
            // Currently no UI-thread commands are processed inside update.
        }

        egui::CentralPanel::default().show(context, |ui| {
            if ui.button("Hello, egui!").clicked() {
                info!("UI: Hello button clicked");
                let _ = self.event_sender.send(UiEvent::HelloClicked);
            }
        });
    }
}
