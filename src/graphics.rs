use crossterm::{cursor, execute, style, terminal};
use std::ops;
use std::*;

// Export format enum
#[derive(Clone, Copy, Debug)]
pub enum ExportFormat {
    Plain,    // No colors, just Braille characters
    Color,    // ANSI color codes (current behavior)
    Html,     // Self-contained HTML with inline styles
}

// Color definitions for ANSI 8-color support
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Color {
    Default, // No color specified (white/default terminal color)
    Black,
    Red,
    Green,
    Yellow,
    Blue,
    Magenta,
    Cyan,
    White,
}

impl Color {
    pub fn to_crossterm_color(&self) -> style::Color {
        match self {
            Color::Default => style::Color::Reset,
            Color::Black => style::Color::Black,
            Color::Red => style::Color::Red,
            Color::Green => style::Color::Green,
            Color::Yellow => style::Color::Yellow,
            Color::Blue => style::Color::Blue,
            Color::Magenta => style::Color::Magenta,
            Color::Cyan => style::Color::Cyan,
            Color::White => style::Color::White,
        }
    }

    pub fn from_string(s: &str) -> Option<Color> {
        match s.to_lowercase().as_str() {
            "default" | "white" => Some(Color::Default),
            "black" => Some(Color::Black),
            "red" => Some(Color::Red),
            "green" => Some(Color::Green),
            "yellow" => Some(Color::Yellow),
            "blue" => Some(Color::Blue),
            "magenta" => Some(Color::Magenta),
            "cyan" => Some(Color::Cyan),
            _ => None,
        }
    }
}

// Graphics rendering constants
const DEFAULT_TERMINAL_DIMENSIONS: (u16, u16) = (80, 24);
const MIN_AXIS_LENGTH: f32 = 5.0;

// Simple 3d point wrapper with color support.
#[derive(Copy, Clone)]
pub struct Point3D {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub color: Color,
}

impl Point3D {
    pub fn new(x: f32, y: f32, z: f32) -> Point3D {
        Point3D {
            x,
            y,
            z,
            color: Color::Default,
        }
    }

    pub fn new_with_color(x: f32, y: f32, z: f32, color: Color) -> Point3D {
        Point3D { x, y, z, color }
    }
}

// Simple 2d point wrapper.
#[derive(Copy, Clone)]
pub struct Point2D {
    pub x: i32,
    pub y: i32,
}

impl Point2D {
    pub fn new(x: i32, y: i32) -> Point2D {
        Point2D { x, y }
    }
}

// Braille pixel struct
#[derive(Clone, Copy)]
pub struct BraillePixel {
    data: [[bool; 2]; 4],
}

impl BraillePixel {
    pub fn new() -> BraillePixel {
        BraillePixel {
            data: [[false; 2]; 4],
        }
    }

    pub fn to_char(&self) -> char {
        let mut unicode: u32 = 0;
        if self.data[0][0] {
            unicode |= 1 << 0
        }
        if self.data[1][0] {
            unicode |= 1 << 1
        }
        if self.data[2][0] {
            unicode |= 1 << 2
        }

        if self.data[0][1] {
            unicode |= 1 << 3
        }
        if self.data[1][1] {
            unicode |= 1 << 4
        }
        if self.data[2][1] {
            unicode |= 1 << 5
        }

        if self.data[3][0] {
            unicode |= 1 << 6
        }
        if self.data[3][1] {
            unicode |= 1 << 7
        }

        unicode |= 0x28 << 8;

        char::from_u32(unicode).unwrap()
    }
}

impl ops::Index<usize> for BraillePixel {
    type Output = [bool; 2];

    fn index(&self, index: usize) -> &Self::Output {
        &self.data[index]
    }
}

impl ops::IndexMut<usize> for BraillePixel {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.data[index]
    }
}

// Screen wrapper with color support and performance optimizations
pub struct Screen {
    pub width: u16,
    pub height: u16,
    content: Vec<Vec<bool>>,
    colors: Vec<Vec<Color>>,                  // Store color for each pixel
    cached_terminal_size: Option<(u16, u16)>, // Cache terminal size to avoid unnecessary checks
}

impl Screen {
    pub fn new() -> Screen {
        execute!(
            io::stdout(),
            cursor::MoveTo(0, 0),
            terminal::Clear(terminal::ClearType::All)
        )
        .unwrap();

        Screen {
            content: Vec::new(),
            colors: Vec::new(),
            width: 0,
            height: 0,
            cached_terminal_size: None,
        }
    }

    pub fn fit_to_terminal(&mut self) {
        let (terminal_width, terminal_height) = terminal::size()
            .unwrap_or(DEFAULT_TERMINAL_DIMENSIONS);

        // Only resize if terminal size has actually changed
        if self.cached_terminal_size == Some((terminal_width, terminal_height)) {
            return; // No change, skip resize
        }

        // Update cache and resize
        self.cached_terminal_size = Some((terminal_width, terminal_height));
        self.resize(terminal_width * 2, (terminal_height - 1) * 4);
    }

    pub fn write(&mut self, val: bool, point: &Point2D) {
        let x_in_bounds = (0..self.width as i32).contains(&point.x);
        let y_in_bounds = (0..self.height as i32).contains(&point.y);
        if x_in_bounds && y_in_bounds {
            self.content[point.y as usize][point.x as usize] = val;
        }
    }

