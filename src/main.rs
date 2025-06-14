use std::io::Read;

use std::sync::Arc;
use std::time::Duration;

use winit::application::ApplicationHandler;
use winit::dpi::PhysicalSize;
use winit::event::{ElementState, KeyEvent, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::keyboard::{Key, NamedKey};
use winit::platform::pump_events::{EventLoopExtPumpEvents, PumpStatus};
use winit::window::{Window, WindowId};

const MEM_BLOCK: u32 = 50;
const THEME_BACKGROUND: [u8; 4] = [40, 42, 54, 255];    // base background color
const THEME_FOREGROUND: [u8; 4] = [248, 248, 242, 255]; // foreground color
const THEME_SELECTION:  [u8; 4] = [68, 71, 90, 255];    // selection color
const THEME_COMMENT:    [u8; 4] = [98, 114, 164, 255];  // comment color. odd using comment for current, but looks better

#[derive(Debug)]
enum Layout {
    Number,
    Ascii,
    Color,
}

impl Default for Layout {
    fn default() -> Self {
        Layout::Number
    }
}

impl Layout {
    fn next(&mut self) {
        *self = match self {
            Layout::Number => Layout::Ascii,
            Layout::Ascii => Layout::Color,
            Layout::Color => Layout::Number,
        };
    }
}

#[derive(Default, Debug)]
struct AppState<'a> {
    title: Option<String>,
    window: Option<Arc<Window>>,
    pixels: Option<pixels::Pixels<'a>>,
    size: PhysicalSize<u32>,
    renderer: Option<Renderer>,
    rom: Vec<u8>,
    ram: Vec<u8>,
    pointer: usize,
    stack: Vec<usize>,
    pc: usize,
    step: bool,
    layout: Layout,
    scroll: u32,
}

impl AppState<'_> {
    fn update(&mut self) {
        let window = match self.window.as_ref() {
            Some(window) => window,
            None => return,
        };

        window.request_redraw();
    }
}

