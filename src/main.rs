use process::exit;
use std::*;
use time::Duration;

use crossterm::{cursor, event, execute, style, terminal};

mod graphics;
use graphics::*;

mod args;
use args::*;

// Config
const VIEWPORT_FOV: f32 = 1.7;
const VIEWPORT_DISTANCE: f32 = 0.1;
const TARGET_DURATION_PER_FRAME: Duration = Duration::from_millis(1000 / 60);
const MOUSE_SPEED_MULTIPLIER: f32 = 30.;
const INITIAL_DISTANCE_MULTIPLIER: f32 = 1.5;
const SCROLL_MULTIPLIER: f32 = 0.03;
const PAN_MULTIPLIER: f32 = 0.1;
// const LINE_DENSITY: f32 = 10.0; // Points per unit length for line rendering

// Command mode state
struct CommandState {
    active: bool,
    buffer: String,
    error_message: Option<String>,
    success_message: Option<String>,
}

impl CommandState {
    fn new() -> Self {
        CommandState {
            active: false,
            buffer: String::new(),
            error_message: None,
            success_message: None,
        }
    }

    fn enter_command_mode(&mut self) {
        self.active = true;
        self.buffer.clear();
        self.error_message = None;
        self.success_message = None;
    }

    fn exit_command_mode(&mut self) {
        self.active = false;
        self.buffer.clear();
    }

    fn add_char(&mut self, c: char) {
        self.buffer.push(c);
    }

    fn backspace(&mut self) {
        self.buffer.pop();
    }

    fn execute_command(&mut self, point_cloud: &mut PointCloud, camera: &Camera) -> bool {
        let command = self.buffer.trim();

        if command.starts_with("load ") {
            let path = command.strip_prefix("load ").unwrap().trim();
            match PointCloud::from_file(path) {
                Ok(new_cloud) => {
                    if new_cloud.points.is_empty() {
                        self.error_message = Some("No points found in file".to_string());
                        return false;
                    }

                    // Add new points to existing point cloud
                    point_cloud.points.extend(new_cloud.points);

                    // Regenerate axes based on combined dataset
                    point_cloud.axes = PointCloud::generate_axes_public(&point_cloud.points);

                    self.exit_command_mode();
                    return false; // Don't reset view parameters
                }
                Err(e) => {
                    self.error_message = Some(format!("Failed to load: {}", e));
                    return false;
                }
            }
        } else if command.starts_with("export ") {
            let path = command.strip_prefix("export ").unwrap().trim();
            match self.export_view(camera, path) {
                Ok(_) => {
                    self.success_message = Some(format!("Exported to: {}", path));
                    self.exit_command_mode();
                    return false;
                }
                Err(e) => {
                    self.error_message = Some(format!("Export failed: {}", e));
                    return false;
                }
            }
        } else if command == "export" {
            // Default export filename with timestamp
            let timestamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs();
            let default_path = format!("altostratus_export_{}.txt", timestamp);
            
            match self.export_view(camera, &default_path) {
                Ok(_) => {
                    self.success_message = Some(format!("Exported to: {}", default_path));
                    self.exit_command_mode();
                    return false;
                }
                Err(e) => {
                    self.error_message = Some(format!("Export failed: {}", e));
                    return false;
                }
            }
        } else if command == "clear" {
            // Clear all points from the point cloud
            point_cloud.points.clear();

            // Regenerate axes (will use minimum length since no points)
            point_cloud.axes = PointCloud::generate_axes_public(&point_cloud.points);

            self.exit_command_mode();
            return false; // Don't reset view parameters
        } else if !command.is_empty() {
            self.error_message = Some("Unknown command".to_string());
            return false;
        }

        false
    }

    fn export_view(&self, camera: &Camera, path: &str) -> Result<(), Box<dyn error::Error>> {
        let content = camera.screen.export_to_string();
        fs::write(path, content)?;
        Ok(())
    }

    fn get_display_text(&self) -> String {
        if let Some(ref error) = self.error_message {
            format!("ERROR: {} (press ESC to continue)", error)
        } else if let Some(ref success) = self.success_message {
            format!("SUCCESS: {} (press ESC to continue)", success)
        } else {
            format!("Command: {}_", self.buffer)
        }
    }

    fn has_message(&self) -> bool {
        self.error_message.is_some() || self.success_message.is_some()
    }

    fn clear_messages(&mut self) {
        self.error_message = None;
        self.success_message = None;
    }
}

