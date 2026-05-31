# Themes

← [Back](README.md)

sdrtop has six built-in color themes. Switch with `--theme <name>` on startup, or set it in your config file.

---

## Available themes

| Name | Description |
|------|-------------|
| `sdr` | Default — dark background, cyan and green accents |
| `nord` | Cool blue-grey palette, easy on the eyes |
| `dracula` | Purple and pink on dark background |
| `gruvbox` | Warm brown and yellow tones |
| `catppuccin` | Soft pastel colors on dark background |
| `solarized` | Classic Solarized dark scheme |

---

## Switching theme

**At startup:**
```sh
sdrtop --theme gruvbox
```

**In your config file** (takes effect next launch):
```toml
[theme]
base = "gruvbox"
```

---

## Custom colors

You can override individual colors in the config file without touching the rest of the theme. The field names map to specific UI elements.

```toml
[theme]
base = "nord"
border_accent = "#88c0d0"
value_hi      = "#ebcb8b"
```

Any field you leave out keeps its default from the base theme.
