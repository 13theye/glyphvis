// src/services/frame_recorder.rs
// FrameRecorder is a service for capturing frames from a wgpu::Texture and encoding them to video.
// It gets its own thread to avoid blocking the main thread.
// Encoding is done by piping frames directly to ffmpeg for h264 encoding.

use nannou::{image::RgbaImage, wgpu};
use std::{
    io::Write,
    path::Path,
    process::{Child, Command, Stdio},
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        mpsc::{channel, Receiver, Sender},
        Arc, Mutex,
    },
    thread::{self, JoinHandle},
};

const BATCH_SIZE: usize = 10;
const RESOLVED_TEXTURE_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;
const VERBOSE: bool = false; // true to show debug msgs

// Type alias for the frame data tuple
type FrameData = (Vec<u8>, u32, u32);

struct WorkerThread {
    thread_handle: JoinHandle<()>,
    frame_sender: Sender<FrameData>,
    shutdown_requested: Arc<AtomicBool>,
    thread_completed: Arc<AtomicBool>,
    frames_in_queue: Arc<AtomicUsize>,

    // FFmpeg process info
    ffmpeg_process: Arc<Mutex<Option<Child>>>,
}

pub struct FrameRecorder {
    worker_thread: Arc<Mutex<Option<WorkerThread>>>,

    is_recording: Arc<Mutex<bool>>,
    frame_limit: u32,
    frame_number: Arc<Mutex<u32>>,
    capture_in_progress: Arc<AtomicBool>,
    frame_time: u64,
    output_dir: String,
    fps: u64,

    // capture pipeline
    texture_reshaper: wgpu::TextureReshaper,
    resolved_texture: wgpu::Texture, // for MSAA resolution
    staging_buffers: Vec<Arc<wgpu::Buffer>>,
    current_buffer_index: Arc<AtomicUsize>,

    // Synchronization
    next_scheduled_capture: Arc<Mutex<u64>>,
}

