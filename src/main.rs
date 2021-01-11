use inputbot;

mod screenshot;

use screenshot::{ScreenshotData, CursorPos};

fn main() {
    println!("Hello, world!");
    inputbot::KeybdKey::TKey.bind(|| {
        analyze_pressed();
    });

    inputbot::handle_input_events();
}

enum AnalyzeError {
    ScreenshotFailed,
}

fn find_top_left_corner(screen: &ScreenshotData, mouse_location: &CursorPos) -> Option<(u32, u32)> {
    let mut x_edge = None;
    let mut y_edge = None;

    let border_color_inv = 0x54_51_49_ff_u32;
    let border_color_overlay_box = 0x60_5d_58_ff_u32;


    for x_offset in 0.. {
        if x_offset > mouse_location.x {
            break;
        }

        let new_x = mouse_location.x - x_offset;
        let color = screen.get_pixel(new_x as usize, mouse_location.y as usize);

        if border_color_overlay_box == color {
            x_edge = Some(new_x);
            break;
        }
    };

    for y_offset in 0.. {
        if y_offset > mouse_location.y {
            break;
        }

        let new_y = mouse_location.y - y_offset;
        let color = screen.get_pixel(mouse_location.x as usize, new_y as usize);

        if border_color_overlay_box == color {
            y_edge = Some(new_y);
            break;
        }
    };

    match (x_edge, y_edge) {
        (Some(x), Some(y)) => Some((x, y)),
        (_, _) => None,
    }
}

fn analyze_pressed() -> Result<(), AnalyzeError> {
    let mouse_location = CursorPos::get();

    let screen = screenshot::take_screenshot().map_err(|_| AnalyzeError::ScreenshotFailed)?;

    dbg!(mouse_location);

    let tl_corner = find_top_left_corner(&screen, &mouse_location);

    let h = 30;
    let w = 700;
    let offset = 20;

    dbg!(tl_corner);

    Ok(())
}
