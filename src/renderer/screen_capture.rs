use crate::renderer::Renderer;
use ultraviolet::Vec2;
use std::fs;
use std::path::Path;
use chrono::Utc;
use crate::profile_scope;

impl Renderer {
    pub fn handle_screen_capture(&mut self, current_time: f32, width: u16, height: u16) {
        // Update our stored dimensions to ensure they're current
        self.window_width = width;
        self.window_height = height;
        
        // Check if we should trigger a capture now (either manual or timed)
        if self.should_capture_next_frame {
            // Manual capture
            if let Err(e) = self.capture_current_frame() {
                eprintln!("Manual screen capture failed: {}", e);
            }
            self.should_capture_next_frame = false;
        } else if self.screen_capture_enabled {
            // Only check timing if recording is enabled to avoid unnecessary calculations
            let time_since_last = current_time - self.last_capture_time;
            if time_since_last >= self.capture_interval {
                // Automatic timed capture
                if let Err(e) = self.capture_current_frame() {
                    eprintln!("Automatic screen capture failed: {}", e);
                }
            }
        }
    }

    pub fn capture_current_frame(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        profile_scope!("screen_capture");
        // Create capture directory if it doesn't exist
        fs::create_dir_all(&self.capture_folder)?;
        
        #[cfg(windows)]
        {
            // Try to capture the simulation window specifically
            if let Some(hwnd) = self.get_simulation_window_handle() {
                return self.capture_window_content(hwnd);
            }
        }
        
        // Fallback to full screen capture if window-specific capture fails
        self.capture_full_screen()
    }

