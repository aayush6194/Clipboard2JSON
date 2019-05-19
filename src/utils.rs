use serde::de::DeserializeOwned;
use serde::Serialize;
use std::error::Error;
use std::fs::{File, OpenOptions};
use std::io::BufReader;

pub fn save_clipboard_to_file<T>(data: T) -> Result<(), Box<dyn Error>>
where
    T: DeserializeOwned + Serialize,
{
    let mut file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open("clipboard.json")?;
    let reader = BufReader::new(&mut file);
    let mut stored_data: Vec<T> = serde_json::from_reader(reader).unwrap_or_default();
    stored_data.push(data);
    drop(file);
    let file = File::create("clipboard.json")?;
    serde_json::to_writer_pretty(file, &stored_data)?;
    Ok(())
}
