use sha2::{Digest, Sha256};
use std::{
    fs::File,
    io::{BufReader, Read},
    path::Path,
};

pub fn sha256_file(path: &Path) -> anyhow::Result<String> {
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);
    let mut hasher = Sha256::new();
    let mut buf = vec![0u8; 64 * 1024];
    loop {
        let n = reader.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    let result = hasher.finalize();
    Ok(hex::encode(result))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DuplicateClass {
    Unchanged,        // same path + same hash
    DuplicateContent, // different path + same hash -> default skip
    Conflict,         // same path + different hash
    New,              // new path + new hash -> copy
}

pub fn classify(
    _cheap_same: Option<bool>,
    same_path: bool,
    same_hash: bool,
    exists: bool,
) -> DuplicateClass {
    if !exists {
        return DuplicateClass::New;
    }
    if same_path && same_hash {
        DuplicateClass::Unchanged
    } else if !same_path && same_hash {
        DuplicateClass::DuplicateContent
    } else if same_path && !same_hash {
        DuplicateClass::Conflict
    } else {
        // different path + different hash but collision on filename? treat as New with different dest?
        // If file exists elsewhere with different hash, it's still New for its own path? But if dest exists with diff hash and we consider path identity, it's Conflict.
        // So this branch is same_path==false && !same_hash -> if dest path doesn't exist it's New; but we already said exists=true means dest there.
        // So if dest path differs from source but dest file exists with different content, it's effectively New at its path but conflict at destination name?
        // For simplicity: if exists and !same_hash -> Conflict (destination collision)
        DuplicateClass::Conflict
    }
}
