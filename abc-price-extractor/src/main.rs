mod cli;
mod parser;

use std::path::PathBuf;

use crate::{cli::Cli, parser::skus_from_214};
use abc_uiautomation::{ensure_abc, reports::generate_report_214, wait};
use clap::Parser;

/// Waits for and retrieves tabular output (TabOutput.tsv) from the specified ABC directory.
///
/// This function repeatedly attempts to read the TabOutput.tsv file within the provided ABC directory.
/// It waits for a maximum of 5 minutes (600 iterations * 500 milliseconds) to retrieve the file.
///
/// # Arguments
///
/// * `abc_dir` - A reference to a `PathBuf` representing the directory where TabOutput.tsv is expected.
///
/// # Returns
///
/// A vector of bytes (`Vec<u8>`) representing the content of TabOutput.tsv.
///
/// # Panics
///
/// This function will panic if it fails to find or read the TabOutput.tsv file within the specified time.
///
/// # Example
///
/// ```
/// use std::path::PathBuf;
///
/// let abc_directory = PathBuf::from("/path/to/abc_directory");
/// let tab_output = your_crate::await_tab_output(&abc_directory);
/// // Use the obtained `tab_output` Vec<u8> for further processing
/// // ...
/// ```
fn await_tab_output(abc_dir: &PathBuf) -> Vec<u8> {
    let mut taboutput: Vec<u8> = Vec::new();
    for _ in 0..600 {
        match std::fs::read(abc_dir.join("TabOutput.tsv")) {
            Ok(_) => {
                wait(5000);
                taboutput = match std::fs::read(abc_dir.join("TabOutput.tsv")) {
                    Ok(o) => o,
                    Err(_) => continue,
                };
                break;
            }
            Err(_) => wait(500),
        }
    }
    if taboutput == Vec::<u8>::new() {
        println!("Could not find TabOutput.tsv");
        ::std::process::exit(1);
    }

    taboutput
}

/// Initializes the directory for storing ABC files within the user's document directory.
///
/// This function attempts to retrieve the user's document directory and creates a directory named
/// "My ABC Files" within it to store ABC-related files.
///
/// # Returns
///
/// A `PathBuf` representing the path to the "My ABC Files" directory.
///
/// # Panics
///
/// This function will panic if it encounters an error while retrieving user directories or
/// document directory.
///
/// # Example
///
/// ```
/// use std::path::PathBuf;
///
/// let abc_dir = your_crate::init_abc_dir();
/// // Use the obtained PathBuf `abc_dir` to manage ABC-related files
/// // ...
/// ```
fn init_abc_dir() -> PathBuf {
    let user_dirs = directories::UserDirs::new().unwrap_or_else(|| {
        println!("Could not read user directories");
        ::std::process::exit(1)
    });
    let docs_dir = user_dirs.document_dir().unwrap_or_else(|| {
        println!("Could not read document directory");
        ::std::process::exit(1)
    });
    docs_dir.join("My ABC Files")
}

/// Retrieves the CLI input for beginning and ending bill numbers.
///
/// This function parses the command-line arguments using `Cli::parse()` and prompts the user to
/// enter the starting and ending bill numbers if they are not provided via command-line arguments.
///
/// # Returns
///
/// A tuple containing:
/// - `Cli`: An instance of the parsed CLI arguments.
/// - `u64`: The beginning bill number.
/// - `u64`: The ending bill number.
///
/// # Panics
///
/// This function will panic if it encounters an error while reading user input for bill numbers.
///
/// # Example
///
/// ```
/// use your_crate::Cli;
///
/// let (cli, begin_bill, end_bill) = your_crate::get_bills();
/// // Use obtained `cli`, `begin_bill`, and `end_bill` for further processing
/// // ...
/// ```
fn get_bills() -> (Cli, u64, u64) {
    let cli = Cli::parse();
    let begin_bill = match cli.begin_bill {
        Some(i) => i,
        None => cli
            .get_bill_stdin("Enter starting bill: ")
            .unwrap_or_else(|e| {
                println!("Could not read starting bill with error: {:?}", e);
                ::std::process::exit(1);
            }),
    };
    let end_bill = match cli.end_bill {
        Some(i) => i,
        None => cli
            .get_bill_stdin("Enter ending bill: ")
            .unwrap_or_else(|e| {
                println!("Could not read ending bill with error: {:?}", e);
                ::std::process::exit(1);
            }),
    };

    (cli, begin_bill, end_bill)
}

fn main() {
    let abc_dir = init_abc_dir();
    let (cli, begin_bill, end_bill) = get_bills();

    println!("Ensure ABC Client4");
    let abc_win = match ensure_abc() {
        Ok(a) => a,
        Err(e) => {
            println!("ABC window failed to load with error {:?}", e);
            ::std::process::exit(1)
        }
    };

    // Create a list of skus to try to fix. This list should be sorted
    let mut skus = match &cli.load_skus {
        // The user provided a file to load the indexed invoices from, so attempt to generate it
        // from the filesystem
        Some(in_path) => Cli::load_skus_from_file(&in_path).unwrap_or_else(|e| {
            println!("Failed to load sorted_skus file with error {:?}", e);
            ::std::process::exit(1);
        }),
        // No command line arg was specified, so generate the index map by parsing a 3-11 report
        // from ABC
        None => {
            // Remove TabOutput.tsv so its presence can be detected later
            println!("Deleting TabOutput.tsv");
            std::fs::remove_file(abc_dir.join("TabOutput.tsv")).unwrap_or_else(|_| {
                println!("TabOutput.tsv file already deleted");
            });

            println!("Generating report 2-14");
            if let Err(e) = generate_report_214(&abc_win, begin_bill, end_bill) {
                println!("Failed to generate 214 report with error {:?}", e);
                ::std::process::exit(1)
            }

            println!("Waiting for TabOutput.tsv");
            let taboutput = await_tab_output(&abc_dir);

            println!("Parsing skus from TabOutput.tsv");
            let skus = match skus_from_214(taboutput.as_slice()) {
                Ok(c) => c,
                Err(e) => {
                    println!(
                        "Encountered `csv::Error` while parsing TabOutput.tsv: {:?}",
                        e
                    );
                    ::std::process::exit(1);
                }
            };

            skus
        }
    };

    skus.sort();
    println!("{:#?}", skus);
}
