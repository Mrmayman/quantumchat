use std::time::Duration;

pub enum State {
    Loading,
    Login(MenuLogin),
    Chats,
    Error(String),
    // Settings
}

pub struct MenuLogin {
    pub timeout: Duration,
    pub qr_code: iced::widget::qr_code::Data,
    pub initial_time: std::time::Instant,
}

impl MenuLogin {
    pub fn new(code: String, timeout: Duration) -> Result<Self, iced::widget::qr_code::Error> {
        Ok(Self {
            qr_code: iced::widget::qr_code::Data::new(&code)?,
            timeout,
            initial_time: std::time::Instant::now(),
        })
    }
}

pub enum ChatKind {
    DirectMessage(String),
    GroupChat(String),
}

pub struct ChatStore {}
