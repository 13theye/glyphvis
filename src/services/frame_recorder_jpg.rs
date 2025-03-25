// src/services/frame_recorder_jpg.rs

// FrameRecorder is a service for capturing frames from a wgpu::Texture and saving them to disk.
// Its gets its own thread to avoid blocking the main thread.
// Saving is done in batches and in parallel for maximum speed.
//
// We have discovered that this is suffering from inconsistent frame timing,
// so it is not currently being used.
//
// The timing issue is not due to disk IO as previously suspected.
// Suspect issue is in device polling and buffer management.

use nannou::{image::RgbaImage, wgpu};
use rayon::prelude::*;
use std::{
    collections::VecDeque,
    fs::{create_dir_all, File},
    io::BufWriter,
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        mpsc::{channel, Sender},
        Arc, Mutex,
    },
};

const BATCH_SIZE: usize = 10; // Process n frames at a time
const RESOLVED_TEXTURE_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;

const VERBOSE: bool = false; // true to show debug msgs

#[derive(Clone, Copy)]
pub enum OutputFormat {
    //PNG,
    JPEG(u8), // u8 parameter for JPEG quality (1-100)
}

// Type alias for the frame data tuple
type FrameData = (u32, Vec<u8>, u32, u32);

pub struct FrameRecorderOld {
    frame_sender: Sender<FrameData>,
    is_recording: Arc<Mutex<bool>>,
    last_capture: Arc<Mutex<u64>>,
    frame_limit: u32,
    frame_number: Arc<Mutex<u32>>,
    frames_in_queue: Arc<AtomicUsize>,
    frames_processed: Arc<AtomicUsize>,
    capture_in_progress: Arc<AtomicBool>,
    frame_time: u64,

    // capture pipeline
    texture_reshaper: wgpu::TextureReshaper,
    resolved_texture: wgpu::Texture, // for MSAA resolution
    staging_buffers: Vec<Arc<wgpu::Buffer>>,
    current_buffer_index: Arc<AtomicUsize>,
}

