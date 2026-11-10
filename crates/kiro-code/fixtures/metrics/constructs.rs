mod outer {
    pub trait Shape {
        fn area(&self) -> f64;
        fn name(&self) -> String {
            String::from("shape")
        }
    }

    pub struct Square {
        pub side: f64,
        pub label: String,
    }

    impl Square {
        pub fn scaled(&self, factor: f64) -> f64 {
            let apply = |value: f64| if value > 0.0 { value * factor } else { 0.0 };
            apply(self.side)
        }
    }

    impl Shape for Square {
        fn area(&self) -> f64 {
            fn square(value: f64) -> f64 {
                value * value
            }
            square(self.side)
        }
    }
}

pub enum Mode {
    Fast,
    Slow,
}

pub type Alias = Vec<Mode>;

pub fn pick<T: Clone>(items: &[T], mode: Mode) -> Option<T> {
    let first = items.first()?;
    loop {
        if matches!(mode, Mode::Fast) {
            return Some(first.clone());
        } else if items.len() > 1 {
            break;
        } else {
            return None;
        }
    }
    None
}