impl FrameRecorder {
    pub fn new(
        device: &wgpu::Device,
        render_texture: &wgpu::Texture,
        output_dir: &str,
        frame_limit: u32,
        fps: u64,
    ) -> Self {
        // Ensure output directory exists
        std::fs::create_dir_all(output_dir).expect("Failed to create output directory");

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

        // Create n staging buffers for GPU->CPU transfer
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
            worker_thread: Arc::new(Mutex::new(None)),
            is_recording: Arc::new(Mutex::new(false)),
            frame_limit,
            frame_number: Arc::new(Mutex::new(0)),
            capture_in_progress: Arc::new(AtomicBool::new(false)),
            frame_time: 1_000_000_000 / fps,
            output_dir: output_dir.to_string(),
            fps,

            texture_reshaper,
            resolved_texture,
            staging_buffers,
            current_buffer_index: Arc::new(AtomicUsize::new(0)),

            next_scheduled_capture: Arc::new(Mutex::new(0)),
        }
    }

    fn create_worker_thread(&self) -> WorkerThread {
        let frames_in_queue = Arc::new(AtomicUsize::new(0));
        let ffmpeg_process = Arc::new(Mutex::new(None));
        let shutdown_requested = Arc::new(AtomicBool::new(false));
        let thread_completed = Arc::new(AtomicBool::new(false));

        let (sender, receiver) = channel();

        let thread_output_dir = self.output_dir.clone();
        let thread_fps = self.fps;

        let frames_in_queue_clone = frames_in_queue.clone();
        let ffmpeg_process_clone = ffmpeg_process.clone();
        let shutdown_requested_clone = shutdown_requested.clone();
        let thread_completed_clone = thread_completed.clone();

        // Spawn worker thread
        let thread_handle = thread::spawn(move || {
            Self::worker_thread_function(
                receiver,
                thread_output_dir,
                thread_fps,
                frames_in_queue_clone,
                ffmpeg_process_clone,
                shutdown_requested_clone,
                thread_completed_clone,
            );
        });

        WorkerThread {
            thread_handle,
            frame_sender: sender,
            shutdown_requested,
            frames_in_queue,
            thread_completed,
            ffmpeg_process,
        }
    }

    fn worker_thread_function(
        receiver: Receiver<FrameData>,
        output_dir: String,
        fps: u64,
        frames_in_queue: Arc<AtomicUsize>,
        ffmpeg_process: Arc<Mutex<Option<Child>>>,
        shutdown_requested: Arc<AtomicBool>,
        thread_completed: Arc<AtomicBool>,
    ) {
        let ffmpeg_stdin = Arc::new(Mutex::new(None));

        // Add batch handling
        let mut frame_batch = Vec::new();
        let mut batch_count = 0;

        loop {
            // Use recv_timeout to allow checking for shutdown
            match receiver.recv_timeout(std::time::Duration::from_millis(50)) {
                Ok((frame_data, width, height)) => {
                    // Initialize FFmpeg if needed
                    if batch_count == 0 {
                        let mut stdin_guard = ffmpeg_stdin.lock().unwrap();
                        if stdin_guard.is_none() {
                            // Initialize FFmpeg on first frame
                            let (process, stdin) =
                                start_ffmpeg_process(&output_dir, width, height, fps);
                            *ffmpeg_process.lock().unwrap() = Some(process);
                            *stdin_guard = Some(stdin);
                        }
                    }

                    // Convert RGBA to RGB and add to batch
                    if let Some(image_buffer) = RgbaImage::from_raw(width, height, frame_data) {
                        let rgb_buffer =
                            nannou::image::DynamicImage::ImageRgba8(image_buffer).to_rgb8();

                        // Add to batch
                        frame_batch.extend_from_slice(rgb_buffer.as_raw());
                        batch_count += 1;

                        // Process batch if full
                        if batch_count >= BATCH_SIZE {
                            // Write batch to FFmpeg
                            let mut stdin_guard = ffmpeg_stdin.lock().unwrap();
                            if let Some(stdin) = stdin_guard.as_mut() {
                                if let Err(e) = stdin.write_all(&frame_batch) {
                                    eprintln!("Failed to write frames to FFmpeg: {}", e);
                                } else {
                                    frames_in_queue.fetch_sub(batch_count, Ordering::SeqCst);
                                }
                            }
                            frame_batch.clear();
                            batch_count = 0;
                        }
                    }
                }
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                    // Check if shutdown requested and handle any partial batch
                    if shutdown_requested.load(Ordering::SeqCst) {
                        // Write any remaining frames in the batch
                        if batch_count > 0 {
                            let mut stdin_guard = ffmpeg_stdin.lock().unwrap();
                            if let Some(stdin) = stdin_guard.as_mut() {
                                if let Err(e) = stdin.write_all(&frame_batch) {
                                    eprintln!("Failed to write remaining frames to FFmpeg: {}", e);
                                } else {
                                    frames_in_queue.fetch_sub(batch_count, Ordering::SeqCst);
                                }
                            }
                        }

                        // Close the FFmpeg stdin stream to signal end of input
                        drop(ffmpeg_stdin.lock().unwrap().take());
                        break; // Exit the loop
                    }
                }
                Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                    // Channel closed, handle any remaining frames
                    if batch_count > 0 {
                        let mut stdin_guard = ffmpeg_stdin.lock().unwrap();
                        if let Some(stdin) = stdin_guard.as_mut() {
                            if let Err(e) = stdin.write_all(&frame_batch) {
                                eprintln!("Failed to write remaining frames to FFmpeg: {}", e);
                            } else {
                                frames_in_queue.fetch_sub(batch_count, Ordering::SeqCst);
                            }
                        }
                    }
                    break;
                }
            }
        }

        // Wait for FFmpeg to finish after exiting the loop
        if let Some(mut process) = ffmpeg_process.lock().unwrap().take() {
            match process.wait() {
                Ok(status) => {
                    if !status.success() {
                        eprintln!("FFmpeg exited with non-zero status: {}", status);
                    } else {
                        println!("FFmpeg process completed successfully");
                    }
                }
                Err(e) => eprintln!("Failed to wait for FFmpeg process: {}", e),
            }
        }
        thread_completed.store(true, Ordering::SeqCst);
        println!("FFmpeg worker thread finished");
    }

    pub fn toggle_recording(&self) {
        let mut is_recording = self.is_recording.lock().unwrap();
        *is_recording = !*is_recording;

        if *is_recording {
            // Starting a new recording - clean up any completed worker first
            self.cleanup_completed_worker();

            let mut worker_thread_guard = self.worker_thread.lock().unwrap();

            // If there's an existing worker thread, just signal it to shut down
            // We'll clean it up later when it's done
            if let Some(worker) = worker_thread_guard.as_ref() {
                Self::request_worker_shutdown(worker);
            }

            // Create new worker thread
            *worker_thread_guard = Some(self.create_worker_thread());

            // Reset recording state
            *self.frame_number.lock().unwrap() = 0;
            *self.next_scheduled_capture.lock().unwrap() = 0;
            println!("Recording started");
        } else {
            // Stopping recording - just signal the worker to shut down
            println!("Recording stopped");
            self.signal_shutdown();
        }
    }

    fn request_worker_shutdown(worker: &WorkerThread) {
        worker.shutdown_requested.store(true, Ordering::SeqCst);
    }

    pub fn request_shutdown(&self) {
        self.signal_shutdown();
    }

    pub fn signal_shutdown(&self) -> bool {
        let worker_thread_guard = self.worker_thread.lock().unwrap();

        // If there's a worker thread, signal it to shut down
        if let Some(worker) = worker_thread_guard.as_ref() {
            worker.shutdown_requested.store(true, Ordering::SeqCst);
            return true; // Worker exists and was signaled
        }

        false // No worker thread to signal
    }

    pub fn is_worker_completed(&self) -> bool {
        let worker_thread_guard = self.worker_thread.lock().unwrap();

        match worker_thread_guard.as_ref() {
            Some(worker) => {
                // Check if thread has marked itself as completed
                worker.thread_completed.load(Ordering::SeqCst)
            }
            None => true, // No worker means "completed"
        }
    }

    pub fn cleanup_completed_worker(&self) {
        let mut worker_thread_guard = self.worker_thread.lock().unwrap();

        if let Some(worker) = worker_thread_guard.as_ref() {
            if worker.thread_completed.load(Ordering::SeqCst) {
                // Thread is done, we can safely take and drop it
                let completed_worker = worker_thread_guard.take();

                // Drop the worker thread - this will join the thread but it's already
                // completed so it won't block
                if let Some(worker) = completed_worker {
                    if let Err(e) = worker.thread_handle.join() {
                        eprintln!("Error joining completed worker thread: {:?}", e);
                    } else {
                        println!("Worker thread cleanup complete.\n");
                    }
                }
            }
        }
    }

    pub fn is_recording(&self) -> bool {
        *self.is_recording.lock().unwrap()
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

        // Get the worker thread
        let worker_thread_guard = self.worker_thread.lock().unwrap();
        let worker_thread = match worker_thread_guard.as_ref() {
            Some(worker) => worker,
            None => return, // No worker thread available
        };

        // Check if enough time has passed since last capture
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64;

        // Get our next scheduled capture time
        let mut next_scheduled = self.next_scheduled_capture.lock().unwrap();
        // If this is the first frame after starting recording, initialize the schedule
        if *next_scheduled == 0 {
            *next_scheduled = now;
        }
        // Check if it's time for the next frame yet
        if now < *next_scheduled {
            // Too early, wait until exactly the scheduled time
            return;
        }

        // If we're more than a frame behind, skip to the next appropriate frame time
        // This prevents frame accumulation if we fall behind
        if now > *next_scheduled + self.frame_time {
            // Calculate how many frames we're behind
            let frames_behind = (now - *next_scheduled) / self.frame_time;

            // Calculate time difference in milliseconds
            let time_diff_ms = (now - *next_scheduled) / 1_000_000;

            // Calculate video timestamp (time since recording started)
            // We use frame_number to calculate the position in the video timeline
            let frame_num = *self.frame_number.lock().unwrap();
            let video_time_ns = frame_num as u64 * self.frame_time;
            let video_time_s = video_time_ns / 1_000_000_000;
            let video_time_ms = (video_time_ns % 1_000_000_000) / 1_000_000;

            let video_timestamp = format!(
                "{:02}:{:02}:{:02}.{:03}",
                (video_time_s / 3600),    // hours
                (video_time_s / 60) % 60, // minutes
                video_time_s % 60,        // seconds
                video_time_ms             // milliseconds
            );

            // Skip to the next valid frame time, dropping any missed frames
            *next_scheduled += (frames_behind + 1) * self.frame_time;

            println!(
                "WARNING: Skipped {} frames, {}ms behind schedule, video time: {}",
                frames_behind, time_diff_ms, video_timestamp
            );

            return; // Skip this frame and catch up on the next one
        }

        // Schedule the next frame at exactly frame_time nanoseconds from the current scheduled time
        *next_scheduled += self.frame_time;

        // Check if we're still processing the previous frame
        if self.capture_in_progress.load(Ordering::SeqCst) {
            println!(
                "WARNING: Previous capture still in progress, skipping frame at scheduled time {}",
                *next_scheduled - self.frame_time
            );
            return;
        }

        // Begin capture process - note the time, set capture in progress flag
        self.capture_in_progress.store(true, Ordering::SeqCst);
        //*last_capture = now;
        let frame_start = std::time::Instant::now();

        // Check if we've reached the frame limit
        let mut frame_number = self.frame_number.lock().unwrap();
        if *frame_number >= self.frame_limit {
            self.toggle_recording();
            return;
        }

        // Increment frame number
        *frame_number += 1;

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
        let sender = worker_thread.frame_sender.clone();
        let frames_in_queue = worker_thread.frames_in_queue.clone();
        let capture_in_progress_outer = self.capture_in_progress.clone();

        let width = render_texture.width();
        let height = render_texture.height();

        // Submit the encoder (prevents buffer mapping deadlock)
        device.poll(wgpu::Maintain::Poll);

        // Map buffer and process data
        let _buffer_map_start = std::time::Instant::now();
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
                        frames_in_queue.fetch_add(1, Ordering::SeqCst);
                        if let Err(e) = sender.send((unpadded_data, width, height)) {
                            frames_in_queue.fetch_sub(1, Ordering::SeqCst);
                            eprintln!("Failed to send frame: {}", e);
                        }
                    }
                    Err(e) => {
                        eprintln!("Buffer mapping error: {}", e);
                        staging_buffer_clone.unmap();
                    }
                }
                capture_in_progress_outer.store(false, Ordering::SeqCst);
            });

        if VERBOSE {
            println!("Total frame capture took: {:?}", frame_start.elapsed());
        }

        // Try a more efficient polling approach
        let start_time = std::time::Instant::now();

        // First poll without waiting to quickly check if operations are done
        if device.poll(wgpu::Maintain::Poll) {
            // No pending operations, we can return immediately
            self.capture_in_progress.store(false, Ordering::SeqCst);
            return;
        }

        // If we need to wait, use a single Wait call with a shorter timeout
        // This avoids the overhead of the polling loop
        device.poll(wgpu::Maintain::Wait);

        // If we've waited too long, log it but continue
        if start_time.elapsed() > std::time::Duration::from_millis(10) {
            println!(
                "WARNING: Device poll took longer than expected: {:?}",
                start_time.elapsed()
            );
        }

        self.capture_in_progress.store(false, Ordering::SeqCst);
    }

    pub fn get_queue_status(&self) -> (usize, usize) {
        // Get the worker thread
        let worker_thread_guard = self.worker_thread.lock().unwrap();

        match worker_thread_guard.as_ref() {
            Some(worker) => {
                let total = worker.frames_in_queue.load(Ordering::SeqCst);

                // Check if FFmpeg process is still running
                let is_running = worker.ffmpeg_process.lock().unwrap().is_some();

                if is_running {
                    // FFmpeg still running - show 0 processed
                    (0, total)
                } else {
                    // FFmpeg finished - all frames processed
                    (total, total)
                }
            }
            None => (0, 0),
        }
    }

    pub fn has_pending_frames(&self) -> bool {
        let worker_thread_guard = self.worker_thread.lock().unwrap();

        match worker_thread_guard.as_ref() {
            Some(worker) => {
                // Thread exists - check if still processing
                worker.ffmpeg_process.lock().unwrap().is_some()
                    || !worker.thread_completed.load(Ordering::SeqCst)
            }
            None => false, // No worker thread, no pending frames
        }
    }
}