impl FrameRecorderOld {
    pub fn new(
        device: &wgpu::Device,
        render_texture: &wgpu::Texture,
        output_dir: &str,
        frame_limit: u32,
        format: OutputFormat,
        fps: u64,
    ) -> Self {
        create_dir_all(output_dir).expect("Failed to create output directory");

        let frames_in_queue = Arc::new(AtomicUsize::new(0));
        let frames_processed = Arc::new(AtomicUsize::new(0));
        let frames_processed_clone = frames_processed.clone();
        let frames_in_queue_clone = frames_in_queue.clone();

        let (sender, receiver) = channel();
        let thread_output_dir = output_dir.to_string();

        std::thread::spawn(move || {
            let mut frame_buffer: VecDeque<FrameData> = VecDeque::new();

            loop {
                match receiver.recv_timeout(std::time::Duration::from_millis(100)) {
                    Ok(frame_data) => {
                        frame_buffer.push_back(frame_data);

                        // Process if we have enough frames or after a timeout
                        if frame_buffer.len() >= BATCH_SIZE {
                            process_frame_batch(
                                &mut frame_buffer,
                                &thread_output_dir,
                                format,
                                &frames_processed_clone,
                                &frames_in_queue_clone,
                            );
                        }
                    }
                    Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                        // Process any remaining frames on timeout
                        if !frame_buffer.is_empty() {
                            process_frame_batch(
                                &mut frame_buffer,
                                &thread_output_dir,
                                format,
                                &frames_processed_clone,
                                &frames_in_queue_clone,
                            );
                        }
                    }
                    Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                        // Process remaining frames and exit
                        if !frame_buffer.is_empty() {
                            process_frame_batch(
                                &mut frame_buffer,
                                &thread_output_dir,
                                format,
                                &frames_processed_clone,
                                &frames_in_queue_clone,
                            );
                        }
                        break;
                    }
                }
            }
        });

        // Create a texture for resolving MSAA
        let resolved_texture = wgpu::TextureBuilder::new()
            .size([render_texture.width(), render_texture.height()])
            .sample_count(1) // No MSAA
            .format(RESOLVED_TEXTURE_FORMAT)
            .usage(
                wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::COPY_SRC
                    | wgpu::TextureUsages::COPY_DST
                    | wgpu::TextureUsages::TEXTURE_BINDING,
            )
            .build(device);

        // Create texture reshaper for MSAA resolution
        let texture_reshaper = wgpu::TextureReshaper::new(
            device,
            &render_texture.view().build(),
            render_texture.sample_count(), // source samples
            render_texture.sample_type(),
            1, // destination samples (no MSAA)
            RESOLVED_TEXTURE_FORMAT,
        );

        // Create triple staging buffers for GPU->CPU transfer
        const NUM_BUFFERS: usize = 3;
        let pixel_size = format_bytes_per_pixel(RESOLVED_TEXTURE_FORMAT);
        let bytes_per_row = wgpu::util::align_to(render_texture.width() * pixel_size, 256);
        let buffer_size = (bytes_per_row * render_texture.height()) as u64;

        let mut staging_buffers = Vec::with_capacity(NUM_BUFFERS);
        for i in 0..NUM_BUFFERS {
            let staging_buffer = Arc::new(device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(&format!("Frame Capture Staging Buffer {}", i)),
                size: buffer_size,
                usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                mapped_at_creation: false,
            }));
            staging_buffers.push(staging_buffer);
        }

        Self {
            frame_sender: sender,
            is_recording: Arc::new(Mutex::new(false)),
            last_capture: Arc::new(Mutex::new(0)),
            frame_limit,
            frame_number: Arc::new(Mutex::new(0)),
            frames_in_queue,
            frames_processed,
            capture_in_progress: Arc::new(AtomicBool::new(false)),
            frame_time: 1000000000 / fps,

            texture_reshaper,
            resolved_texture,
            staging_buffers,
            current_buffer_index: Arc::new(AtomicUsize::new(0)),
        }
    }

    pub fn toggle_recording(&self) {
        let mut is_recording = self.is_recording.lock().unwrap();
        *is_recording = !*is_recording;
        if *is_recording {
            *self.frame_number.lock().unwrap() = 0;
            self.frames_in_queue.store(0, Ordering::SeqCst);
            self.frames_processed.store(0, Ordering::SeqCst);
            println!("Recording started");
        } else {
            println!("Recording stopped");
        }
    }

    pub fn is_recording(&self) -> bool {
        *self.is_recording.lock().unwrap()
    }

    pub fn send_frame_data(&self, frame_data: FrameData) -> Result<(), String> {
        self.frames_in_queue.fetch_add(1, Ordering::SeqCst);
        if let Err(e) = self.frame_sender.send(frame_data) {
            self.frames_in_queue.fetch_sub(1, Ordering::SeqCst);
            return Err(format!("Failed to send frame: {}", e));
        }
        Ok(())
    }

    pub fn capture_frame(
        &self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        render_texture: &wgpu::Texture,
    ) {
        if !self.is_recording() {
            return;
        }

        // Check if enough time has passed since last capture
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64;

        // Check for timing gaps
        let mut last_capture = self.last_capture.lock().unwrap();
        let time_since_last = now - *last_capture;
        if time_since_last > self.frame_time {
            println!(
                "WARNING: Frame timing gap detected - {}ms since last capture (expected {}ms)",
                time_since_last / 1_000_000,
                self.frame_time / 1_000_000
            );

            // Check if previous capture is still in progress
            if self.capture_in_progress.load(Ordering::SeqCst) {
                println!(
                    "DEBUG: Previous capture still processing after {}ms",
                    time_since_last / 1_000_000
                );
                return;
            }
        }

        // Skip this capture if not enough time has passed
        if now - *last_capture < self.frame_time {
            return;
        }

        // Begin capture process - note the time, set capture in progress flag
        self.capture_in_progress.store(true, Ordering::SeqCst);
        *last_capture = now;
        let frame_start = std::time::Instant::now();

        // Check if we've reached the frame limit
        let mut frame_number = self.frame_number.lock().unwrap();
        if *frame_number >= self.frame_limit {
            self.toggle_recording();
            return;
        }

        // Increment frame number
        *frame_number += 1;
        let frame_num = *frame_number;

        // Get the next staging buffer
        let buffer_index = {
            let current = self.current_buffer_index.load(Ordering::SeqCst);
            let next = (current + 1) % self.staging_buffers.len();
            self.current_buffer_index.store(next, Ordering::SeqCst);
            current
        };
        let staging_buffer = self.staging_buffers[buffer_index].clone();

        // GPU
        // Step 1: Use the reshaper to resolve MSAA
        let msaa_start = std::time::Instant::now();
        self.texture_reshaper
            .encode_render_pass(&self.resolved_texture.view().build(), encoder);
        if VERBOSE {
            println!("MSAA resolve took: {:?}", msaa_start.elapsed());
        }

        // Step 2: Copy from resolved texture to staging buffer
        // Calculate minimum bytes per row required by wgpu
        let pixel_size = format_bytes_per_pixel(RESOLVED_TEXTURE_FORMAT);
        let bytes_per_row = wgpu::util::align_to(self.resolved_texture.width() * pixel_size, 256);
        let copy_start = std::time::Instant::now();
        encoder.copy_texture_to_buffer(
            self.resolved_texture.as_image_copy(),
            wgpu::ImageCopyBuffer {
                buffer: &staging_buffer,
                layout: wgpu::ImageDataLayout {
                    offset: 0,
                    bytes_per_row: Some(bytes_per_row),
                    rows_per_image: Some(render_texture.height()),
                },
            },
            wgpu::Extent3d {
                width: render_texture.width(),
                height: render_texture.height(),
                depth_or_array_layers: 1,
            },
        );
        if VERBOSE {
            println!("Texture to buffer copy took: {:?}", copy_start.elapsed());
        }

        // Step 3: Map the buffer and send the data
        let staging_buffer_clone = staging_buffer.clone();
        let sender = self.frame_sender.clone();
        let frames_in_queue = self.frames_in_queue.clone();
        let capture_in_progress_outer = self.capture_in_progress.clone();

        let width = render_texture.width();
        let height = render_texture.height();

        // Submit the encoder (prevents buffer mapping deadlock)
        device.poll(wgpu::Maintain::Poll);

        // Map buffer and process data
        let buffer_map_start = std::time::Instant::now();
        staging_buffer
            .slice(..)
            .map_async(wgpu::MapMode::Read, move |result| {
                match result {
                    Ok(()) => {
                        let unpad_start = std::time::Instant::now();

                        let unpadded_data = {
                            let mapped_memory = staging_buffer_clone.slice(..).get_mapped_range();
                            let mut unpadded_data =
                                Vec::with_capacity((width * height * pixel_size) as usize);

                            // Use copy_from_slice for bulk copying of consecutive rows
                            let actual_row_bytes = (width * pixel_size) as usize;
                            let mut src_offset = 0;

                            // Pre-allocate the full buffer
                            unpadded_data.resize((width * height * pixel_size) as usize, 0);

                            // Copy each row efficiently
                            for row in 0..height {
                                let dest_offset = row as usize * actual_row_bytes;
                                let src_start = src_offset;
                                let src_end = src_start + actual_row_bytes;

                                unpadded_data[dest_offset..dest_offset + actual_row_bytes]
                                    .copy_from_slice(&mapped_memory[src_start..src_end]);

                                src_offset += bytes_per_row as usize;
                            }
                            // return
                            unpadded_data
                        };
                        if VERBOSE {
                            println!("Unpadding took: {:?}", unpad_start.elapsed());
                        }

                        staging_buffer_clone.unmap();

                        // Send the frame data
                        let send_start = std::time::Instant::now();
                        frames_in_queue.fetch_add(1, Ordering::SeqCst);
                        if let Err(e) = sender.send((frame_num, unpadded_data, width, height)) {
                            frames_in_queue.fetch_sub(1, Ordering::SeqCst);
                            eprintln!("Failed to send frame: {}", e);
                        }
                        if VERBOSE {
                            println!("Frame send took: {:?}", send_start.elapsed());
                        }
                    }
                    Err(e) => {
                        eprintln!("Buffer mapping error: {}", e);
                        staging_buffer_clone.unmap();
                    }
                }
                capture_in_progress_outer.store(false, Ordering::SeqCst);
                if VERBOSE {
                    println!(
                        "Total buffer mapping and processing took: {:?}",
                        buffer_map_start.elapsed()
                    );
                }
            });

        if VERBOSE {
            println!("Total frame capture took: {:?}", frame_start.elapsed());
        }

        // Poll the device with a timeout to avoid infinite waiting
        let timeout_duration = std::time::Duration::from_millis(50);
        let start_time = std::time::Instant::now();

        while start_time.elapsed() < timeout_duration {
            match device.poll(wgpu::Maintain::Wait) {
                // If maintenance returns true, it means there are no more pending operations
                true => {
                    return;
                }
                false => {
                    // Sleep a tiny bit to prevent tight polling
                    println!("DEBUG: Sleeping 1ms to prevent tight polling");
                    std::thread::sleep(std::time::Duration::from_millis(1));
                }
            }
        }
        // If we reach this point, the poll timed out. Clean up pending operations.
        device.poll(wgpu::Maintain::Poll);
        self.capture_in_progress.store(false, Ordering::SeqCst);
        println!(
            "WARNING: Device poll timed out after {:?}",
            timeout_duration
        );
    }

    pub fn get_queue_status(&self) -> (usize, usize) {
        let processed = self.frames_processed.load(Ordering::SeqCst);
        let total = self.frames_in_queue.load(Ordering::SeqCst);
        println!("Queue status - Processed: {}, Total: {}", processed, total);
        (processed, total)
    }

    pub fn has_pending_frames(&self) -> bool {
        let (processed, total) = self.get_queue_status();
        processed < total
    }
}

