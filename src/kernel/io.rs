use std::collections::VecDeque;

#[derive(Clone, Debug)]
pub enum SpecialKey {
    #[expect(dead_code)]
    Enter,
    #[expect(dead_code)]
    Backspace,
    #[expect(dead_code)]
    Up,
    #[expect(dead_code)]
    Down,
    #[expect(dead_code)]
    Left,
    #[expect(dead_code)]
    Right,
    #[expect(dead_code)]
    Tab,
    #[expect(dead_code)]
    Escape,
    #[expect(dead_code)]
    F(u8),
}

#[derive(Clone, Debug)]
pub enum IoEvent {
    #[expect(dead_code)]
    KeyPress(char),
    #[expect(dead_code)]
    KeySpecial(SpecialKey),
    #[expect(dead_code)]
    DeviceReady(u32),
}

pub struct IoSubsystem {
    pub event_queue: VecDeque<IoEvent>,
    pub output_buffer: Vec<String>,
}

impl IoSubsystem {
    pub fn new() -> Self {
        IoSubsystem {
            event_queue: VecDeque::new(),
            output_buffer: Vec::new(),
        }
    }

    #[expect(dead_code)]
    pub fn push_event(&mut self, event: IoEvent) {
        self.event_queue.push_back(event);
    }

    #[expect(dead_code)]
    pub fn pop_event(&mut self) -> Option<IoEvent> {
        self.event_queue.pop_front()
    }

    #[expect(dead_code)]
    pub fn write_output(&mut self, s: &str) {
        self.output_buffer.push(s.to_string());
    }

    #[expect(dead_code)]
    pub fn drain_output(&mut self) -> Vec<String> {
        self.output_buffer.drain(..).collect()
    }
}
