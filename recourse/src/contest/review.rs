//! `recourse contest review <contest-id> --uphold|--reject [--note "…"]`
//!
//! The ONLY code path that moves a contest off `pending`.
//! No --auto flag. No batch flag. Human action only by construction.

use crate::contest::store::ContestStore;
use crate::receipt::store as receipt_store;
use std::path::PathBuf;

pub fn cmd_contest_review(
    contest_id: &str,
    uphold: bool,
    reject: bool,
    note: Option<&str>,
    data_dir: Option<PathBuf>,
) -> Result<(), Box<dyn std::error::Error>> {
    if uphold == reject {
        // Both true or both false — exactly one must be specified
        return Err("specify exactly one of --uphold or --reject".into());
    }

    let data = receipt_store::data_dir(data_dir.as_deref());
    let store = ContestStore::new(&data);

    if uphold {
        store.uphold(contest_id, note)?;
        println!("contest {contest_id} upheld");
    } else {
        store.reject(contest_id, note)?;
        println!("contest {contest_id} rejected");
    }

    Ok(())
}
