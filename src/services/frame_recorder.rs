// src/services/frame_recorder.rs
// FrameRecorder is a service for capturing frames from a wgpu::Texture and saving them to disk
// Its own thread is used to process frames in batches to avoid blocking the main thread

use std::fs::{ create_dir_all, File };
use std::io::BufWriter;
use std::sync::{ Arc, Mutex };
use std::sync::mpsc::{ channel, Sender };
use std::collections::VecDeque;
use nannou::wgpu;
use nannou::image::RgbaImage;
use std::sync::atomic::{ AtomicUsize, AtomicBool, Ordering };

const BATCH_SIZE: usize = 10; // Process n frames at a time
const FPS: u64 = 30;
const FRAME_TIME: u64 = 1_000_000_000 / FPS; // Duration in nanoseconds between frames

#[derive(Clone, Copy)]
pub enum OutputFormat {
    //PNG,
    JPEG(u8),  // u8 parameter for JPEG quality (1-100)
}

// Type alias for the frame data tuple
type FrameData = (u32, Vec<u8>, u32, u32);

pub struct FrameRecorder {
    frame_sender: Sender<FrameData>,
    is_recording: Arc<Mutex<bool>>,
    last_capture: Arc<Mutex<u64>>,
    frame_limit: u32,
    frame_number: Arc<Mutex<u32>>,
    texture_capturer: wgpu::TextureCapturer,
    frames_in_queue: Arc<AtomicUsize>,
    frames_processed: Arc<AtomicUsize>,
    capture_in_progress: Arc<AtomicBool>,
}

