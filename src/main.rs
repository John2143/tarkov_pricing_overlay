use std::{error::Error, fs};

use apis::market::TarkovMarketItem;
use clap::Parser;
use closestmatch::ClosestMatch;
use colored::{ColoredString, Colorize};
use ocrs::{OcrEngine, OcrEngineParams};
use once_cell::{self, sync::Lazy};
use rten::Model;
use rten_tensor::NdTensorView;
use screenshot::{CursorPos, ScreenshotData};

mod apis;
mod closestmatch;
mod screenshot;

static MARKET_API_KEY: Lazy<String> =
    Lazy::new(|| include_str!("../market_api_key.txt").trim().to_owned());

static WORDS: Lazy<ClosestMatch> = Lazy::new(|| {
    let titles = include_str!("../wiki_titles.txt");
    ClosestMatch::new(
        titles.lines().map(|x| x.to_owned()).collect(),
        vec![3, 4, 5, 6],
    )
});

#[derive(clap::Parser)]
struct Cli {
    /// print out a color table to show all the tier values
    #[arg(short, long)]
    print_table: bool,
}

fn main() {
    let cli = Cli::parse();
    if cli.print_table {
        print_color_table();
        return;
    }

    input();
    //println!("{}", WORDS.get_closest("water ootle wit filter Aquamari").unwrap());
    //println!("{}", *MARKET_API_KEY);
}

#[cfg(feature = "input")]
fn input() {
    inputbot::KeybdKey::TKey.bind(|| {
        std::thread::spawn(move || {
            match analyze_pressed() {
                Ok(_) => {}
                Err(e) => {
                    println!("{:?}", e);
                }
            };
            println!("");
        });
    });

    println!("Bot ready");

    //create_window();

    let t = std::thread::spawn(|| inputbot::handle_input_events());

    t.join().unwrap();
}
#[cfg(not(feature = "input"))]
fn input() {}

#[derive(Debug)]
enum AnalyzeError {
    ScreenshotFailed,
    CannotFindInspectBox,
    InvalidOcr,
    BadRequest(&'static str),
    BadMarketJson,
    NoCloseWord(String),
    Other(Box<dyn Error>),
}

impl From<Box<dyn Error>> for AnalyzeError {
    fn from(value: Box<dyn Error>) -> Self {
        Self::Other(value)
    }
}

fn find_top_left_corner(screen: &ScreenshotData, mouse_location: &CursorPos) -> Option<(u32, u32)> {
    let mut x_edge = None;
    let mut y_edge = None;

    if mouse_location.x >= 1920 || mouse_location.y >= 1090 {
        return None;
    }

    //let border_color_inv = 0x54_51_49_ff_u32;
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
    }

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
    }

    match (x_edge, y_edge) {
        (Some(x), Some(y)) => Some((x, y)),
        (_, _) => None,
    }
}

///Get the cost to list the item on the flea market. formula from wike
fn get_flea_tax(value_to_traders: i64, list_price: i64) -> i64 {
    let v_o = list_price as f64;
    let v_r = (value_to_traders as f64) * 2.0; //sell price * 2 = buy price from trade
    let t_i = 0.05;
    let t_r = 0.05;
    let p_r = (v_r / v_o).log10();
    let p_o = (v_o / v_r).log10();

    let flea_price = v_o * t_i * 4f64.powf(p_o) + v_r * t_r * 4f64.powf(p_r);

    flea_price as i64
}

