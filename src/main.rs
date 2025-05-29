use std::io::Read;

use std::sync::Arc;
use std::thread::sleep;
use std::time::Duration;

use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::platform::pump_events::{EventLoopExtPumpEvents, PumpStatus};
use winit::window::{Window, WindowId};

#[derive(Default, Debug)]
struct AppState<'a> {
    window: Option<Arc<Window>>,
    pixels: Option<pixels::Pixels<'a>>,
}

impl<'a> ApplicationHandler for AppState<'a> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window_attributes = Window::default_attributes().with_title("A fantastic window!");
        let window = Arc::new(event_loop.create_window(window_attributes).unwrap());

        let window_size = window.as_ref().inner_size();
        let surface_texture = pixels::SurfaceTexture::new(
            window_size.width, 
            window_size.height, 
            window.clone()
        );
        
        self.pixels = Some(pixels::Pixels::new(1280, 720, surface_texture)
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
                    // Use pixels here for rendering
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
    let mut app = Arc::new(AppState::default());

    loop {
        let timeout = Some(Duration::ZERO);
        let status = event_loop.pump_app_events(timeout, &mut Arc::get_mut(&mut app)
            .expect("Failed to get mutable reference"));

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

fn draw(frame: &mut [u8]) {
    // Clear screen to black
    for pixel in frame.chunks_exact_mut(4) {
        pixel[0] = 0x00; // R
        pixel[1] = 0x00; // G  
        pixel[2] = 0x00; // B
        pixel[3] = 0xff; // A
    }
    
    // Draw a simple pattern
    for (i, pixel) in frame.chunks_exact_mut(4).enumerate() {
        let x = (i % 1280 as usize) as i16;
        let y = (i / 1280 as usize) as i16;
        
        // Draw a red square in the middle
        if x > 350 && x < 450 && y > 250 && y < 350 {
            pixel[0] = 0xff; // R
            pixel[1] = 0x00; // G
            pixel[2] = 0x00; // B
            pixel[3] = 0xff; // A
        }
    }
}
