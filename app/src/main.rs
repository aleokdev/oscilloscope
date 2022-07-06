use std::time::Duration;

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
        read_buf: [u8; 128],
        read_buf_idx: usize,
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
            read_buf: [0; 128],
            read_buf_idx: 0,
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
                                .parity(serialport::Parity::None)
                                .stop_bits(serialport::StopBits::One)
                                .timeout(Duration::from_secs(10));
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
                read_buf,
                read_buf_idx,
                value_buf,
                value_buf_idx,
            } => {
                let bytes_read = port.read(&mut read_buf[*read_buf_idx..]).unwrap();
                if bytes_read > 0 {
                    println!("read {bytes_read}");
                }
                *read_buf_idx += bytes_read;

                let serial_read = &read_buf[..*read_buf_idx];

                let mut current_val: usize = 0;
                let mut current_digit = 0;
                let mut total_rotation = 0;
                for byte in serial_read.iter().copied() {
                    match byte as char {
                        '0'..='9' => {
                            let value = byte - '0' as u8;
                            current_val += value as usize * 10usize.pow(current_digit);
                            current_digit += 1;
                        }
                        ',' => {
                            // Value is encoded as simple decimal, length is log2 of the value + comma character
                            let value_serial_length = current_digit as usize + 1;
                            total_rotation += value_serial_length;
                            *read_buf_idx -= value_serial_length;
                            value_buf[*value_buf_idx] = current_val as f32;

                            *value_buf_idx += 1;
                            *value_buf_idx %= value_buf.len();
                            println!("{}", current_val);

                            current_digit = 0;
                            current_val = 0;
                        }
                        '\0' => {
                            total_rotation += 1;
                        }
                        byte => {
                            eprintln!("Got unknown byte '{byte:?}'");
                            total_rotation += 1;
                        }
                    }
                }

                if *read_buf_idx == read_buf.len() {
                    *read_buf_idx = 0;
                }

                read_buf.rotate_left(total_rotation);

                ui.label(format!(
                    "Read buffer: {}/{} bytes used",
                    read_buf_idx,
                    read_buf.len()
                ));

                ui.label(format!("{:?}", read_buf));

                egui::plot::Plot::new("plot")
                    .include_y(1023.)
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
