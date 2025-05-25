use std::io::Read;

fn main() {
    println!("Hello, world!");

    let args: Vec<String> = std::env::args().collect();

    // open file
    let file = std::fs::File::open(&args[1]).expect("Unable to open file");
    // read file
    let mut reader = std::io::BufReader::new(file);

    let mut memory = vec![0_u8; 30_000];
    let mut pointer = 0;

    // read file byte by byte
    let mut buffer = [0; 1];
    loop {
        let n = reader.read(&mut buffer).expect("Unable to read file");
        if n == 0 {
            break;
        }
        
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
            '+' => {
                memory[pointer] += 1;
            }
            '-' => {
                memory[pointer] -= 1;
            }
            '[' => todo!("Implement loop start"),
            ']' => todo!("Implement loop end"),
            ',' => todo!("Implement input"),
            '.' => {
                print!("{}", memory[pointer] as char);
            }
            _ => {}
        }
    }
}
