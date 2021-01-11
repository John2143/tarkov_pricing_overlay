use inputbot;

mod screenshot;
mod closestmatch;

use screenshot::{ScreenshotData, CursorPos};
use once_cell::{self, sync::Lazy};
use closestmatch::ClosestMatch;

static OCR_API_KEY: Lazy<String> = Lazy::new(|| {
    include_str!("../ocr_api_key.txt").trim().to_owned()
});

static MARKET_API_KEY: Lazy<String> = Lazy::new(|| {
    include_str!("../market_api_key.txt").trim().to_owned()
});

static WORDS: Lazy<ClosestMatch> = Lazy::new(|| {
    let titles = include_str!("../wiki_titles.txt");
    ClosestMatch::new(titles.lines().map(|x| x.to_owned()).collect(), vec![1,2,3,4,5])
});

fn main() {
    inputbot::KeybdKey::TKey.bind(|| {
        match analyze_pressed() {
            Ok(_) => println!("was ok"),
            Err(e) => {
                println!("{:?}", e);
            },
        };
    });

    println!("{}", WORDS.get_closest("water ootle wit filter Aquamari").unwrap());
    println!("{}", *MARKET_API_KEY);
    println!("{}", *OCR_API_KEY);

    println!("Bot ready");

    inputbot::handle_input_events();
}

#[derive(Debug)]
enum AnalyzeError {
    ScreenshotFailed,
    BadMousePosition,
    BadRequest,
    BadJson,
    NoCloseWord,
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

pub mod apis;

fn get_flea_tax(value_to_traders: i64, list_price: i64) -> i64 {
    let v_o = list_price as f64;
    let v_r = value_to_traders as f64;
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

    let tl_corner = find_top_left_corner(&screen, &mouse_location).ok_or(AnalyzeError::BadMousePosition)?;

    let h = 30;
    let w = 500;
    let offset = 20;

    let i = screen.to_image().unwrap();
    let subimage = image::SubImage::new(&i, tl_corner.0 + offset, tl_corner.1, w, h);
    let mut png_buffer = vec![];
    let enc = image::codecs::png::PngEncoder::new(&mut png_buffer);
    enc.encode(&subimage.to_image().as_raw(), w, h, image::ColorType::Rgb8).unwrap();

    let client = reqwest::blocking::Client::new();
    let form = reqwest::blocking::multipart::Form::new()
        .text("base64Image", format!("data:image/png;base64,{}", base64::encode(png_buffer)))
        .text("language", "eng")
        .text("OCREngine", "1");

    println!("Screenshot taken, parsing image as text...");

    let d = client
        .post("https://apipro1.ocr.space/parse/image")
        .header("apikey", &*OCR_API_KEY)
        .multipart(form)
        .send()
        .map_err(|e| {
            println!("is bad {}", e);
            AnalyzeError::BadRequest
        })?;

    let t = d.text().unwrap();

    let j: apis::ocr::Root = serde_json::from_str(&t)
        .map_err(|e| {
            println!("is bad {}", e);
            println!("{}", &t);
            AnalyzeError::BadJson
        })?;

    let text_ocr = &j.parsed_results[0].parsed_text.trim();

    let text = WORDS.get_closest(&text_ocr).ok_or(AnalyzeError::NoCloseWord)?;

    println!("Detected text was '{}'. Closest was '{}'. Reading market data... ", &text_ocr, &text);

    let d = client
        .get("https://tarkov-market.com/api/v1/item")
        .query(&[("q", &text)])
        .header("x-api-key", &*MARKET_API_KEY)
        .send()
        .map_err(|e| {
            println!("is bad {}", e);
            AnalyzeError::BadRequest
        })?;

    //dbg!(d.text().unwrap());

    let js: Vec<apis::market::Root> = d.json()
        .map_err(|e| {
            println!("is bad {}", e);
            AnalyzeError::BadJson
        })?;

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
        println!("Trader Price: {} -> {}{} ({}{}/slot)", j.trader_name, S(j.trader_price), j.trader_price_cur, S(j.trader_price / j.slots), j.trader_price_cur);
        let rb_price = if j.trader_price_cur == "₽" {
            j.trader_price
        }else{
            j.trader_price * 126
        };

        for (price, why) in &[(j.price, "Lowest"), (j.avg24h_price, "24h"), (j.avg7d_price, "7d ")] {
            let price = *price;
            let ft = get_flea_tax(rb_price, price);
            println!("{} Flea\t{}₽ - {}₽ = {}₽ ({}₽/slot)", why, S(price), S(ft), S(price - ft), S((price - ft) / j.slots));
        }
    }

    Ok(())
}
