use doido_generators::default_registry;

pub fn run_generate(generator: &str, args: &[&str]) {
    let registry = default_registry();
    match registry.run(generator, args) {
        Ok(files) => {
            for file in &files {
                println!("  create  {}", file.path);
            }
            if files.is_empty() {
                println!("  (no files generated)");
            }
        }
        Err(e) => {
            eprintln!("Error: {e}");
            std::process::exit(1);
        }
    }
}
