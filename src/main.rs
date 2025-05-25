use std::io::{Read, Seek};

fn main() {
    let args: Vec<String> = std::env::args().collect();

    // open file
    let file = std::fs::File::open(&args[1]).expect("Unable to open file");
    // read file
    let mut reader = std::io::BufReader::new(file);

    let mut memory = vec![0_u8; 30_000];
    let mut pointer = 0;
    let mut stack = Vec::new();
    let mut pc = 0_u64;

    // read file byte by byte
    let mut buffer = [0; 1];
    loop {
        let n = reader.read(&mut buffer).expect("Unable to read file");
        if n == 0 {
            break;
        }
        
        // print!("{:?}", buffer[0] as char);
        match buffer[0] as char {
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
                    reader.seek(std::io::SeekFrom::Start(start)).expect("Unable to seek");
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
    }
}
