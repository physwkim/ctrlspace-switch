# ctrlspace-switch

A macOS background agent that intercepts Ctrl+Space with its own CGEventTap and
toggles between English and Korean input sources directly.

The system "Select the previous input source" shortcut has a race: releasing
both keys together makes it revert the switch 100–500 ms later
([FB24297598](https://developer.apple.com/forums/thread/841799)). This tool
never goes through the system hotkey path, so the race cannot occur.

## Build

```sh
cargo build --release
```

## Check input sources

```sh
./target/release/ctrlspace-switch --list
```

The default pair is `com.apple.keylayout.ABC` ↔
`com.apple.inputmethod.Korean.2SetKorean`. Use `--english <ID>` /
`--korean <ID>` to pick others.

## Run directly (for testing)

```sh
./target/release/ctrlspace-switch
```

Your terminal app needs Accessibility permission. Each switch and its latency
is printed to stderr. Stop with `Ctrl+C`.

## Install (start at login)

1. Copy the binary

   ```sh
   mkdir -p ~/.local/bin
   cp target/release/ctrlspace-switch ~/.local/bin/
   ```

2. Register the LaunchAgent

   Adjust the paths in `ctrlspace-switch.plist` (`/Users/stevek/...`) to your
   home directory, then:

   ```sh
   cp ctrlspace-switch.plist ~/Library/LaunchAgents/
   launchctl bootstrap gui/$(id -u) ~/Library/LaunchAgents/ctrlspace-switch.plist
   ```

3. Grant Accessibility permission

   The first launch shows a permission prompt. Enable `ctrlspace-switch` under
   System Settings → Privacy & Security → Accessibility, then restart it:

   ```sh
   launchctl kickstart -k gui/$(id -u)/local.ctrlspace-switch
   ```

4. Disable the system shortcut

   System Settings → Keyboard → Keyboard Shortcuts → Input Sources →
   turn off "Select the previous input source".

## Logs

```sh
tail -f ~/Library/Logs/ctrlspace-switch.log
```

Each switch logs `from -> to: status, ms`. If an occasional slow switch shows
hundreds of ms here, the input method itself is waking up late, which this
tool does not address.

## Updating

A rebuilt binary has a new signature, so Accessibility permission may be
dropped. After copying, remove and re-add the entry in the Accessibility list,
then restart with `kickstart -k`.

## Uninstall

```sh
launchctl bootout gui/$(id -u)/local.ctrlspace-switch
rm ~/Library/LaunchAgents/ctrlspace-switch.plist ~/.local/bin/ctrlspace-switch
```

Re-enable the system shortcut and remove the entry from the Accessibility list.
