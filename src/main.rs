use std::io::Read;

use std::sync::Arc;
use std::thread::sleep;
use std::time::Duration;

use winit::application::ApplicationHandler;
use winit::dpi::PhysicalSize;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::platform::pump_events::{EventLoopExtPumpEvents, PumpStatus};
use winit::window::{Window, WindowId};

#[derive(Default, Debug)]
struct AppState<'a> {
    window: Option<Arc<Window>>,
    pixels: Option<pixels::Pixels<'a>>,
    size: PhysicalSize<u32>,
    font: Option<fontdue::Font>,
    rom: Vec<u8>,
    memory: Vec<u8>,
    pointer: usize,
    stack: Vec<usize>,
    pc: usize,
}

impl<'a> ApplicationHandler for AppState<'a> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window_attributes = Window::default_attributes().with_title("A fantastic window!");
        let window = Arc::new(event_loop.create_window(window_attributes).unwrap());

        let window_size = window.as_ref().outer_size();
        let surface_texture = pixels::SurfaceTexture::new(
            window_size.width, 
            window_size.height, 
            window.clone()
        );
        
        // 1280 x 720
        self.size = PhysicalSize::new(window.outer_size().width, window.outer_size().height);
        self.pixels = Some(pixels::Pixels::new(self.size.width, self.size.height, surface_texture)
                .expect("Failed to create pixels"));
        self.window = Some(window);
        self.window.as_ref().unwrap().request_redraw();
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        let window = match self.window.as_ref() {
            Some(window) => window,
            None => return,
        };

        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::RedrawRequested => {
                if let Some(pixels) = &mut self.pixels {
                    let offset = self.size.width / 2;
                    clear_background(pixels.frame_mut(), [80, 80, 80, 255]);

                    for i in 0..9 {
                        draw_rect(self.size, pixels.frame_mut(), 96 + i * 66, 200, 64, 64, [0, 255, 0, 255]);
                    }
                    for i in 0..9 {
                        draw_rect(self.size, pixels.frame_mut(), 96 + i * 66, 400, 64, 64, [0, 255, 0, 255]);
                    }

                    let font = self.font.as_ref().unwrap();
                    let (metrics, bitmap) = font.rasterize('0', 32.0);
                    for i in 0..16 {
                        for j in 0..32 {
                            draw_rect(self.size, pixels.frame_mut(), offset+i*38, j*38, 36, 36, [255, 0, 0, 255]);
                            for (k, b) in bitmap.iter().enumerate() {
                                if *b < 64 {
                                    continue;
                                }
                                let x = k as u32 % metrics.width as u32 + offset + i * 38;
                                let y = k as u32 / metrics.width as u32 + j * 38;
                                if x < self.size.width && y < self.size.height {
                                    let pixel_index = (y * self.size.width + x) as usize * 4;
                                    if pixel_index < pixels.frame().len() {
                                        pixels.frame_mut()[pixel_index]     = 255; // R
                                        pixels.frame_mut()[pixel_index + 1] = 255; // G
                                        pixels.frame_mut()[pixel_index + 2] = 255; // B
                                        pixels.frame_mut()[pixel_index + 3] = 255; // A
                                    }
                                }
                            }
                        }
                    }

                    pixels.render().unwrap();
                }
                window.request_redraw();
            },
            _ => (),
        }
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();

    let mut delay = 0;
    let mut mem_size = 30_000;
    let mut file = "";
    let mut i = 0;
    while i < args.len() {
        if args[i] == "--ops" {
            let ops = args[i + 1].parse::<f32>().expect("Unable to parse ops");
            if ops > 0.0 {
                delay = (1.0 / ops * 1000.0) as u64;
            }
            i += 1;
        } else if args[i] == "--mem" {
            mem_size = args[i+1].parse::<usize>().expect("Unable to parse mem");
            if mem_size < 1 {
                panic!("Memory size must be greater than 0");
            }
            i += 1;
        } else {
            file = &args[i];
        }

        i += 1;
    }

    if file.is_empty() {
        panic!("file must be specified")
    }

    let mut event_loop = EventLoop::new().unwrap();
    let app = Arc::new(std::sync::Mutex::new(AppState::default()));

    {
        app.lock().unwrap().rom = std::fs::read(file).expect("Unable to read file");

        // Read the font data.
        let font = include_bytes!("../res/JetBrainsMono-Medium.ttf") as &[u8];
        // Parse it into the font type.
        app.lock().unwrap().font = Some(fontdue::Font::from_bytes(font, fontdue::FontSettings::default()).unwrap());
    }

    loop {
        let timeout = Some(Duration::ZERO);
        let status = event_loop.pump_app_events(timeout, &mut *app.lock().unwrap());

        if let PumpStatus::Exit(_code) = status {
            println!("Exiting application");
            break;
        }

        // Sleep for 1/60 second to simulate application work
        //
        // Since `pump_events` doesn't block it will be important to
        // throttle the loop in the app somehow.
        sleep(Duration::from_millis(16));
    }
    return;

    let mut memory = vec![0_u8; mem_size];
    let mut pointer = 0;
    let mut stack = Vec::new();
    let mut pc = 0_usize;
    let rom = std::fs::read(file).expect("Unable to read file");

    loop {
        if pc >= rom.len() {
            break;
        }

        match rom[pc] as char {
            '>' => {
                pointer += 1;
                if pointer >= memory.len() {
                    panic!("Pointer out of bounds");
                }
            }
            '<' => {
                if pointer == 0 {
                    panic!("Pointer out of bounds");
                }
                pointer -= 1;
            }
            '+' => memory[pointer] += 1,
            '-' => memory[pointer] -= 1,
            '[' => stack.push(pc),
            ']' => {
                if stack.is_empty() {
                    panic!("Unmatched closing bracket");
                }
                if memory[pointer] != 0 {
                    let start = stack.pop().unwrap();
                    pc = start-1;
                } else {
                    stack.pop();
                }
            },
            ',' => {
                // reading line is not correct, what if they want to insert \n
                // has many problems, but cant handle in simple way
                // read from stdin with newline
                // thought there was a getch, but maybe not
                // doesnt matter though, i will have ui anyway
                let mut input = [0; 2];
                std::io::stdin().read(&mut input).expect("Unable to read input");
                memory[pointer] = input[0];
            },
            '.' => {
                print!("{}", memory[pointer] as char);
                std::io::Write::flush(&mut std::io::stdout()).unwrap();
            },
            _ => {}
        }

        pc += 1;
        if delay > 0 {
            std::thread::sleep(std::time::Duration::from_millis(delay));
        }
    }
}

fn clear_background(frame: &mut [u8], color: [u8; 4]) {
    // Clear screen to black
    for pixel in frame.chunks_exact_mut(4) {
        pixel[0] = color[0]; // R
        pixel[1] = color[1]; // G  
        pixel[2] = color[2]; // B
        pixel[3] = color[3]; // A
    }
}

fn draw_rect(size: PhysicalSize<u32>, frame: &mut [u8], x: u32, y: u32, width: u32, height: u32, color: [u8; 4]) {
    for j in 0..height {
        for i in 0..width {
            // u32 is big enough (4k*4k*4) = 64MB, so we use u32
            let pixel_index = ((y + j) * (size.width * 4) + (x + i) * 4) as usize;
            if pixel_index < frame.len() {
                frame[pixel_index] = color[0];     // R
                frame[pixel_index + 1] = color[1]; // G
                frame[pixel_index + 2] = color[2]; // B
                frame[pixel_index + 3] = color[3]; // A
            }
        }
    }
}