    pub fn write_colored(&mut self, val: bool, point: &Point2D, color: Color) {
        let x_in_bounds = (0..self.width as i32).contains(&point.x);
        let y_in_bounds = (0..self.height as i32).contains(&point.y);
        if x_in_bounds && y_in_bounds {
            self.content[point.y as usize][point.x as usize] = val;
            self.colors[point.y as usize][point.x as usize] = color;
        }
    }

    pub fn clear(&mut self) {
        // Reuse existing memory instead of reallocating
        for row in &mut self.content {
            row.fill(false);
        }
        for row in &mut self.colors {
            row.fill(Color::Default);
        }
    }

    pub fn resize(&mut self, width: u16, height: u16) {
        // Early return if size hasn't changed to avoid unnecessary work
        if width == self.width && height == self.height {
            return;
        }

        if height > self.height {
            self.content.extend(
                (0..(height - self.height) as usize).map(|_| vec![false; width as usize])
            );
            self.colors.extend(
                (0..(height - self.height) as usize).map(|_| vec![Color::Default; width as usize])
            );
        } else {
            self.content.truncate(height as usize);
            self.colors.truncate(height as usize);
        }
        self.height = height;

        if width > self.width {
            for row in self.content.iter_mut() {
                row.extend((0..(width - self.width) as usize).map(|_| false));
            }
            for row in self.colors.iter_mut() {
                row.extend((0..(width - self.width) as usize).map(|_| Color::Default));
            }
        } else {
            for row in self.content.iter_mut() {
                row.truncate(width as usize);
            }
            for row in self.colors.iter_mut() {
                row.truncate(width as usize);
            }
        }
        self.width = width;
    }

    pub fn line(&mut self, start: &Point2D, end: &Point2D) {
        let delta_x = start.x.abs_diff(end.x) as i32;
        let step_x: i32 = if start.x < end.x { 1 } else { -1 };
        let delta_y = -(start.y.abs_diff(end.y) as i32);
        let step_y: i32 = if start.y < end.y { 1 } else { -1 };
        let mut err = delta_x + delta_y;

        let mut x = start.x;
        let mut y = start.y;

        self.write(true, &Point2D::new(x, y));

        while x != end.x || y != end.y {
            self.write(true, &Point2D::new(x, y));
            let curr_err = err;

            if curr_err * 2 >= delta_y {
                err += delta_y;
                x += step_x;
            }

            if curr_err * 2 <= delta_x {
                err += delta_x;
                y += step_y;
            }
        }
    }

    pub fn render(&self) {
        // Calculate approximate output size to pre-allocate string buffer
        let num_rows = self.content.len().div_ceil(4);
        let chars_per_row = self.width.div_ceil(2) + 2; // +2 for \r\n
        let estimated_size = (num_rows * chars_per_row as usize) + 20; // +20 for cursor movement

        let mut output = String::with_capacity(estimated_size);

        // Add cursor movement to beginning
        output.push_str("\x1b[H"); // ANSI escape for cursor to home position

        let chunked_rows = self.content.chunks(4);
        let chunked_color_rows = self.colors.chunks(4);

        let mut current_color = Color::Default;

        for (subrows, color_subrows) in chunked_rows.zip(chunked_color_rows) {
            let real_row_width = self.width.div_ceil(2) as usize;
            let mut real_row = vec![BraillePixel::new(); real_row_width];
            let mut real_row_colors = vec![Color::Default; real_row_width];

            for (subpixel_y, (subrow, color_subrow)) in
                subrows.iter().zip(color_subrows.iter()).enumerate()
            {
                let chunked_subrow = subrow.chunks_exact(2);
                let remainder = chunked_subrow.remainder();

                let chunked_color_subrow = color_subrow.chunks_exact(2);
                let color_remainder = chunked_color_subrow.remainder();

                for (real_x, (pixel_row, color_row)) in
                    chunked_subrow.zip(chunked_color_subrow).enumerate()
                {
                    if real_x < real_row_width {
                        real_row[real_x][subpixel_y][..pixel_row.len()].copy_from_slice(pixel_row);

                        // Determine dominant color for this Braille character section
                        if real_row_colors[real_x] == Color::Default {
                            // Find the first non-default color in this section
                            for (pixel_set, &color) in pixel_row.iter().zip(color_row.iter()) {
                                if *pixel_set && color != Color::Default {
                                    real_row_colors[real_x] = color;
                                    break;
                                }
                            }
                        }
                    }
                }

                // Handle remainder
                if real_row_width > 0 && !remainder.is_empty() {
                    real_row[real_row_width - 1][subpixel_y][..remainder.len()]
                        .copy_from_slice(remainder);

                    // Handle color remainder
                    if !color_remainder.is_empty()
                        && real_row_colors[real_row_width - 1] == Color::Default
                    {
                        for (pixel_set, &color) in remainder.iter().zip(color_remainder.iter()) {
                            if *pixel_set && color != Color::Default {
                                real_row_colors[real_row_width - 1] = color;
                                break;
                            }
                        }
                    }
                }
            }

            // Render the row with color changes
            for (pixel, &pixel_color) in real_row.iter().zip(real_row_colors.iter()) {
                // Only change color if it's different from current
                if pixel_color != current_color {
                    let color_code = match pixel_color {
                        Color::Default => "\x1b[39m".to_string(),
                        Color::Black => "\x1b[30m".to_string(),
                        Color::Red => "\x1b[31m".to_string(),
                        Color::Green => "\x1b[32m".to_string(),
                        Color::Yellow => "\x1b[33m".to_string(),
                        Color::Blue => "\x1b[34m".to_string(),
                        Color::Magenta => "\x1b[35m".to_string(),
                        Color::Cyan => "\x1b[36m".to_string(),
                        Color::White => "\x1b[37m".to_string(),
                    };
                    output.push_str(&color_code);
                    current_color = pixel_color;
                }

                output.push(pixel.to_char());
            }
            output.push_str("\r\n");
        }

        // Reset color at the end
        if current_color != Color::Default {
            output.push_str("\x1b[39m"); // Reset to default color
        }

        // Output everything at once instead of many small writes
        execute!(io::stdout(), style::Print(output)).unwrap();
    }

