// frame_recorder.rs
use std::fs::{ create_dir_all, File };
use std::io::BufWriter;
use std::sync::{Arc, Mutex};
use std::sync::mpsc::{channel, Sender, Receiver};
use std::collections::VecDeque;
use nannou::wgpu;
use nannou::image::RgbaImage;

const BATCH_SIZE: usize = 30; // Process 30 frames at a time
const FPS: u64 = 30;
const FRAME_TIME: u64 = 1_000_000_000 / FPS; // Duration in nanoseconds between frames

#[derive(Clone, Copy)]
pub enum OutputFormat {
    PNG,
    JPEG(u8),  // u8 parameter for JPEG quality (1-100)
    BMP,
}

// Type alias for the frame data tuple
type FrameData = (u32, Vec<u8>, u32, u32);

pub struct FrameRecorder {
    frame_sender: Sender<FrameData>,
    is_recording: Arc<Mutex<bool>>,
    last_capture: Arc<Mutex<u64>>,
    frame_limit: u32,
    frame_number: Arc<Mutex<u32>>,
    texture_capturer: wgpu::TextureCapturer,}

impl FrameRecorder {
    pub fn new(output_dir: &str, frame_limit: u32, format: OutputFormat) -> Self {
        create_dir_all(output_dir).expect("Failed to create output directory");
        
        let (sender, receiver): (Sender<FrameData>, Receiver<FrameData>) = channel();
        let thread_output_dir = output_dir.to_string();
        let format = format;

        // Create a buffer for batching frames
        let mut frame_buffer: VecDeque<FrameData> = VecDeque::new();

        std::thread::spawn(move || {
            while let Ok(frame_data) = receiver.recv() {
                frame_buffer.push_back(frame_data);

                // Process frames in batches
                if frame_buffer.len() >= BATCH_SIZE {
                    process_frame_batch(&mut frame_buffer, &thread_output_dir, format);
                }
            }
            // Process remaining frames when channel is closed
            if !frame_buffer.is_empty() {
                process_frame_batch(&mut frame_buffer, &thread_output_dir, format);
            }
        });

        Self {
            frame_sender: sender,
            is_recording: Arc::new(Mutex::new(false)),
            last_capture: Arc::new(Mutex::new(0)),
            frame_limit,
            frame_number: Arc::new(Mutex::new(0)),
            texture_capturer: wgpu::TextureCapturer::default(),        }
    }

    pub fn toggle_recording(&self) {
        let mut is_recording = self.is_recording.lock().unwrap();
        *is_recording = !*is_recording;
        if *is_recording {
            *self.frame_number.lock().unwrap() = 0;
            println!("Recording started");
        } else {
            println!("Recording stopped");
        }
    }

    pub fn is_recording(&self) -> bool {
        *self.is_recording.lock().unwrap()
    }

    pub fn capture_frame(
        &self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        texture: &wgpu::Texture,
    ) {
        if !self.is_recording() {
            return;
        }

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64;

        let mut last_capture = self.last_capture.lock().unwrap();
        if now - *last_capture < FRAME_TIME {
            return;
        }
        *last_capture = now;

        let mut frame_number = self.frame_number.lock().unwrap();
        if *frame_number > self.frame_limit {
            self.toggle_recording();
            return;
        }

        let frame_num = *frame_number;
        *frame_number += 1;

        let snapshot = self.texture_capturer.capture(device, encoder, texture);
        let sender = self.frame_sender.clone();
        let width = texture.width();
        let height = texture.height();

        if let Err(e) = snapshot.read(move |result| {
            match result {
                Ok(buffer) => {
                    // Create a new Vec and copy the buffer data into it
                    let data = buffer.to_owned().into_raw();
                    if let Err(e) = sender.send((frame_num, data, width, height)) {
                        eprintln!("Failed to send frame: {}", e);
                    }
                },
                Err(e) => {
                    eprintln!("Failed to read texture: {:?}", e);
                }
            }
        }) {
            eprintln!("Failed to read snapshot: {:?}", e);
        }
    }
}

fn process_frame_batch(frame_buffer: &mut VecDeque<FrameData>, output_dir: &str, format: OutputFormat) {
    while let Some((frame_number, frame_data, width, height)) = frame_buffer.pop_front() {
        // Convert the raw pixel data to an image buffer
        if let Some(image_buffer) = RgbaImage::from_raw(width, height, frame_data) {
            // Create the filename based on format
            let filename = match format {
                OutputFormat::PNG => format!("{}/frame{:04}.png", output_dir, frame_number),
                OutputFormat::JPEG(_) => format!("{}/frame{:04}.jpg", output_dir, frame_number),
                OutputFormat::BMP => format!("{}/frame{:04}.bmp", output_dir, frame_number),
            };

            let result = match format {
                OutputFormat::PNG => image_buffer.save(&filename),
                OutputFormat::JPEG(quality) => {
                    // Convert to RGB for JPEG (removes alpha channel)
                    let rgb_buffer = nannou::image::DynamicImage::ImageRgba8(image_buffer).to_rgb8();
                    let file = File::create(&filename).ok();
                    if let Some(file) = file {
                        let mut buf_writer = BufWriter::new(file);
                        nannou::image::codecs::jpeg::JpegEncoder::new_with_quality(&mut buf_writer, quality)
                            .encode(
                                rgb_buffer.as_raw(),
                                rgb_buffer.width(),
                                rgb_buffer.height(),
                                nannou::image::ColorType::Rgb8
                            )
                    } else {
                        Err(nannou::image::ImageError::IoError(
                            std::io::Error::new(std::io::ErrorKind::Other, "Failed to create file")
                        ))
                    }
                },
                OutputFormat::BMP => {
                    image_buffer.save_with_format(&filename, nannou::image::ImageFormat::Bmp)
                }
            };

            if let Err(e) = result {
                eprintln!("Failed to save frame {}: {}", frame_number, e);
            }
        }
    }
}