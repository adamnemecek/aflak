use glium::backend::Facade;
use imgui::{ImGuiMouseCursor, ImMouseButton, ImStr, ImString, ImVec2, Ui};
use imgui_glium_renderer::Texture;

use super::Error;
use hist;
use image;
use lut::{self, BuiltinLUT, ColorLUT};

pub struct State {
    pub lut: ColorLUT,
    pub vmin: f32,
    pub vmax: f32,
    pub mouse_pos: (f32, f32),
    pub hist_logscale: bool,
    lut_min_moving: bool,
    lut_max_moving: bool,
}

impl Default for State {
    fn default() -> Self {
        use std::f32;
        Self {
            lut: BuiltinLUT::Flame.lut(),
            vmin: f32::NAN,
            vmax: f32::NAN,
            mouse_pos: (f32::NAN, f32::NAN),
            hist_logscale: true,
            lut_min_moving: false,
            lut_max_moving: false,
        }
    }
}

impl State {
    pub fn show_bar<P, S>(&mut self, ui: &Ui, pos: P, size: S)
    where
        P: Into<ImVec2>,
        S: Into<ImVec2>,
    {
        let pos = pos.into();
        let size = size.into();

        ui.set_cursor_screen_pos(pos);
        ui.invisible_button(im_str!("image_bar"), size);
        if ui.is_item_hovered() {
            if ui.imgui().is_mouse_clicked(ImMouseButton::Right) {
                ui.open_popup(im_str!("swap-lut"))
            }
        }
        ui.popup(im_str!("swap-lut"), || {
            ui.text("Swap LUT");
            ui.separator();
            for builtin_lut in BuiltinLUT::values() {
                ui.push_id(*builtin_lut as i32);
                if ui.menu_item(builtin_lut.name()).build() {
                    self.lut.set_gradient(*builtin_lut);
                }
                ui.pop_id();
            }
        });

        let draw_list = ui.get_window_draw_list();

        // Show triangle to change contrast
        {
            const TRIANGLE_LEFT_PADDING: f32 = 10.0;
            const TRIANGLE_HEIGHT: f32 = 20.0;
            const TRIANGLE_WIDTH: f32 = 15.0;
            let lims = self.lut.lims();

            // Min triangle
            let min_color = lut::util::to_u32_color(&self.lut.color_at(lims.0));
            let x_pos = pos.x + size.x + TRIANGLE_LEFT_PADDING;
            let y_pos = pos.y + size.y * (1.0 - lims.0);
            draw_list
                .add_triangle(
                    [x_pos, y_pos],
                    [x_pos + TRIANGLE_WIDTH, y_pos + TRIANGLE_HEIGHT / 2.0],
                    [x_pos + TRIANGLE_WIDTH, y_pos - TRIANGLE_HEIGHT / 2.0],
                    min_color,
                )
                .filled(true)
                .build();
            draw_list
                .add_triangle(
                    [x_pos, y_pos],
                    [x_pos + TRIANGLE_WIDTH, y_pos + TRIANGLE_HEIGHT / 2.0],
                    [x_pos + TRIANGLE_WIDTH, y_pos - TRIANGLE_HEIGHT / 2.0],
                    lut::util::invert_color(min_color),
                )
                .build();
            ui.set_cursor_screen_pos([x_pos, y_pos - TRIANGLE_HEIGHT / 2.0]);
            ui.invisible_button(im_str!("set_min"), [TRIANGLE_WIDTH, TRIANGLE_HEIGHT]);
            if ui.is_item_hovered() {
                ui.imgui().set_mouse_cursor(ImGuiMouseCursor::ResizeNS);
                if ui.imgui().is_mouse_clicked(ImMouseButton::Left) {
                    self.lut_min_moving = true;
                }
            }
            if self.lut_min_moving {
                ui.imgui().set_mouse_cursor(ImGuiMouseCursor::ResizeNS);
                let (_, mouse_y) = ui.imgui().mouse_pos();
                let min = 1.0 - (mouse_y - pos.y) / size.y;
                self.lut.set_min(min);
            }
            if !ui.imgui().is_mouse_down(ImMouseButton::Left) {
                self.lut_min_moving = false;
            }

            // Max triangle
            let max_color = lut::util::to_u32_color(&self.lut.color_at(lims.1));
            let y_pos = pos.y + size.y * (1.0 - lims.1);
            draw_list
                .add_triangle(
                    [x_pos, y_pos],
                    [x_pos + TRIANGLE_WIDTH, y_pos + TRIANGLE_HEIGHT / 2.0],
                    [x_pos + TRIANGLE_WIDTH, y_pos - TRIANGLE_HEIGHT / 2.0],
                    max_color,
                )
                .filled(true)
                .build();
            draw_list
                .add_triangle(
                    [x_pos, y_pos],
                    [x_pos + TRIANGLE_WIDTH, y_pos + TRIANGLE_HEIGHT / 2.0],
                    [x_pos + TRIANGLE_WIDTH, y_pos - TRIANGLE_HEIGHT / 2.0],
                    lut::util::invert_color(max_color),
                )
                .build();
            ui.set_cursor_screen_pos([x_pos, y_pos - TRIANGLE_HEIGHT / 2.0]);
            ui.invisible_button(im_str!("set_max"), [TRIANGLE_WIDTH, TRIANGLE_HEIGHT]);
            if ui.is_item_hovered() {
                ui.imgui().set_mouse_cursor(ImGuiMouseCursor::ResizeNS);
                if ui.imgui().is_mouse_clicked(ImMouseButton::Left) {
                    self.lut_max_moving = true;
                }
            }
            if self.lut_max_moving {
                ui.imgui().set_mouse_cursor(ImGuiMouseCursor::ResizeNS);
                let (_, mouse_y) = ui.imgui().mouse_pos();
                let max = 1.0 - (mouse_y - pos.y) / size.y;
                self.lut.set_max(max);
            }
            if !ui.imgui().is_mouse_down(ImMouseButton::Left) {
                self.lut_max_moving = false;
            }
        }

        let x_pos = pos.x + 5.0;
        for ((v1, c1), (v2, c2)) in self.lut.bounds() {
            let bottom_col = lut::util::to_u32_color(&c1);
            let top_col = lut::util::to_u32_color(&c2);
            let bottom_y_pos = pos.y + size.y * (1.0 - v1);
            let top_y_pos = pos.y + size.y * (1.0 - v2);
            draw_list.add_rect_filled_multicolor(
                [x_pos, top_y_pos],
                [x_pos + size.x, bottom_y_pos],
                top_col,
                top_col,
                bottom_col,
                bottom_col,
            );
        }
        let mut i = 1.0;
        let text_height = ui.get_text_line_height_with_spacing();
        const LABEL_HORIZONTAL_PADDING: f32 = 2.0;
        const COLOR: u32 = 0xFFFFFFFF;
        const TICK_SIZE: f32 = 3.0;
        const TICK_COUNT: usize = 10;
        const TICK_STEP: f32 = 1.0 / TICK_COUNT as f32;
        while i >= -0.01 {
            let tick_y_pos = pos.y + size.y - i * size.y;
            let y_pos = tick_y_pos - text_height / 2.5;
            let val = self.vmin + (self.vmax - self.vmin) * i;
            draw_list.add_text(
                [x_pos + size.x + LABEL_HORIZONTAL_PADDING, y_pos],
                COLOR,
                &format!("{:.2}", val),
            );
            draw_list
                .add_line(
                    [x_pos + size.x - TICK_SIZE, tick_y_pos],
                    [x_pos + size.x, tick_y_pos],
                    COLOR,
                )
                .build();
            // TODO: Make step editable
            i -= TICK_STEP;
        }
    }

