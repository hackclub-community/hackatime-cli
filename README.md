# Hackatime Terminal Viewer

A Rust terminal app that signs into Hackatime with OAuth PKCE and prints a simple stats summary.

## Install

- From crates.io: `cargo install hackatime`
- Then run: `hackatime`

## Commands

- `hackatime` shows the multi-range summary plus lifetime languages
- `hackatime .` shows stats for the current folder if it matches one of your Hackatime projects
- `hackatime --fetch` (`-f`) shows a neofetch-style overview
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
- Can show the current folder as a project with `hackatime .`
- Lets you clear the saved token with `hackatime logout`
- Fetches your profile
- Prints a plain text stats summary and exits