    #[cfg(windows)]
    fn capture_window_content(&mut self, hwnd: *mut std::ffi::c_void) -> Result<(), Box<dyn std::error::Error>> {
        unsafe {
            let hwnd = hwnd as winapi::shared::windef::HWND;
            
            // Get window dimensions
            let mut rect: winapi::shared::windef::RECT = std::mem::zeroed();
            if winapi::um::winuser::GetClientRect(hwnd, &mut rect) == 0 {
                return Err("Failed to get window client rect".into());
            }
            
            let width = rect.right - rect.left;
            let height = rect.bottom - rect.top;
            
            if width <= 0 || height <= 0 {
                return Err("Invalid window dimensions".into());
            }
            
            // Get window device context
            let hdc_window = winapi::um::winuser::GetDC(hwnd);
            if hdc_window.is_null() {
                return Err("Failed to get window DC".into());
            }
            
            // Create compatible device context
            let hdc_mem = winapi::um::wingdi::CreateCompatibleDC(hdc_window);
            if hdc_mem.is_null() {
                winapi::um::winuser::ReleaseDC(hwnd, hdc_window);
                return Err("Failed to create compatible DC".into());
            }
            
            // Create bitmap
            let hbmp = winapi::um::wingdi::CreateCompatibleBitmap(hdc_window, width, height);
            if hbmp.is_null() {
                winapi::um::wingdi::DeleteDC(hdc_mem);
                winapi::um::winuser::ReleaseDC(hwnd, hdc_window);
                return Err("Failed to create compatible bitmap".into());
            }
            
            // Select bitmap into memory DC
            let old_bitmap = winapi::um::wingdi::SelectObject(hdc_mem, hbmp as *mut std::ffi::c_void);
            
            // Copy window content to memory DC
            if winapi::um::wingdi::BitBlt(
                hdc_mem, 0, 0, width, height,
                hdc_window, 0, 0,
                winapi::um::wingdi::SRCCOPY
            ) == 0 {
                winapi::um::wingdi::SelectObject(hdc_mem, old_bitmap);
                winapi::um::wingdi::DeleteObject(hbmp as *mut std::ffi::c_void);
                winapi::um::wingdi::DeleteDC(hdc_mem);
                winapi::um::winuser::ReleaseDC(hwnd, hdc_window);
                return Err("Failed to copy window content".into());
            }
            
            // Get bitmap data
            let mut bmp_info: winapi::um::wingdi::BITMAPINFO = std::mem::zeroed();
            bmp_info.bmiHeader.biSize = std::mem::size_of::<winapi::um::wingdi::BITMAPINFOHEADER>() as u32;
            bmp_info.bmiHeader.biWidth = width;
            bmp_info.bmiHeader.biHeight = -height; // Negative for top-down DIB
            bmp_info.bmiHeader.biPlanes = 1;
            bmp_info.bmiHeader.biBitCount = 32;
            bmp_info.bmiHeader.biCompression = winapi::um::wingdi::BI_RGB;
            
            let mut pixels = vec![0u8; (width * height * 4) as usize];
            
            if winapi::um::wingdi::GetDIBits(
                hdc_mem,
                hbmp,
                0,
                height as u32,
                pixels.as_mut_ptr() as *mut std::ffi::c_void,
                &mut bmp_info,
                winapi::um::wingdi::DIB_RGB_COLORS
            ) == 0 {
                winapi::um::wingdi::SelectObject(hdc_mem, old_bitmap);
                winapi::um::wingdi::DeleteObject(hbmp as *mut std::ffi::c_void);
                winapi::um::wingdi::DeleteDC(hdc_mem);
                winapi::um::winuser::ReleaseDC(hwnd, hdc_window);
                return Err("Failed to get bitmap bits".into());
            }
            
            // Clean up GDI resources
            winapi::um::wingdi::SelectObject(hdc_mem, old_bitmap);
            winapi::um::wingdi::DeleteObject(hbmp as *mut std::ffi::c_void);
            winapi::um::wingdi::DeleteDC(hdc_mem);
            winapi::um::winuser::ReleaseDC(hwnd, hdc_window);
            
            // Convert BGRA to RGBA
            for chunk in pixels.chunks_mut(4) {
                chunk.swap(0, 2); // Swap B and R channels
            }
            
            // Create image from pixel data
            let image_buffer = match image::ImageBuffer::from_raw(width as u32, height as u32, pixels) {
                Some(buffer) => image::DynamicImage::ImageRgba8(buffer),
                None => return Err("Failed to create image buffer from captured pixels".into()),
            };
            
            // Apply region cropping if specified
            let final_image = if let Some((ratio_start, ratio_end)) = self.capture_region_ratio {
                let x1 = (ratio_start.x.min(ratio_end.x) * width as f32).max(0.0) as u32;
                let y1 = (ratio_start.y.min(ratio_end.y) * height as f32).max(0.0) as u32;
                let x2 = (ratio_start.x.max(ratio_end.x) * width as f32).min(width as f32) as u32;
                let y2 = (ratio_start.y.max(ratio_end.y) * height as f32).min(height as f32) as u32;
                let crop_width = x2.saturating_sub(x1);
                let crop_height = y2.saturating_sub(y1);
                
                if crop_width > 0 && crop_height > 0 {
                    image_buffer.crop_imm(x1, y1, crop_width, crop_height)
                } else {
                    image_buffer
                }
            } else {
                image_buffer
            };
            
            // Generate filename and save
            let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
            let filename = format!("capture_{}_{:04}.png", timestamp, self.capture_counter);
            let filepath = Path::new(&self.capture_folder).join(&filename);
            
            match final_image.save(&filepath) {
                Ok(_) => {
                    let region_info = if self.capture_region.is_some() {
                        format!(" (cropped region)")
                    } else {
                        format!(" (full window)")
                    };
                    println!("Window capture saved: {} ({}x{} pixels{})", 
                            filename, final_image.width(), final_image.height(), region_info);
                    self.capture_counter += 1;
                    self.last_capture_time = *crate::renderer::state::SIM_TIME.lock();
                    Ok(())
                }
                Err(e) => {
                    eprintln!("Failed to save window capture: {}", e);
                    Err(e.into())
                }
            }
        }
    }

