use std::{
    collections::HashMap,
    error::Error,
    io::{self, Split, Write},
    path::PathBuf,
};

use clap::{builder::Str, Parser};

/// Controls a client of Advanced Business Computers to automatically generate a list of unpaid
/// bills that are associated with customers who have a John Deere Financial account
#[derive(Parser)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    /// The first bill in the sequence to send to John Deere Financial
    #[arg(short, long)]
    pub begin_bill: Option<u64>,

    /// The last bill in the sequence to send to John Deere Financial
    #[arg(short, long)]
    pub end_bill: Option<u64>,

    /// Path to the file containing the sorted list of skus to fix pricing for
    #[arg(short = 's', long)]
    pub load_skus: Option<PathBuf>,
}

/*--- Method Definitions for `Cli` ---*/
impl Cli {
    /// Reads and retrieves an bill number entered via standard input (stdin).
    ///
    /// This function prompts the user for an bill number and reads the input from the console.
    /// It validates the input and expects a valid integer. If an invalid input is provided, it prompts
    /// the user to enter a valid integer and continues until a valid integer is entered.
    ///
    /// # Arguments
    ///
    /// * `prompt` - A string slice representing the prompt message to ask for the bill number.
    ///
    /// # Returns
    ///
    /// A Result containing the parsed u64 bill number if successful, otherwise an io::Error.
    ///
    /// # Errors
    ///
    /// Returns an `io::Error` if there is an issue reading from stdin or flushing stdout.
    ///
    /// # Example
    ///
    /// ```
    /// use std::io;
    ///
    /// let cli = your_crate::Cli::new(); // Assume Cli is an instance of your struct
    /// match cli.get_bill_stdin("Enter bill number: ") {
    ///     Ok(bill_number) => {
    ///         // Use the obtained `bill_number` u64 for further processing
    ///         // ...
    ///     }
    ///     Err(e) => {
    ///         // Handle the io::Error
    ///         // ...
    ///     }
    /// }
    /// ```
    pub fn get_bill_stdin(&self, prompt: &str) -> Result<u64, io::Error> {
        loop {
            let mut bill_buf = String::new();
            print!("{}", prompt);
            io::stdout().flush()?;
            io::stdin().read_line(&mut bill_buf)?;
            let trimmed_buf = bill_buf.trim();
            let bill: u64 = match trimmed_buf.parse() {
                Ok(i) => i,
                Err(_) => {
                    println!("Please enter a valid integer");
                    continue;
                }
            };
            return Ok(bill);
        }
    }
}

impl Cli {
    pub fn load_skus_from_file(in_path: &PathBuf) -> Result<Vec<String>, io::Error> {
        let file_text = std::fs::read_to_string(in_path)?;
        let skus: Vec<String> = file_text.split('\n').map(|sku| sku.to_string()).collect();
        Ok(skus)
    }
}
