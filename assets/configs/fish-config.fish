# ~/.config/fish/conf.d/8sync.fish — managed by 8sync setup
# Aliases & shell helpers. Safe to edit; setup never overwrites without backup.

alias ls "eza --icons"
alias ll "eza -lah --icons"
alias la "eza -lah --icons"
alias lt "eza --tree --icons --level=2"
alias cat "bat --paging=never"
alias lg "lazygit"
alias v  "hx"
alias g  "git"

# zoxide
if type -q zoxide
    zoxide init fish | source
end

# fnm (node manager) — autoload if present
if test -d ~/.local/share/fnm
    set -gx PATH ~/.local/share/fnm $PATH
    fnm env --use-on-cd | source 2>/dev/null
end

# bun
if test -d ~/.bun
    set -gx BUN_INSTALL ~/.bun
    set -gx PATH $BUN_INSTALL/bin $PATH
end

# uv local bin
if test -d ~/.local/bin
    set -gx PATH ~/.local/bin $PATH
end

# 8sync shortcuts
alias .. "cd .."
alias ... "cd ../.."
alias 8s "8sync"

# Show short cheatsheet when starting interactive shell (one-time per terminal)
if status is-interactive
    if not set -q __8SYNC_GREETED
        set -gx __8SYNC_GREETED 1
        echo "8sync — type `8sync` for verbs, `8sync setup` first time."
    end
end