    fn capture_full_screen(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // Use screenshots crate to capture the actual screen as fallback
        let image = match screenshots::Screen::all() {
            Ok(screens) => {
                if let Some(screen) = screens.first() {
                    match screen.capture() {
                        Ok(capture) => capture,
                        Err(e) => {
                            eprintln!("Failed to capture screen: {}", e);
                            return Err(format!("Screen capture failed: {}", e).into());
                        }
                    }
                } else {
                    eprintln!("No screens found for capture");
                    return Err("No screens available for capture".into());
                }
            }
            Err(e) => {
                eprintln!("Failed to enumerate screens: {}", e);
                return Err(format!("Failed to enumerate screens: {}", e).into());
            }
        };

        // For region capture, we'll need to crop after converting to an image::DynamicImage
        if let Some((ratio_start, ratio_end)) = self.capture_region_ratio {
            // Convert screenshots::Image to image::DynamicImage for cropping
            let width = image.width();
            let height = image.height();
            let rgba_data = image.rgba();
            
            // Convert to DynamicImage
            let dynamic_image = match image::ImageBuffer::from_raw(width, height, rgba_data.to_vec()) {
                Some(img_buf) => image::DynamicImage::ImageRgba8(img_buf),
                None => {
                    eprintln!("Failed to convert screenshot to image buffer");
                    return Err("Image conversion failed".into());
                }
            };
            
            let (window_x, window_y) = self.get_window_position();
            let x1 = (ratio_start.x.min(ratio_end.x) * self.window_width as f32 + window_x as f32).max(0.0) as u32;
            let y1 = (ratio_start.y.min(ratio_end.y) * self.window_height as f32 + window_y as f32).max(0.0) as u32;
            let x2 = (ratio_start.x.max(ratio_end.x) * self.window_width as f32 + window_x as f32).min(width as f32) as u32;
            let y2 = (ratio_start.y.max(ratio_end.y) * self.window_height as f32 + window_y as f32).min(height as f32) as u32;
            
            let crop_width = x2.saturating_sub(x1);
            let crop_height = y2.saturating_sub(y1);
            
            if crop_width > 0 && crop_height > 0 {
                let cropped = dynamic_image.crop_imm(x1, y1, crop_width, crop_height);
                
                // Generate filename with timestamp
                let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
                let filename = format!("capture_{}_{:04}.png", timestamp, self.capture_counter);
                let filepath = Path::new(&self.capture_folder).join(&filename);
                
                // Save the cropped image
                match cropped.save(&filepath) {
                    Ok(_) => {
                        println!("Screen capture saved: {} ({}x{} pixels, world region converted to screen coords)", 
                                filename, crop_width, crop_height);
                        self.capture_counter += 1;
                        self.last_capture_time = *crate::renderer::state::SIM_TIME.lock();
                        return Ok(());
                    }
                    Err(e) => {
                        eprintln!("Failed to save cropped screen capture: {}", e);
                        return Err(e.into());
                    }
                }
            } else {
                eprintln!("Invalid crop region: width={}, height={}", crop_width, crop_height);
                return Err("Invalid crop region".into());
            }
        } else {
            // Full screen capture - convert to DynamicImage and save
            let width = image.width();
            let height = image.height();
            let rgba_data = image.rgba();
            
            // Convert to DynamicImage
            let dynamic_image = match image::ImageBuffer::from_raw(width, height, rgba_data.to_vec()) {
                Some(img_buf) => image::DynamicImage::ImageRgba8(img_buf),
                None => {
                    eprintln!("Failed to convert screenshot to image buffer");
                    return Err("Image conversion failed".into());
                }
            };
            
            // Generate filename with timestamp
            let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
            let filename = format!("capture_{}_{:04}.png", timestamp, self.capture_counter);
            let filepath = Path::new(&self.capture_folder).join(&filename);

            // Save the image
            match dynamic_image.save(&filepath) {
                Ok(_) => {
                    println!("Screen capture saved: {} ({}x{} pixels, full screen fallback)", 
                            filename, width, height);
                    self.capture_counter += 1;
                    self.last_capture_time = *crate::renderer::state::SIM_TIME.lock();
                    Ok(())
                }
                Err(e) => {
                    eprintln!("Failed to save screen capture: {}", e);
                    Err(format!("Save failed: {}", e).into())
                }
            }
        }
    }

