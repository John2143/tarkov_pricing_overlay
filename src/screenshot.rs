pub struct ScreenshotData {
    height: usize,
    width: usize,
    pixels: Vec<u8>,
}

pub fn take_screenshot() -> Result<ScreenshotData, ()> {
    let disp = scrap::Display::primary().unwrap();
    let mut cap = scrap::Capturer::new(disp).unwrap();
    let width = cap.width();
    let height = cap.height();

    let sleep = 50;

    //max 2 seconds before fail
    let maxloops = 2000 / sleep;

    for _ in 0..maxloops {
        match cap.frame() {
            Ok(fr) => {
                return Ok(ScreenshotData {
                    height,
                    width,
                    pixels: fr.to_vec(),
                })
            }
            Err(_) => {}
        }
        std::thread::sleep(std::time::Duration::from_millis(sleep));
    }

    Err(())
}

use image;

impl ScreenshotData {
    //return RGBA8888 pixel as u32
    pub fn get_pixel(&self, x: usize, y: usize) -> u32 {
        assert!(x < self.width);
        assert!(y < self.height);

        let pos: usize = y * self.width + x;
        let pos = pos * 4; //pixel format ARGB8888;

        //TODO find the rust idiomatic way to do this
        unsafe {
            std::mem::transmute([
                self.pixels[pos + 3],
                self.pixels[pos + 2],
                self.pixels[pos + 1],
                self.pixels[pos],
            ])
        }
    }

    pub fn to_image(self) -> Option<image::RgbImage> {
        let mut img = image::RgbImage::new(self.width as u32, self.height as u32);

        for x in 0..self.width {
            for y in 0 ..self.height {
                let p = self.get_pixel(x, y);
                img.put_pixel(x as u32, y as u32, image::Rgb([
                    (p >> 8 & 0xff) as u8, 
                    (p >> 16 & 0xff) as u8,
                    (p >> 24 & 0xff) as u8,
                ]));
            }
        }

        Some(img)
    }
}

use libc::{c_int, c_long};

#[repr(C)]
struct CCursorPos {
    x: c_long,
    y: c_long,
}

#[link(name="user32")]
extern "system" {
    fn GetCursorPos(lpPoint: &mut CCursorPos) -> c_int;
}

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub struct CursorPos {
    pub x: u32,
    pub y: u32,
}

impl CursorPos {
    pub fn get() -> Self {
        let mut ccp = CCursorPos {
            x: 0, y: 0,
        };

        let ok = unsafe { GetCursorPos(&mut ccp) };

        if ok == 0 {
            panic!("Something stopped us getting the cursor pos");
        }

        Self {
            x: ccp.x as u32,
            y: ccp.y as u32,
        }
    }
}
