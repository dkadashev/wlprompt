# wlprompt

## About

wlprompt is a tool to start programs under wayland that supports tab completions and history. Think
of it as a shell prompt without having to start a terminal (but also without most of the shell
features). Inspired by [AwesomeWM](https://awesomewm.org/)'s `awful.widget.prompt`.

The reason various dmenu-like tools aren't a viable alternative for me is they often do not really
work well with command args, and when they do they usually do not keep history, and when they do
they do not have tab completion.

## Goals and non-goals

This is a personal project that pursues several goals:

* Scratch my own itch and provide a replacements for `awful.widget.prompt`;
* Keeping it as fast and lean as possible;
* Provide an example of code using extremely poorly documented smithay-client-toolkit for myself and
  hopefully others;
* Play with / gain some experience with Rust;

Everything else is not a goal, so the feature list is likely to be quite minimal.

## Building

LATER: make this more human-friendly, for now this is mainly to catch the system level deps.

1. Install cargo: https://doc.rust-lang.org/cargo/getting-started/installation.html;
2. Install system level dependencies: `xkbcommon-dev`, `libfontconfig-dev`;
3. Run `cargo build`;
