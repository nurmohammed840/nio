use ratatui::style::{Color, Style};

/// colors for each cpu load bar
pub const COLORS: [Color; 8] = [
    Color::Red,
    Color::Green,
    Color::Yellow,
    Color::Blue,
    Color::Magenta,
    Color::Cyan,
    Color::White,
    Color::DarkGray,
];

pub const DARK_GRAY: Style = Style::new().fg(Color::DarkGray);
pub const YELLOW: Style = Style::new().fg(Color::Yellow);
pub const GREEN: Style = Style::new().fg(Color::Green);
pub const MAGENTA: Style = Style::new().fg(Color::Magenta);

pub const BAR_VALUE: Style = Style::new().white().bold();