impl<'a> ApplicationHandler for AppState<'a> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let title = self.title.clone().unwrap_or("Beefu".to_string());
        let window_attributes = Window::default_attributes().with_title(title);
        let window = Arc::new(event_loop.create_window(window_attributes).unwrap());

        let window_size = window.as_ref().inner_size();
        let surface_texture = pixels::SurfaceTexture::new(
            window_size.width, 
            window_size.height, 
            window.clone()
        );
        
        // 1280 x 720
        self.size = PhysicalSize::new(window.inner_size().width, window.inner_size().height);
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
            WindowEvent::Resized(size) => {
                // TODO likely wont work on hyprland
                // wanted to make it on there, but on here
                // wouldve been better, but oh well, next time
                self.size = size;
                if let Some(pixels) = &mut self.pixels {
                    pixels.resize_surface(size.width, size.height).unwrap();
                }
                window.request_redraw();
            },
            WindowEvent::KeyboardInput {
                event: KeyEvent { logical_key: key, state: ElementState::Pressed, .. },
                ..
            } => match key.as_ref() {
                Key::Named(NamedKey::Space) => self.step = true,
                Key::Character("l") => {
                    self.layout.next();
                    self.update();
                }
                Key::Named(NamedKey::ArrowUp) => {
                    if self.scroll > 0 {
                        self.scroll -= 1;
                    }
                    self.update();
                }
                Key::Named(NamedKey::ArrowDown) => {
                    if self.scroll < self.ram.len() as u32 / 16 {
                        self.scroll += 1;
                    }
                    self.update();
                }
                _ => (),
            },
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::RedrawRequested => {
                if let Some(pixels) = &mut self.pixels {
                    let offset = self.size.width / 2;
                    clear_background(pixels.frame_mut(), THEME_BACKGROUND);
                    let renderer = self.renderer.as_mut().unwrap();

                    // print the memory nearby
                    for i in 0..9 {
                        if (self.pointer + i) as i32 - 4 < 0 || self.pointer+i >= 30_000 {
                            continue;
                        }

                        let pointer = self.pointer + i - 4;
                        let x = (96 + i * 66) as u32;
                        let color = if pointer == self.pointer {
                            THEME_COMMENT
                        } else {
                            THEME_SELECTION
                        };
                        draw_rect(self.size, pixels.frame_mut(), x, 200, 64, 64, color);
                        renderer.draw_number(
                            self.size,
                            pixels.frame_mut(), 
                            self.ram[pointer],
                            x, 
                            200,
                            THEME_FOREGROUND
                        );
                    }

                    // print the rom data
                    for i in 0..9 {
                        if (self.pc + i) as i32 - 4 < 0 || self.pc+i >= self.rom.len() {
                            continue;
                        }

                        let pc = self.pc + i - 4;
                        let x = (96 + i * 66) as u32;
                        let color = if pc == self.pc {
                            THEME_COMMENT
                        } else {
                            THEME_SELECTION
                        };
                        draw_rect(self.size, pixels.frame_mut(), x, 400, 64, 64, color);
                        renderer.draw_char(self.size, pixels.frame_mut(), self.rom[pc as usize] as char, x, 400, THEME_FOREGROUND);
                        renderer.draw_number(self.size, pixels.frame_mut(), pc as u8, x, 440, THEME_FOREGROUND);
                    }

                    let js = self.size.height / MEM_BLOCK - 1;
                    for j in 0..js {
                        let y = j as u32 * MEM_BLOCK;
                        renderer.draw_number(self.size, pixels.frame_mut(), (j+self.scroll) as u8, offset-MEM_BLOCK, y, THEME_FOREGROUND);
                    }

                    for i in 0..16 {
                        for j in 0..js {
                            let pointer = (i + (j+self.scroll) * 16) as usize;
                            if pointer >= self.ram.len() {
                                continue;
                            }

                            let x = offset + i as u32 * MEM_BLOCK;
                            let y = j as u32 * MEM_BLOCK;
                            // handles underflow. usize.saturating_sub(4)
                            let color = if pointer+4 >= self.pointer && pointer <= self.pointer+4 {
                                THEME_COMMENT
                            } else {
                                THEME_SELECTION
                            };
                            draw_rect(self.size, pixels.frame_mut(), x, y, MEM_BLOCK-2, MEM_BLOCK-2, color);
                            match self.layout {
                                Layout::Number => {
                                    renderer.draw_number(
                                        self.size,
                                        pixels.frame_mut(), 
                                        self.ram[pointer],
                                        x + 2,
                                        y + 2,
                                        THEME_FOREGROUND
                                    );
                                },
                                Layout::Ascii => {
                                    let c = self.ram[pointer] as char;
                                    if c.is_ascii() && !c.is_control() {
                                        renderer.draw_char(
                                            self.size,
                                            pixels.frame_mut(), 
                                            c, 
                                            x + 2, 
                                            y + 2,
                                            THEME_FOREGROUND
                                        );
                                    } else {
                                        renderer.draw_number(
                                            self.size,
                                            pixels.frame_mut(), 
                                            self.ram[pointer],
                                            x + 2, 
                                            y + 2,
                                            THEME_BACKGROUND
                                        );
                                    }
                                },
                                Layout::Color => {
                                    let color = [
                                        self.ram[pointer],
                                        self.ram[pointer],
                                        self.ram[pointer],
                                        255,
                                    ];
                                    draw_rect(self.size, pixels.frame_mut(), x + 4, y + 4, MEM_BLOCK-8, MEM_BLOCK-4, color);
                                },
                            }
                        }
                    }

                    pixels.render().unwrap();
                }
            },
            _ => (),
        }
    }
}

#[derive(Debug)]
struct Renderer {
    font: fontdue::Font,
    characters: std::collections::HashMap<char, (fontdue::Metrics, Vec<u8>)>,
}

impl Renderer {
    fn new(font: fontdue::Font) -> Self {
        let characters = std::collections::HashMap::new();

        Self { font, characters }
    }

