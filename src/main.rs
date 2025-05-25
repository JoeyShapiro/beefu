use std::io::{Read, Seek};

fn main() {
    let args: Vec<String> = std::env::args().collect();

    let mut delay = 0;
    let mut mem_size = 30_000;
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
        }

        i += 1;
    }

    let mut memory = vec![0_u8; mem_size];
    let mut pointer = 0;
    let mut stack = Vec::new();
    let mut pc = 0_usize;
    let rom = std::fs::read(&args[1]).expect("Unable to read file");

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
