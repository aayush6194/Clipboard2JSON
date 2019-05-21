use failure::Error;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::fmt::Debug;
use std::fs::{File, OpenOptions};
use std::io::BufReader;

/// Reads the stored clipboard data, appends the new incoming data, and
/// overwrites the JSON file.
pub fn save_clipboard_to_file<T>(data: T) -> Result<(), Error>
where
    T: DeserializeOwned + Serialize + Debug,
{
    println!("Clipboard change detected!");
    let mut file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open("clipboard.json")?;
    let reader = BufReader::new(&mut file);
    let mut stored_data: Vec<T> = serde_json::from_reader(reader).unwrap_or_default();
    println!("Writing {:#?} to file...", data);
    stored_data.push(data);
    drop(file); // closes the file so we can overwrite it
    let file = File::create("clipboard.json")?;
    serde_json::to_writer(file, &stored_data)?;
    println!("Successfuly wrote to clipboard.json\n");
    Ok(())
}

// TODO: Write a function that can store it to some external API?
