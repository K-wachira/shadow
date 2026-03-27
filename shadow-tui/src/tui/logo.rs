use ratatui::style::Color;
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::text::Span;

const ACCENT: Color = Color::Rgb(207, 106, 76);
const DIM: Color = Color::Rgb(120, 120, 130);
const BORDER: Color = Color::Rgb(80, 80, 90);

// Mascot pixel art using block elements
// const MASCOT: [&str; 6] = [
//     "  ██████  ",
//     "  █ ██ █  ",
//     "  ██████  ",
//     "  ██  ██  ",
//     "  █    █  ",
//     "          ",
// ];
// const MASCOT: [&str; 2] = [
//     "⠀⠀⡤⢤⠀⣄⢄⡤⡀⡠⡀⡄⠀⡄⠀⠀",
//     "⠀⠀⠭⠿⠉⠿⠹⠧⠊⠢⠊⠧⠧⠇⠀⠀",
// ];

const MASCOT: [&str; 5] = [
    "                                ",
    "█▀▀█ █▄▀▄█▀▄▀▄█   █             ",
    "▀▀██▀██▀██ █ ██ ▄ █             ",
    "▀▀▀▀ ▀▀ ▀▀▀ ▀ ▀▀▀▀▀             ",
    "▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀             ",
];
pub fn logo_lines() -> Vec<Line<'static>> {
    let dim = Style::default().fg(DIM);
    let border = Style::default().fg(BORDER);
    let mascot_style = Style::default().fg(ACCENT);

    // Width matches a standard 80-col terminal, leaving breathing room
    let w: usize = 50;

    let top    = format!("┌{}┐", "─".repeat(w));
    let bottom = format!("└{}┘", "─".repeat(w));

    let pad = |s: &str| -> String {
        let len = s.chars().count();
        let total_pad = w.saturating_sub(len);
        let left_pad = total_pad / 2;
        let right_pad = total_pad - left_pad;
        format!("{}{}{}", " ".repeat(left_pad), s, " ".repeat(right_pad))
    };


    let border_row = |content: &'static str, style: Style| -> Line<'static> {
        Line::from(vec![
            Span::styled("│", border),
            Span::styled(pad(content), style),
            Span::styled("│", border),
        ])
    };

    vec![
        Line::raw(""),
        Line::from(Span::styled(top, border)),

        // Mascot
        Line::from(vec![
            Span::styled("│", border),
            Span::styled(pad(MASCOT[0]), mascot_style),
            Span::styled("│", border),
        ]),
        Line::from(vec![
            Span::styled("│", border),
            Span::styled(pad(MASCOT[1]), mascot_style),
            Span::styled("│", border),
        ]),
        Line::from(vec![
            Span::styled("│", border),
            Span::styled(pad(MASCOT[2]), mascot_style),
            Span::styled("│", border),
        ]),
        Line::from(vec![
            Span::styled("│", border),
            Span::styled(pad(MASCOT[3]), mascot_style),
            Span::styled("│", border),
        ]),
        Line::from(vec![
            Span::styled("│", border),
            Span::styled(pad(MASCOT[4]), mascot_style),
            Span::styled("│", border),
        ]),


        // Meta info
        border_row("kelvin ·  gemma3:12b  ·  v0.1.0", dim),

        
        Line::from(Span::styled(bottom, border)),
        Line::raw(""),
    ]
}