fn start_ffmpeg_process(
    output_dir: &str,
    width: u32,
    height: u32,
    fps: u64,
) -> (Child, std::process::ChildStdin) {
    // Find the next available output file name
    let output_file = find_next_output_filename(output_dir);
    let output_path = format!("{}/{}", output_dir, output_file);

    println!("Starting FFmpeg process to encode to {}", output_path);

    // Set up FFmpeg command with appropriate parameters
    let mut command = Command::new("ffmpeg");
    command
        .args([
            "-f",
            "rawvideo", // Input format is raw video data
            "-pixel_format",
            "rgb24", // Input pixel format (matching our RGB8 conversion)
            "-video_size",
            &format!("{}x{}", width, height), // Video dimensions
            "-framerate",
            &fps.to_string(), // Frame rate
            "-i",
            "-", // Read from stdin
            "-vsync",
            "cfr", // constant frame rate
            "-r",
            &fps.to_string(), // force output frame rate
            "-c:v",
            "libx264", // Use H.264 codec
            "-preset",
            "slow", // Encoding speed/quality tradeoff
            "-crf",
            "10", // Quality level (lower is better quality, 23 is default)
            "-pix_fmt",
            "yuv420p",    // Output pixel format
            "-y",         // Overwrite output file if it exists
            &output_path, // Output file path
        ])
        .stdin(Stdio::piped()) // Capture stdin
        .stdout(Stdio::null()) // Discard stdout
        .stderr(if VERBOSE {
            Stdio::inherit()
        } else {
            Stdio::null()
        }); // Show or hide stderr

    // Start the FFmpeg process
    let mut process = command.spawn().expect("Failed to start FFmpeg process");

    // Get the stdin handle that we'll write frames to
    let stdin = process
        .stdin
        .take()
        .expect("Failed to open stdin for FFmpeg process");

    (process, stdin)
}

fn find_next_output_filename(output_dir: &str) -> String {
    // Try output.mp4 first
    let base_name = "output";
    let extension = "mp4";
    let mut index = 0;

    loop {
        let file_name = if index == 0 {
            format!("{}.{}", base_name, extension)
        } else {
            format!("{}{}.{}", base_name, index, extension)
        };

        let path = Path::new(output_dir).join(&file_name);

        if !path.exists() {
            return file_name;
        }

        index += 1;
    }
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
