use std::{error::Error, fs};

use closestmatch::ClosestMatch;
use colored::{Colorize, ColoredString};
use ocrs::{OcrEngine, OcrEngineParams};
use once_cell::{self, sync::Lazy};
use rten::Model;
use rten_tensor::NdTensorView;
use screenshot::{CursorPos, ScreenshotData};

use inputbot;

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

fn main() {
    inputbot::KeybdKey::TKey.bind(|| {
        match analyze_pressed() {
            Ok(_) => println!("was ok"),
            Err(e) => {
                println!("{:?}", e);
            }
        };
    });

    //println!("{}", WORDS.get_closest("water ootle wit filter Aquamari").unwrap());
    //println!("{}", *MARKET_API_KEY);

    println!("Bot ready");

    //create_window();

    let t = std::thread::spawn(|| inputbot::handle_input_events());

    t.join().unwrap();
}

#[derive(Debug)]
enum AnalyzeError {
    ScreenshotFailed,
    CannotFindInspectBox,
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
    let detection_model_data = fs::read("text-detection.rten").expect("Could not find text-detection.rten");
    let rec_model_data = fs::read("text-recognition.rten").expect("Could not find text-recognition.rten");

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

    //ocrs::OcrEngine::prepare_input(&self, image)?;

    //let t = d.text().unwrap();

    //let j: apis::ocr::Root = serde_json::from_str(&t).map_err(|_| AnalyzeError::BadOCRJson)?;

    //let text_ocr = &j.parsed_results[0].parsed_text.trim();
    let text_ocr = &valid_text[0];

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
        dbg!(text);
        dbg!(e);
        AnalyzeError::BadMarketJson
    })?;

    for item in items_to_price {
        struct S(i64);
        impl std::fmt::Display for S {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
                use num_format::{Locale, ToFormattedString};
                write!(f, "{}", self.0.to_formatted_string(&Locale::en))?;
                Ok(())
            }
        }

        println!("Name: {} --- {}", item.short_name.red(), item.name.red());
        println!(
            "Trader Price: {} -> {} ({}/slot)",
            item.trader_name,
            color_currency(item.trader_price, &item.trader_price_cur),
            color_currency(item.trader_price / item.slots, &item.trader_price_cur),
        );

        let rb_price = if item.trader_price_cur == "₽" {
            item.trader_price
        } else {
            item.trader_price * 126
        };

        for (price, why) in &[
            (item.price, "Lowest"),
            (item.avg24h_price, "24h"),
            (item.avg7days_price, "7d "),
        ] {
            let price = *price;
            let flea_tax = get_flea_tax(rb_price, price);
            let rub = "₽";
            println!(
                "{} Flea\t{}₽ - {}₽ = {}₽ ({}₽/slot)",
                why,
                color_currency(price, &rub),
                color_currency(flea_tax, &rub),
                color_currency(price - flea_tax, &rub),
                color_currency((price - flea_tax) / item.slots, &rub)
            );
        }
    }

    Ok(())
}

fn color_currency(value: i64, cur_type: &str) -> ColoredString {
    let value_str = format!("{value}{cur_type}");
    match cur_type {
        "₽" => value_str.bright_blue(),
        "$" => value_str.bright_cyan(),
        _ => value_str.magenta(),
    }
}