    pub fn export_to_string(&self, format: ExportFormat) -> String {
        match format {
            ExportFormat::Plain => self.export_plain(),
            ExportFormat::Color => self.export_color(),
            ExportFormat::Html => self.export_html(),
        }
    }

    fn export_plain(&self) -> String {
        // Similar to color export but strip all color information
        let chunked_rows = self.content.chunks(4);

        let mut rows_output = Vec::new();

        for subrows in chunked_rows {
            let real_row_width = self.width.div_ceil(2) as usize;
            let mut real_row = vec![BraillePixel::new(); real_row_width];

            for (subpixel_y, subrow) in subrows.iter().enumerate() {
                let chunked_subrow = subrow.chunks_exact(2);
                let remainder = chunked_subrow.remainder();

                for (real_x, pixel_row) in chunked_subrow.enumerate() {
                    if real_x < real_row_width {
                        real_row[real_x][subpixel_y][..pixel_row.len()].copy_from_slice(pixel_row);
                    }
                }

                // Handle remainder
                if real_row_width > 0 && !remainder.is_empty() {
                    real_row[real_row_width - 1][subpixel_y][..remainder.len()]
                        .copy_from_slice(remainder);
                }
            }

            // Build row string without any color codes
            let mut row_string = String::new();
            for pixel in real_row.iter() {
                row_string.push(pixel.to_char());
            }
            rows_output.push(row_string);
        }

        // Trim the output with 2-character margin
        self.trim_output_with_margin(rows_output, 2)
    }

    fn export_color(&self) -> String {
        // Export the content with ANSI color codes (color-specific export)
        let chunked_rows = self.content.chunks(4);
        let chunked_color_rows = self.colors.chunks(4);

        let mut rows_output = Vec::new();
        let mut current_color = Color::Default;

        for (subrows, color_subrows) in chunked_rows.zip(chunked_color_rows) {
            let real_row_width = self.width.div_ceil(2) as usize;
            let mut real_row = vec![BraillePixel::new(); real_row_width];
            let mut real_row_colors = vec![Color::Default; real_row_width];

            for (subpixel_y, (subrow, color_subrow)) in
                subrows.iter().zip(color_subrows.iter()).enumerate()
            {
                let chunked_subrow = subrow.chunks_exact(2);
                let remainder = chunked_subrow.remainder();

                let chunked_color_subrow = color_subrow.chunks_exact(2);
                let color_remainder = chunked_color_subrow.remainder();

                for (real_x, (pixel_row, color_row)) in
                    chunked_subrow.zip(chunked_color_subrow).enumerate()
                {
                    if real_x < real_row_width {
                        real_row[real_x][subpixel_y][..pixel_row.len()].copy_from_slice(pixel_row);

                        // Determine dominant color for this Braille character section
                        if real_row_colors[real_x] == Color::Default {
                            // Find the first non-default color in this section
                            for (pixel_set, &color) in pixel_row.iter().zip(color_row.iter()) {
                                if *pixel_set && color != Color::Default {
                                    real_row_colors[real_x] = color;
                                    break;
                                }
                            }
                        }
                    }
                }

                // Handle remainder
                if real_row_width > 0 && !remainder.is_empty() {
                    real_row[real_row_width - 1][subpixel_y][..remainder.len()]
                        .copy_from_slice(remainder);

                    // Handle color remainder
                    if !color_remainder.is_empty()
                        && real_row_colors[real_row_width - 1] == Color::Default
                    {
                        for (pixel_set, &color) in remainder.iter().zip(color_remainder.iter()) {
                            if *pixel_set && color != Color::Default {
                                real_row_colors[real_row_width - 1] = color;
                                break;
                            }
                        }
                    }
                }
            }

            // Build row string with color changes
            let mut row_string = String::new();
            for (pixel, &pixel_color) in real_row.iter().zip(real_row_colors.iter()) {
                // Only change color if it's different from current
                if pixel_color != current_color {
                    let color_code = match pixel_color {
                        Color::Default => "\x1b[39m".to_string(),
                        Color::Black => "\x1b[30m".to_string(),
                        Color::Red => "\x1b[31m".to_string(),
                        Color::Green => "\x1b[32m".to_string(),
                        Color::Yellow => "\x1b[33m".to_string(),
                        Color::Blue => "\x1b[34m".to_string(),
                        Color::Magenta => "\x1b[35m".to_string(),
                        Color::Cyan => "\x1b[36m".to_string(),
                        Color::White => "\x1b[37m".to_string(),
                    };
                    row_string.push_str(&color_code);
                    current_color = pixel_color;
                }

                row_string.push(pixel.to_char());
            }
            rows_output.push(row_string);
        }

        // Reset color at the end if needed
        if current_color != Color::Default {
            if let Some(last_row) = rows_output.last_mut() {
                last_row.push_str("\x1b[39m"); // Reset to default color
            }
        }

        // Trim the output with 2-character margin
        self.trim_output_with_margin(rows_output, 2)
    }

