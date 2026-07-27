Output showcase/output/ani-cli-rs-showcase.mp4
Output docs/assets/ani-cli-rs-showcase.gif

Set Shell "bash"
Set FontFamily "Cascadia Mono"
Set FontSize 22
Set Width 1280
Set Height 720
Set Padding 24
Set Margin 30
Set MarginFill "#ff5b3d"
Set BorderRadius 14
Set WindowBar ColorfulRight
Set Framerate 30
Set TypingSpeed 24ms
Set CursorBlink false
Set LoopOffset 2%
Set Theme { "name": "AniPlay", "black": "#17131e", "red": "#ff5b3d", "green": "#7bd88f", "yellow": "#f6c177", "blue": "#78a9ff", "magenta": "#c4a7e7", "cyan": "#7dcfff", "white": "#f4eff7", "brightBlack": "#62586b", "brightRed": "#ff8068", "brightGreen": "#9ce7aa", "brightYellow": "#f9dc9a", "brightBlue": "#a4c5ff", "brightMagenta": "#dcc5f5", "brightCyan": "#a8e5ff", "brightWhite": "#ffffff", "background": "#0d0a10", "foreground": "#f4eff7", "selection": "#553246", "cursor": "#ff5b3d" }

Env NO_COLOR "1"
Env RUST_LOG "off"
Env ANI_CLI_PROVIDER "anikoto"

Ctrl+L

Type "./target/debug/ani-cli-rs --version"
Sleep 400ms
Enter
Wait+Screen@10s /ani-cli-rs/
Sleep 3s

Type "./target/debug/ani-cli-rs --demo-mode --provider anikoto search starfall"
Sleep 500ms
Enter
Wait+Screen@10s /Starfall Atelier/
Sleep 5s
Screenshot showcase/output/screenshots/01-search.png

Ctrl+L
Type "./target/debug/ani-cli-rs --demo-mode --provider anikoto2 search signal --json"
Sleep 500ms
Enter
Wait+Screen@10s /Signal Bloom/
Sleep 6s
Screenshot showcase/output/screenshots/02-provider-json.png

Ctrl+L
Type "./target/debug/ani-cli-rs --demo-mode episodes anikoto:showcase-starfall-atelier --json"
Sleep 500ms
Enter
Wait+Screen@10s /\"12\"/
Sleep 5s
Screenshot showcase/output/screenshots/03-episodes.png

Ctrl+L
Type "./target/debug/ani-cli-rs --demo-mode --provider anikoto2 links anikoto2:showcase-starfall-atelier 1 --quality 720p --json"
Sleep 500ms
Enter
Wait+Screen@10s /\"resolution\": \"720p\"/
Sleep 7s
Screenshot showcase/output/screenshots/04-streams.png

Ctrl+L
Type "./target/debug/ani-cli-rs --help"
Sleep 500ms
Enter
Wait+Screen@10s /anikoto2/
Sleep 7s
Screenshot showcase/output/screenshots/05-help.png

Ctrl+L
Type "echo 'Search. Resolve. Play. Download.'"
Sleep 500ms
Enter
Wait+Screen@10s /Search. Resolve/
Type "echo 'Anikoto API + Anikoto.cz, from one native Rust CLI.'"
Sleep 400ms
Enter
Wait+Screen@10s /native Rust CLI/
Sleep 9s
