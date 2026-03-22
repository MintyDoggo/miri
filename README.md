# Miri

An extension to the [niri window manager](https://github.com/niri-wm/niri) to allow for Master layout and other modes, similar to hyprland or mangowc.

https://github.com/user-attachments/assets/b415a86f-6775-40c6-8370-d418a5cf905b

**Supported Layout Modes**
- [x] Master *(in progress)*
- [ ] Grid
- [ ] Hybrid
- [ ] Drag and Pan
- ~~Dwindle~~ *(not planned)*

Other than adding more layout modes, I'd like to make a DMS plugin for the bar which shows the current mode of the current workspace on that output
> [!WARNING]
> This project is in development. **There will be bugs**, so if you find anything, please feel free to report it or submit a pull request. Master layout mode is currently in a usable state but there are certain edge cases which need to be ironed out

## Install
Run the install script from the latest release and follow the instructions
```sh
curl -fsSL https://github.com/MintyDoggo/miri/releases/latest/download/install.sh \
  -o /tmp/miri-install.sh \
  && sh /tmp/miri-install.sh \
  && rm /tmp/miri-install.sh
```
Once installed, the `miri` command will be available. Be sure `~/.local/bin` is in your `PATH`:

## Keybinds setup
All miri actions can be spawned via `miri action <action-name>`. You can list all available actions by running `miri action`. To add an action to a keybind, edit your niri config and put the spawn command for the keybind you want
Example:
`Mod+M { spawn "miri" "action" "cycle-focused-workspace-mode"; }`

## Overrides setup
Now it's time to setup the overrides!
> [!IMPORTANT]
> If you intend to use miri with the systemd user service (recommended) please follow these steps to ensure a consistent experience in other layout modes. For scroll mode (or if the miri service is not running), these overrides behave identical to vanilla niri. If you only want to use the miri cli for actions, you can skip this section entirely

1. Go to your niri config `~/.config/niri/config.kdl`
2. Add or replace these keybinds with their equivalent miri override:
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

They function identical, but require using `spawn`. Here is an example of my niri configuration with miri overrides:
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
Mod+Shift+3 { spawn "miri" "override" "move-column-to-workspace" "3"; }           // { move-column-to-workspace 3; }
Mod+Shift+4 { spawn "miri" "override" "move-column-to-workspace" "4"; }           // { move-column-to-workspace 4; }
Mod+Shift+5 { spawn "miri" "override" "move-column-to-workspace" "5"; }           // { move-column-to-workspace 5; }
Mod+Shift+6 { spawn "miri" "override" "move-column-to-workspace" "6"; }           // { move-column-to-workspace 6; }
Mod+Shift+7 { spawn "miri" "override" "move-column-to-workspace" "7"; }           // { move-column-to-workspace 7; }
Mod+Shift+8 { spawn "miri" "override" "move-column-to-workspace" "8"; }           // { move-column-to-workspace 8; }
Mod+Shift+9 { spawn "miri" "override" "move-column-to-workspace" "9"; }           // { move-column-to-workspace 9; }
Mod+Ctrl+I { spawn "miri" "override" "move-column-to-workspace-up"; }             // { move-column-to-workspace-up; }
Mod+Ctrl+U { spawn "miri" "override" "move-column-to-workspace-down"; }           // { move-column-to-workspace-down; }
Mod+Ctrl+WheelScrollDown cooldown-ms=150 { spawn "miri" "override" "move-column-to-workspace-down"; } // { move-column-to-workspace-down; }
Mod+Ctrl+WheelScrollUp cooldown-ms=150 { spawn "miri" "override" "move-column-to-workspace-up"; }     // { move-column-to-workspace-up; }
```
Yours may look different so please be sure to check for all cases where the listed actions are used

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
║⠿⠿⠯⠥      ║╚════════╝    ║>_ miri  ║║>_       ║
║⠿⠿⠶⠶⠶⠶⠶⠦⠤ ║╔═════-□×╗    ╚═════════╝╚═════════╝
║⠿⠿⠿⠯⠭⠭⠉⠉  ║╚════════╝    ╔══════-□×╗╔══════-□×╗
║>_ miri   ║╔═════-□×╗    ║>_       ║║>_       ║
╚══════════╝╚════════╝    ╚═════════╝╚═════════╝
```
