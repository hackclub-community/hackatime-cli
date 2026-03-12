# Hackatime Terminal Viewer

A Rust terminal app that signs into Hackatime with OAuth PKCE and prints a simple stats summary.

## Install

- From crates.io: `cargo install hackatime`
- Then run: `hackatime`

## Commands

- `hackatime` shows the multi-range summary plus lifetime languages
- `hackatime --current` shows the current project report
- `hackatime --today` shows today's total plus today's languages
- `hackatime --week` shows this week's total plus this week's languages
- `hackatime --month` shows this month's total plus this month's languages
- `hackatime --year` shows this year's total plus this year's languages
- `hackatime --lifetime` shows lifetime total plus lifetime languages

## Current features

- Browser-based OAuth login using PKCE
- Reuses your saved access token on later runs so you do not log in each time
- Fetches your profile
- Prints a plain text stats summary and exits
