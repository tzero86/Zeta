use anyhow::Result;

use zeta::App;

fn main() -> Result<()> {
    // Clean up any leftover .bak file from an interrupted Windows self-update.
    #[cfg(windows)]
    zeta::cleanup_update_backup();

    let mut app = App::bootstrap()?;
    app.run()
}
