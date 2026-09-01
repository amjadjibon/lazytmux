use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{Block, BorderType, Borders};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum ThemePreset {
    #[default]
    Default,
    TokyoNight,
    Catppuccin,
    Nord,
    Gruvbox,
    Dracula,
    RosePine,
}

impl ThemePreset {
    pub const ALL: &'static [ThemePreset] = &[
        ThemePreset::Default,
        ThemePreset::TokyoNight,
        ThemePreset::Catppuccin,
        ThemePreset::Nord,
        ThemePreset::Gruvbox,
        ThemePreset::Dracula,
        ThemePreset::RosePine,
    ];

    pub fn name(&self) -> &'static str {
        match self {
            ThemePreset::Default => "Default Cyan",
            ThemePreset::TokyoNight => "Tokyo Night",
            ThemePreset::Catppuccin => "Catppuccin Mocha",
            ThemePreset::Nord => "Nord Frost",
            ThemePreset::Gruvbox => "Gruvbox Dark",
            ThemePreset::Dracula => "Dracula",
            ThemePreset::RosePine => "Rosé Pine",
        }
    }

    pub fn next(&self) -> Self {
        match self {
            ThemePreset::Default => ThemePreset::TokyoNight,
            ThemePreset::TokyoNight => ThemePreset::Catppuccin,
            ThemePreset::Catppuccin => ThemePreset::Nord,
            ThemePreset::Nord => ThemePreset::Gruvbox,
            ThemePreset::Gruvbox => ThemePreset::Dracula,
            ThemePreset::Dracula => ThemePreset::RosePine,
            ThemePreset::RosePine => ThemePreset::Default,
        }
    }

    pub fn prev(&self) -> Self {
        match self {
            ThemePreset::Default => ThemePreset::RosePine,
            ThemePreset::TokyoNight => ThemePreset::Default,
            ThemePreset::Catppuccin => ThemePreset::TokyoNight,
            ThemePreset::Nord => ThemePreset::Catppuccin,
            ThemePreset::Gruvbox => ThemePreset::Nord,
            ThemePreset::Dracula => ThemePreset::Gruvbox,
            ThemePreset::RosePine => ThemePreset::Dracula,
        }
    }

    pub fn to_theme(self, border_type: BorderType) -> Theme {
        match self {
            ThemePreset::Default => Theme {
                preset: self,
                border_style: Style::default().fg(Color::DarkGray),
                border_focused: Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
                border_type,
                title: Style::default()
                    .fg(Color::Gray)
                    .add_modifier(Modifier::BOLD),
                title_focused: Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
                selection: Style::default()
                    .bg(Color::Rgb(30, 45, 65))
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
                attached_session: Style::default().fg(Color::Green),
                detached_session: Style::default().fg(Color::DarkGray),
                favorite: Style::default().fg(Color::Yellow),
                active_item: Style::default()
                    .fg(Color::LightCyan)
                    .add_modifier(Modifier::BOLD),
                dim: Style::default().fg(Color::DarkGray),
                error: Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
                success: Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
                warning: Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
                info: Style::default().fg(Color::Cyan),
                breadcrumb_label: Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
                breadcrumb_val: Style::default().fg(Color::White),
            },
            ThemePreset::TokyoNight => Theme {
                preset: self,
                border_style: Style::default().fg(Color::Rgb(65, 72, 104)),
                border_focused: Style::default()
                    .fg(Color::Rgb(122, 162, 247))
                    .add_modifier(Modifier::BOLD),
                border_type,
                title: Style::default()
                    .fg(Color::Rgb(169, 177, 214))
                    .add_modifier(Modifier::BOLD),
                title_focused: Style::default()
                    .fg(Color::Rgb(122, 162, 247))
                    .add_modifier(Modifier::BOLD),
                selection: Style::default()
                    .bg(Color::Rgb(41, 46, 66))
                    .fg(Color::Rgb(192, 202, 245))
                    .add_modifier(Modifier::BOLD),
                attached_session: Style::default().fg(Color::Rgb(158, 206, 106)),
                detached_session: Style::default().fg(Color::Rgb(86, 95, 137)),
                favorite: Style::default().fg(Color::Rgb(224, 175, 104)),
                active_item: Style::default()
                    .fg(Color::Rgb(187, 154, 247))
                    .add_modifier(Modifier::BOLD),
                dim: Style::default().fg(Color::Rgb(86, 95, 137)),
                error: Style::default()
                    .fg(Color::Rgb(247, 118, 142))
                    .add_modifier(Modifier::BOLD),
                success: Style::default()
                    .fg(Color::Rgb(158, 206, 106))
                    .add_modifier(Modifier::BOLD),
                warning: Style::default()
                    .fg(Color::Rgb(224, 175, 104))
                    .add_modifier(Modifier::BOLD),
                info: Style::default().fg(Color::Rgb(122, 162, 247)),
                breadcrumb_label: Style::default()
                    .fg(Color::Rgb(122, 162, 247))
                    .add_modifier(Modifier::BOLD),
                breadcrumb_val: Style::default().fg(Color::Rgb(192, 202, 245)),
            },
            ThemePreset::Catppuccin => Theme {
                preset: self,
                border_style: Style::default().fg(Color::Rgb(88, 91, 112)),
                border_focused: Style::default()
                    .fg(Color::Rgb(203, 166, 247))
                    .add_modifier(Modifier::BOLD),
                border_type,
                title: Style::default()
                    .fg(Color::Rgb(186, 194, 222))
                    .add_modifier(Modifier::BOLD),
                title_focused: Style::default()
                    .fg(Color::Rgb(203, 166, 247))
                    .add_modifier(Modifier::BOLD),
                selection: Style::default()
                    .bg(Color::Rgb(69, 71, 90))
                    .fg(Color::Rgb(205, 214, 244))
                    .add_modifier(Modifier::BOLD),
                attached_session: Style::default().fg(Color::Rgb(166, 227, 161)),
                detached_session: Style::default().fg(Color::Rgb(108, 112, 134)),
                favorite: Style::default().fg(Color::Rgb(249, 226, 175)),
                active_item: Style::default()
                    .fg(Color::Rgb(137, 180, 250))
                    .add_modifier(Modifier::BOLD),
                dim: Style::default().fg(Color::Rgb(108, 112, 134)),
                error: Style::default()
                    .fg(Color::Rgb(243, 139, 168))
                    .add_modifier(Modifier::BOLD),
                success: Style::default()
                    .fg(Color::Rgb(166, 227, 161))
                    .add_modifier(Modifier::BOLD),
                warning: Style::default()
                    .fg(Color::Rgb(250, 179, 135))
                    .add_modifier(Modifier::BOLD),
                info: Style::default().fg(Color::Rgb(116, 199, 236)),
                breadcrumb_label: Style::default()
                    .fg(Color::Rgb(203, 166, 247))
                    .add_modifier(Modifier::BOLD),
                breadcrumb_val: Style::default().fg(Color::Rgb(205, 214, 244)),
            },
            ThemePreset::Nord => Theme {
                preset: self,
                border_style: Style::default().fg(Color::Rgb(76, 86, 106)),
                border_focused: Style::default()
                    .fg(Color::Rgb(136, 192, 208))
                    .add_modifier(Modifier::BOLD),
                border_type,
                title: Style::default()
                    .fg(Color::Rgb(229, 233, 240))
                    .add_modifier(Modifier::BOLD),
                title_focused: Style::default()
                    .fg(Color::Rgb(136, 192, 208))
                    .add_modifier(Modifier::BOLD),
                selection: Style::default()
                    .bg(Color::Rgb(67, 76, 94))
                    .fg(Color::Rgb(236, 239, 244))
                    .add_modifier(Modifier::BOLD),
                attached_session: Style::default().fg(Color::Rgb(163, 190, 140)),
                detached_session: Style::default().fg(Color::Rgb(94, 129, 172)),
                favorite: Style::default().fg(Color::Rgb(235, 203, 139)),
                active_item: Style::default()
                    .fg(Color::Rgb(143, 188, 187))
                    .add_modifier(Modifier::BOLD),
                dim: Style::default().fg(Color::Rgb(94, 129, 172)),
                error: Style::default()
                    .fg(Color::Rgb(191, 97, 106))
                    .add_modifier(Modifier::BOLD),
                success: Style::default()
                    .fg(Color::Rgb(163, 190, 140))
                    .add_modifier(Modifier::BOLD),
                warning: Style::default()
                    .fg(Color::Rgb(235, 203, 139))
                    .add_modifier(Modifier::BOLD),
                info: Style::default().fg(Color::Rgb(129, 161, 193)),
                breadcrumb_label: Style::default()
                    .fg(Color::Rgb(136, 192, 208))
                    .add_modifier(Modifier::BOLD),
                breadcrumb_val: Style::default().fg(Color::Rgb(236, 239, 244)),
            },
            ThemePreset::Gruvbox => Theme {
                preset: self,
                border_style: Style::default().fg(Color::Rgb(102, 92, 84)),
                border_focused: Style::default()
                    .fg(Color::Rgb(254, 128, 25))
                    .add_modifier(Modifier::BOLD),
                border_type,
                title: Style::default()
                    .fg(Color::Rgb(235, 219, 178))
                    .add_modifier(Modifier::BOLD),
                title_focused: Style::default()
                    .fg(Color::Rgb(254, 128, 25))
                    .add_modifier(Modifier::BOLD),
                selection: Style::default()
                    .bg(Color::Rgb(60, 56, 54))
                    .fg(Color::Rgb(251, 241, 199))
                    .add_modifier(Modifier::BOLD),
                attached_session: Style::default().fg(Color::Rgb(184, 187, 38)),
                detached_session: Style::default().fg(Color::Rgb(146, 131, 116)),
                favorite: Style::default().fg(Color::Rgb(250, 189, 47)),
                active_item: Style::default()
                    .fg(Color::Rgb(142, 192, 124))
                    .add_modifier(Modifier::BOLD),
                dim: Style::default().fg(Color::Rgb(146, 131, 116)),
                error: Style::default()
                    .fg(Color::Rgb(251, 73, 52))
                    .add_modifier(Modifier::BOLD),
                success: Style::default()
                    .fg(Color::Rgb(184, 187, 38))
                    .add_modifier(Modifier::BOLD),
                warning: Style::default()
                    .fg(Color::Rgb(250, 189, 47))
                    .add_modifier(Modifier::BOLD),
                info: Style::default().fg(Color::Rgb(131, 165, 152)),
                breadcrumb_label: Style::default()
                    .fg(Color::Rgb(254, 128, 25))
                    .add_modifier(Modifier::BOLD),
                breadcrumb_val: Style::default().fg(Color::Rgb(251, 241, 199)),
            },
            ThemePreset::Dracula => Theme {
                preset: self,
                border_style: Style::default().fg(Color::Rgb(98, 114, 164)),
                border_focused: Style::default()
                    .fg(Color::Rgb(189, 147, 249))
                    .add_modifier(Modifier::BOLD),
                border_type,
                title: Style::default()
                    .fg(Color::Rgb(248, 248, 242))
                    .add_modifier(Modifier::BOLD),
                title_focused: Style::default()
                    .fg(Color::Rgb(189, 147, 249))
                    .add_modifier(Modifier::BOLD),
                selection: Style::default()
                    .bg(Color::Rgb(68, 71, 90))
                    .fg(Color::Rgb(248, 248, 242))
                    .add_modifier(Modifier::BOLD),
                attached_session: Style::default().fg(Color::Rgb(80, 250, 123)),
                detached_session: Style::default().fg(Color::Rgb(98, 114, 164)),
                favorite: Style::default().fg(Color::Rgb(241, 250, 140)),
                active_item: Style::default()
                    .fg(Color::Rgb(255, 121, 198))
                    .add_modifier(Modifier::BOLD),
                dim: Style::default().fg(Color::Rgb(98, 114, 164)),
                error: Style::default()
                    .fg(Color::Rgb(255, 85, 85))
                    .add_modifier(Modifier::BOLD),
                success: Style::default()
                    .fg(Color::Rgb(80, 250, 123))
                    .add_modifier(Modifier::BOLD),
                warning: Style::default()
                    .fg(Color::Rgb(255, 184, 108))
                    .add_modifier(Modifier::BOLD),
                info: Style::default().fg(Color::Rgb(139, 233, 253)),
                breadcrumb_label: Style::default()
                    .fg(Color::Rgb(189, 147, 249))
                    .add_modifier(Modifier::BOLD),
                breadcrumb_val: Style::default().fg(Color::Rgb(248, 248, 242)),
            },
            ThemePreset::RosePine => Theme {
                preset: self,
                border_style: Style::default().fg(Color::Rgb(110, 106, 134)),
                border_focused: Style::default()
                    .fg(Color::Rgb(235, 188, 186))
                    .add_modifier(Modifier::BOLD),
                border_type,
                title: Style::default()
                    .fg(Color::Rgb(224, 222, 244))
                    .add_modifier(Modifier::BOLD),
                title_focused: Style::default()
                    .fg(Color::Rgb(235, 188, 186))
                    .add_modifier(Modifier::BOLD),
                selection: Style::default()
                    .bg(Color::Rgb(38, 35, 58))
                    .fg(Color::Rgb(224, 222, 244))
                    .add_modifier(Modifier::BOLD),
                attached_session: Style::default().fg(Color::Rgb(49, 116, 143)),
                detached_session: Style::default().fg(Color::Rgb(110, 106, 134)),
                favorite: Style::default().fg(Color::Rgb(246, 193, 119)),
                active_item: Style::default()
                    .fg(Color::Rgb(196, 167, 231))
                    .add_modifier(Modifier::BOLD),
                dim: Style::default().fg(Color::Rgb(110, 106, 134)),
                error: Style::default()
                    .fg(Color::Rgb(235, 111, 146))
                    .add_modifier(Modifier::BOLD),
                success: Style::default()
                    .fg(Color::Rgb(156, 207, 216))
                    .add_modifier(Modifier::BOLD),
                warning: Style::default()
                    .fg(Color::Rgb(246, 193, 119))
                    .add_modifier(Modifier::BOLD),
                info: Style::default().fg(Color::Rgb(156, 207, 216)),
                breadcrumb_label: Style::default()
                    .fg(Color::Rgb(235, 188, 186))
                    .add_modifier(Modifier::BOLD),
                breadcrumb_val: Style::default().fg(Color::Rgb(224, 222, 244)),
            },
        }
    }
}

pub struct Theme {
    pub preset: ThemePreset,
    pub border_style: Style,
    pub border_focused: Style,
    pub border_type: BorderType,
    pub title: Style,
    pub title_focused: Style,
    pub selection: Style,
    pub attached_session: Style,
    pub detached_session: Style,
    pub favorite: Style,
    pub active_item: Style,
    pub dim: Style,
    pub error: Style,
    pub success: Style,
    pub warning: Style,
    pub info: Style,
    pub breadcrumb_label: Style,
    pub breadcrumb_val: Style,
}

impl Default for Theme {
    fn default() -> Self {
        ThemePreset::Default.to_theme(BorderType::Rounded)
    }
}

impl Theme {
    pub fn block<'a>(&self, title: &'a str, focused: bool) -> Block<'a> {
        let border_style = if focused {
            self.border_focused
        } else {
            self.border_style
        };
        let title_style = if focused {
            self.title_focused
        } else {
            self.title
        };

        Block::default()
            .borders(Borders::ALL)
            .border_type(self.border_type)
            .border_style(border_style)
            .title(format!(" {title} "))
            .title_style(title_style)
    }
}
