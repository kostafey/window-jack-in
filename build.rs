// Embeds the application icon as a Win32 resource so Explorer, the desktop,
// the taskbar and Alt-Tab show it. The tray loads the same resource at
// runtime (see tray::load_embedded_icon), so the icon ships once.
//
// Needs a resource compiler: `windres` (mingw-w64 binutils) on the GNU target,
// or `rc.exe` / `llvm-rc` on MSVC.
fn main() {
    println!("cargo:rerun-if-changed=src/window-jack-in.ico");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let mut res = winresource::WindowsResource::new();
        res.set_icon_with_id("src/window-jack-in.ico", "1");
        res.compile().expect("failed to embed Windows resource icon");
    }
}
