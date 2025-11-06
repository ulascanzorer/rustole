use wgpu_text::glyph_brush::{Layout, OwnedSection, Section, Text};

/// This is a structure in order to realize rows of lines on our terminal, which we can later manipulate based on incoming control sequences coming from the shell.
pub struct Screen {
    //  Each Section represents a glyph on the screen. Therefore it is defined as a vector of vectors, representing the 2D screen.
    pub glyphs: Vec<Vec<OwnedSection>>,
    pub font_size: f32,
    pub char_width: f32,
    pub row_index: usize,
    pub column_index: usize,
    pub screen_width: u32,
    pub screen_height: u32,
}

impl Screen {
    pub fn new(
        font_size: f32,
        char_width: f32,
        screen_width: u32,
        screen_height: u32,
        offset_from_left: f32,
        offset_from_top: f32,
    ) -> Self {
        let mut glyphs: Vec<Vec<OwnedSection>> = vec![];
        let num_rows = screen_height / font_size as u32;
        let num_cols = screen_width / font_size as u32;

        // TODO: Set the line properties correctly.
        for row_idx in 0..num_rows {
            let screen_pos_y =
                (screen_height as f32 * offset_from_top) + (row_idx as f32 * font_size);

            let mut glyphs_in_line = vec![];

            for col_idx in 0..num_cols {
                let screen_pos_x = offset_from_left + (char_width * col_idx as f32);

                let section = Section::default()
                    .add_text(
                        Text::new("")
                            .with_scale(font_size)
                            .with_color([0.6, 0.6, 0.5, 1.0]),
                    )
                    .with_bounds((screen_width as f32 * 0.95, screen_height as f32))
                    .with_layout(Layout::default_single_line())
                    .with_screen_position((screen_pos_x, screen_pos_y))
                    .to_owned();

                glyphs_in_line.push(section);
            }

            glyphs.push(glyphs_in_line);
        }

        Screen {
            glyphs,
            font_size,
            char_width,
            row_index: 0,
            column_index: 0,
            screen_width,
            screen_height,
        }
    }
}
