#[derive(Debug)]
pub enum Event {
    Connected,
    Beamer(BeamerCommand),
    Screen(ScreenDirection),
    RiiKey,
    Log(String),
    WifiLost,
}

#[derive(Debug)]
pub enum ScreenDirection {
    Up,
    Down,
}

#[derive(Debug)]
pub enum BeamerCommand {
    On,
    Off,
}
