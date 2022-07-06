use std::time::{Duration, Instant};

use eframe::{egui, epaint::Color32};
use serialport::{FlowControl, SerialPort, SerialPortInfo};

fn main() {
    let native_options = eframe::NativeOptions::default();
    eframe::run_native(
        "SimpleOscilloscope",
        native_options,
        Box::new(|cc| Box::new(MyEguiApp::new(cc))),
    );
}

enum State {
    SelectingSerialPort {
        ports: serialport::Result<Vec<SerialPortInfo>>,
        selected: usize,
        creation_error: Option<serialport::Error>,
    },
    Reading {
        port: Box<dyn SerialPort>,
        value_buf: Vec<f32>,
        value_buf_idx: usize,

        samples_read_this_second: usize,
        samples_read_prev_second: usize,
        next_second: Instant,
    },
}

impl State {
    pub fn new_selecting_serial_port() -> Self {
        State::SelectingSerialPort {
            ports: serialport::available_ports(),
            selected: 0,
            creation_error: None,
        }
    }

    pub fn new_reading(port: Box<dyn SerialPort>) -> Self {
        State::Reading {
            port,
            value_buf: vec![0.; 128],
            value_buf_idx: 0,

            samples_read_prev_second: 0,
            samples_read_this_second: 0,
            next_second: Instant::now(),
        }
    }
}

struct MyEguiApp {
    state: State,
}

impl MyEguiApp {
    fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        // Customize egui here with cc.egui_ctx.set_fonts and cc.egui_ctx.set_visuals.
        // Restore app state using cc.storage (requires the "persistence" feature).
        // Use the cc.gl (a glow::Context) to create graphics shaders and buffers that you can use
        // for e.g. egui::PaintCallback.
        Self {
            state: State::new_selecting_serial_port(),
        }
    }
}

impl eframe::App for MyEguiApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| match &mut self.state {
            State::SelectingSerialPort {
                ports,
                selected,
                creation_error,
            } => {
                match ports {
                    Ok(ports) if ports.len() == 0 => {
                        ui.label(
                            "No serial ports available (Is the Arduino plugged in correctly?)",
                        );
                    }
                    Ok(ports) => {
                        egui::ComboBox::from_label("Serial port")
                            .selected_text(&ports[*selected].port_name)
                            .show_ui(ui, |ui| {
                                for (idx, port) in ports.iter().enumerate() {
                                    ui.selectable_value(selected, idx, &port.port_name);
                                }
                            });

                        if let Some(err) = creation_error {
                            ui.colored_label(Color32::RED, format!("{}", err));
                        }

                        if ui.button("Open").clicked() {
                            let port = serialport::new(&ports[*selected].port_name, 9600)
                                .data_bits(serialport::DataBits::Eight)
                                .flow_control(serialport::FlowControl::None)
                                .parity(serialport::Parity::Even)
                                .stop_bits(serialport::StopBits::One);
                            match port.open() {
                                Ok(port) => self.state = State::new_reading(port),
                                Err(err) => *creation_error = Some(err),
                            }
                        }
                    }
                    Err(err) => {
                        ui.label(format!("Error obtaining serial ports: {}", err));
                    }
                }
                if ui.button("Reload").clicked() {
                    self.state = State::new_selecting_serial_port();
                }
            }
            State::Reading {
                port,

                value_buf,
                value_buf_idx,

                samples_read_this_second,
                samples_read_prev_second,
                next_second,
            } => {
                let mut bytes = [0u8; 2];

                while let Ok(()) = port.read_exact(&mut bytes) {
                    let value = u16::from_le_bytes(bytes);
                    if value < 1024 {
                        value_buf[*value_buf_idx] = value as f32 * 5. / 1023.;
                        *value_buf_idx += 1;
                        *value_buf_idx %= value_buf.len();
                        *samples_read_this_second += 1;
                    }
                }

                let instant = Instant::now();
                if instant >= *next_second {
                    *samples_read_prev_second = *samples_read_this_second;
                    *samples_read_this_second = 0;
                    *next_second = instant + Duration::from_secs(1);
                }

                ui.label(format!(
                    "Sampling frequency: {}Hz",
                    samples_read_prev_second
                ));

                egui::plot::Plot::new("plot")
                    .include_y(5.)
                    .include_y(0.)
                    .show(ui, |plot| {
                        plot.line(egui::plot::Line::new(egui::plot::Values::from_ys_f32(
                            &value_buf,
                        )))
                    });
                ctx.request_repaint();
            }
        });
    }
}
