use crate::error::Error;
use crate::Result;

pub mod albumsongs;
pub mod artistsearch;

fn get_adjusted_list_column(target_col: usize, adjusted_cols: &[usize]) -> Result<usize> {
    adjusted_cols
        .get(target_col)
        .ok_or(Error::Other(format!(
            "Unable to sort column, doesn't match up with underlying list. {}",
            target_col,
        )))
        .copied()
}