    fn get_char(&mut self, c: char) -> (fontdue::Metrics, Vec<u8>) {
        if let Some((metrics, bitmap)) = self.characters.get(&c) {
            return (metrics.clone(), bitmap.clone());
        }

        let (metrics, bitmap) = self.font.rasterize(c, 32.0);
        self.characters.insert(c, (metrics.clone(), bitmap.clone()));
        (metrics, bitmap)
    }

    fn draw_char(&mut self, size: PhysicalSize<u32>, frame: &mut [u8], c: char, i: u32, j: u32, color: [u8; 4]) {
        let (metrics, bitmap) = self.get_char(c);

        for (k, b) in bitmap.iter().enumerate() {
            if *b < 64 {
                continue;
            }
            let x = k as u32 % metrics.width as u32 + i;
            let y = k as u32 / metrics.width as u32 + j;
            if x < size.width && y < size.height {
                let pixel_index = (y * size.width + x) as usize * 4;
                if pixel_index < frame.len() {
                    frame[pixel_index]     = color[0]; // R
                    frame[pixel_index + 1] = color[1]; // G
                    frame[pixel_index + 2] = color[2]; // B
                    frame[pixel_index + 3] = color[3]; // A
                }
            }
        }
    }

    fn draw_number(&mut self, size: PhysicalSize<u32>, frame: &mut [u8], n: u8, x: u32, y: u32, color: [u8; 4]) {
        // assumes mono font
        let (metrics, _) = self.get_char('0');
        let w = metrics.width as u32;

        // could use n % 10; n /= 10
        for (i, c) in n.to_string().chars().enumerate() {
            self.draw_char(size, frame, c, x + i as u32 * w, y, color);
        }
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();

    let mut delay = 0;
    let mut mem_size = 30_000;
    let mut file = "";
    let mut i = 1;
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
        let file_name = file.split('/').last().unwrap();
        app.lock().unwrap().title = Some(format!("Beefu - {} ({}B {}ms)", file_name, mem_size, delay));

        app.lock().unwrap().rom = std::fs::read(file).expect("Unable to read file");

        // Read the font data.
        let data = include_bytes!("../res/JetBrainsMono-Medium.ttf") as &[u8];
        // Parse it into the font type.
        let font = fontdue::Font::from_bytes(data, fontdue::FontSettings::default()).unwrap();
        app.lock().unwrap().renderer = Some(Renderer::new(font));

        app.lock().unwrap().ram = vec![0_u8; mem_size];
    }

    loop {
        let timeout = Some(Duration::ZERO);
        let status = event_loop.pump_app_events(timeout, &mut *app.lock().unwrap());

        if let PumpStatus::Exit(_code) = status {
            println!("Exiting application");
            break;
        }

        // vm loop
        if app.lock().unwrap().step {
            app.lock().unwrap().step = false;
            let mut app = app.lock().unwrap();
            let mut pointer = app.pointer;
            let mut pc = app.pc;
            
            if pc >= app.rom.len() {
                break;
            }
    
            match app.rom[pc] as char {
                '>' => {
                    pointer += 1;
                    if pointer >= app.ram.len() {
                        panic!("Pointer out of bounds");
                    }
                }
                '<' => {
                    if pointer == 0 {
                        panic!("Pointer out of bounds");
                    }
                    pointer -= 1;
                }
                '+' => app.ram[pointer] += 1,
                '-' => app.ram[pointer] -= 1,
                '[' => app.stack.push(pc),
                ']' => {
                    if app.stack.is_empty() {
                        panic!("Unmatched closing bracket");
                    }
                    if app.ram[pointer] != 0 {
                        let start = app.stack.pop().unwrap();
                        pc = start - 1;
                    } else {
                        app.stack.pop();
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
                    app.ram[pointer] = input[0];
                },
                '.' => {
                    print!("{}", app.ram[pointer] as char);
                    std::io::Write::flush(&mut std::io::stdout()).unwrap();
                },
                _ => {}
            }
    
            pc += 1;
            app.pc = pc;
            app.pointer = pointer;

            app.update();
            // good enough
            if delay > 0 {
                std::thread::sleep(std::time::Duration::from_millis(delay));
                app.step = true;
            }
        }

        std::thread::sleep(std::time::Duration::from_millis(16));
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