fn graceful_close() -> ! {
    execute!(io::stdout(), cursor::Show, event::DisableMouseCapture,).unwrap();
    terminal::disable_raw_mode().unwrap();
    exit(0)
}

fn error_close(msg: &dyn fmt::Display) -> ! {
    execute!(io::stderr(), style::Print(msg)).unwrap();
    graceful_close()
}

fn load_multiple_files(file_paths: &[String]) -> Result<PointCloud, Box<dyn error::Error>> {
    let mut combined_points = Vec::new();

    for path in file_paths {
        match PointCloud::from_file(path) {
            Ok(cloud) => {
                if cloud.points.is_empty() {
                    eprintln!("Warning: No points found in file: {}", path);
                    continue;
                }
                let points_count = cloud.points.len();
                combined_points.extend(cloud.points);
                println!("Loaded {} points from {}", points_count, path);
            }
            Err(e) => {
                return Err(format!("Failed to load {}: {}", path, e).into());
            }
        }
    }

    if combined_points.is_empty() {
        return Err("No points found in any of the provided files".into());
    }

    let axes = PointCloud::generate_axes_public(&combined_points);
    Ok(PointCloud {
        points: combined_points,
        axes,
    })
}

fn main() {
    // Parse command line arguments
    match parse_arguments() {
        ParseResult::ShowUsage => {
            print_usage();
            graceful_close();
        }
        ParseResult::ShowDetailedHelp => {
            print_detailed_help();
            graceful_close();
        }
        ParseResult::LoadFiles(file_paths) => {
            // Continue with the main application
            run_application(file_paths);
        }
    }
}

