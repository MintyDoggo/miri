# miri (modal niri)

A [niri](https://github.com/niri-wm/niri) extension adding optional tiling layouts, such as **Master Stack**, similar to [hyprland](https://hypr.land/) or [mangowm](https://mangowm.github.io/)

https://github.com/user-attachments/assets/c31b51f0-7328-478a-9061-3b0b2978ae61

**Supported Layout Modes**
| Mode         | Status        |
| ------------ | ------------- |
| Master       | 🟡 In Progress (usable)|
| Grid         | 📋 Planned     |
| Hybrid       | 📋 Planned     |
| Drag and Pan | 📋 Planned     |
| Dwindle      | ❌ Not Planned |

Also planned: a DMS bar plugin to display the current workspace layout mode
> [!WARNING]
> This project is in development. **There will be bugs**! Master layout is usable, but rough edges remain. Issues and PRs welcome!

## Installation
Run the install script from the latest release and follow the instructions
```sh
curl -fsSL https://github.com/MintyDoggo/miri/releases/latest/download/install.sh \
  -o /tmp/miri-install.sh \
  && sh /tmp/miri-install.sh \
  && rm /tmp/miri-install.sh
```
Once installed, the `miri` command will be available. Be sure `~/.local/bin` is in your `PATH`

> [!NOTE]
> The installer script also has an uninstall option, so feel free to try it out commitment free!

## Keybinds setup
All miri actions can be spawned via `miri action <action-name>`. You can list all available actions by running `miri action`. To add an action to a keybind, edit your niri config and put the spawn command for the keybind you want
**Example:**
```js
Mod+S { spawn "miri" "action" "set-focused-workspace-mode" "scroll"; }
Mod+M { spawn "miri" "action" "set-focused-workspace-mode" "master"; }
Mod+C { spawn "miri" "action" "cycle-focused-workspace-mode"; }
```
## Overrides setup
> [!IMPORTANT]
> Skip this section if you only use the miri CLI. Required if you're using the miri service

Checking the current list of overrides is the same as is for actions: `miri override` will display all of them

In your niri config (`~/.config/niri/config.kdl`), **add or replace** these keybinds with their equivalent miri override:
- `move-column-left`
- `move-column-right`
- `move-column-to-first`
- `move-column-to-last`
- `move-column-to-monitor-up`
- `move-column-to-monitor-down`
- `move-column-to-monitor-left`
- `move-column-to-monitor-right`
- `move-column-to-workspace-up`
- `move-column-to-workspace-down`
- `move-column-to-workspace`

**Example** of my niri configuration with miri overrides:
```js
Mod+Shift+Left { spawn "miri" "override" "move-column-left"; }                    // { move-column-left; }
Mod+Shift+Right { spawn "miri" "override" "move-column-right"; }                  // { move-column-right; }
Mod+Shift+Home { spawn "miri" "override" "move-column-to-first"; }                // { move-column-to-first; }
Mod+Shift+End { spawn "miri" "override" "move-column-to-last"; }                  // { move-column-to-last; }
Mod+Ctrl+Shift+WheelScrollUp { spawn "miri" "override" "move-column-left"; }      // { move-column-left; }
Mod+Ctrl+Shift+WheelScrollDown { spawn "miri" "override" "move-column-right"; }   // { move-column-right; }
Mod+Ctrl+WheelScrollLeft { spawn "miri" "override" "move-column-left"; }          // { move-column-left; }
Mod+Ctrl+WheelScrollRight { spawn "miri" "override" "move-column-right"; }        // { move-column-right; }
Mod+Shift+Ctrl+Up { spawn "miri" "override" "move-column-to-monitor-up"; }        // { move-column-to-monitor-up; }
Mod+Shift+Ctrl+Down { spawn "miri" "override" "move-column-to-monitor-down"; }    // { move-column-to-monitor-down; }
Mod+Shift+Ctrl+Left { spawn "miri" "override" "move-column-to-monitor-left"; }    // { move-column-to-monitor-left; }
Mod+Shift+Ctrl+Right { spawn "miri" "override" "move-column-to-monitor-right"; }  // { move-column-to-monitor-right; }
Mod+Shift+1 { spawn "miri" "override" "move-column-to-workspace" "1"; }           // { move-column-to-workspace 1; }
Mod+Shift+2 { spawn "miri" "override" "move-column-to-workspace" "2"; }           // { move-column-to-workspace 2; }
// ... etc                                                                        // { move-column-to-workspace <index>; }
Mod+Ctrl+I { spawn "miri" "override" "move-column-to-workspace-up"; }             // { move-column-to-workspace-up; }
Mod+Ctrl+U { spawn "miri" "override" "move-column-to-workspace-down"; }           // { move-column-to-workspace-down; }
Mod+Ctrl+WheelScrollDown cooldown-ms=150 { spawn "miri" "override" "move-column-to-workspace-down"; } // { move-column-to-workspace-down; }
Mod+Ctrl+WheelScrollUp cooldown-ms=150 { spawn "miri" "override" "move-column-to-workspace-up"; }     // { move-column-to-workspace-up; }
```
> [!NOTE]
> In the case that the miri service is not running or something goes wrong, miri will fallback on executing the niri action normally. This ensures your config always works as expected as long as miri is installed

Current version supported is `v25.11`. Will likely work on `niri-git` but it is currently untested

You're now all setup! Continue reading for further configuration

## Configuration
Miri can be configured though `~/.config/miri/config.toml`. These options are not finalized and will likely change, but if you are interested, here is the current configuration list:

```toml
[global]
# Default mode for new workspaces: "master" or "scroll"
default_workspace_mode = "master"

# Settings for the master layout mode
[master]
# Width of the master column (0-100)
column_width_percentage = 50.0
# If true, single windows take full width
maximize_single_window = true

# Settings for the scroll layout mode
[scroll]
# If true, focus stays on the current window when new windows open
maintain_focus_on_new_window = false
```

## Misc
ASCII art for installer
```
╔═══════-□×╗╔═════-□×╗    ╔══════-□×╗╔══════-□×╗
║          ║╚════════╝    ║>_ miri  ║║>_       ║
║          ║╔═════-□×╗    ╚═════════╝╚═════════╝
║          ║╚════════╝    ╔══════-□×╗╔══════-□×╗
║>_ miri   ║╔═════-□×╗    ║>_       ║║>_       ║
╚══════════╝╚════════╝    ╚═════════╝╚═════════╝
```