impl FrameRecorder {
    pub fn new(output_dir: &str, frame_limit: u32, format: OutputFormat) -> Self {
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
                                &frames_in_queue_clone
                            );
                        }
                    },
                    Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                        // Process any remaining frames on timeout
                        if !frame_buffer.is_empty() {
                            process_frame_batch(
                                &mut frame_buffer,
                                &thread_output_dir,
                                format,
                                &frames_processed_clone,
                                &frames_in_queue_clone
                            );
                        }
                    },
                    Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                        // Process remaining frames and exit
                        if !frame_buffer.is_empty() {
                            process_frame_batch(
                                &mut frame_buffer,
                                &thread_output_dir,
                                format,
                                &frames_processed_clone,
                                &frames_in_queue_clone
                            );
                        }
                        break;
                    }
                }
            }
        });

        Self {
            frame_sender: sender,
            is_recording: Arc::new(Mutex::new(false)),
            last_capture: Arc::new(Mutex::new(0)),
            frame_limit,
            frame_number: Arc::new(Mutex::new(0)),
            texture_capturer: wgpu::TextureCapturer::default(),
            frames_in_queue,
            frames_processed,
            capture_in_progress: Arc::new(AtomicBool::new(false)),

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
        texture: &wgpu::Texture,
    ) {
        if !self.is_recording() {
            return;
        }

        // Check if previous capture is still in progress
        if self.capture_in_progress.load(Ordering::SeqCst) {
            return;
        }

        // Check if enough time has passed since last capture
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
        if *frame_number >= self.frame_limit {
            self.toggle_recording();
            return;
        }

        let frame_num = *frame_number;
        *frame_number += 1;

        // Set capture in progress flag
        self.capture_in_progress.store(true, Ordering::SeqCst);

        let sender = self.frame_sender.clone();
        let frames_in_queue = self.frames_in_queue.clone();
        let frames_in_queue_outer = self.frames_in_queue.clone();
        let capture_in_progress_outer = self.capture_in_progress.clone();

        
        // Create snapshot in its own scope to ensure proper cleanup
        let snapshot = {
            let snapshot = self.texture_capturer.capture(device, encoder, texture);
            snapshot
        };
        
        let width = texture.width();
        let height = texture.height();

        if let Err(e) = snapshot.read(move |result| {
            match result {
                Ok(buffer) => {
                    // Scope the buffer data conversion
                    let data = {
                        let raw_data = buffer.to_owned().into_raw();
                        raw_data
                    }; // buffer is dropped here
                    frames_in_queue.fetch_add(1, Ordering::SeqCst);
                    if let Err(e) = sender.send((frame_num, data, width, height)) {
                        frames_in_queue.fetch_sub(1, Ordering::SeqCst);
                        eprintln!("Failed to send frame: {}", e);
                    }
                },
                Err(e) => {
                    frames_in_queue.fetch_sub(1, Ordering::SeqCst);
                    eprintln!("Failed to read texture: {:?}", e);
                }
            }

            // Clear capture in progress flag
            capture_in_progress_outer.store(false, Ordering::SeqCst);

        }) {
            frames_in_queue_outer.fetch_sub(1, Ordering::SeqCst);
            self.capture_in_progress.store(false, Ordering::SeqCst);
            eprintln!("Failed to read snapshot: {:?}", e);
        }
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
    while let Some((frame_number, frame_data, width, height)) = frame_buffer.pop_front() {
        if let Some(image_buffer) = RgbaImage::from_raw(width, height, frame_data) {
            let filename = match format {
                //OutputFormat::PNG => format!("{}/frame{:05}.png", output_dir, frame_number),
                OutputFormat::JPEG(_) => format!("{}/frame{:05}.jpg", output_dir, frame_number),
            };

            let result = match format {
                //OutputFormat::PNG => image_buffer.save(&filename),
                OutputFormat::JPEG(quality) => {
                    // Process JPEG in a scope to ensure memory is freed immediately
                    let result = {
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
                    };
                    result
                },
            };

            if let Err(e) = result {
                eprintln!("Failed to save frame {}: {}", frame_number, e);
            } else {
                frames_processed.fetch_add(1, Ordering::SeqCst);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use std::fs;

    fn create_test_frame(width: u32, height: u32) -> Vec<u8> {
        vec![255; (width * height * 4) as usize]
    }

    fn create_test_dir() -> String {
        let mut attempts = 0;
        loop {
            let test_dir = format!("test_frames_{}", std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos());
                
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
    fn test_queue_status_initial() {
        let test_dir = create_test_dir();
        let recorder = FrameRecorder::new(
            &test_dir,
            100,
            OutputFormat::JPEG(85),
        );

        let (processed, total) = recorder.get_queue_status();
        assert_eq!(processed, 0, "Initial processed count should be 0");
        assert_eq!(total, 0, "Initial total count should be 0");
        assert!(!recorder.has_pending_frames(), "Should not have pending frames initially");
        
        cleanup_test_dir(&test_dir);
    }

    #[test]
    fn test_queue_status_after_frame() {
        let test_dir = create_test_dir();
        let recorder = FrameRecorder::new(
            &test_dir,
            100,
            OutputFormat::JPEG(85),
        );

        let frame_data = create_test_frame(100, 100);
        recorder.send_frame_data((0, frame_data, 100, 100)).unwrap();

        // Give a small amount of time for the frame to be processed
        std::thread::sleep(Duration::from_millis(1000));

        let (processed, total) = recorder.get_queue_status();
        assert_eq!(total, 1, "Total frames should be 1");
        assert!(processed <= total, "Processed frames should not exceed total frames");
        
        cleanup_test_dir(&test_dir);
    }

    #[test]
    fn test_queue_status_batch_processing() {
        let test_dir = create_test_dir();
        let recorder = FrameRecorder::new(
            &test_dir,
            100,
            OutputFormat::JPEG(85),
        );

        let frame_data = create_test_frame(100, 100);
        for i in 0..BATCH_SIZE + 1 {
            recorder.send_frame_data((i as u32, frame_data.clone(), 100, 100)).unwrap();
        }

        // Give time for batch processing
        std::thread::sleep(Duration::from_millis(500));

        let (processed, total) = recorder.get_queue_status();
        assert_eq!(total, BATCH_SIZE + 1, "Total should match number of frames sent");
        assert!(processed >= BATCH_SIZE, "At least one batch should be processed");
        
        cleanup_test_dir(&test_dir);
    }

    #[test]
    fn test_pending_frames_accuracy() {
        let test_dir = create_test_dir();
        let recorder = FrameRecorder::new(
            &test_dir,
            100,
            OutputFormat::JPEG(85)
        );

        let frame_data = create_test_frame(100, 100);
        for i in 0..5 {
            recorder.send_frame_data((i, frame_data.clone(), 100, 100)).unwrap();
            
            // Check immediately after sending
            let (processed, total) = recorder.get_queue_status();
            assert!(total > processed, "Should have pending frames immediately after sending");
            assert!(recorder.has_pending_frames(), "has_pending_frames should be true");
        }

        // Wait for processing to complete
        std::thread::sleep(Duration::from_secs(1));
        
        let (processed, total) = recorder.get_queue_status();
        assert_eq!(processed, total, "All frames should be processed after waiting");
        assert!(!recorder.has_pending_frames(), "Should not have pending frames after processing");
        
        cleanup_test_dir(&test_dir);
    }

    #[test]
    fn test_frame_counter_increment() {
        let test_dir = create_test_dir();
        let recorder = FrameRecorder::new(
            &test_dir,
            100,
            OutputFormat::JPEG(85),
        );

        // Track counters before and after sending frames
        let (_initial_processed, initial_total) = recorder.get_queue_status();
        
        let frame_data = create_test_frame(100, 100);
        for i in 0..3 {
            recorder.send_frame_data((i, frame_data.clone(), 100, 100)).unwrap();
            
            // Check that total increases immediately
            let (_, current_total) = recorder.get_queue_status();
            assert_eq!(current_total, initial_total + (i as usize) + 1, 
                      "Total count should increment by one for each frame");
        }
        
        cleanup_test_dir(&test_dir);
    }

    #[test]
    fn test_processed_never_exceeds_total() {
        let test_dir = create_test_dir();
        let recorder = FrameRecorder::new(
            &test_dir,
            100,
            OutputFormat::JPEG(85),
        );

        let frame_data = create_test_frame(100, 100);
        for i in 0..10 {
            recorder.send_frame_data((i, frame_data.clone(), 100, 100)).unwrap();
            
            let (processed, total) = recorder.get_queue_status();
            assert!(processed <= total, 
                   "Processed count should never exceed total count");
        }

        // Check multiple times during processing
        for _ in 0..5 {
            std::thread::sleep(Duration::from_millis(100));
            let (processed, total) = recorder.get_queue_status();
            assert!(processed <= total, 
                   "Processed count should never exceed total during processing");
        }
        
        cleanup_test_dir(&test_dir);
    }

    #[test]
    fn test_recording_toggle() {
        let test_dir = create_test_dir();
        let recorder = FrameRecorder::new(
            &test_dir,
            100,
            OutputFormat::JPEG(85),
        );

        assert!(!recorder.is_recording(), "Should not be recording initially");
        
        recorder.toggle_recording();
        assert!(recorder.is_recording(), "Should be recording after toggle");
        
        recorder.toggle_recording();
        assert!(!recorder.is_recording(), "Should not be recording after second toggle");
        
        cleanup_test_dir(&test_dir);
    }

    #[test]
    fn test_cleanup_after_recording() {
        let test_dir = create_test_dir();
        let recorder = FrameRecorder::new(
            &test_dir,
            100,
            OutputFormat::JPEG(85),
        );

        recorder.toggle_recording();
        
        // Send some frames
        let frame_data = create_test_frame(100, 100);
        for i in 0..5 {
            recorder.send_frame_data((i, frame_data.clone(), 100, 100)).unwrap();
        }
        
        recorder.toggle_recording();
        
        // Wait for processing to complete
        std::thread::sleep(Duration::from_secs(1));
        
        let (processed, total) = recorder.get_queue_status();
        assert_eq!(processed, total, "All frames should be processed after recording stops");
        
        cleanup_test_dir(&test_dir);
    }
}