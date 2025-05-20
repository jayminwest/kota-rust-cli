use std::io::{self, Write};

mod llm;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("KOTA - Type '/quit' to exit.");
    loop {
        println!("You: ");
        io::stdout().flush()?;

        let mut user_input = String::new();
        io::stdin().read_line(&mut user_input)?;

        let trimmed_input = user_input.trim();

        if trimmed_input == "/quit" {
            println!("Exiting KOTA.");
            break;
        }

        if trimmed_input.is_empty() {
            continue;
        }

        match llm::ask_model(trimmed_input).await {
            Ok(response) => {
                println!("KOTA: {}", response);
            }
            Err(e) => {
                println!("Error: {}", e);
            }
        }
    }
    Ok(())
}
