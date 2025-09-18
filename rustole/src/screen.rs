use wgpu_text::glyph_brush::{BuiltInLineBreaker, Layout, OwnedSection, Section, Text};

/// This is a structure in order to realize rows of lines on our terminal, which we can later manipulate based on incoming control sequences coming from the shell.
pub struct Screen {
    pub lines: Vec<OwnedSection>, // Each Section represents a line on the screen (Section width = columns), number of Sections represents the number of lines (rows).
    pub font_size: f32,
    pub row_index: usize,
    pub column_index: usize,
}

impl Screen {
    pub fn new(
        num_rows: u32,
        num_columns: u32,
        font_size: f32,
        screen_width: u32,
        screen_height: u32,
        offset_from_left: f32,
        offset_from_top: f32,
    ) -> Self {
        let mut lines: Vec<OwnedSection> = vec![];

        // TODO: Set the line properties correctly.
        for row_idx in 0..num_rows {
            let screen_pos_x = offset_from_left;
            let screen_pos_y = offset_from_top + (row_idx as f32 * font_size);

            let section = Section::default()
                .add_text(
                    Text::new("")
                        .with_scale(font_size)
                        .with_color([0.6, 0.6, 0.5, 1.0]),
                )
                .with_bounds((screen_width as f32 * 0.95, screen_height as f32))
                .with_layout(Layout::default().line_breaker(BuiltInLineBreaker::AnyCharLineBreaker))
                .with_screen_position((screen_pos_x, screen_pos_y))
                .to_owned();

            lines.push(section);
        }

        Screen {
            lines: lines,
            font_size: font_size,
            row_index: 0,
            column_index: 0,
        }
    }
}
