use clap::ValueEnum;
use serde::{Deserialize, Serialize};
use strum::EnumIter;

#[derive(ValueEnum, EnumIter, Serialize, Deserialize, Eq, PartialEq, Hash, Clone, Copy, Debug)]
#[value(rename_all = "kebab-case")]
pub enum ReviewSurface {
    Terminal,
    Iterm2,
    Kitty,
    Wezterm,
    Tmux,
    Zellij,
    Zsh,
    Vim,
    Neovim,
    Emacs,
    VsCode,
    Browser,
    Desktop,
}
