use anyhow::Result;

use zeta::App;

fn main() -> Result<()> {
    let mut app = App::bootstrap()?;
    app.run()
}