fn run_application(file_paths: Vec<String>) {
    terminal::enable_raw_mode().unwrap();
    execute!(io::stdout(), cursor::Hide, event::EnableMouseCapture,).unwrap();

    // Load point cloud(s)
    let mut point_cloud = match load_multiple_files(&file_paths) {
        Ok(cloud) => cloud,
        Err(error) => error_close(&error),
    };

    if point_cloud.points.is_empty() {
        error_close(&"No points found in any files");
    }

    // Get dimensions
    let center = point_cloud.get_center();
    let diagonal = point_cloud.get_diagonal().max(1.0); // Ensure we don't get zero diagonal

    // Setup camera
    let mut camera = Camera::new(center, 0., 0., 0., VIEWPORT_DISTANCE, VIEWPORT_FOV);

    let mut view_yaw: f32 = std::f32::consts::PI / 2.0;
    let mut view_pitch: f32 = 0.0;
    let mut distance_to_data = diagonal * INITIAL_DISTANCE_MULTIPLIER;
    let mut pan_mode = false;

    // Setup events
    let mut mouse_speed: (f32, f32) = (0., 0.);
    let mut last_mouse_position = Point2D::new(0, 0);
    let mut center_point = center;

    // Setup command state
    let mut command_state = CommandState::new();

    // Start main loop
    loop {
        let start = time::Instant::now();
        let mut start_mouse_position = last_mouse_position;

        // Look through the queue while there is an available event
        let mut event_count = 0;
        while event::poll(Duration::from_secs(0)).unwrap() {
            if let Ok(event) = event::read() {
                match event {
                    event::Event::Key(key_event) => {
                        if command_state.active {
                            // Handle command mode input
                            match key_event.code {
                                event::KeyCode::Esc => {
                                    if command_state.has_message() {
                                        command_state.clear_messages();
                                    } else {
                                        command_state.exit_command_mode();
                                    }
                                }
                                event::KeyCode::Enter => {
                                    command_state.execute_command(&mut point_cloud, &camera);
                                }
                                event::KeyCode::Backspace => {
                                    command_state.backspace();
                                }
                                event::KeyCode::Char(c) => {
                                    command_state.add_char(c);
                                }
                                _ => {}
                            }
                        } else {
                            // Handle normal mode input
                            let is_ctrl_c = key_event.modifiers == event::KeyModifiers::CONTROL
                                && key_event.code == event::KeyCode::Char('c');

                            if is_ctrl_c {
                                graceful_close()
                            } else if key_event.code == event::KeyCode::Char('/') {
                                command_state.enter_command_mode();
                            }
                        }
                    }

                    // Mouse controls (only when not in command mode)
                    event::Event::Mouse(mouse_event) if !command_state.active => {
                        let (x, y) = (mouse_event.column, mouse_event.row);
                        match mouse_event.kind {
                            event::MouseEventKind::Down(_) => {
                                pan_mode = mouse_event.modifiers == event::KeyModifiers::CONTROL;
                                last_mouse_position.x = x as i32;
                                last_mouse_position.y = y as i32;
                                start_mouse_position = last_mouse_position;
                                event_count += 1;
                            }

                            event::MouseEventKind::Drag(_) => {
                                pan_mode = mouse_event.modifiers == event::KeyModifiers::CONTROL;
                                let delta_x = x as f32 - start_mouse_position.x as f32;
                                let delta_y = start_mouse_position.y as f32 - y as f32;
                                mouse_speed.0 =
                                    delta_x / camera.screen.width as f32 * MOUSE_SPEED_MULTIPLIER;
                                mouse_speed.1 =
                                    delta_y / camera.screen.width as f32 * MOUSE_SPEED_MULTIPLIER;
                                last_mouse_position.x = x as i32;
                                last_mouse_position.y = y as i32;
                                event_count += 1;
                            }

                            event::MouseEventKind::ScrollDown => {
                                distance_to_data += diagonal * SCROLL_MULTIPLIER;
                            }

                            event::MouseEventKind::ScrollUp => {
                                distance_to_data -= diagonal * SCROLL_MULTIPLIER;
                                distance_to_data = distance_to_data.max(0.1);
                            }
                            _ => {}
                        }
                    }
                    _ => {}
                }
            }
        }

        // If no event happened, reset the mouse
        if event_count == 0 {
            mouse_speed = (0., 0.);
            pan_mode = false;
        }

        // Update viewer params
        if pan_mode {
            // Handle horizontal pan
            center_point.x -= mouse_speed.0 * camera.yaw.cos() * diagonal * PAN_MULTIPLIER;
            center_point.z += mouse_speed.0 * camera.yaw.sin() * diagonal * PAN_MULTIPLIER;

            // Handle vertical pan
            center_point.y -= mouse_speed.1 * camera.pitch.cos() * diagonal * PAN_MULTIPLIER;
            center_point.x +=
                mouse_speed.1 * camera.yaw.sin() * camera.pitch.sin() * diagonal * PAN_MULTIPLIER;
            center_point.z +=
                mouse_speed.1 * camera.yaw.cos() * camera.pitch.sin() * diagonal * PAN_MULTIPLIER;
        } else {
            view_yaw -= mouse_speed.0;
            view_pitch -= mouse_speed.1;
        }

        // Update camera position
        camera.coordinates.z =
            -view_yaw.cos() * view_pitch.cos() * distance_to_data + center_point.z;
        camera.coordinates.x =
            view_yaw.sin() * view_pitch.cos() * distance_to_data + center_point.x;
        camera.coordinates.y = view_pitch.sin() * distance_to_data + center_point.y;
        camera.yaw = -view_yaw;
        camera.pitch = -view_pitch;

        // Render
        camera.screen.fit_to_terminal();
        camera.screen.clear();

        // Render axes with arrowheads and labels
        for axis in &point_cloud.axes {
            // Draw main axis line
            camera.plot_line(&axis.axis_line.0, &axis.axis_line.1);

            // Draw arrowhead lines
            for (start, end) in &axis.arrowhead_lines {
                camera.plot_line(start, end);
            }
        }

        // Render points as vertices
        for point in &point_cloud.points {
            camera.plot_point(point);
        }

        camera.screen.render();

        // Add buffer time to hit 60 fps
        if let Some(time) = TARGET_DURATION_PER_FRAME.checked_sub(start.elapsed()) {
            thread::sleep(time);
        }

        // Status message
        let final_msg = if command_state.active || command_state.error_message.is_some() {
            command_state.get_display_text()
        } else {
            let fps_msg = format!("fps: {:3.0}", 1. / start.elapsed().as_secs_f32());
            let resolution_msg = format!(
                "resolution: {} x {}",
                camera.screen.width, camera.screen.height,
            );
            let points_msg = format!("points: {}", point_cloud.points.len());

            let full_msg = format!(
                "{} | {} | {} | Press '/' for commands",
                points_msg, resolution_msg, fps_msg
            );
            let short_msg = format!("{} | {} | '/' for commands", points_msg, fps_msg);

            match terminal::size().unwrap().0 as usize {
                width if width > full_msg.len() => full_msg,
                width if width > short_msg.len() => short_msg,
                _ => format!("{} | '/'", points_msg),
            }
        };

        execute!(
            io::stdout(),
            terminal::Clear(terminal::ClearType::CurrentLine),
            style::Print(final_msg),
        )
        .unwrap();
    }
}