fn process_frame_batch(
    frame_buffer: &mut VecDeque<FrameData>,
    output_dir: &str,
    format: OutputFormat,
    frames_processed: &AtomicUsize,
    _frames_in_queue: &AtomicUsize,
) {
    // Convert batch of frames to a vector for parallel processing
    let frames: Vec<_> = frame_buffer.drain(..).collect();

    // Process frames in parallel
    frames
        .into_par_iter()
        .for_each(|(frame_number, frame_data, width, height)| {
            let jpeg_start = std::time::Instant::now();

            if let Some(image_buffer) = RgbaImage::from_raw(width, height, frame_data) {
                let filename = match format {
                    OutputFormat::JPEG(_) => format!("{}/frame{:05}.jpg", output_dir, frame_number),
                };

                let result = match format {
                    OutputFormat::JPEG(quality) => {
                        // Process JPEG in a scope to ensure memory is freed immediately
                        let result = {
                            let rgb_buffer =
                                nannou::image::DynamicImage::ImageRgba8(image_buffer).to_rgb8();
                            let file = File::create(&filename).ok();
                            if let Some(file) = file {
                                let mut buf_writer = BufWriter::new(file);
                                nannou::image::codecs::jpeg::JpegEncoder::new_with_quality(
                                    &mut buf_writer,
                                    quality,
                                )
                                .encode(
                                    rgb_buffer.as_raw(),
                                    rgb_buffer.width(),
                                    rgb_buffer.height(),
                                    nannou::image::ColorType::Rgb8,
                                )
                            } else {
                                Err(nannou::image::ImageError::IoError(std::io::Error::new(
                                    std::io::ErrorKind::Other,
                                    "Failed to create file",
                                )))
                            }
                        };
                        if VERBOSE {
                            println!(
                                "Frame {:?} encoding took: {:?}",
                                frame_number,
                                jpeg_start.elapsed()
                            );
                        }
                        result
                    }
                };

                if let Err(e) = result {
                    eprintln!("Failed to save frame {}: {}", frame_number, e);
                } else {
                    frames_processed.fetch_add(1, Ordering::SeqCst);
                }
            }
        });
}