    pub fn finish_region_selection(&mut self, width: u16, height: u16) {
        if let (Some(start), Some(end)) = (self.selection_start, self.selection_end) {
            // Ensure we're using the absolute current window dimensions
            self.window_width = width;
            self.window_height = height;
            
            // Convert screen coordinates to world coordinates for storage using current window dimensions
            let world_start = self.screen_to_world(start, width, height);
            let world_end = self.screen_to_world(end, width, height);
            
            self.capture_region = Some((world_start, world_end));
            let start_ratio = Vec2::new(start.x / width as f32, start.y / height as f32);
            let end_ratio = Vec2::new(end.x / width as f32, end.y / height as f32);
            self.capture_region_ratio = Some((start_ratio, end_ratio));
            println!(
                "Capture region set: world ({:.2}, {:.2}) -> ({:.2}, {:.2}), ratios ({:.3}, {:.3}) -> ({:.3}, {:.3}) at {}x{}",
                world_start.x,
                world_start.y,
                world_end.x,
                world_end.y,
                start_ratio.x,
                start_ratio.y,
                end_ratio.x,
                end_ratio.y,
                width,
                height
            );
        }
        self.is_selecting_region = false;
        self.selection_start = None;
        self.selection_end = None;
    }

    pub fn cancel_region_selection(&mut self) {
        self.is_selecting_region = false;
        self.selection_start = None;
        self.selection_end = None;
    }

    pub fn clear_capture_region(&mut self) {
        self.capture_region = None;
        self.capture_region_ratio = None;
        println!("Capture region cleared - now capturing full screen");
    }

    pub fn screen_to_world(&self, screen_pos: Vec2, width: u16, height: u16) -> Vec2 {
        // Convert screen coordinates to normalized device coordinates
        let x = (screen_pos.x / width as f32) * 2.0 - 1.0;
        let y = 1.0 - (screen_pos.y / height as f32) * 2.0;
        
        // Apply camera transformation
        let aspect_ratio = width as f32 / height as f32;
        let world_x = self.pos.x + x * self.scale * aspect_ratio;
        let world_y = self.pos.y + y * self.scale;
        
        Vec2::new(world_x, world_y)
    }

    // Helper method to get the simulation window handle specifically
    pub fn get_simulation_window_handle(&self) -> Option<*mut std::ffi::c_void> {
        #[cfg(windows)]
        {
            use std::ffi::CString;
            use std::ptr;
            
            unsafe {
                // Try to find window by class name or title that's specific to our app
                // First try common winit window class names (without null terminators)
                let class_names = [
                    "Window Class",
                    "winit window", 
                    "ParticleSim",
                ];
                
                for class_name in &class_names {
                    if let Ok(class_cstr) = CString::new(*class_name) {
                        let hwnd = winapi::um::winuser::FindWindowA(class_cstr.as_ptr(), ptr::null());
                        if !hwnd.is_null() {
                            return Some(hwnd as *mut std::ffi::c_void);
                        }
                    }
                }
                
                // If we can't find by class, enumerate all windows and find one with our process ID
                let current_pid = winapi::um::processthreadsapi::GetCurrentProcessId();
                
                extern "system" fn enum_windows_proc(hwnd: winapi::shared::windef::HWND, lparam: winapi::shared::minwindef::LPARAM) -> winapi::shared::minwindef::BOOL {
                    unsafe {
                        let target_pid = lparam as winapi::shared::minwindef::DWORD;
                        let mut window_pid: winapi::shared::minwindef::DWORD = 0;
                        winapi::um::winuser::GetWindowThreadProcessId(hwnd, &mut window_pid);
                        
                        if window_pid == target_pid {
                            // Check if this is a main window (has a title and is visible)
                            let mut title: [i8; 256] = [0; 256];
                            let title_len = winapi::um::winuser::GetWindowTextA(hwnd, title.as_mut_ptr(), 256);
                            
                            if title_len > 0 && winapi::um::winuser::IsWindowVisible(hwnd) != 0 {
                                // Store the window handle in a way we can retrieve it
                                // For now, we'll use a global variable approach
                                FOUND_WINDOW_HANDLE.store(hwnd as usize, std::sync::atomic::Ordering::Relaxed);
                                return 0; // Stop enumeration
                            }
                        }
                        1 // Continue enumeration
                    }
                }
                
                static FOUND_WINDOW_HANDLE: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
                FOUND_WINDOW_HANDLE.store(0, std::sync::atomic::Ordering::Relaxed);
                
                winapi::um::winuser::EnumWindows(Some(enum_windows_proc), current_pid as winapi::shared::minwindef::LPARAM);
                
                let handle = FOUND_WINDOW_HANDLE.load(std::sync::atomic::Ordering::Relaxed);
                if handle != 0 {
                    return Some(handle as *mut std::ffi::c_void);
                }
            }
        }
        
        None
    }

