use rand::Rng;

pub struct HandleFactory {
    current: i32
}

impl HandleFactory {
    pub fn new() -> Self {
        HandleFactory {
            current: 1 //rand::thread_rng().gen()
        }
    }

    pub fn next(&mut self) -> i32 {
        if self.current >= i32::max_value() {
            self.current = 0;
        }

        self.current = self.current + 1;
        self.current
    }
}