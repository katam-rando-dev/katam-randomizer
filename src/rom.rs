use std::io::prelude::*;
use std::fs::File;

pub struct Rom {
    buffer: Vec<u8>
}

impl Rom {
    pub fn new(mut file: File) -> Rom {
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer).unwrap();
        Rom {
            buffer: buffer
        }
    }

    pub fn write_byte(&mut self, byte: u8, address: usize) {
        self.buffer[address] = byte;
    }

    pub fn write_bytes(&mut self, bytes: &[u8], address: usize) {
        for (index, byte) in bytes.iter().enumerate() {
            self.write_byte(*byte, address + index);
        }
    }

    pub fn create_randomized_rom(&self) {
        let mut rando_buffer = File::create("Randomized Kirby and the Amazing Mirror.gba").unwrap();
        rando_buffer.write_all(&self.buffer[..]).unwrap();
    }
}