    fn export_html(&self) -> String {
        // Generate HTML with inline styles for self-contained web embedding
        let chunked_rows = self.content.chunks(4);
        let chunked_color_rows = self.colors.chunks(4);

        let mut rows_output = Vec::new();
        let mut current_color = Color::Default;

        for (subrows, color_subrows) in chunked_rows.zip(chunked_color_rows) {
            let real_row_width = self.width.div_ceil(2) as usize;
            let mut real_row = vec![BraillePixel::new(); real_row_width];
            let mut real_row_colors = vec![Color::Default; real_row_width];

            for (subpixel_y, (subrow, color_subrow)) in
                subrows.iter().zip(color_subrows.iter()).enumerate()
            {
                let chunked_subrow = subrow.chunks_exact(2);
                let remainder = chunked_subrow.remainder();

                let chunked_color_subrow = color_subrow.chunks_exact(2);
                let color_remainder = chunked_color_subrow.remainder();

                for (real_x, (pixel_row, color_row)) in
                    chunked_subrow.zip(chunked_color_subrow).enumerate()
                {
                    if real_x < real_row_width {
                        real_row[real_x][subpixel_y][..pixel_row.len()].copy_from_slice(pixel_row);

                        // Determine dominant color for this Braille character section
                        if real_row_colors[real_x] == Color::Default {
                            // Find the first non-default color in this section
                            for (pixel_set, &color) in pixel_row.iter().zip(color_row.iter()) {
                                if *pixel_set && color != Color::Default {
                                    real_row_colors[real_x] = color;
                                    break;
                                }
                            }
                        }
                    }
                }

                // Handle remainder
                if real_row_width > 0 && !remainder.is_empty() {
                    real_row[real_row_width - 1][subpixel_y][..remainder.len()]
                        .copy_from_slice(remainder);

                    // Handle color remainder
                    if !color_remainder.is_empty()
                        && real_row_colors[real_row_width - 1] == Color::Default
                    {
                        for (pixel_set, &color) in remainder.iter().zip(color_remainder.iter()) {
                            if *pixel_set && color != Color::Default {
                                real_row_colors[real_row_width - 1] = color;
                                break;
                            }
                        }
                    }
                }
            }

            // Build row string with HTML span tags
            let mut row_string = String::new();
            let mut span_open = false;
            
            for (pixel, &pixel_color) in real_row.iter().zip(real_row_colors.iter()) {
                // Handle color changes with HTML spans
                if pixel_color != current_color {
                    // Close previous span if open
                    if span_open {
                        row_string.push_str("</span>");
                        span_open = false;
                    }
                    
                    // Open new span if color is not default
                    if pixel_color != Color::Default {
                        let color_style = match pixel_color {
                            Color::Black => "color: #000000",
                            Color::Red => "color: #ff0000",
                            Color::Green => "color: #00ff00",
                            Color::Yellow => "color: #ffff00",
                            Color::Blue => "color: #0000ff",
                            Color::Magenta => "color: #ff00ff",
                            Color::Cyan => "color: #00ffff",
                            Color::White => "color: #ffffff",
                            Color::Default => "", // This case won't be reached due to the outer if condition
                        };
                        row_string.push_str(&format!("<span style=\"{}\">", color_style));
                        span_open = true;
                    }
                    current_color = pixel_color;
                }

                // Escape HTML special characters in Braille characters (though they're unlikely)
                let char_str = pixel.to_char().to_string();
                let escaped = char_str
                    .replace('&', "&amp;")
                    .replace('<', "&lt;")
                    .replace('>', "&gt;");
                row_string.push_str(&escaped);
            }
            
            // Close span at end of row if open
            if span_open {
                row_string.push_str("</span>");
            }
            
            rows_output.push(row_string);
        }

        // Trim the content first
        let trimmed_content = self.trim_output_with_margin(rows_output, 2);
        
        // Wrap in a complete HTML structure for easy embedding
        format!(
            "<pre style=\"font-family: monospace; line-height: 1; background-color: #000000; color: #ffffff; padding: 10px; margin: 0; white-space: pre; overflow-x: auto;\">{}</pre>",
            trimmed_content
        )
    }

    fn trim_output_with_margin(&self, rows: Vec<String>, margin: usize) -> String {
        if rows.is_empty() {
            return String::new();
        }

        // Find content bounds (ignoring ANSI color codes)
        let mut top_bound = None;
        let mut bottom_bound = None;
        let mut left_bound = None;
        let mut right_bound = None;

        // Find top and bottom bounds
        for (row_idx, row) in rows.iter().enumerate() {
            if self.row_has_content(row) {
                if top_bound.is_none() {
                    top_bound = Some(row_idx);
                }
                bottom_bound = Some(row_idx);
            }
        }

        // If no content found, return empty string
        let (top, bottom) = match (top_bound, bottom_bound) {
            (Some(t), Some(b)) => (t, b),
            _ => return String::new(),
        };

        // Find left and right bounds
        for row in &rows[top..=bottom] {
            let (left, right) = self.find_row_content_bounds(row);
            if let (Some(l), Some(r)) = (left, right) {
                left_bound = Some(left_bound.map_or(l, |current: usize| current.min(l)));
                right_bound = Some(right_bound.map_or(r, |current: usize| current.max(r)));
            }
        }

        let (left, right) = match (left_bound, right_bound) {
            (Some(l), Some(r)) => (l, r),
            _ => return String::new(),
        };

        // Apply margins (but don't go negative)
        let final_top = top.saturating_sub(margin);
        let final_bottom = (bottom + margin).min(rows.len() - 1);
        let final_left = left.saturating_sub(margin);
        let final_right = right + margin;

        // Extract the trimmed region
        let mut result = String::new();
        for row in &rows[final_top..=final_bottom] {
            let trimmed_row = self.substring_with_ansi(row, final_left, final_right);
            result.push_str(&trimmed_row);
            result.push('\n');
        }

        result
    }