fn format_bytes_per_pixel(format: wgpu::TextureFormat) -> u32 {
    match format {
        wgpu::TextureFormat::Rgba8Unorm | wgpu::TextureFormat::Rgba8UnormSrgb => 4,
        wgpu::TextureFormat::Rgba16Float => 8,
        wgpu::TextureFormat::Rgba32Float => 16,
        wgpu::TextureFormat::Bgra8Unorm | wgpu::TextureFormat::Bgra8UnormSrgb => 4,
        wgpu::TextureFormat::Rg16Float => 4,
        wgpu::TextureFormat::Rg32Float => 8,
        wgpu::TextureFormat::R32Float => 4,
        // Add other formats as needed
        _ => panic!("Unsupported texture format: {:?}", format),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nannou::wgpu;
    use std::fs;
    use std::time::Duration;

    // Helper to create a test wgpu device
    fn create_test_device() -> (wgpu::Device, wgpu::Queue) {
        let instance = nannou::wgpu::Instance::default();

        pollster::block_on(async {
            let adapter = instance
                .request_adapter(&nannou::wgpu::RequestAdapterOptions {
                    power_preference: nannou::wgpu::PowerPreference::HighPerformance,
                    force_fallback_adapter: false,
                    compatible_surface: None,
                })
                .await
                .expect("Failed to find appropriate adapter");

            adapter
                .request_device(
                    &nannou::wgpu::DeviceDescriptor {
                        label: None,
                        features: nannou::wgpu::Features::empty(),
                        limits: nannou::wgpu::Limits::default(),
                    },
                    None,
                )
                .await
                .expect("Failed to create device")
        })
    }

    fn create_test_texture(device: &wgpu::Device, width: u32, height: u32) -> wgpu::Texture {
        wgpu::TextureBuilder::new()
            .size([width, height])
            .sample_count(1)
            .format(wgpu::TextureFormat::Rgba8UnormSrgb)
            .usage(
                wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::COPY_SRC
                    | wgpu::TextureUsages::TEXTURE_BINDING,
            )
            .build(device)
    }

    fn create_test_frame(width: u32, height: u32) -> Vec<u8> {
        let size = (width * height * 4) as usize;
        let mut data = Vec::with_capacity(size);
        for i in 0..size {
            data.push((i % 255) as u8); // Creates a gradient pattern
        }
        data
    }

    fn create_test_dir() -> String {
        let mut attempts = 0;
        loop {
            let test_dir = format!(
                "test_frames_{}",
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            );

            if let Err(e) = fs::create_dir(&test_dir) {
                if e.kind() == std::io::ErrorKind::AlreadyExists {
                    attempts += 1;
                    if attempts > 10 {
                        panic!("Failed to create unique test directory after 10 attempts");
                    }
                    continue;
                }
                panic!("Failed to create test directory: {}", e);
            }
            return test_dir;
        }
    }

    fn cleanup_test_dir(dir: &str) {
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn test_initialization() {
        let (device, _) = create_test_device();
        let texture = create_test_texture(&device, 640, 480);
        let test_dir = create_test_dir();

        let recorder = FrameRecorderOld::new(
            &device,
            &texture,
            &test_dir,
            100,
            OutputFormat::JPEG(85),
            30,
        );

        assert!(
            !recorder.is_recording(),
            "Should not be recording initially"
        );
        assert!(
            !recorder.has_pending_frames(),
            "Should not have pending frames initially"
        );

        let (processed, total) = recorder.get_queue_status();
        assert_eq!(processed, 0, "Initial processed count should be 0");
        assert_eq!(total, 0, "Initial total count should be 0");

        cleanup_test_dir(&test_dir);
    }

    #[test]
    fn test_recording_toggle() {
        let (device, _) = create_test_device();
        let texture = create_test_texture(&device, 640, 480);
        let test_dir = create_test_dir();

        let recorder = FrameRecorderOld::new(
            &device,
            &texture,
            &test_dir,
            100,
            OutputFormat::JPEG(85),
            30,
        );

        assert!(
            !recorder.is_recording(),
            "Should not be recording initially"
        );

        recorder.toggle_recording();
        assert!(recorder.is_recording(), "Should be recording after toggle");

        recorder.toggle_recording();
        assert!(
            !recorder.is_recording(),
            "Should not be recording after second toggle"
        );

        cleanup_test_dir(&test_dir);
    }

    #[test]
    fn test_frame_capture() {
        let (device, queue) = create_test_device();
        let texture = create_test_texture(&device, 640, 480);
        let test_dir = create_test_dir();

        let recorder = FrameRecorderOld::new(
            &device,
            &texture,
            &test_dir,
            100,
            OutputFormat::JPEG(85),
            30,
        );

        recorder.toggle_recording();

        // Create and submit a test frame
        let mut encoder =
            device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        recorder.capture_frame(&device, &mut encoder, &texture);
        queue.submit(Some(encoder.finish()));

        // Wait for processing
        std::thread::sleep(Duration::from_millis(100));

        let (processed, total) = recorder.get_queue_status();
        assert!(total > 0, "Should have frames in queue");
        assert!(
            processed <= total,
            "Processed frames should not exceed total"
        );

        cleanup_test_dir(&test_dir);
    }

    #[test]
    fn test_queue_status_batch_processing() {
        let (device, _) = create_test_device();
        let texture = create_test_texture(&device, 100, 100);
        let test_dir = create_test_dir();

        let recorder = FrameRecorderOld::new(
            &device,
            &texture,
            &test_dir,
            100,
            OutputFormat::JPEG(85),
            30,
        );

        let frame_data = create_test_frame(100, 100);
        for i in 0..BATCH_SIZE + 1 {
            recorder
                .send_frame_data((i as u32, frame_data.clone(), 100, 100))
                .unwrap();
        }

        // Give time for batch processing
        std::thread::sleep(Duration::from_millis(500));

        let (processed, total) = recorder.get_queue_status();
        assert_eq!(
            total,
            BATCH_SIZE + 1,
            "Total should match number of frames sent"
        );
        assert!(
            processed >= BATCH_SIZE,
            "At least one batch should be processed"
        );

        cleanup_test_dir(&test_dir);
    }

    #[test]
    fn test_frame_limit() {
        let (device, queue) = create_test_device();
        let texture = create_test_texture(&device, 640, 480);
        let test_dir = create_test_dir();
        let frame_limit = 3;

        let recorder = FrameRecorderOld::new(
            &device,
            &texture,
            &test_dir,
            frame_limit,
            OutputFormat::JPEG(85),
            30,
        );

        recorder.toggle_recording();

        // Try to capture more frames than the limit
        for _ in 0..frame_limit + 2 {
            let mut encoder =
                device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
            recorder.capture_frame(&device, &mut encoder, &texture);
            queue.submit(Some(encoder.finish()));
            std::thread::sleep(Duration::from_millis(50));
        }

        // Wait for processing
        std::thread::sleep(Duration::from_millis(200));

        let (processed, total) = recorder.get_queue_status();
        assert!(
            total <= frame_limit as usize,
            "Should not exceed frame limit"
        );
        assert_eq!(processed, total, "All frames should be processed");
        assert!(
            !recorder.is_recording(),
            "Recording should stop at frame limit"
        );

        cleanup_test_dir(&test_dir);
    }

    #[test]
    fn test_frame_timing() {
        let (device, queue) = create_test_device();
        let texture = create_test_texture(&device, 640, 480);
        let test_dir = create_test_dir();

        let recorder = FrameRecorderOld::new(
            &device,
            &texture,
            &test_dir,
            100,
            OutputFormat::JPEG(85),
            30,
        );

        recorder.toggle_recording();

        // Try to capture frames faster than FPS limit
        let mut encoder =
            device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        let time_between_captures = Duration::from_millis(10); // Much faster than FPS allows
        let start = std::time::Instant::now();
        //let mut captures = 0;

        for _ in 0..5 {
            recorder.capture_frame(&device, &mut encoder, &texture);
            //captures += 1;
            queue.submit(Some(encoder.finish()));
            std::thread::sleep(time_between_captures);
            encoder =
                device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        }

        let fps = 30;
        let elapsed = start.elapsed();
        let theoretical_frames = (elapsed.as_secs_f64() * fps as f64).floor() as u32;

        let (_processed, total) = recorder.get_queue_status();
        assert!(
            total <= theoretical_frames as usize,
            "Should not capture more frames than FPS limit allows"
        );

        cleanup_test_dir(&test_dir);
    }

    #[test]
    fn test_output_files() {
        let (device, queue) = create_test_device();
        let texture = create_test_texture(&device, 320, 240);
        let test_dir = create_test_dir();

        let recorder = FrameRecorderOld::new(
            &device,
            &texture,
            &test_dir,
            100,
            OutputFormat::JPEG(85),
            30,
        );

        recorder.toggle_recording();

        // Capture a few frames
        for _ in 0..3 {
            let mut encoder =
                device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
            recorder.capture_frame(&device, &mut encoder, &texture);
            queue.submit(Some(encoder.finish()));
            std::thread::sleep(Duration::from_millis(50));
        }

        // Wait for processing to complete
        std::thread::sleep(Duration::from_millis(500));

        // Check if output files exist and have reasonable sizes
        for i in 1..=3 {
            let file_path = format!("{}/frame{:05}.jpg", test_dir, i);
            let metadata = fs::metadata(&file_path).expect("Output file should exist");
            assert!(
                metadata.len() > 1000,
                "JPEG file should have reasonable size"
            );
        }

        cleanup_test_dir(&test_dir);
    }

    #[test]
    fn test_cleanup_after_recording() {
        let (device, queue) = create_test_device();
        let texture = create_test_texture(&device, 640, 480);
        let test_dir = create_test_dir();

        let recorder = FrameRecorderOld::new(
            &device,
            &texture,
            &test_dir,
            100,
            OutputFormat::JPEG(85),
            30,
        );

        recorder.toggle_recording();

        // Capture some frames
        for _ in 0..3 {
            let mut encoder =
                device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
            recorder.capture_frame(&device, &mut encoder, &texture);
            queue.submit(Some(encoder.finish()));
            std::thread::sleep(Duration::from_millis(50));
        }

        recorder.toggle_recording();

        // Wait for processing to complete
        std::thread::sleep(Duration::from_millis(500));

        let (processed, total) = recorder.get_queue_status();
        assert_eq!(
            processed, total,
            "All frames should be processed after recording stops"
        );

        cleanup_test_dir(&test_dir);
    }
}
