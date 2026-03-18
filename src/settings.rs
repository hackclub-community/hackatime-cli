use std::{
    fs,
    io::{self, IsTerminal, Write},
    os::fd::AsRawFd,
    path::PathBuf,
};

use anyhow::{Context, Result};
use libc::{TCSANOW, VMIN, VTIME, cfmakeraw, tcgetattr, tcsetattr, termios};
use serde::{Deserialize, Serialize};

use crate::{models::FetchTheme, output, storage};

#[derive(Debug, Clone, Copy)]
pub struct UserSettings {
    pub fetch_theme: FetchTheme,
    pub clear_terminal: bool,
}

impl Default for UserSettings {
    fn default() -> Self {
        Self {
            fetch_theme: FetchTheme::Red,
            clear_terminal: true,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct StoredSettings {
    fetch_theme: String,
    clear_terminal: bool,
}

const THEME_OPTIONS: [FetchTheme; 7] = [
    FetchTheme::Red,
    FetchTheme::Blue,
    FetchTheme::Green,
    FetchTheme::Yellow,
    FetchTheme::Pink,
    FetchTheme::Cyan,
    FetchTheme::Noir,
];

pub fn load_settings() -> Result<UserSettings> {
    let path = settings_file_path()?;
    if !path.exists() {
        return Ok(UserSettings::default());
    }

    let contents = fs::read_to_string(&path)
        .with_context(|| format!("failed to read settings file at {}", path.display()))?;
    let stored: StoredSettings = serde_json::from_str(&contents)
        .with_context(|| format!("failed to parse settings file at {}", path.display()))?;

    Ok(UserSettings {
        fetch_theme: FetchTheme::parse(&stored.fetch_theme).unwrap_or(FetchTheme::Red),
        clear_terminal: stored.clear_terminal,
    })
}

pub fn save_settings(settings: &UserSettings) -> Result<()> {
    let path = settings_file_path()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).with_context(|| {
            format!(
                "failed to create settings dir at {} (set {} to override the storage location)",
                parent.display(),
                storage::CONFIG_DIR_ENV
            )
        })?;
    }

    let stored = StoredSettings {
        fetch_theme: settings.fetch_theme.as_str().to_string(),
        clear_terminal: settings.clear_terminal,
    };

    let json =
        serde_json::to_string_pretty(&stored).context("failed to serialize user settings")?;
    fs::write(&path, json).with_context(|| {
        format!(
            "failed to write settings file at {} (set {} to override the storage location)",
            path.display(),
            storage::CONFIG_DIR_ENV
        )
    })
}

pub fn format_settings_summary(settings: &UserSettings) -> String {
    format!(
        "Theme Color: {}\nClear Terminal: {}\nSettings File: {}",
        settings.fetch_theme.as_str(),
        if settings.clear_terminal { "on" } else { "off" },
        settings_file_path()
            .map(|path| path.display().to_string())
            .unwrap_or_else(|_| "Unavailable".to_string())
    )
}

pub fn open_color_picker(settings: &mut UserSettings) -> Result<bool> {
    let stdin = io::stdin();
    let stdout = io::stdout();
    if !stdin.is_terminal() || !stdout.is_terminal() {
        anyhow::bail!("`hackatime settings color` requires an interactive terminal.");
    }

    let mut stdout = stdout.lock();
    let terminal = RawTerminalGuard::new(stdin.as_raw_fd(), &mut stdout)?;
    let mut selected_index = theme_index(settings.fetch_theme);

    loop {
        render_color_picker(&mut stdout, THEME_OPTIONS[selected_index])?;

        match read_picker_key(terminal.fd)? {
            PickerKey::Previous => {
                selected_index = if selected_index == 0 {
                    THEME_OPTIONS.len() - 1
                } else {
                    selected_index - 1
                };
            }
            PickerKey::Next => {
                selected_index = (selected_index + 1) % THEME_OPTIONS.len();
            }
            PickerKey::Confirm => {
                settings.fetch_theme = THEME_OPTIONS[selected_index];
                save_settings(settings)?;
                break Ok(true);
            }
            PickerKey::Cancel => break Ok(false),
            PickerKey::Ignore => {}
        }
    }
}

fn settings_file_path() -> Result<PathBuf> {
    Ok(storage::app_config_dir()?.join("settings.json"))
}

fn theme_index(theme: FetchTheme) -> usize {
    THEME_OPTIONS
        .iter()
        .position(|option| *option == theme)
        .unwrap_or(0)
}

fn render_color_picker(stdout: &mut impl Write, selected_theme: FetchTheme) -> Result<()> {
    let mut screen = String::new();
    screen.push_str("\x1b[2J\x1b[H");
    push_line(&mut screen, "Choose Theme Color");
    push_line(&mut screen, "==================");
    push_line(&mut screen, "");

    for line in picker_logo(selected_theme) {
        push_line(&mut screen, &line);
    }

    push_line(&mut screen, "");
    push_line(
        &mut screen,
        &format!(
            "{} {}",
            paint(
                "Selected Color:",
                ColorStyle {
                    fg: fetch_palette(selected_theme).label,
                    bold: true,
                }
            ),
            paint(
                selected_theme.as_str(),
                ColorStyle {
                    fg: fetch_palette(selected_theme).title,
                    bold: true,
                }
            )
        ),
    );
    push_line(&mut screen, "");
    push_line(&mut screen, "Use Left/Right arrows to change the color.");
    push_line(&mut screen, "Press Enter to save or Esc to cancel.");
    push_line(&mut screen, "");
    push_line(&mut screen, &picker_options_row(selected_theme));

    stdout
        .write_all(screen.as_bytes())
        .context("failed to draw color picker")?;
    stdout.flush().context("failed to draw color picker")
}

fn push_line(buffer: &mut String, line: &str) {
    buffer.push_str(line);
    buffer.push_str("\r\n");
}

fn picker_logo(theme: FetchTheme) -> Vec<String> {
    output::fetch_logo_preview(14, theme)
}

fn picker_options_row(selected_theme: FetchTheme) -> String {
    THEME_OPTIONS
        .iter()
        .map(|theme| {
            let palette = fetch_palette(*theme);
            let label = format!(" {} ", theme.as_str());
            if *theme == selected_theme {
                format!(
                    "{}{}{}",
                    ansi_bg(palette.title),
                    paint(&label, ColorStyle { fg: 16, bold: true }),
                    ansi_reset()
                )
            } else {
                paint(
                    &label,
                    ColorStyle {
                        fg: palette.label,
                        bold: false,
                    },
                )
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[derive(Clone, Copy)]
struct FetchPalette {
    title: u8,
    label: u8,
}

fn fetch_palette(theme: FetchTheme) -> FetchPalette {
    match theme {
        FetchTheme::Red => FetchPalette {
            title: 203,
            label: 210,
        },
        FetchTheme::Blue => FetchPalette {
            title: 39,
            label: 45,
        },
        FetchTheme::Green => FetchPalette {
            title: 40,
            label: 48,
        },
        FetchTheme::Yellow => FetchPalette {
            title: 220,
            label: 228,
        },
        FetchTheme::Pink => FetchPalette {
            title: 176,
            label: 213,
        },
        FetchTheme::Cyan => FetchPalette {
            title: 44,
            label: 51,
        },
        FetchTheme::Noir => FetchPalette {
            title: 252,
            label: 248,
        },
    }
}

#[derive(Clone, Copy)]
struct ColorStyle {
    fg: u8,
    bold: bool,
}

fn paint(text: &str, style: ColorStyle) -> String {
    format!("{}{}{}", ansi(style.fg, style.bold), text, ansi_reset())
}

fn ansi(code: u8, bold: bool) -> String {
    if bold {
        format!("\x1b[1;38;5;{code}m")
    } else {
        format!("\x1b[38;5;{code}m")
    }
}

fn ansi_bg(code: u8) -> String {
    format!("\x1b[48;5;{code}m")
}

fn ansi_reset() -> &'static str {
    "\x1b[0m"
}

enum PickerKey {
    Previous,
    Next,
    Confirm,
    Cancel,
    Ignore,
}

fn read_picker_key(fd: i32) -> Result<PickerKey> {
    loop {
        let Some(byte) = read_byte(fd)? else {
            continue;
        };

        return Ok(match byte {
            b'\r' | b'\n' => PickerKey::Confirm,
            b'q' | b'Q' | 3 => PickerKey::Cancel,
            b'\x1b' => match read_byte(fd)? {
                Some(b'[') => match read_byte(fd)? {
                    Some(b'D') | Some(b'A') => PickerKey::Previous,
                    Some(b'C') | Some(b'B') => PickerKey::Next,
                    _ => PickerKey::Cancel,
                },
                _ => PickerKey::Cancel,
            },
            _ => PickerKey::Ignore,
        });
    }
}

fn read_byte(fd: i32) -> Result<Option<u8>> {
    let mut byte = [0_u8; 1];
    let read_result = unsafe { libc::read(fd, byte.as_mut_ptr().cast(), 1) };
    if read_result < 0 {
        return Err(io::Error::last_os_error()).context("failed to read keyboard input");
    }
    if read_result == 0 {
        return Ok(None);
    }
    Ok(Some(byte[0]))
}

struct RawTerminalGuard {
    fd: i32,
    original: termios,
}

impl RawTerminalGuard {
    fn new(fd: i32, stdout: &mut impl Write) -> Result<Self> {
        let mut original = unsafe { std::mem::zeroed::<termios>() };
        let get_result = unsafe { tcgetattr(fd, &mut original) };
        if get_result != 0 {
            return Err(io::Error::last_os_error()).context("failed to read terminal state");
        }

        let mut raw = original;
        unsafe {
            cfmakeraw(&mut raw);
        }
        raw.c_cc[VMIN] = 0;
        raw.c_cc[VTIME] = 1;

        let set_result = unsafe { tcsetattr(fd, TCSANOW, &raw) };
        if set_result != 0 {
            return Err(io::Error::last_os_error()).context("failed to enable raw terminal mode");
        }

        write!(stdout, "\x1b[?1049h\x1b[?25l")?;
        stdout.flush()?;

        Ok(Self { fd, original })
    }
}

impl Drop for RawTerminalGuard {
    fn drop(&mut self) {
        unsafe {
            tcsetattr(self.fd, TCSANOW, &self.original);
        }
        print!("\x1b[?25h\x1b[?1049l");
        let _ = io::stdout().flush();
    }
}