fn analyze_pressed() -> Result<(), AnalyzeError> {
    let mouse_location = CursorPos::get();

    let screen = screenshot::take_screenshot().map_err(|_| AnalyzeError::ScreenshotFailed)?;

    let tl_corner =
        find_top_left_corner(&screen, &mouse_location).ok_or(AnalyzeError::CannotFindInspectBox)?;

    let h = 30;
    let w = 500;
    let offset = 20;

    let i = screen.to_image().unwrap();
    let subimage = image::SubImage::new(&i, tl_corner.0 + offset, tl_corner.1, w, h);
    let subimage = subimage.to_image();

    //let (width, height) = subimage.dimensions();
    let layout = subimage.sample_layout();

    let image_tensor = NdTensorView::from_slice(
        subimage.as_raw().as_slice(),
        [h as usize, w as usize, 3],
        Some([
            layout.height_stride,
            layout.width_stride,
            layout.channel_stride,
        ]),
    )
    .unwrap()
    .permuted([2, 0, 1]) // HWC => CHW
    .to_tensor() // Make tensor contiguous, which makes `map` faster
    .map(|x| *x as f32 / 255.); // Rescale from [0, 255] to [0, 1]

    // https://github.com/robertknight/ocrs/blob/main/ocrs/examples/hello_ocr.rs
    let detection_model_data =
        fs::read("text-detection.rten").expect("Could not find text-detection.rten");
    let rec_model_data =
        fs::read("text-recognition.rten").expect("Could not find text-recognition.rten");

    let detection_model = Model::load(&detection_model_data).unwrap();
    let recognition_model = Model::load(&rec_model_data).unwrap();

    let engine = OcrEngine::new(OcrEngineParams {
        detection_model: Some(detection_model),
        recognition_model: Some(recognition_model),
        ..Default::default()
    })?;
    // Apply standard image pre-processing expected by this library (convert
    // to greyscale, map range to [-0.5, 0.5]).
    let ocr_input = engine.prepare_input(image_tensor.view())?;

    // Phase 1: Detect text words
    let word_rects = engine.detect_words(&ocr_input)?;

    // Phase 2: Perform layout analysis
    let line_rects = engine.find_text_lines(&ocr_input, &word_rects);

    // Phase 3: Recognize text
    let line_texts = engine.recognize_text(&ocr_input, &line_rects)?;
    let valid_text: Vec<_> = line_texts
        .iter()
        .flatten()
        // Filter likely spurious detections. With future model improvements
        // this should become unnecessary.
        .map(|l| l.to_string())
        .filter(|l| l.len() > 1)
        .collect();

    // We have pretty strict text detection, just assume the first match is the text
    let text_ocr = valid_text.get(0).ok_or(AnalyzeError::InvalidOcr)?;

    // Find the closest matching tarkov item
    let text = WORDS
        .get_closest(&text_ocr)
        .ok_or_else(|| AnalyzeError::NoCloseWord(text_ocr.to_string()))?;

    println!(
        "Detected text was '{}'. Closest was '{}'. Reading market data... ",
        &text_ocr, &text
    );

    let client = reqwest::blocking::Client::new();
    let d = client
        .get("https://api.tarkov-market.app/api/v1/item")
        .query(&[("q", &text)])
        .header("x-api-key", &*MARKET_API_KEY)
        .send()
        .map_err(|_| AnalyzeError::BadRequest("Something went wrong with the tarkov market api"))?;

    let text = d.text().unwrap();

    let items_to_price: apis::market::Root = serde_json::from_str(&text).map_err(|e| {
        // if we fail, just dump the whole payload
        dbg!(text);
        dbg!(e);
        AnalyzeError::BadMarketJson
    })?;

    for item in items_to_price {
        print_item(&item);
    }

    Ok(())
}

fn format_slots(value_in: ColoredString, cur_type: &str, item: &TarkovMarketItem) -> String {
    if item.slots > 1 {
        format!(
            " ({}{} x {})",
            value_in,
            cur_type,
            item.slots.to_string().bright_yellow(),
        )
    } else {
        format!("")
    }
}

fn print_item(item: &TarkovMarketItem) {
    println!("Name: {} ({})", item.name.red(), item.short_name.italic());

    // If this is a larger than 1x1, then display the per-slot value too
    let slot_value = color_currency(item.trader_price / item.slots, &item.trader_price_cur);
    let slots = format_slots(slot_value, &item.trader_price_cur, item);

    println!(
        "Trader Price: {} -> {}{}{slots}",
        item.trader_name,
        color_currency(item.trader_price, &item.trader_price_cur),
        item.trader_price_cur,
    );

    // the flea tax is based on how much the trader buys it for
    let trader_ruble_value = ruble_value(item.trader_price, &item.trader_price_cur);
    for (price, why) in &[
        (item.price, "Lowest"),
        (item.avg24h_price, "24h"),
        (item.avg7days_price, "7d "),
    ] {
        let price = *price;
        let flea_tax = get_flea_tax(trader_ruble_value, price);
        let rub = "₽";

        let slot_value = color_currency((price - flea_tax) / item.slots, rub);
        let slots = format_slots(slot_value, rub, item);

        println!(
            "{} Flea\t{}₽ - {}₽ = {}₽{slots}",
            why,
            color_currency(price, &rub),
            flea_tax,
            color_currency(price - flea_tax, &rub),
        );
    }
}

fn ruble_value(value: i64, cur_type: &str) -> i64 {
    match cur_type {
        "₽" => value,
        "$" => value * 142,
        "€" => value * 160,
        _ => unreachable!(),
    }
}

fn color_currency(value: i64, cur_type: &str) -> ColoredString {
    use num_format::{Locale, ToFormattedString};
    let value_str = format!("{}", value.to_formatted_string(&Locale::en));
    let rb_price = ruble_value(value, cur_type);

    match rb_price {
        x if x <= 2500 => value_str.white(),
        x if x <= 5000 => value_str.blue(),
        x if x <= 7500 => value_str.yellow(),
        x if x <= 10000 => value_str.cyan(),
        x if x <= 15000 => value_str.magenta(),
        x if x <= 25000 => value_str.green(),
        x if x <= 50000 => value_str.blue().on_red(),
        x if x <= 100000 => value_str.green().on_red(),
        x if x <= 200000 => value_str.yellow().on_red(),
        _ => value_str.white().on_red().underline(),
    }
}

fn print_color_table() {
    for x in [
        0, 1000, 2000, 3000, 5000, 7500, 10000, 15000, 25000, 50000, 75000, 100000, 125000, 150000,
        175000, 200000, 250000, 400000,
    ] {
        println!(
            "{} {}",
            color_currency(x, &"₽"),
            color_currency(x / 142, &"$")
        )
    }
}
