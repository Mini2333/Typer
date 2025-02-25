use std::{thread, time::Duration};
use std::io::{self, Write};
use std::fs;
use std::path::PathBuf;
use rand::Rng;
use rand::seq::SliceRandom;
use enigo::*;
use serde::{Serialize, Deserialize};

type Range<T> = std::ops::Range<T>;

struct TypingConfig {
    base_delay: Range<u64>,
    thinking_delay: Range<u64>,
    mistake_probability: u32,
    correction_delay: Range<u64>,
    long_pause_probability: u32,
    long_pause_delay: Range<u64>,
}

impl Default for TypingConfig {
    fn default() -> Self {
        TypingConfig {
            base_delay: 20..100,
            thinking_delay: 500..1500,
            mistake_probability: 10,
            correction_delay: 300..700,
            long_pause_probability: 5,
            long_pause_delay: 1000..3000,
        }
    }
}

#[derive(Serialize, Deserialize)]
struct Config {
    base_delay_min: u64,
    base_delay_max: u64,
    thinking_delay_min: u64,
    thinking_delay_max: u64,
    mistake_probability: u32,
    correction_delay_min: u64,
    correction_delay_max: u64,
    long_pause_probability: u32,
    long_pause_delay_min: u64,
    long_pause_delay_max: u64,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            base_delay_min: 20,
            base_delay_max: 100,
            thinking_delay_min: 500,
            thinking_delay_max: 1500,
            mistake_probability: 10,
            correction_delay_min: 300,
            correction_delay_max: 700,
            long_pause_probability: 5,
            long_pause_delay_min: 1000,
            long_pause_delay_max: 3000,
        }
    }
}

impl Config {
    fn to_typing_config(&self) -> TypingConfig {
        TypingConfig {
            base_delay: self.base_delay_min..self.base_delay_max,
            thinking_delay: self.thinking_delay_min..self.thinking_delay_max,
            mistake_probability: self.mistake_probability,
            correction_delay: self.correction_delay_min..self.correction_delay_max,
            long_pause_probability: self.long_pause_probability,
            long_pause_delay: self.long_pause_delay_min..self.long_pause_delay_max,
        }
    }
}

fn get_config_path() -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("config.json");
    path
}

fn get_text_file_path() -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("typethis.txt");
    path
}

fn ensure_config_exists() -> Result<Config, Box<dyn std::error::Error>> {
    let config_path = get_config_path();

    if (!config_path.exists()) {
        let config = Config::default();
        let config_str = serde_json::to_string_pretty(&config)?;
        fs::write(&config_path, config_str)?;
        return Ok(config);
    }

    let config_str = fs::read_to_string(&config_path)?;
    match serde_json::from_str(&config_str) {
        Ok(config) => Ok(config),
        Err(_) => {
            // If there's an error loading the config, create a new one
            println!("Warning: Invalid or outdated config file. Creating new config...");
            let config = Config::default();
            let config_str = serde_json::to_string_pretty(&config)?;
            fs::write(&config_path, config_str)?;
            Ok(config)
        }
    }
}

fn ensure_text_file_exists() -> Result<String, Box<dyn std::error::Error>> {
    let text_path = get_text_file_path();

    if (!text_path.exists()) {
        let default_text = "Type your text here.\nType your text here.";
        fs::write(&text_path, default_text)?;
        return Ok(default_text.to_string());
    }

    let content = fs::read_to_string(&text_path)?;
    if content.trim().is_empty() {
        let default_text = "Type your text here.\nType your text here.";
        fs::write(&text_path, default_text)?;
        Ok(default_text.to_string())
    } else {
        // Normalize line endings and ensure proper text handling
        let normalized = content.replace("\r\n", "\n");
        if normalized.ends_with('\n') {
            Ok(normalized[..normalized.len()-1].to_string())
        } else {
            Ok(normalized)
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
        for c in text.chars() {
            match c {
                '\n' => {
                    self.enigo.key_click(Key::Return);
                    thread::sleep(Duration::from_millis(
                        self.rng.gen_range(self.config.thinking_delay.clone()),
                    ));
                },
                '\r' => continue, // Skip carriage returns
                _ => {
                    // Thinking pause on whitespace
                    if self.rng.gen_ratio(1, 100) && c.is_whitespace() {
                        thread::sleep(Duration::from_millis(
                            self.rng.gen_range(self.config.thinking_delay.clone()),
                        ));
                    }

                    self.type_character(c);

                    // Long pause after punctuation (after typing the character)
                    if self.rng.gen_ratio(self.config.long_pause_probability, 100)
                        && ".,?!;:".contains(c) {
                        thread::sleep(Duration::from_millis(
                            self.rng.gen_range(self.config.long_pause_delay.clone()),
                        ));
                    }
                }
            }

            thread::sleep(Duration::from_millis(
                self.rng.gen_range(self.config.base_delay.clone()),
            ));
        }
    }

    fn type_character(&mut self, intended_char: char) {
        if self.rng.gen_ratio(1, self.config.mistake_probability) {
            // Make a simple mistake
            let mistake_char = self.keyboard.get_nearby_key(intended_char);
            self.enigo.key_sequence(&mistake_char.to_string());

            // Wait a bit before correcting
            thread::sleep(Duration::from_millis(
                self.rng.gen_range(self.config.correction_delay.clone()),
            ));

            // Correct the mistake
            self.enigo.key_click(Key::Backspace);
            self.enigo.key_sequence(&intended_char.to_string());
        } else {
            self.enigo.key_sequence(&intended_char.to_string());
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = ensure_config_exists()?;
    let text = ensure_text_file_exists()?;

    println!("Text file location: {}", get_text_file_path().display());
    println!("Config file location: {}", get_config_path().display());
    println!("\nText to type:");
    println!("{}", text);

    print!("\nEnter the number of seconds to wait before starting: ");
    io::stdout().flush()?;

    let mut delay_secs = String::new();
    io::stdin().read_line(&mut delay_secs)?;
    let delay_secs: u64 = delay_secs.trim().parse().unwrap_or(5);

    println!("\nStarting in...");
    for i in (1..=delay_secs).rev() {
        println!("{i}...");
        thread::sleep(Duration::from_secs(1));
    }
    println!("Go!");

    let mut typist = HumanTypist::new();
    typist.config = config.to_typing_config();
    typist.type_text(&text);

    Ok(())
}