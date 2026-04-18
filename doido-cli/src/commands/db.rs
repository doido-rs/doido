use clap::Subcommand;

#[derive(Subcommand)]
pub enum DbCommand {
    /// Run pending migrations
    Migrate,
    /// Rollback last migration
    Rollback,
    /// Seed the database
    Seed,
    /// Reset the database (drop + migrate + seed)
    Reset,
}

pub fn run(cmd: DbCommand) {
    match cmd {
        DbCommand::Migrate => println!("Running migrations..."),
        DbCommand::Rollback => println!("Rolling back last migration..."),
        DbCommand::Seed => println!("Seeding database..."),
        DbCommand::Reset => println!("Resetting database (drop, migrate, seed)..."),
    }
}