    fn row_has_content(&self, row: &str) -> bool {
        // Check if row has any non-space Braille characters (ignoring ANSI codes)
        let mut chars = row.chars().peekable();
        while let Some(ch) = chars.next() {
            if ch == '\x1b' {
                // Skip ANSI escape sequence
                while let Some(ch) = chars.next() {
                    if ch == 'm' {
                        break;
                    }
                }
            } else if ch != ' ' && ch != '⠀' {
                // ⠀ is the empty Braille character (U+2800)
                return true;
            }
        }
        false
    }

    fn find_row_content_bounds(&self, row: &str) -> (Option<usize>, Option<usize>) {
        let mut char_positions = Vec::new();
        let mut chars = row.chars().peekable();
        let mut pos = 0;

        // Extract just the content characters (no ANSI codes)
        while let Some(ch) = chars.next() {
            if ch == '\x1b' {
                // Skip ANSI escape sequence
                while let Some(ch) = chars.next() {
                    if ch == 'm' {
                        break;
                    }
                }
            } else {
                char_positions.push((pos, ch));
                pos += 1;
            }
        }

        let mut left = None;
        let mut right = None;

        for (pos, ch) in char_positions {
            if ch != ' ' && ch != '⠀' {
                // ⠀ is the empty Braille character (U+2800)
                if left.is_none() {
                    left = Some(pos);
                }
                right = Some(pos);
            }
        }

        (left, right)
    }

    fn substring_with_ansi(&self, text: &str, start: usize, end: usize) -> String {
        let mut result = String::new();
        let mut chars = text.chars().peekable();
        let mut pos = 0;
        let mut in_ansi = false;

        while let Some(ch) = chars.next() {
            if ch == '\x1b' {
                in_ansi = true;
                result.push(ch);
            } else if in_ansi {
                result.push(ch);
                if ch == 'm' {
                    in_ansi = false;
                }
            } else {
                if pos >= start && pos <= end {
                    result.push(ch);
                }
                pos += 1;
            }
        }

        result
    }
}

pub struct Camera {
    pub coordinates: Point3D,
    pub yaw: f32,
    pub pitch: f32,
    pub roll: f32,
    pub viewport_distance: f32,
    pub viewport_fov: f32,
    pub screen: Screen,
}

impl Camera {
    pub fn new(
        coordinates: Point3D,
        yaw: f32,
        pitch: f32,
        roll: f32,
        viewport_distance: f32,
        viewport_fov: f32,
    ) -> Camera {
        Camera {
            coordinates,
            yaw,
            pitch,
            roll,
            viewport_distance,
            viewport_fov,
            screen: Screen::new(),
        }
    }

    fn world_to_camera(&self, point: &Point3D) -> Point3D {
        let (s_yaw, s_pitch, s_roll) = (self.yaw.sin(), self.pitch.sin(), self.roll.sin());
        let (c_yaw, c_pitch, c_roll) = (self.yaw.cos(), self.pitch.cos(), self.roll.cos());

        let delta_x = point.x - self.coordinates.x;
        let delta_y = point.y - self.coordinates.y;
        let delta_z = point.z - self.coordinates.z;

        // Undo yaw
        let unyawed_x = delta_x * c_yaw - delta_z * s_yaw;
        let unyawed_y = delta_y;
        let unyawed_z = delta_x * s_yaw + delta_z * c_yaw;

        // Undo pitch
        let unpitched_x = unyawed_x;
        let unpitched_y = unyawed_y * c_pitch - unyawed_z * s_pitch;
        let unpitched_z = unyawed_y * s_pitch + unyawed_z * c_pitch;

        // Undo roll
        let unrolled_x = unpitched_x * c_roll - unpitched_y * s_roll;
        let unrolled_y = unpitched_x * s_roll + unpitched_y * c_roll;
        let unrolled_z = unpitched_z;

        Point3D::new_with_color(unrolled_x, unrolled_y, unrolled_z, point.color)
    }

    fn camera_to_screen(&self, point: &Point3D) -> Point2D {
        let viewport_x = point.x * self.viewport_distance / point.z;
        let viewport_y = point.y * self.viewport_distance / point.z;

        let viewport_width = 2. * self.viewport_distance * (self.viewport_fov / 2.).tan();
        let viewport_height =
            (self.screen.height as f32 / self.screen.width as f32) * viewport_width;

        let screen_x = (viewport_x / viewport_width + 0.5) * self.screen.width as f32;
        let screen_y = (1.0 - (viewport_y / viewport_height + 0.5)) * self.screen.height as f32;

        Point2D::new(screen_x.round() as i32, screen_y.round() as i32)
    }

    pub fn plot_point(&mut self, point: &Point3D) {
        let camera_point = self.world_to_camera(point);
        if camera_point.z >= self.viewport_distance {
            self.screen
                .write_colored(true, &self.camera_to_screen(&camera_point), point.color);
        }
    }

