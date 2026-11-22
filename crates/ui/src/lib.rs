//! Design tokens and shared, accessible UI components for Devdeck.
//! No webviews; every widget is rendered natively by Iced.
#![forbid(unsafe_code)]

use devdeck_core::ThemeName;
use iced::widget::{button, column, container, row, text, text_input, Column};
use iced::{Background, Color, Element, Length};
use unicode_normalization::UnicodeNormalization;

pub const SPACE_XS: f32 = 4.0;
pub const SPACE_SM: f32 = 8.0;
pub const SPACE_MD: f32 = 12.0;
pub const SPACE_LG: f32 = 16.0;
pub const RADIUS: f32 = 6.0;
pub const BODY_SIZE: f32 = 14.0;
pub const SIDEBAR_WIDTH: f32 = 200.0;
pub const STATUS_HEIGHT: f32 = 28.0;

/// Design tokens. All colors come from here; no widget hard-codes a color.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Tokens {
    pub background: Color,
    pub surface: Color,
    pub surface_alt: Color,
    pub border: Color,
    pub text: Color,
    pub text_muted: Color,
    pub accent: Color,
    pub danger: Color,
}

impl Tokens {
    pub fn for_theme(theme: ThemeName) -> Self {
        match theme {
            ThemeName::Dark => Self {
                background: rgb(0x14, 0x18, 0x1f),
                surface: rgb(0x1b, 0x21, 0x2b),
                surface_alt: rgb(0x24, 0x2c, 0x38),
                border: rgb(0x35, 0x40, 0x50),
                text: rgb(0xe6, 0xea, 0xf0),
                text_muted: rgb(0x94, 0xa0, 0xb0),
                accent: rgb(0x5b, 0xa8, 0xff),
                danger: rgb(0xff, 0x6b, 0x6b),
            },
            ThemeName::Light => Self {
                background: rgb(0xf5, 0xf7, 0xfa),
                surface: rgb(0xff, 0xff, 0xff),
                surface_alt: rgb(0xea, 0xef, 0xf5),
                border: rgb(0xc8, 0xd2, 0xdf),
                text: rgb(0x18, 0x1f, 0x2a),
                text_muted: rgb(0x5a, 0x66, 0x76),
                accent: rgb(0x1c, 0x63, 0xc8),
                danger: rgb(0xb0, 0x20, 0x20),
            },
            ThemeName::HighContrast => Self {
                background: rgb(0x00, 0x00, 0x00),
                surface: rgb(0x0a, 0x0a, 0x0a),
                surface_alt: rgb(0x1a, 0x1a, 0x1a),
                border: rgb(0xff, 0xff, 0x00),
                text: rgb(0xff, 0xff, 0xff),
                text_muted: rgb(0xff, 0xff, 0x00),
                accent: rgb(0x00, 0xff, 0xff),
                danger: rgb(0xff, 0x55, 0x55),
            },
        }
    }
}

const fn rgb(r: u8, g: u8, b: u8) -> Color {
    Color {
        r: r as f32 / 255.0,
        g: g as f32 / 255.0,
        b: b as f32 / 255.0,
        a: 1.0,
    }
}

/// A command shown in the palette. `label` is localized; `keywords` may hold
/// English and localized aliases so both are searchable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandItem {
    pub id: String,
    pub label: String,
    pub hint: String,
    pub keywords: Vec<String>,
}

/// Diacritic-insensitive matching: "viet" finds "việt". NFD decomposition
/// strips combining marks; case is folded as well.
pub fn normalize_search(input: &str) -> String {
    // U+0110/U+0111 have no canonical decomposition, so map them explicitly.
    let pre: String = input
        .chars()
        .map(|c| match c {
            'đ' | 'Đ' => 'd',
            _ => c,
        })
        .collect();
    pre.nfd()
        .filter(|c| !unicode_normalization::char::is_combining_mark(*c))
        .flat_map(char::to_lowercase)
        .collect()
}

pub fn filter_commands<'a>(query: &str, items: &'a [CommandItem]) -> Vec<&'a CommandItem> {
    let needle = normalize_search(query);
    if needle.is_empty() {
        return items.iter().collect();
    }
    items
        .iter()
        .filter(|item| {
            let haystack = [item.label.as_str(), item.hint.as_str()]
                .into_iter()
                .chain(item.keywords.iter().map(String::as_str))
                .map(normalize_search)
                .collect::<Vec<_>>()
                .join(" ");
            haystack.contains(&needle)
        })
        .collect()
}

fn panel_style(tokens: Tokens) -> impl Fn(&iced::Theme) -> iced::widget::container::Style {
    move |_| iced::widget::container::Style {
        background: Some(Background::Color(tokens.surface)),
        text_color: Some(tokens.text),
        border: iced::Border {
            color: tokens.border,
            width: 1.0,
            radius: iced::border::Radius::new(RADIUS),
        },
        ..Default::default()
    }
}