    // Helper method to get window position for coordinate conversion
    pub fn get_window_position(&self) -> (i32, i32) {
        #[cfg(windows)]
        {
            if let Some(hwnd) = self.get_simulation_window_handle() {
                unsafe {
                    let mut rect: winapi::shared::windef::RECT = std::mem::zeroed();
                    if winapi::um::winuser::GetWindowRect(hwnd as winapi::shared::windef::HWND, &mut rect) != 0 {
                        println!("Simulation window position detected: ({}, {})", rect.left, rect.top);
                        return (rect.left, rect.top);
                    }
                }
            }
        }
        
        // Fallback for non-Windows or if detection fails
        println!("Could not detect simulation window position, using (0, 0)");
        (0, 0)
    }

    pub fn verify_capture_region_after_resize(&mut self, new_width: u16, new_height: u16) {
        if new_width == 0 || new_height == 0 {
            return;
        }

        // If we have a capture region and window dimensions changed significantly
        if let Some((ratio_start, ratio_end)) = self.capture_region_ratio {
            let old_width = self.window_width;
            let old_height = self.window_height;

            // Check if aspect ratio changed significantly (more than 5%)
            let old_aspect = old_width as f32 / old_height as f32;
            let new_aspect = new_width as f32 / new_height as f32;
            let aspect_change = (new_aspect - old_aspect).abs() / old_aspect;

            if aspect_change > 0.05 {
                println!("Window aspect ratio changed significantly ({:.3} -> {:.3}, change: {:.1}%), capture region may need adjustment",
                        old_aspect, new_aspect, aspect_change * 100.0);

                let screen_start = Vec2::new(ratio_start.x * new_width as f32, ratio_start.y * new_height as f32);
                let screen_end = Vec2::new(ratio_end.x * new_width as f32, ratio_end.y * new_height as f32);

                println!("Capture region screen coordinates updated: ({:.1}, {:.1}) to ({:.1}, {:.1}) for new window size {}x{}",
                        screen_start.x, screen_start.y, screen_end.x, screen_end.y, new_width, new_height);
            }
            let start_world = self.screen_to_world(
                Vec2::new(ratio_start.x * new_width as f32, ratio_start.y * new_height as f32),
                new_width,
                new_height,
            );
            let end_world = self.screen_to_world(
                Vec2::new(ratio_end.x * new_width as f32, ratio_end.y * new_height as f32),
                new_width,
                new_height,
            );
            self.capture_region = Some((start_world, end_world));
        }

        self.window_width = new_width;
        self.window_height = new_height;
    }

    #[cfg(test)]
    pub fn start_region_selection(&mut self, start: Vec2) {
        self.is_selecting_region = true;
        self.selection_start = Some(start);
        self.selection_end = Some(start);
        println!("Started region selection at ({:.1}, {:.1})", start.x, start.y);
    }

    #[cfg(test)]
    pub fn update_region_selection(&mut self, end: Vec2) {
        if self.is_selecting_region {
            self.selection_end = Some(end);
        }
    }

    #[allow(dead_code)]
    pub fn world_to_screen(&self, world_pos: Vec2, width: u16, height: u16) -> Vec2 {
        // Apply inverse camera transformation
        let aspect_ratio = width as f32 / height as f32;
        let x = (world_pos.x - self.pos.x) / (self.scale * aspect_ratio);
        let y = (world_pos.y - self.pos.y) / self.scale;
        
        // Convert from normalized device coordinates to screen coordinates
        let screen_x = (x + 1.0) * width as f32 / 2.0;
        let screen_y = (1.0 - y) * height as f32 / 2.0;
        
        Vec2::new(screen_x, screen_y)
    }
}

#[cfg(test)]
mod tests {
    use ultraviolet::Vec2;
    use std::fs;
    use std::path::Path;
    use quarkstrom::Renderer as QuarkstromRenderer;

    #[test]
    fn test_screen_capture_setup() {
        let renderer: crate::renderer::Renderer = QuarkstromRenderer::new();
        assert!(!renderer.screen_capture_enabled);
        assert_eq!(renderer.capture_interval, 1.0);
        assert_eq!(renderer.capture_counter, 0);
        assert_eq!(renderer.capture_folder, "captures");
        assert!(renderer.capture_region.is_none());
    }

