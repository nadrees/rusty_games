use ash::Entry;
use rusty_games::VkInstanceGuard;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let entry = Entry::linked();
    let instance = VkInstanceGuard::try_new(&entry)?;
    Ok(())
}
