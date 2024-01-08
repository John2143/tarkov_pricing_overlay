use std::error::Error;

use closestmatch::ClosestMatch;
use ocrs::{OcrEngine, OcrEngineParams};
use once_cell::{self, sync::Lazy};
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
    BadOCRJson,
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
    let engine_params = OcrEngineParams {
        ..Default::default()
    };
    let engine = OcrEngine::new(engine_params)?;
    // Apply standard image pre-processing expected by this library (convert
    // to greyscale, map range to [-0.5, 0.5]).
    let ocr_input = engine.prepare_input(image_tensor.view())?;

    // Phase 1: Detect text words
    let word_rects = engine.detect_words(&ocr_input)?;

    // Phase 2: Perform layout analysis
    let line_rects = engine.find_text_lines(&ocr_input, &word_rects);

    // Phase 3: Recognize text
    let line_texts = engine.recognize_text(&ocr_input, &line_rects)?;
    let mut valid_text = vec![];

    for line in line_texts
        .iter()
        .flatten()
        // Filter likely spurious detections. With future model improvements
        // this should become unnecessary.
        .filter(|l| l.to_string().len() > 1)
    {
        println!("{}", line);
        valid_text.push(line.to_string());
    }
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
        .get("https://tarkov-market.com/api/v1/item")
        .query(&[("q", &text)])
        .header("x-api-key", &*MARKET_API_KEY)
        .send()
        .map_err(|_| AnalyzeError::BadRequest("Something went wrong with the tarkov market api"))?;

    //dbg!(d.text().unwrap());

    let js: Vec<apis::market::Root> = d.json().map_err(|_| AnalyzeError::BadMarketJson)?;

    for j in js {
        struct S(i64);
        impl std::fmt::Display for S {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
                use num_format::{Locale, ToFormattedString};
                write!(f, "{}", self.0.to_formatted_string(&Locale::en))?;
                Ok(())
            }
        }

        println!("Name: {} --- {}", j.short_name, j.name);
        println!(
            "Trader Price: {} -> {}{} ({}{}/slot)",
            j.trader_name,
            S(j.trader_price),
            j.trader_price_cur,
            S(j.trader_price / j.slots),
            j.trader_price_cur
        );

        let rb_price = if j.trader_price_cur == "₽" {
            j.trader_price
        } else {
            j.trader_price * 126
        };

        for (price, why) in &[
            (j.price, "Lowest"),
            (j.avg24h_price, "24h"),
            (j.avg7d_price, "7d "),
        ] {
            let price = *price;
            let ft = get_flea_tax(rb_price, price);
            println!(
                "{} Flea\t{}₽ - {}₽ = {}₽ ({}₽/slot)",
                why,
                S(price),
                S(ft),
                S(price - ft),
                S((price - ft) / j.slots)
            );
        }
    }

    Ok(())
}
