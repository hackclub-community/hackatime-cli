# Hackatime Terminal Viewer

A Rust terminal app that signs into Hackatime with OAuth PKCE and prints a simple stats summary.

## Install

- From crates.io: `cargo install hackatime`
- Then run: `hackatime`
- Enjoy :)

## Commands

- `hackatime` shows the multi-range summary plus lifetime languages
- `hackatime .` shows stats for the current folder if it matches one of your Hackatime projects
- `hackatime settings` shows the current saved settings
- `hackatime settings color` opens an arrow-key color picker with the fetch `h` preview
- `hackatime settings clear toggle` toggles terminal clearing on or off
- `hackatime settings clear on|off` explicitly enables or disables terminal clearing
- `HACKATIME_CONFIG_DIR=/some/folder hackatime settings color` stores settings in a custom config directory
- `hackatime --lookup <username>` shows another person's public Hackatime stats
- `hackatime --lookup <username> -g` shows another person's graph
- `hackatime --lookup <username> --projects --year` shows another person's top projects for that range
- `hackatime --fetch` (`-f`) shows a neofetch-style overview
- `hackatime --graph` (`-g`) shows a GitHub-style coding heatmap for the last 365 days
- `hackatime --projects` (`-p`) shows your top projects with per-project language graphs and a key
- `hackatime --projects --week|--month|--year` scopes that projects view to a time range
- `hackatime --current` (`-c`) shows the current project report
- `hackatime --today` (`-d`) shows today's total plus today's languages
- `hackatime --week` (`-w`) shows this week's total plus this week's languages
- `hackatime --month` (`-m`) shows this month's total plus this month's languages
- `hackatime --year` (`-y`) shows this year's total plus this year's languages
- `hackatime --lifetime` (`-l`) shows lifetime total plus lifetime languages
- `hackatime logout` clears your saved Hackatime login

## Current features

- Browser-based OAuth login using PKCE
- Reuses your saved access token on later runs so you do not log in each time
- Saves your preferred fetch theme and terminal-clearing behavior
- Lets you change saved settings directly from the command line
- Supports a custom config directory with `HACKATIME_CONFIG_DIR`
- Can show the current folder as a project with `hackatime .`
- Lets you clear the saved token with `hackatime logout`
- Fetches your profile
- Prints a plain text stats summary and exits

## Planned features