    pub fn show_image<F>(
        &mut self,
        ui: &Ui,
        ctx: &F,
        name: &ImStr,
        image: &Vec<Vec<f32>>,
    ) -> Result<[(f32, f32); 2], Error>
    where
        F: Facade,
    {
        const IMAGE_LEFT_PADDING: f32 = 20.0;
        const IMAGE_TOP_PADDING: f32 = 0.0;

        // Returns a handle to the mutable texture (we could write on it)
        let raw = image::make_raw_image(image, self)?;
        let texture = ui.replace_texture(name, Texture::from_data(ctx, raw)?);
        let tex_size = texture.get_size();
        let size = image::get_size(tex_size);
        let p = ui.get_cursor_screen_pos();
        ui.set_cursor_screen_pos([p.0 + IMAGE_LEFT_PADDING, p.1 + IMAGE_TOP_PADDING]);
        let p = ui.get_cursor_screen_pos();

        ui.image(&texture, size).build();
        let abs_mouse_pos = ui.imgui().mouse_pos();
        let mouse_pos = (abs_mouse_pos.0 - p.0, -abs_mouse_pos.1 + p.1 + size.1);
        self.mouse_pos = (
            mouse_pos.0 / size.0 * tex_size.0 as f32,
            mouse_pos.1 / size.1 * tex_size.1 as f32,
        );

        if ui.is_item_hovered() {
            ui.dummy((0.0, 5.0));
            ui.text(format!(
                "X: {:.1}, Y: {:.1}",
                self.mouse_pos.0, self.mouse_pos.1
            ));
        }

        // Add ticks
        const COLOR: u32 = 0xFFFFFFFF;
        let draw_list = ui.get_window_draw_list();
        const TICK_COUNT: u32 = 10;
        const TICK_SIZE: f32 = 3.0;
        const LABEL_HORIZONTAL_PADDING: f32 = 2.0;

        // X-axis
        let x_step = size.0 / TICK_COUNT as f32;
        let mut x_pos = p.0;
        let y_pos = p.1 + size.1;
        for i in 0..=TICK_COUNT {
            draw_list
                .add_line([x_pos, y_pos], [x_pos, y_pos - TICK_SIZE], COLOR)
                .build();
            let label = ImString::new(format!("{:.0}", i * tex_size.0 / TICK_COUNT));
            let text_size = ui.calc_text_size(&label, false, -1.0);
            draw_list.add_text([x_pos - text_size.x / 2.0, y_pos], COLOR, label.to_str());
            x_pos += x_step;
        }

        // Y-axis
        let y_step = size.1 / TICK_COUNT as f32;
        let mut y_pos = p.1 + size.1;
        let x_pos = p.0;
        for i in 0..=TICK_COUNT {
            draw_list
                .add_line([x_pos, y_pos], [x_pos + TICK_SIZE, y_pos], COLOR)
                .build();
            let label = ImString::new(format!("{:.0}", i * tex_size.1 / TICK_COUNT));
            let text_size = ui.calc_text_size(&label, false, -1.0);
            draw_list.add_text(
                [
                    x_pos - text_size.x - LABEL_HORIZONTAL_PADDING,
                    y_pos - text_size.y / 2.0,
                ],
                COLOR,
                label.to_str(),
            );
            y_pos -= y_step;
        }

        Ok([p, size])
    }