    pub fn plot_line(&mut self, start: &Point3D, end: &Point3D) {
        let camera_start = self.world_to_camera(start);
        let camera_end = self.world_to_camera(end);
        let clip_start = camera_start.z < self.viewport_distance;
        let clip_end = camera_end.z < self.viewport_distance;

        if clip_start && clip_end {
            return;
        }

        if !clip_start && !clip_end {
            self.screen.line(
                &self.camera_to_screen(&camera_start),
                &self.camera_to_screen(&camera_end),
            );
            return;
        }

        let (clipped, unclipped) = if clip_start {
            (camera_start, camera_end)
        } else {
            (camera_end, camera_start)
        };

        let distance_behind_viewport = self.viewport_distance - clipped.z;
        let (delta_x, delta_y, delta_z) = (
            unclipped.x - clipped.x,
            unclipped.y - clipped.y,
            unclipped.z - clipped.z,
        );
        let lambda = distance_behind_viewport / delta_z;
        let new_clipped = Point3D::new_with_color(
            lambda * delta_x + clipped.x,
            lambda * delta_y + clipped.y,
            self.viewport_distance,
            clipped.color,
        );

        self.screen.line(
            &self.camera_to_screen(&new_clipped),
            &self.camera_to_screen(&unclipped),
        )
    }
}

pub struct AxisDecoration {
    pub axis_line: (Point3D, Point3D),
    pub arrowhead_lines: Vec<(Point3D, Point3D)>,
}

pub struct PointCloud {
    pub points: Vec<Point3D>,
    pub axes: Vec<AxisDecoration>,
}

impl PointCloud {
    pub fn from_file(path: &str) -> Result<PointCloud, Box<dyn error::Error>> {
        let content = fs::read_to_string(path)?;
        let mut points = Vec::new();

        for (line_num, line) in content.lines().enumerate() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.is_empty() {
                continue;
            }

            match parts[0] {
                "p" => {
                    // Point format: p x y z [color]
                    if parts.len() < 4 || parts.len() > 5 {
                        return Err(format!(
                            "Invalid point format on line {}: {}. Expected 'p x y z [color]'",
                            line_num + 1,
                            line
                        )
                        .into());
                    }

                    let file_x: f32 = parts[1].parse().map_err(|_| {
                        format!(
                            "Invalid x coordinate on line {}: {}",
                            line_num + 1,
                            parts[1]
                        )
                    })?;
                    let file_y: f32 = parts[2].parse().map_err(|_| {
                        format!(
                            "Invalid y coordinate on line {}: {}",
                            line_num + 1,
                            parts[2]
                        )
                    })?;
                    let file_z: f32 = parts[3].parse().map_err(|_| {
                        format!(
                            "Invalid z coordinate on line {}: {}",
                            line_num + 1,
                            parts[3]
                        )
                    })?;

                    // Parse optional color
                    let color = if parts.len() == 5 {
                        Color::from_string(parts[4]).unwrap_or(Color::Default)
                    } else {
                        Color::Default
                    };

                    // Remap coordinates: file_z becomes viewer_y (up axis)
                    points.push(Point3D::new_with_color(file_x, file_z, file_y, color));
                }

                "pc" => {
                    // Colored point format: pc x y z color
                    if parts.len() != 5 {
                        return Err(format!("Invalid colored point format on line {}: {}. Expected 'pc x y z color'", line_num + 1, line).into());
                    }

                    let file_x: f32 = parts[1].parse().map_err(|_| {
                        format!(
                            "Invalid x coordinate on line {}: {}",
                            line_num + 1,
                            parts[1]
                        )
                    })?;
                    let file_y: f32 = parts[2].parse().map_err(|_| {
                        format!(
                            "Invalid y coordinate on line {}: {}",
                            line_num + 1,
                            parts[2]
                        )
                    })?;
                    let file_z: f32 = parts[3].parse().map_err(|_| {
                        format!(
                            "Invalid z coordinate on line {}: {}",
                            line_num + 1,
                            parts[3]
                        )
                    })?;

                    let color = Color::from_string(parts[4]).ok_or_else(|| {
                        format!("Invalid color '{}' on line {}", parts[4], line_num + 1)
                    })?;

                    // Remap coordinates: file_z becomes viewer_y (up axis)
                    points.push(Point3D::new_with_color(file_x, file_z, file_y, color));
                }

                "l" => {
                    // Line format: l x1 y1 z1 x2 y2 z2
                    if parts.len() != 7 {
                        return Err(format!(
                            "Invalid line format on line {}: {}",
                            line_num + 1,
                            line
                        )
                        .into());
                    }

                    let x1: f32 = parts[1].parse().map_err(|_| {
                        format!(
                            "Invalid x1 coordinate on line {}: {}",
                            line_num + 1,
                            parts[1]
                        )
                    })?;
                    let y1: f32 = parts[2].parse().map_err(|_| {
                        format!(
                            "Invalid y1 coordinate on line {}: {}",
                            line_num + 1,
                            parts[2]
                        )
                    })?;
                    let z1: f32 = parts[3].parse().map_err(|_| {
                        format!(
                            "Invalid z1 coordinate on line {}: {}",
                            line_num + 1,
                            parts[3]
                        )
                    })?;
                    let x2: f32 = parts[4].parse().map_err(|_| {
                        format!(
                            "Invalid x2 coordinate on line {}: {}",
                            line_num + 1,
                            parts[4]
                        )
                    })?;
                    let y2: f32 = parts[5].parse().map_err(|_| {
                        format!(
                            "Invalid y2 coordinate on line {}: {}",
                            line_num + 1,
                            parts[5]
                        )
                    })?;
                    let z2: f32 = parts[6].parse().map_err(|_| {
                        format!(
                            "Invalid z2 coordinate on line {}: {}",
                            line_num + 1,
                            parts[6]
                        )
                    })?;

                    // Convert line to points using LINE_DENSITY
                    let line_points = Self::line_to_points(
                        Point3D::new(x1, z1, y1), // Remap coordinates
                        Point3D::new(x2, z2, y2), // Remap coordinates
                    );
                    points.extend(line_points);
                }

                "lc" => {
                    // Colored line format: lc x1 y1 z1 x2 y2 z2 color
                    if parts.len() != 8 {
                        return Err(format!("Invalid colored line format on line {}: {}. Expected 'lc x1 y1 z1 x2 y2 z2 color'", line_num + 1, line).into());
                    }

                    let x1: f32 = parts[1].parse().map_err(|_| {
                        format!(
                            "Invalid x1 coordinate on line {}: {}",
                            line_num + 1,
                            parts[1]
                        )
                    })?;
                    let y1: f32 = parts[2].parse().map_err(|_| {
                        format!(
                            "Invalid y1 coordinate on line {}: {}",
                            line_num + 1,
                            parts[2]
                        )
                    })?;
                    let z1: f32 = parts[3].parse().map_err(|_| {
                        format!(
                            "Invalid z1 coordinate on line {}: {}",
                            line_num + 1,
                            parts[3]
                        )
                    })?;
                    let x2: f32 = parts[4].parse().map_err(|_| {
                        format!(
                            "Invalid x2 coordinate on line {}: {}",
                            line_num + 1,
                            parts[4]
                        )
                    })?;
                    let y2: f32 = parts[5].parse().map_err(|_| {
                        format!(
                            "Invalid y2 coordinate on line {}: {}",
                            line_num + 1,
                            parts[5]
                        )
                    })?;
                    let z2: f32 = parts[6].parse().map_err(|_| {
                        format!(
                            "Invalid z2 coordinate on line {}: {}",
                            line_num + 1,
                            parts[6]
                        )
                    })?;

                    let color = Color::from_string(parts[7]).ok_or_else(|| {
                        format!("Invalid color '{}' on line {}", parts[7], line_num + 1)
                    })?;

                    // Convert line to colored points using LINE_DENSITY
                    let line_points = Self::line_to_points(
                        Point3D::new_with_color(x1, z1, y1, color), // Remap coordinates with color
                        Point3D::new_with_color(x2, z2, y2, color), // Remap coordinates with color
                    );
                    points.extend(line_points);
                }

                _ => {
                    // Legacy format: assume three numbers are x y z coordinates
                    if parts.len() != 3 {
                        return Err(format!("Invalid format on line {}: {}. Expected 'p x y z', 'pc x y z color', 'l x1 y1 z1 x2 y2 z2', 'lc x1 y1 z1 x2 y2 z2 color', or legacy 'x y z'", line_num + 1, line).into());
                    }

                    let file_x: f32 = parts[0].parse().map_err(|_| {
                        format!(
                            "Invalid x coordinate on line {}: {}",
                            line_num + 1,
                            parts[0]
                        )
                    })?;
                    let file_y: f32 = parts[1].parse().map_err(|_| {
                        format!(
                            "Invalid y coordinate on line {}: {}",
                            line_num + 1,
                            parts[1]
                        )
                    })?;
                    let file_z: f32 = parts[2].parse().map_err(|_| {
                        format!(
                            "Invalid z coordinate on line {}: {}",
                            line_num + 1,
                            parts[2]
                        )
                    })?;

                    // Remap coordinates: file_z becomes viewer_y (up axis)
                    points.push(Point3D::new(file_x, file_z, file_y));
                }
            }
        }

