use anyhow::Context;

pub mod albumsongs;
pub mod artistsearch;

fn get_adjusted_list_column(target_col: usize, adjusted_cols: &[usize]) -> anyhow::Result<usize> {
    adjusted_cols
        .get(target_col)
        .with_context(|| {
            format!(
                "Unable to sort column, doesn't match up with underlying list. {}",
                target_col,
            )
        })
        .copied()
}