    pub fn show_hist<P, S>(&self, ui: &Ui, pos: P, size: S, image: &Vec<Vec<f32>>)
    where
        P: Into<ImVec2>,
        S: Into<ImVec2>,
    {
        let pos = pos.into();
        let size = size.into();

        const FILL_COLOR: u32 = 0xFF999999;
        const BORDER_COLOR: u32 = 0xFF000000;
        let hist = hist::histogram(image, self.vmin, self.vmax);
        if let Some(max_count) = hist.iter().map(|bin| bin.count).max() {
            let draw_list = ui.get_window_draw_list();

            let x_pos = pos.x;
            for bin in hist {
                let y_pos = pos.y + size.y / (self.vmax - self.vmin) * (self.vmax - bin.start);
                let y_pos_end = pos.y + size.y / (self.vmax - self.vmin) * (self.vmax - bin.end);
                let length = size.x * if self.hist_logscale {
                    (bin.count as f32).log10() / (max_count as f32).log10()
                } else {
                    (bin.count as f32) / (max_count as f32)
                };
                draw_list
                    .add_rect(
                        [x_pos + size.x - length, y_pos],
                        [x_pos + size.x, y_pos_end],
                        FILL_COLOR,
                    )
                    .filled(true)
                    .build();
            }

            draw_list
                .add_rect(pos, [pos.x + size.x, pos.y + size.y], BORDER_COLOR)
                .build();
        } // TODO show error
    }
}
