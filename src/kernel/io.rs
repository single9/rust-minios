use std::collections::VecDeque;

#[derive(Clone, Debug)]
pub enum SpecialKey {
    Enter,
    Backspace,
    Up,
    Down,
    Left,
    Right,
    Tab,
    Escape,
    F(u8),
}

#[derive(Clone, Debug)]
pub enum IoEvent {
    KeyPress(char),
    KeySpecial(SpecialKey),
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

    pub fn push_event(&mut self, event: IoEvent) {
        self.event_queue.push_back(event);
    }

    pub fn pop_event(&mut self) -> Option<IoEvent> {
        self.event_queue.pop_front()
    }

    pub fn write_output(&mut self, s: &str) {
        self.output_buffer.push(s.to_string());
    }

    pub fn drain_output(&mut self) -> Vec<String> {
        self.output_buffer.drain(..).collect()
    }
}