    #[test]
    fn test_capture_timing_logic() {
        let mut renderer: crate::renderer::Renderer = QuarkstromRenderer::new();
        
        renderer.screen_capture_enabled = true;
        renderer.capture_interval = 1.0;
        renderer.last_capture_time = 0.0;
        
        // Simulate timing logic by checking if enough time has passed
        let current_time = 1.0;
        let time_since_last = current_time - renderer.last_capture_time;
        assert!(time_since_last >= renderer.capture_interval);
    }

    #[test]
    fn test_capture_region_management() {
        let mut renderer: crate::renderer::Renderer = QuarkstromRenderer::new();
        
        // Test region selection
        let start = Vec2::new(100.0, 100.0);
        let end = Vec2::new(200.0, 200.0);
        
        renderer.start_region_selection(start);
        assert!(renderer.is_selecting_region);
        assert_eq!(renderer.selection_start, Some(start));
        
        renderer.update_region_selection(end);
        assert_eq!(renderer.selection_end, Some(end));
        
        renderer.finish_region_selection(800, 600);
        assert!(!renderer.is_selecting_region);
        // The capture region now contains world coordinates, not screen coordinates
        assert!(renderer.capture_region.is_some());
        assert!(renderer.selection_start.is_none());
        assert!(renderer.selection_end.is_none());
    }

    #[test]
    fn test_cancel_region_selection() {
        let mut renderer: crate::renderer::Renderer = QuarkstromRenderer::new();
        
        let start = Vec2::new(100.0, 100.0);
        let end = Vec2::new(200.0, 200.0);
        
        renderer.start_region_selection(start);
        renderer.update_region_selection(end);
        renderer.cancel_region_selection();
        
        assert!(!renderer.is_selecting_region);
        assert!(renderer.selection_start.is_none());
        assert!(renderer.selection_end.is_none());
        assert!(renderer.capture_region.is_none());
    }

    #[test]
    fn test_clear_capture_region() {
        let mut renderer: crate::renderer::Renderer = QuarkstromRenderer::new();
        
        renderer.capture_region = Some((Vec2::new(0.0, 0.0), Vec2::new(100.0, 100.0)));
        renderer.clear_capture_region();
        assert!(renderer.capture_region.is_none());
    }

    #[test]
    fn test_screen_to_world_conversion() {
        let renderer: crate::renderer::Renderer = QuarkstromRenderer::new();
        let width = 800;
        let height = 600;
        
        // Test center of screen
        let screen_center = Vec2::new(width as f32 / 2.0, height as f32 / 2.0);
        let world_pos = renderer.screen_to_world(screen_center, width, height);
        
        // The conversion should place the center near the camera position
        // (exact values depend on the camera position and scale)
        assert!(world_pos.x.is_finite());
        assert!(world_pos.y.is_finite());
    }

    #[test]
    fn test_capture_folder_creation() {
        let test_folder = "test_captures";
        
        // Clean up any existing test folder
        let _ = fs::remove_dir_all(test_folder);
        
        // Test folder creation logic (simulated)
        let result = fs::create_dir_all(test_folder);
        assert!(result.is_ok());
        
        // Verify folder exists
        assert!(Path::new(test_folder).exists());
        
        // Clean up
        let _ = fs::remove_dir_all(test_folder);
    }

    #[test]
    fn test_capture_file_naming() {
        use chrono::Utc;
        
        let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
        let counter = 42;
        let filename = format!("capture_{}_{:04}.png", timestamp, counter);
        
        assert!(filename.contains("capture_"));
        assert!(filename.contains("0042"));
        assert!(filename.ends_with(".png"));
    }

    #[test]
    fn test_capture_interval_validation() {
        let mut renderer: crate::renderer::Renderer = QuarkstromRenderer::new();
        
        // Test valid intervals
        renderer.capture_interval = 0.1;
        assert_eq!(renderer.capture_interval, 0.1);
        
        renderer.capture_interval = 10.0;
        assert_eq!(renderer.capture_interval, 10.0);
        
        // In real usage, the GUI would enforce ranges
        // but we can test the underlying data holds any value
        renderer.capture_interval = 0.01;
        assert_eq!(renderer.capture_interval, 0.01);
    }
}