/// Left sidebar with accessible, localized labels.
pub fn sidebar<'a, M: Clone + 'a>(
    tokens: Tokens,
    entries: &'a [(String, String)],
    selected: usize,
    on_select: impl Fn(usize) -> M + 'a,
) -> Element<'a, M> {
    let mut children: Vec<Element<'a, M>> = Vec::new();
    for (index, (label, tooltip)) in entries.iter().enumerate() {
        let accent = if index == selected {
            tokens.accent
        } else {
            tokens.surface_alt
        };
        let button = button(text(label).size(BODY_SIZE))
            .width(Length::Fill)
            .on_press(on_select(index))
            .style(move |_, _| iced::widget::button::Style {
                background: Some(Background::Color(accent)),
                text_color: tokens.text,
                border: iced::Border {
                    color: tokens.border,
                    width: 1.0,
                    radius: iced::border::Radius::new(RADIUS),
                },
                ..Default::default()
            });
        children.push(button.into());
        let _ = tooltip; // accessible name is the localized button text
    }
    let items = Column::with_children(children)
        .spacing(SPACE_SM)
        .padding(SPACE_MD);
    container(items)
        .width(Length::Fixed(SIDEBAR_WIDTH))
        .height(Length::Fill)
        .style(panel_style(tokens))
        .into()
}

pub fn status_bar<'a, M: 'a>(tokens: Tokens, message: String) -> Element<'a, M> {
    container(row![text(message).size(BODY_SIZE - 2.0)].padding(SPACE_SM))
        .width(Length::Fill)
        .height(Length::Fixed(STATUS_HEIGHT))
        .style(panel_style(tokens))
        .into()
}

#[derive(Debug, Clone, Copy)]
pub struct PaletteLabels<'a> {
    pub placeholder: &'a str,
    pub empty: &'a str,
}

/// Command palette (Ctrl/Cmd+K): localized placeholder, keyboard-first list.
pub fn command_palette<'a, M: Clone + 'a>(
    tokens: Tokens,
    query: &'a str,
    results: &'a [CommandItem],
    labels: PaletteLabels<'a>,
    on_input: impl Fn(String) -> M + 'a,
    on_select: impl Fn(String) -> M + 'a,
    on_submit: impl Fn() -> M + 'a,
) -> Element<'a, M> {
    let input = text_input(labels.placeholder, query)
        .on_input(on_input)
        .on_submit(on_submit())
        .size(18.0)
        .padding(SPACE_MD);

    let mut children: Vec<Element<'a, M>> = Vec::new();
    if results.is_empty() {
        children.push(text(labels.empty).size(BODY_SIZE).into());
    }
    for item in results {
        let id = item.id.clone();
        let hint = item.hint.clone();
        let label = item.label.clone();
        children.push(
            button(
                row![
                    text(label).size(BODY_SIZE),
                    text(hint).size(BODY_SIZE - 2.0)
                ]
                .spacing(SPACE_MD)
                .width(Length::Fill),
            )
            .width(Length::Fill)
            .on_press(on_select(id))
            .style(move |_, _| iced::widget::button::Style {
                background: Some(Background::Color(tokens.surface_alt)),
                text_color: tokens.text,
                ..Default::default()
            })
            .into(),
        );
    }
    let inner = column(children).spacing(SPACE_XS);
    let content = Column::with_children(vec![
        input.into(),
        iced::widget::Space::new().height(SPACE_SM).into(),
        inner.into(),
    ])
    .spacing(SPACE_SM);
    container(content)
        .width(Length::Fixed(560.0))
        .padding(SPACE_LG)
        .style(panel_style(tokens))
        .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(label: &str, hint: &str) -> CommandItem {
        CommandItem {
            id: label.into(),
            label: label.into(),
            hint: hint.into(),
            keywords: Vec::new(),
        }
    }

    #[test]
    fn search_is_diacritic_insensitive() {
        let items = vec![
            item("Đọc tệp", "files.read"),
            item("Terminal", "terminal.new"),
        ];
        assert_eq!(filter_commands("doc tep", &items).len(), 1);
        assert_eq!(filter_commands("files.read", &items)[0].id, "Đọc tệp");
        assert_eq!(filter_commands("", &items).len(), 2);
    }

    #[test]
    fn tokens_cover_all_themes() {
        assert_ne!(
            Tokens::for_theme(ThemeName::Dark).background,
            Tokens::for_theme(ThemeName::Light).background
        );
        assert_eq!(Tokens::for_theme(ThemeName::HighContrast).border.a, 1.0);
    }
}
