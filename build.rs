use std::io;

fn main() -> io::Result<()> {
    if cfg!(target_os = "windows") {
        let mut res = winres::WindowsResource::new();
        res.set_icon("icons/hypixel-bazaar-tui.ico");
        res.compile()?;
    }
    Ok(())
}

