use std::{thread, time::Duration};
use std::io::{self, Write};
use rand::Rng;
use rand::seq::SliceRandom;
use enigo::*;

type Range<T> = std::ops::Range<T>;

struct TypingConfig {
    base_delay: Range<u64>,
    thinking_delay: Range<u64>,
    mistake_probability: u32,
    correction_delay: Range<u64>,
    long_pause_probability: u32,
}

impl Default for TypingConfig {
    fn default() -> Self {
        TypingConfig {
            base_delay: 20..100,
            thinking_delay: 500..1500,
            mistake_probability: 10,
            correction_delay: 300..700,
            long_pause_probability: 5,
        }
    }
}

struct KeyboardLayout {
    nearby_keys: std::collections::HashMap<char, Vec<char>>,
}

impl KeyboardLayout {
    fn new() -> Self {
        let mut layout = std::collections::HashMap::new();
        layout.insert('a', vec!['s', 'q', 'w', 'z']);
        layout.insert('b', vec!['v', 'n', 'h', 'g']);
        layout.insert('c', vec!['x', 'v', 'd', 'f']);
        layout.insert('d', vec!['s', 'f', 'e', 'r']);
        layout.insert('e', vec!['w', 'r', 'd', 'f']);
        layout.insert('f', vec!['d', 'g', 'r', 't']);
        layout.insert('g', vec!['f', 'h', 't', 'y']);
        layout.insert('h', vec!['g', 'j', 'y', 'u']);
        layout.insert('i', vec!['u', 'o', 'k', 'l']);
        layout.insert('j', vec!['h', 'k', 'u', 'i']);
        layout.insert('k', vec!['j', 'l', 'i', 'o']);
        layout.insert('l', vec!['k', ';', 'o', 'p']);
        layout.insert('m', vec!['n', ',', 'j', 'k']);
        layout.insert('n', vec!['b', 'm', 'h', 'j']);
        layout.insert('o', vec!['i', 'p', 'k', 'l']);
        layout.insert('p', vec!['o', '[', 'l', ';']);
        layout.insert('q', vec!['w', 'a', '1', '2']);
        layout.insert('r', vec!['e', 't', 'd', 'f']);
        layout.insert('s', vec!['a', 'd', 'w', 'e']);
        layout.insert('t', vec!['r', 'y', 'f', 'g']);
        layout.insert('u', vec!['y', 'i', 'h', 'j']);
        layout.insert('v', vec!['c', 'b', 'f', 'g']);
        layout.insert('w', vec!['q', 'e', 'a', 's']);
        layout.insert('x', vec!['z', 'c', 's', 'd']);
        layout.insert('y', vec!['t', 'u', 'g', 'h']);
        layout.insert('z', vec!['a', 'x', 's', 'd']);

        KeyboardLayout { nearby_keys: layout }
    }

    fn get_nearby_key(&self, c: char) -> char {
        let c_lower = c.to_ascii_lowercase();
        if let Some(nearby) = self.nearby_keys.get(&c_lower) {
            let result = *nearby.choose(&mut rand::thread_rng()).unwrap_or(&c_lower);
            if c.is_uppercase() {
                result.to_ascii_uppercase()
            } else {
                result
            }
        } else {
            c
        }
    }
}

struct HumanTypist {
    config: TypingConfig,
    keyboard: KeyboardLayout,
    rng: rand::rngs::ThreadRng,
    enigo: Enigo,
    mistake_buffer: Vec<char>,
}

impl HumanTypist {
    fn new() -> Self {
        HumanTypist {
            config: TypingConfig::default(),
            keyboard: KeyboardLayout::new(),
            rng: rand::thread_rng(),
            enigo: Enigo::new(),
            mistake_buffer: Vec::new(),
        }
    }

    fn type_text(&mut self, text: &str) {
        let mut chars = text.chars().peekable();
        while let Some(c) = chars.next() {

            if self.rng.gen_ratio(1, 100) && c.is_whitespace() {
                thread::sleep(Duration::from_millis(
                    self.rng.gen_range(self.config.thinking_delay.clone()),
                ));
            }


            if self.rng.gen_ratio(1, 100) && ".,?!;:".contains(c) {
                thread::sleep(Duration::from_millis(
                    self.rng.gen_range(1000..3000),
                ));
            }


            self.type_character(c);


            thread::sleep(Duration::from_millis(
                self.rng.gen_range(self.config.base_delay.clone()),
            ));
        }


        self.correct_mistakes();
    }

    fn type_character(&mut self, intended_char: char) {
        if self.rng.gen_ratio(1, self.config.mistake_probability) {

            self.make_mistake(intended_char);
        } else {

            self.enigo.key_sequence(&intended_char.to_string());
        }
    }

    fn make_mistake(&mut self, intended_char: char) {

        let mistake_type = self.rng.gen_range(0..3);
        let mistake_char = match mistake_type {
            0 => self.keyboard.get_nearby_key(intended_char),
            1 => intended_char,
            2 => {

                if let Some(&next_char) = self.mistake_buffer.last() {
                    self.enigo.key_sequence(&next_char.to_string());
                    self.mistake_buffer.pop();
                }
                intended_char
            }
            _ => intended_char,
        };

        if mistake_type != 1 {

            self.enigo.key_sequence(&mistake_char.to_string());
        }


        self.mistake_buffer.push(intended_char);


        thread::sleep(Duration::from_millis(
            self.rng.gen_range(self.config.correction_delay.clone()),
        ));
        for _ in 0..self.mistake_buffer.len() {
            self.enigo.key_click(Key::Backspace);
        }

        for &c in &self.mistake_buffer {
            self.enigo.key_sequence(&c.to_string());
        }
        self.mistake_buffer.clear();
    }

    fn correct_mistakes(&mut self) {
        if !self.mistake_buffer.is_empty() {

            thread::sleep(Duration::from_millis(
                self.rng.gen_range(self.config.correction_delay.clone()),
            ));
            for _ in 0..self.mistake_buffer.len() {
                self.enigo.key_click(Key::Backspace);
            }
            for &c in &self.mistake_buffer {
                self.enigo.key_sequence(&c.to_string());
            }
            self.mistake_buffer.clear();
        }
    }
}

fn main() {
    print!("Enter the text you want to type: ");
    io::stdout().flush().unwrap();

    let mut input_text = String::new();
    io::stdin().read_line(&mut input_text).unwrap();
    let input_text = input_text.trim();

    print!("Enter the number of seconds to wait before starting: ");
    io::stdout().flush().unwrap();

    let mut delay_secs = String::new();
    io::stdin().read_line(&mut delay_secs).unwrap();
    let delay_secs: u64 = delay_secs.trim().parse().unwrap_or(5);

    println!("\nStarting in...");
    for i in (1..=delay_secs).rev() {
        println!("{i}...");
        thread::sleep(Duration::from_secs(1));
    }
    println!("Go!");

    let mut typist = HumanTypist::new();
    typist.type_text(input_text);
}