        let axes = Self::generate_axes(&points);

        Ok(PointCloud { points, axes })
    }

    pub fn generate_axes_public(points: &[Point3D]) -> Vec<AxisDecoration> {
        Self::generate_axes(points)
    }

    fn line_to_points(start: Point3D, end: Point3D) -> Vec<Point3D> {
        // Calculate line length
        let dx = end.x - start.x;
        let dy = end.y - start.y;
        let dz = end.z - start.z;
        let length = (dx * dx + dy * dy + dz * dz).sqrt();

        if length == 0.0 {
            return vec![start]; // Degenerate line, just return start point
        }

        // Calculate number of points needed based on LINE_DENSITY
        // Import LINE_DENSITY from main module
        const LINE_DENSITY: f32 = 10.0; // Points per unit length
        let num_points = (length * LINE_DENSITY).max(2.0) as usize; // At least 2 points

        let mut points = Vec::with_capacity(num_points);

        // Generate points along the line
        for i in 0..num_points {
            let t = i as f32 / (num_points - 1) as f32; // Parameter from 0 to 1
            let point = Point3D::new_with_color(
                start.x + t * dx,
                start.y + t * dy,
                start.z + t * dz,
                start.color, // Use start point's color for the entire line
            );
            points.push(point);
        }

        points
    }

    fn generate_axes(points: &[Point3D]) -> Vec<AxisDecoration> {
        let max_distance = if points.is_empty() {
            MIN_AXIS_LENGTH
        } else {
            let furthest_point_distance = points
                .iter()
                .map(|p| (p.x.powi(2) + p.y.powi(2) + p.z.powi(2)).sqrt())
                .fold(0.0, f32::max);

            // Use minimum axis length or 110% of furthest point, whichever is larger
            (furthest_point_distance * 1.1).max(MIN_AXIS_LENGTH)
        };

        let origin = Point3D::new(0., 0., 0.);
        let x_end = Point3D::new(max_distance, 0., 0.);
        let y_end = Point3D::new(0., max_distance, 0.);
        let z_end = Point3D::new(0., 0., max_distance);

        vec![
            Self::create_axis_decoration(origin, x_end, max_distance),
            Self::create_axis_decoration(origin, y_end, max_distance),
            Self::create_axis_decoration(origin, z_end, max_distance),
        ]
    }

    fn create_axis_decoration(start: Point3D, end: Point3D, scale: f32) -> AxisDecoration {
        let arrowhead_lines = Self::generate_arrowhead(&start, &end, scale);

        AxisDecoration {
            axis_line: (start, end),
            arrowhead_lines,
        }
    }

    fn generate_arrowhead(start: &Point3D, end: &Point3D, scale: f32) -> Vec<(Point3D, Point3D)> {
        // Calculate direction vector
        let dx = end.x - start.x;
        let dy = end.y - start.y;
        let dz = end.z - start.z;
        let length = (dx * dx + dy * dy + dz * dz).sqrt();

        if length == 0.0 {
            return vec![];
        }

        // Normalized direction
        let dir_x = dx / length;
        let dir_y = dy / length;
        let dir_z = dz / length;

        // Arrowhead size
        let arrow_length = scale * 0.05;
        let arrow_angle = 0.5f32; // radians (~30 degrees)

        // Find two perpendicular vectors to the axis direction
        let (perp1_x, perp1_y, perp1_z, perp2_x, perp2_y, perp2_z) = if dir_z.abs() < 0.9 {
            // If not too aligned with Z, use Z cross product
            let p1_x = -dir_y;
            let p1_y = dir_x;
            let p1_z = 0.0;
            let p1_len = (p1_x * p1_x + p1_y * p1_y).sqrt();
            let (p1_x, p1_y, p1_z) = if p1_len > 0.0 {
                (p1_x / p1_len, p1_y / p1_len, p1_z / p1_len)
            } else {
                (1.0, 0.0, 0.0)
            };

            // Second perpendicular: dir cross perp1
            let p2_x = dir_y * p1_z - dir_z * p1_y;
            let p2_y = dir_z * p1_x - dir_x * p1_z;
            let p2_z = dir_x * p1_y - dir_y * p1_x;

            (p1_x, p1_y, p1_z, p2_x, p2_y, p2_z)
        } else {
            // Use X cross product if aligned with Z
            let p1_x = 0.0;
            let p1_y = -dir_z;
            let p1_z = dir_y;
            let p1_len = (p1_y * p1_y + p1_z * p1_z).sqrt();
            let (p1_x, p1_y, p1_z) = if p1_len > 0.0 {
                (p1_x / p1_len, p1_y / p1_len, p1_z / p1_len)
            } else {
                (0.0, 1.0, 0.0)
            };

            let p2_x = dir_y * p1_z - dir_z * p1_y;
            let p2_y = dir_z * p1_x - dir_x * p1_z;
            let p2_z = dir_x * p1_y - dir_y * p1_x;

            (p1_x, p1_y, p1_z, p2_x, p2_y, p2_z)
        };

        // Create arrowhead points
        let cos_angle = arrow_angle.cos();
        let sin_angle = arrow_angle.sin();

        let arrow1 = Point3D::new(
            end.x - arrow_length * (dir_x * cos_angle + perp1_x * sin_angle),
            end.y - arrow_length * (dir_y * cos_angle + perp1_y * sin_angle),
            end.z - arrow_length * (dir_z * cos_angle + perp1_z * sin_angle),
        );

        let arrow2 = Point3D::new(
            end.x - arrow_length * (dir_x * cos_angle + perp2_x * sin_angle),
            end.y - arrow_length * (dir_y * cos_angle + perp2_y * sin_angle),
            end.z - arrow_length * (dir_z * cos_angle + perp2_z * sin_angle),
        );

        vec![(*end, arrow1), (*end, arrow2)]
    }

    pub fn get_bounds(&self) -> (Point3D, Point3D) {
        if self.points.is_empty() {
            return (Point3D::new(0., 0., 0.), Point3D::new(0., 0., 0.));
        }

        let mut min_bounds = self.points[0];
        let mut max_bounds = self.points[0];

        for point in &self.points {
            min_bounds.x = f32::min(point.x, min_bounds.x);
            min_bounds.y = f32::min(point.y, min_bounds.y);
            min_bounds.z = f32::min(point.z, min_bounds.z);

            max_bounds.x = f32::max(point.x, max_bounds.x);
            max_bounds.y = f32::max(point.y, max_bounds.y);
            max_bounds.z = f32::max(point.z, max_bounds.z);
        }

        (min_bounds, max_bounds)
    }

    pub fn get_center(&self) -> Point3D {
        let (min_bounds, max_bounds) = self.get_bounds();
        Point3D::new(
            (min_bounds.x + max_bounds.x) / 2.,
            (min_bounds.y + max_bounds.y) / 2.,
            (min_bounds.z + max_bounds.z) / 2.,
        )
    }

    pub fn get_diagonal(&self) -> f32 {
        let (min_bounds, max_bounds) = self.get_bounds();
        ((min_bounds.x - max_bounds.x).powi(2)
            + (min_bounds.y - max_bounds.y).powi(2)
            + (min_bounds.z - max_bounds.z).powi(2))
        .sqrt()
    }
}
