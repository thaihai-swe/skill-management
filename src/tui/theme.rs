use ratatui::{
    style::{Color, Modifier, Style},
    text::Span,
    widgets::BorderType,
};

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct Theme {
    pub border_type: BorderType,
    pub border_normal: Color,
    pub border_focused: Color,

    pub accent: Color,
    pub highlight: Color,
    pub success: Color,
    pub danger: Color,
    pub muted: Color,
    pub text_bright: Color,
    pub text_dim: Color,
    pub cursor_bg: Color,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            border_type: BorderType::Rounded,
            border_normal: Color::DarkGray,
            border_focused: Color::Cyan,

            accent: Color::Cyan,
            highlight: Color::Yellow,
            success: Color::Green,
            danger: Color::Red,
            muted: Color::DarkGray,
            text_bright: Color::White,
            text_dim: Color::Rgb(140, 145, 165),
            cursor_bg: Color::Rgb(35, 42, 58),
        }
    }
}

impl Theme {
    /// Creates a high-contrast styled keycap: e.g. [ Key ]
    pub fn keycap(key: &str, bg: Color) -> Span<'static> {
        Span::styled(
            format!(" {} ", key),
            Style::default()
                .fg(Color::Black)
                .bg(bg)
                .add_modifier(Modifier::BOLD),
        )
    }

    /// Creates label text following a keycap
    pub fn keycap_label(label: &str) -> Span<'static> {
        Span::styled(format!(" {}  ", label), Style::default().fg(Color::White))
    }

    /// Renders a full keycap pill: [ Key ] Label
    pub fn pill(key: &str, label: &str, color: Color) -> Vec<Span<'static>> {
        vec![Self::keycap(key, color), Self::keycap_label(label)]
    }

    pub fn pill_primary(key: &str, label: &str) -> Vec<Span<'static>> {
        Self::pill(key, label, Color::Cyan)
    }

    pub fn pill_warning(key: &str, label: &str) -> Vec<Span<'static>> {
        Self::pill(key, label, Color::Yellow)
    }

    pub fn pill_success(key: &str, label: &str) -> Vec<Span<'static>> {
        Self::pill(key, label, Color::Green)
    }

    pub fn pill_danger(key: &str, label: &str) -> Vec<Span<'static>> {
        Self::pill(key, label, Color::Red)
    }

    pub fn pill_muted(key: &str, label: &str) -> Vec<Span<'static>> {
        Self::pill(key, label, Color::DarkGray)
    }
}
