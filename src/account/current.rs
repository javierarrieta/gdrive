use crate::config;
use crate::config::Config;

pub fn current() -> Result<(), Error> {
    let accounts = Config::list_accounts().map_err(Error::Config)?;

    if accounts.is_empty() {
        println!("No accounts found");
        println!("Use `gdrive account add` to add an account.");
    } else if !Config::has_current_account() {
        println!("No account has been selected");
        println!("Use `gdrive account list` to show all accounts.");
        println!("Use `gdrive account switch` to select an account.");
    } else {
        let config = Config::load_current_account().map_err(Error::Config)?;
        println!("{}", config.account.name);
    }

    Ok(())
}

#[derive(Debug)]
pub enum Error {
    Config(config::Error),
}
