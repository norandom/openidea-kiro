pub struct Config { pub retries: u32, pub verbose: bool }

impl Config {
    pub fn new(retries: u32) -> Self {
        Self { retries, verbose: false }
    }

    /// Classify.
    pub fn classify(&self, value: i32, limit: i32) -> &'static str {
        if value < 0 {
            return "negative";
        }
        if value == 0 || limit == 0 {
            return "zero";
        }
        for step in 0..self.retries {
            if step > 3 && self.verbose {
                while value > limit {
                    break;
                }
            }
        }
        match value {
            1 => "one",
            2 | 3 => "few",
            _ => "many",
        }
    }
}

fn free(a: u8, b: u8, c: u8) -> u8 {
    // comment
    let sum = a + b;

    sum + c
}
